//! Read-only and copy-on-write triangle mesh cleanup utilities.
//! Face duplicates use identical vertex indices regardless of winding;
//! coincident but unwelded coordinates are NOT considered equal.
use crate::{KernelError, MeshEdgeReport, Result, TriangleMesh, mesh_degenerate_faces, mesh_edge_report};
use std::collections::BTreeMap;

const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;

fn validate(mesh: &TriangleMesh) -> Result<()> {
    if mesh.vertices.len() > MAX_VERTICES || mesh.triangles.len() > MAX_FACES {
        return Err(KernelError::Budget);
    }
    if mesh.vertices.iter().any(|p| !p.is_finite() || [p.x, p.y, p.z].iter().any(|v| v.abs() > 1e12)) {
        return Err(KernelError::Invalid("mesh coordinate"));
    }
    if mesh.triangles.iter().flatten().any(|&i| i as usize >= mesh.vertices.len()) {
        return Err(KernelError::Invalid("mesh triangle index"));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateFaceReport {
    /// [duplicate triangle index, first occurrence index], in source order.
    pub duplicates: Vec<[usize; 2]>,
}

/// Find duplicate faces, including reversed winding and cyclic rotations.
/// Indexed connectivity only. Does not mutate or remove faces.
pub fn mesh_duplicate_faces(mesh: &TriangleMesh) -> Result<DuplicateFaceReport> {
    validate(mesh)?;
    let mut seen = BTreeMap::<[u32; 3], usize>::new();
    let mut duplicates = Vec::new();
    for (index, face) in mesh.triangles.iter().enumerate() {
        let mut key = *face;
        key.sort_unstable();
        match seen.entry(key) {
            std::collections::btree_map::Entry::Occupied(first) => {
                duplicates.push([index, *first.get()]);
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(index);
            }
        }
    }
    Ok(DuplicateFaceReport { duplicates })
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompactMeshResult {
    pub mesh: TriangleMesh,
    /// Maps each original vertex index to its new index, or None when unused.
    pub old_to_new: Vec<Option<u32>>,
    pub removed_vertex_indices: Vec<u32>,
}

/// Remove vertices that no triangle references. Returns a new indexed mesh,
/// preserving face count, winding and order. Does not weld nearby positions.
pub fn mesh_remove_unused_vertices(mesh: &TriangleMesh) -> Result<CompactMeshResult> {
    validate(mesh)?;
    let mut used = Vec::new();
    used.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    used.resize(mesh.vertices.len(), false);
    for face in &mesh.triangles {
        for &index in face {
            used[index as usize] = true;
        }
    }
    let mut vertices = Vec::new();
    let mut old_to_new = Vec::new();
    let mut removed_vertex_indices = Vec::new();
    vertices.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    old_to_new.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    for (index, &vertex) in mesh.vertices.iter().enumerate() {
        if used[index] {
            let next = u32::try_from(vertices.len()).map_err(|_| KernelError::Budget)?;
            vertices.push(vertex);
            old_to_new.push(Some(next));
        } else {
            old_to_new.push(None);
            removed_vertex_indices.push(u32::try_from(index).map_err(|_| KernelError::Budget)?);
        }
    }
    let mut triangles = Vec::new();
    triangles.try_reserve_exact(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    for face in &mesh.triangles {
        let mut remapped = [0; 3];
        for (target, &original) in remapped.iter_mut().zip(face) {
            *target = old_to_new[original as usize].ok_or(KernelError::Invalid("unused face vertex"))?;
        }
        triangles.push(remapped);
    }
    Ok(CompactMeshResult { mesh: TriangleMesh { vertices, triangles }, old_to_new, removed_vertex_indices })
}

/// Combined diagnostic report on indexed topology and face quality.
/// Zero-area/repeated-index triangles are diagnosed, but edge analysis is
/// unavailable when faces repeat vertex indices. This is not a guarantee
/// of watertightness: self-intersections and isolated non-manifold vertices
/// are outside this diagnostic's current scope.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshValidationReport {
    pub vertex_count: usize,
    pub face_count: usize,
    pub unused_vertex_indices: Vec<u32>,
    pub duplicate_faces: DuplicateFaceReport,
    pub degenerate_face_indices: Vec<usize>,
    pub edge_report: Option<MeshEdgeReport>,
}

pub fn mesh_validation_report(mesh: &TriangleMesh, relative_area_tolerance: f64) -> Result<MeshValidationReport> {
    validate(mesh)?;
    let compact = mesh_remove_unused_vertices(mesh)?;
    let duplicate_faces = mesh_duplicate_faces(mesh)?;
    // A completely empty mesh has no faces to diagnose.
    let degenerate_face_indices = if mesh.vertices.is_empty() && mesh.triangles.is_empty() {
        if !relative_area_tolerance.is_finite() || !(0.0..0.5).contains(&relative_area_tolerance) {
            return Err(KernelError::Invalid("mesh relative area tolerance"));
        }
        Vec::new()
    } else {
        mesh_degenerate_faces(mesh, relative_area_tolerance)?
    };
    let repeated_indices = mesh.triangles.iter().any(|&[a, b, c]| a == b || b == c || a == c);
    let edge_report = if repeated_indices { None } else { Some(mesh_edge_report(mesh)?) };
    Ok(MeshValidationReport {
        vertex_count: mesh.vertices.len(),
        face_count: mesh.triangles.len(),
        unused_vertex_indices: compact.removed_vertex_indices,
        duplicate_faces,
        degenerate_face_indices,
        edge_report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn sample() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::Z, Vec3::new(8.0, 8.0, 8.0)],
            triangles: vec![[0, 1, 2], [1, 2, 0], [2, 1, 0], [0, 1, 3]],
        }
    }

    #[test]
    fn detects_reversed_and_rotated_duplicates() {
        let r = mesh_duplicate_faces(&sample());
        assert!(r.is_ok_and(|r| r.duplicates == vec![[1, 0], [2, 0]]));
    }

    #[test]
    fn compacts_unused_vertices_and_preserves_source() {
        let original = sample();
        let before = original.clone();
        let result = mesh_remove_unused_vertices(&original);
        assert!(result.is_ok_and(|r| {
            r.removed_vertex_indices == vec![4]
                && r.old_to_new == vec![Some(0), Some(1), Some(2), Some(3), None]
                && r.mesh.vertices.len() == 4
                && r.mesh.triangles == before.triangles
        }));
        assert_eq!(original, before);
    }

    #[test]
    fn reports_face_and_topology_issues_after_weld() {
        let mesh =
            TriangleMesh { vertices: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)], triangles: vec![[0, 1, 2], [0, 2, 1]] };
        let r = mesh_validation_report(&mesh, 0.0);
        assert!(r.is_ok_and(|r| r.duplicate_faces.duplicates == vec![[1, 0]]
            && r.degenerate_face_indices.is_empty()
            && r.edge_report.is_some_and(|e| e.boundary_edges.is_empty())));
    }

    #[test]
    fn repeated_index_faces_remain_diagnosable() {
        let mesh = TriangleMesh { vertices: vec![Vec3::ZERO, Vec3::Z], triangles: vec![[0, 0, 1]] };
        assert!(mesh_validation_report(&mesh, 0.0).is_ok_and(|r| r.degenerate_face_indices == vec![0] && r.edge_report.is_none()));
    }

    #[test]
    fn handles_empty_mesh_and_bad_inputs() {
        let empty = TriangleMesh { vertices: vec![], triangles: vec![] };
        assert!(mesh_validation_report(&empty, 0.0).is_ok());
        assert!(mesh_remove_unused_vertices(&empty).is_ok());
        let invalid = TriangleMesh { vertices: vec![Vec3::ZERO], triangles: vec![[0, 1, 0]] };
        assert!(mesh_duplicate_faces(&invalid).is_err());
        assert!(mesh_validation_report(&empty, f64::NAN).is_err());
    }
}
