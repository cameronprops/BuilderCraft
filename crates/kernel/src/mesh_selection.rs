//! Revision-bound mesh component selection for CAD viewport and graph callers.
//! Indices are transient handles: a stale selection must be rejected after edits.
use crate::{KernelError, Result, TriangleMesh, mesh_add_triangle};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshSelection {
    /// Revision of the particular mesh object the viewport picked from.
    pub revision: u64,
    /// Indexed mesh vertices, selected in user pick order.
    pub vertices: Vec<u32>,
    /// Undirected, canonical [low,high] existing mesh edges.
    pub edges: Vec<[u32; 2]>,
    /// Existing triangle indices.
    pub faces: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectedTriangleResult {
    pub mesh: TriangleMesh,
    pub revision: u64,
    pub new_face_index: u32,
    /// Selection for the newly created triangle, safe for the returned revision.
    pub selection: MeshSelection,
}

/// Check whether transient handles refer to the current mesh and revision.
/// An edge selection must be a real indexed triangle edge, not just two vertices.
/// Duplicate handles are rejected to avoid accidental repeated edits.
pub fn mesh_validate_selection(mesh: &TriangleMesh, current_revision: u64, selection: &MeshSelection) -> Result<()> {
    if selection.revision != current_revision {
        return Err(KernelError::Conflict { expected: selection.revision, actual: current_revision });
    }
    if mesh.vertices.len() > 1_000_000 || mesh.triangles.len() > 1_000_000 {
        return Err(KernelError::Budget);
    }
    if mesh.triangles.iter().flatten().any(|&index| index as usize >= mesh.vertices.len()) {
        return Err(KernelError::Invalid("mesh triangle index"));
    }
    let mut chosen_vertices = BTreeSet::new();
    for &index in &selection.vertices {
        if index as usize >= mesh.vertices.len() {
            return Err(KernelError::Invalid("selected vertex index"));
        }
        if !chosen_vertices.insert(index) {
            return Err(KernelError::Invalid("duplicate selected vertex"));
        }
    }
    let mut chosen_faces = BTreeSet::new();
    for &index in &selection.faces {
        if index as usize >= mesh.triangles.len() {
            return Err(KernelError::Invalid("selected face index"));
        }
        if !chosen_faces.insert(index) {
            return Err(KernelError::Invalid("duplicate selected face"));
        }
    }
    if !selection.edges.is_empty() {
        let mut existing = BTreeSet::new();
        for &[a, b, c] in &mesh.triangles {
            for (x, y) in [(a, b), (b, c), (c, a)] {
                existing.insert((x.min(y), x.max(y)));
            }
        }
        let mut chosen_edges = BTreeSet::new();
        for &[a, b] in &selection.edges {
            if a >= b || !existing.contains(&(a, b)) {
                return Err(KernelError::Invalid("selected mesh edge"));
            }
            if !chosen_edges.insert((a, b)) {
                return Err(KernelError::Invalid("duplicate selected edge"));
            }
        }
    }
    Ok(())
}

/// Add a face from exactly three picked vertices and return a selection
/// for that new face. Vertex pick order defines winding. A failed edit
/// changes neither the mesh nor the revision. This is a headless command
/// protocol; no viewport picking implementation is implied.
pub fn mesh_add_triangle_from_selection(mesh: &TriangleMesh, current_revision: u64, selection: &MeshSelection) -> Result<SelectedTriangleResult> {
    mesh_validate_selection(mesh, current_revision, selection)?;
    if selection.vertices.len() != 3 || !selection.edges.is_empty() || !selection.faces.is_empty() {
        return Err(KernelError::Invalid("triangle requires exactly three selected vertices"));
    }
    let revision = current_revision.checked_add(1).ok_or(KernelError::Budget)?;
    let new_face_index = u32::try_from(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    let vertices = [selection.vertices[0], selection.vertices[1], selection.vertices[2]];
    let new_mesh = mesh_add_triangle(mesh, vertices)?;
    Ok(SelectedTriangleResult {
        mesh: new_mesh,
        revision,
        new_face_index,
        selection: MeshSelection { revision, vertices: vertices.to_vec(), edges: Vec::new(), faces: vec![new_face_index] },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn sample() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
            triangles: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn validates_real_edges_and_mixed_components() {
        let s = MeshSelection { revision: 7, vertices: vec![0, 3], edges: vec![[0, 1], [1, 2]], faces: vec![0] };
        assert_eq!(mesh_validate_selection(&sample(), 7, &s), Ok(()));
        let invalid = MeshSelection { edges: vec![[0, 3]], ..s };
        assert!(mesh_validate_selection(&sample(), 7, &invalid).is_err());
    }

    #[test]
    fn stale_revision_is_a_conflict() {
        let s = MeshSelection { revision: 4, vertices: vec![0], ..MeshSelection::default() };
        assert_eq!(mesh_validate_selection(&sample(), 5, &s), Err(KernelError::Conflict { expected: 4, actual: 5 }));
    }

    #[test]
    fn rejects_duplicate_and_invalid_handles() {
        let s = MeshSelection { revision: 1, vertices: vec![0, 0], ..MeshSelection::default() };
        assert!(mesh_validate_selection(&sample(), 1, &s).is_err());
        let s = MeshSelection { revision: 1, faces: vec![5], ..MeshSelection::default() };
        assert!(mesh_validate_selection(&sample(), 1, &s).is_err());
        let s = MeshSelection { revision: 1, edges: vec![[1, 0]], ..MeshSelection::default() };
        assert!(mesh_validate_selection(&sample(), 1, &s).is_err());
    }

    #[test]
    fn selected_triangle_returns_new_revision_and_face_selection() {
        let m = sample();
        let old = m.clone();
        let s = MeshSelection { revision: 10, vertices: vec![0, 2, 3], ..MeshSelection::default() };
        let result = mesh_add_triangle_from_selection(&m, 10, &s);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.revision, 11);
            assert_eq!(result.new_face_index, 1);
            assert_eq!(result.mesh.triangles, vec![[0, 1, 2], [0, 2, 3]]);
            assert_eq!(result.selection.faces, vec![1]);
            assert_eq!(result.selection.revision, 11);
            assert!(mesh_validate_selection(&result.mesh, 11, &s).is_err());
        }
        assert_eq!(m, old);
    }

    #[test]
    fn rejects_incorrect_selection_shape_and_revision_overflow() {
        let s = MeshSelection { revision: 3, vertices: vec![0, 2], ..MeshSelection::default() };
        assert!(mesh_add_triangle_from_selection(&sample(), 3, &s).is_err());
        let s = MeshSelection { revision: u64::MAX, vertices: vec![0, 2, 3], ..MeshSelection::default() };
        assert!(mesh_add_triangle_from_selection(&sample(), u64::MAX, &s).is_err());
    }
}
