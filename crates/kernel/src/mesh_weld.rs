//! Deterministic mesh vertex welding and index compaction.
//! Returns a new mesh; never mutates the source or silently removes faces.
use crate::{KernelError, Result, TriangleMesh, mesh_vertex_weld_map};
use serde::{Deserialize, Serialize};

const MAX_FACES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollapsedFacePolicy {
    /// Fail the entire operation if remapping merges any two triangle corners.
    Reject,
    /// Explicitly omit collapsed triangles and report their source indices.
    Remove,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshWeldResult {
    pub mesh: TriangleMesh,
    /// For each source vertex, its new compact vertex index.
    pub old_to_new: Vec<u32>,
    /// For each retained face, its original triangle index.
    pub retained_face_indices: Vec<u32>,
    /// Source triangle indices omitted under Remove policy.
    pub removed_face_indices: Vec<u32>,
}

/// Weld vertices within an absolute drawing-unit tolerance.
/// Vertex positions use the earliest encountered representative, not an average.
/// Unreferenced representative vertices are retained, including for an empty face list.
/// A collapsed triangle is one with repeated remapped vertex indices. Geometrically
/// collinear but distinct corners and duplicate coplanar faces require other tools.
/// Valid input vertex coordinates and triangle indices are checked by the weld-map service.
pub fn mesh_weld(mesh: &TriangleMesh, tolerance: f64, policy: CollapsedFacePolicy) -> Result<MeshWeldResult> {
    if mesh.triangles.len() > MAX_FACES {
        return Err(KernelError::Budget);
    }
    let map = mesh_vertex_weld_map(mesh, tolerance)?;
    // Build the complete remapping and compacted vertices before touching any faces.
    let mut compact = Vec::new();
    compact.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    let mut old_to_new = Vec::new();
    old_to_new.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    let mut representative_to_new = Vec::new();
    representative_to_new.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    representative_to_new.resize(mesh.vertices.len(), u32::MAX);
    for (i, &rep) in map.representative.iter().enumerate() {
        let rep_index = rep as usize;
        if rep_index == i {
            let index = u32::try_from(compact.len()).map_err(|_| KernelError::Budget)?;
            compact.push(mesh.vertices[i]);
            representative_to_new[i] = index;
        }
        let new_index = representative_to_new[rep_index];
        if new_index == u32::MAX {
            return Err(KernelError::Invalid("weld representative order"));
        }
        old_to_new.push(new_index);
    }

    let mut triangles = Vec::new();
    let mut retained_face_indices = Vec::new();
    let mut removed_face_indices = Vec::new();
    triangles.try_reserve_exact(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    retained_face_indices.try_reserve_exact(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    for (source_index, triangle) in mesh.triangles.iter().enumerate() {
        let remapped = triangle.map(|index| old_to_new[index as usize]);
        let collapsed = remapped[0] == remapped[1] || remapped[1] == remapped[2] || remapped[0] == remapped[2];
        let source_index = u32::try_from(source_index).map_err(|_| KernelError::Budget)?;
        if collapsed {
            match policy {
                CollapsedFacePolicy::Reject => return Err(KernelError::Invalid("weld collapses triangle")),
                CollapsedFacePolicy::Remove => {
                    removed_face_indices.push(source_index);
                    continue;
                }
            }
        }
        triangles.push(remapped);
        retained_face_indices.push(source_index);
    }
    Ok(MeshWeldResult { mesh: TriangleMesh { vertices: compact, triangles }, old_to_new, retained_face_indices, removed_face_indices })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn source() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 0.0)],
            triangles: vec![[0, 1, 2], [3, 1, 2]],
        }
    }

    #[test]
    fn weld_compacts_vertices_and_preserves_faces_and_source() {
        let source = source();
        let original = source.clone();
        let result = mesh_weld(&source, 0.0, CollapsedFacePolicy::Reject);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.mesh.vertices.len(), 3);
            assert_eq!(result.mesh.triangles, vec![[0, 1, 2], [0, 1, 2]]);
            assert_eq!(result.old_to_new, vec![0, 1, 2, 0]);
            assert_eq!(result.retained_face_indices, vec![0, 1]);
            assert!(result.removed_face_indices.is_empty());
        }
        assert_eq!(source, original);
    }

    #[test]
    fn collapse_policy_rejects_or_removes_explicitly() {
        let mesh = TriangleMesh {
            vertices: vec![Vec3::ZERO, Vec3::new(0.01, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            triangles: vec![[0, 1, 2], [0, 3, 2]],
        };
        assert!(mesh_weld(&mesh, 0.02, CollapsedFacePolicy::Reject).is_err());
        let result = mesh_weld(&mesh, 0.02, CollapsedFacePolicy::Remove);
        assert!(result.is_ok_and(|r| r.mesh.triangles == vec![[0, 2, 1]] && r.removed_face_indices == vec![0] && r.retained_face_indices == vec![1]));
    }

    #[test]
    fn zero_tolerance_keeps_distinct_vertices() {
        let mesh = TriangleMesh { vertices: vec![Vec3::ZERO, Vec3::new(0.01, 0.0, 0.0), Vec3::Z], triangles: vec![[0, 1, 2]] };
        assert!(mesh_weld(&mesh, 0.0, CollapsedFacePolicy::Reject).is_ok_and(|r| r.mesh.vertices.len() == 3 && r.mesh.triangles == vec![[0, 1, 2]]));
    }

    #[test]
    fn empty_mesh_and_unused_vertices() {
        let mesh = TriangleMesh { vertices: vec![Vec3::ZERO, Vec3::ZERO, Vec3::Z], triangles: vec![] };
        assert!(
            mesh_weld(&mesh, 0.0, CollapsedFacePolicy::Remove)
                .is_ok_and(|r| r.mesh.vertices == vec![Vec3::ZERO, Vec3::Z] && r.mesh.triangles.is_empty() && r.old_to_new == vec![0, 0, 1])
        );
    }

    #[test]
    fn rejects_invalid_indices_and_coordinates() {
        let mut mesh = source();
        mesh.triangles[0] = [0, 10, 2];
        assert!(mesh_weld(&mesh, 0.0, CollapsedFacePolicy::Remove).is_err());
        mesh = source();
        mesh.vertices[1].x = f64::NAN;
        assert!(mesh_weld(&mesh, 0.0, CollapsedFacePolicy::Remove).is_err());
    }
}
