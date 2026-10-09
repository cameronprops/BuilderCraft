//! Configurable, non-destructive mesh repair orchestration.
//! All removal is explicit and every removed face is reported in ORIGINAL indices.
use crate::{
    CollapsedFacePolicy, KernelError, MeshValidationReport, Result, TriangleMesh, mesh_duplicate_faces, mesh_remove_unused_vertices,
    mesh_validation_report, mesh_weld,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshRepairOptions {
    /// None disables welding. Some(0.0) welds exact matches only.
    pub weld_tolerance: Option<f64>,
    /// Reject or remove faces collapsed by welding. Defaults to reject.
    pub collapsed_faces: CollapsedFacePolicy,
    pub remove_duplicate_faces: bool,
    pub remove_degenerate_faces: bool,
    pub remove_unused_vertices: bool,
    /// Relative to longest-edge squared; must be in [0, 0.5).
    pub relative_area_tolerance: f64,
}

impl Default for MeshRepairOptions {
    fn default() -> Self {
        Self {
            weld_tolerance: None,
            collapsed_faces: CollapsedFacePolicy::Reject,
            remove_duplicate_faces: false,
            remove_degenerate_faces: false,
            remove_unused_vertices: false,
            relative_area_tolerance: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshRepairResult {
    pub mesh: TriangleMesh,
    pub before: MeshValidationReport,
    pub after: MeshValidationReport,
    /// Original input face indices surviving all stages, in final face order.
    pub retained_source_faces: Vec<usize>,
    pub collapsed_source_faces: Vec<usize>,
    pub duplicate_source_faces: Vec<usize>,
    pub degenerate_source_faces: Vec<usize>,
    /// Original input vertex indices with no corresponding final vertex.
    /// Some original vertices may be merged into surviving representatives.
    pub removed_unused_vertices: Vec<u32>,
    /// Maps every original input vertex to a final index, or None if unused.
    pub original_to_final_vertices: Vec<Option<u32>>,
}

fn remove_marked_faces(mesh: &mut TriangleMesh, provenance: &mut Vec<usize>, marked: &[usize], removed: &mut Vec<usize>) -> Result<()> {
    let mut mask = vec![false; mesh.triangles.len()];
    for &index in marked {
        if index >= mask.len() {
            return Err(KernelError::Invalid("face removal index"));
        }
        mask[index] = true;
    }
    let mut kept_faces = Vec::new();
    let mut kept_provenance = Vec::new();
    kept_faces.try_reserve_exact(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    kept_provenance.try_reserve_exact(provenance.len()).map_err(|_| KernelError::Budget)?;
    for (index, (&face, &original)) in mesh.triangles.iter().zip(provenance.iter()).enumerate() {
        if mask[index] {
            removed.push(original);
        } else {
            kept_faces.push(face);
            kept_provenance.push(original);
        }
    }
    mesh.triangles = kept_faces;
    *provenance = kept_provenance;
    Ok(())
}

/// Order: baseline diagnostics -> optional weld -> duplicate removal ->
/// degenerate removal -> unused vertex compaction -> final diagnostics.
/// Rejects invalid source data and never mutates it. Does not fill holes,
/// fix winding, resolve self intersections or certify watertightness.
pub fn mesh_repair(mesh: &TriangleMesh, options: MeshRepairOptions) -> Result<MeshRepairResult> {
    // Validate options even when an input mesh has no faces.
    if !options.relative_area_tolerance.is_finite() || !(0.0..0.5).contains(&options.relative_area_tolerance) {
        return Err(KernelError::Invalid("mesh relative area tolerance"));
    }
    let before = mesh_validation_report(mesh, options.relative_area_tolerance)?;
    let mut work = mesh.clone();
    let mut provenance: Vec<usize> = (0..mesh.triangles.len()).collect();
    let mut source_to_current: Vec<Option<u32>> = (0..mesh.vertices.len()).map(|i| u32::try_from(i).ok()).collect();
    let mut collapsed_source_faces = Vec::new();
    let mut duplicate_source_faces = Vec::new();
    let mut degenerate_source_faces = Vec::new();
    let mut removed_unused_vertices = Vec::new();

    if let Some(tolerance) = options.weld_tolerance {
        let welded = mesh_weld(&work, tolerance, options.collapsed_faces)?;
        collapsed_source_faces = welded.removed_face_indices.iter().map(|&i| provenance[i as usize]).collect();
        provenance = welded.retained_face_indices.iter().map(|&i| provenance[i as usize]).collect();
        for mapped in &mut source_to_current {
            if let Some(index) = *mapped {
                *mapped = Some(welded.old_to_new[index as usize]);
            }
        }
        work = welded.mesh;
    }

    if options.remove_duplicate_faces {
        let duplicates = mesh_duplicate_faces(&work)?;
        let indices: Vec<usize> = duplicates.duplicates.iter().map(|pair| pair[0]).collect();
        remove_marked_faces(&mut work, &mut provenance, &indices, &mut duplicate_source_faces)?;
    }

    if options.remove_degenerate_faces {
        // Reuse validated face diagnostics rather than recalculate a separate predicate.
        let analysis = mesh_validation_report(&work, options.relative_area_tolerance)?;
        remove_marked_faces(&mut work, &mut provenance, &analysis.degenerate_face_indices, &mut degenerate_source_faces)?;
    }

    if options.remove_unused_vertices {
        let compact = mesh_remove_unused_vertices(&work)?;
        for (original, mapped) in source_to_current.iter_mut().enumerate() {
            if let Some(index) = *mapped {
                *mapped = compact.old_to_new[index as usize];
                if mapped.is_none() {
                    removed_unused_vertices.push(u32::try_from(original).map_err(|_| KernelError::Budget)?);
                }
            }
        }
        work = compact.mesh;
    }

    let after = mesh_validation_report(&work, options.relative_area_tolerance)?;
    Ok(MeshRepairResult {
        mesh: work,
        before,
        after,
        retained_source_faces: provenance,
        collapsed_source_faces,
        duplicate_source_faces,
        degenerate_source_faces,
        removed_unused_vertices,
        original_to_final_vertices: source_to_current,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn fixture() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, 2.0),
                Vec3::new(9.0, 9.0, 9.0),
            ],
            triangles: vec![[0, 1, 2], [3, 1, 2], [0, 0, 4], [0, 1, 4]],
        }
    }

    #[test]
    fn default_is_noop_and_does_not_modify_source() {
        let mesh = fixture();
        let original = mesh.clone();
        let result = mesh_repair(&mesh, MeshRepairOptions::default());
        assert!(result.is_ok_and(|r| r.mesh == original && r.retained_source_faces == vec![0, 1, 2, 3]));
        assert_eq!(mesh, original);
    }

    #[test]
    fn complete_pipeline_tracks_original_faces_and_vertices() {
        let source = fixture();
        let options = MeshRepairOptions {
            weld_tolerance: Some(0.0),
            collapsed_faces: CollapsedFacePolicy::Remove,
            remove_duplicate_faces: true,
            remove_degenerate_faces: true,
            remove_unused_vertices: true,
            relative_area_tolerance: 0.0,
        };
        let repaired = mesh_repair(&source, options);
        assert!(repaired.is_ok());
        if let Ok(r) = repaired {
            assert_eq!(r.retained_source_faces, vec![0, 3]);
            assert_eq!(r.collapsed_source_faces, vec![2]);
            assert_eq!(r.duplicate_source_faces, vec![1]);
            assert!(r.degenerate_source_faces.is_empty());
            assert_eq!(r.mesh.triangles, vec![[0, 1, 2], [0, 1, 3]]);
            assert_eq!(r.mesh.vertices.len(), 4);
            assert_eq!(r.original_to_final_vertices, vec![Some(0), Some(1), Some(2), Some(0), Some(3), None,]);
            assert_eq!(r.removed_unused_vertices, vec![5]);
            assert_eq!(r.after.face_count, 2);
        }
    }

    #[test]
    fn reject_policy_is_atomic() {
        let input = fixture();
        let snapshot = input.clone();
        let options = MeshRepairOptions { weld_tolerance: Some(0.0), ..MeshRepairOptions::default() };
        assert!(mesh_repair(&input, options).is_err());
        assert_eq!(input, snapshot);
    }

    #[test]
    fn removes_geometrically_degenerate_faces_without_welding() {
        let source = TriangleMesh {
            vertices: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
            triangles: vec![[0, 1, 2], [0, 1, 3]],
        };
        let options = MeshRepairOptions { remove_degenerate_faces: true, ..MeshRepairOptions::default() };
        assert!(mesh_repair(&source, options).is_ok_and(|r| r.degenerate_source_faces == vec![0] && r.retained_source_faces == vec![1]));
    }

    #[test]
    fn invalid_options_and_empty_mesh() {
        let empty = TriangleMesh { vertices: vec![], triangles: vec![] };
        assert!(mesh_repair(&empty, MeshRepairOptions::default()).is_ok());
        assert!(mesh_repair(&empty, MeshRepairOptions { relative_area_tolerance: f64::NAN, ..MeshRepairOptions::default() }).is_err());
        assert!(mesh_repair(&empty, MeshRepairOptions { weld_tolerance: Some(-1.0), ..MeshRepairOptions::default() }).is_err());
    }
}
