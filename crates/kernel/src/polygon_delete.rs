//! Non-destructive native polygon face deletion with provenance and revision checks.
//! Face indices are transient; remap selections using the returned old-to-new map.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_validate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonDeleteResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    /// Original face index maps to new face index; removed faces map to None.
    pub old_to_new_faces: Vec<Option<u32>>,
    /// Original selected face indices, sorted and unique.
    pub removed_faces: Vec<u32>,
}

/// Delete explicitly selected native polygon faces without modifying the input.
/// No vertices are removed, preventing unexpected vertex reindexing.
/// Require the caller's picked revision to match the current revision.
pub fn polygon_mesh_delete_faces(
    mesh: &PolygonMesh,
    current_revision: u64,
    selected_revision: u64,
    selected_faces: &[u32],
) -> Result<PolygonDeleteResult> {
    if selected_revision != current_revision {
        return Err(KernelError::Conflict { expected: selected_revision, actual: current_revision });
    }
    polygon_mesh_validate(mesh)?;
    if selected_faces.is_empty() {
        return Err(KernelError::Invalid("no polygon faces selected"));
    }
    if selected_faces.len() > mesh.faces.len() {
        return Err(KernelError::Invalid("too many selected polygon faces"));
    }
    let mut marked = BTreeSet::new();
    for &face in selected_faces {
        if face as usize >= mesh.faces.len() {
            return Err(KernelError::Invalid("selected polygon face index"));
        }
        if !marked.insert(face) {
            return Err(KernelError::Invalid("duplicate selected polygon face"));
        }
    }
    let next_revision = current_revision.checked_add(1).ok_or(KernelError::Budget)?;
    let mut faces: Vec<PolygonFace> = Vec::new();
    faces.try_reserve_exact(mesh.faces.len() - marked.len()).map_err(|_| KernelError::Budget)?;
    let mut old_to_new_faces = Vec::new();
    old_to_new_faces.try_reserve_exact(mesh.faces.len()).map_err(|_| KernelError::Budget)?;
    for (index, &face) in mesh.faces.iter().enumerate() {
        if marked.contains(&(index as u32)) {
            old_to_new_faces.push(None);
        } else {
            let new_id = u32::try_from(faces.len()).map_err(|_| KernelError::Budget)?;
            faces.push(face);
            old_to_new_faces.push(Some(new_id));
        }
    }
    Ok(PolygonDeleteResult {
        mesh: PolygonMesh { vertices: mesh.vertices.clone(), faces },
        revision: next_revision,
        old_to_new_faces,
        removed_faces: marked.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn fixture() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(1., 0., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(0., 1., 0.),
                Vec3::new(2., 0., 0.),
                Vec3::new(2., 1., 0.),
            ],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3]), PolygonFace::Triangle([1, 4, 2]), PolygonFace::Triangle([4, 5, 2])],
        }
    }

    #[test]
    fn removes_selected_face_preserving_quad_and_vertex_indices() {
        let source = fixture();
        let original = source.clone();
        let result = polygon_mesh_delete_faces(&source, 4, 4, &[1]);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.revision, 5);
            assert_eq!(result.removed_faces, vec![1]);
            assert_eq!(result.old_to_new_faces, vec![Some(0), None, Some(1)]);
            assert_eq!(result.mesh.vertices, original.vertices);
            assert_eq!(result.mesh.faces, vec![PolygonFace::Quad([0, 1, 2, 3]), PolygonFace::Triangle([4, 5, 2]),]);
        }
        assert_eq!(source, original);
    }

    #[test]
    fn deletion_accepts_unordered_indices_and_sorts_report() {
        let result = polygon_mesh_delete_faces(&fixture(), 2, 2, &[2, 0]);
        assert!(result.is_ok_and(|result| result.removed_faces == vec![0, 2]
            && result.old_to_new_faces == vec![None, Some(0), None]
            && result.mesh.faces == vec![PolygonFace::Triangle([1, 4, 2])]));
    }

    #[test]
    fn detects_stale_selection_without_touching_geometry() {
        let source = fixture();
        assert_eq!(polygon_mesh_delete_faces(&source, 8, 7, &[1]), Err(KernelError::Conflict { expected: 7, actual: 8 }));
    }

    #[test]
    fn rejects_empty_duplicate_and_out_of_range_selections() {
        let source = fixture();
        assert!(polygon_mesh_delete_faces(&source, 1, 1, &[]).is_err());
        assert!(polygon_mesh_delete_faces(&source, 1, 1, &[1, 1]).is_err());
        assert!(polygon_mesh_delete_faces(&source, 1, 1, &[5]).is_err());
    }

    #[test]
    fn can_delete_all_faces_leaving_vertices_for_rebuilding() {
        let result = polygon_mesh_delete_faces(&fixture(), 0, 0, &[0, 1, 2]);
        assert!(result.is_ok_and(|r| r.mesh.faces.is_empty() && r.mesh.vertices.len() == 6 && r.old_to_new_faces == vec![None, None, None]));
    }

    #[test]
    fn rejects_revision_overflow() {
        assert_eq!(polygon_mesh_delete_faces(&fixture(), u64::MAX, u64::MAX, &[1]), Err(KernelError::Budget));
    }
}
