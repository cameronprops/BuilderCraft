//! Read-only triangle-mesh diagnostics for CAD and Scan.
//! The source mesh is never modified. Degenerate faces return no normal.
use crate::{KernelError, Result, TriangleMesh};
use cadcraft_geom::Vec3;

const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;
const MAX_COORDINATE: f64 = 1e12;

/// Report for one triangle. A zero-area triangle has no unit normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshFaceAnalysis {
    pub normal: Option<Vec3>,
    pub area: f64,
    pub degenerate: bool,
}

/// Reject malformed indices and nonfinite coordinates, but allow geometric
/// degeneracies such as coincident positions for diagnostic reporting.
pub fn validate_triangle_mesh(mesh: &TriangleMesh) -> Result<()> {
    if mesh.vertices.is_empty() || mesh.vertices.len() > MAX_VERTICES || mesh.triangles.len() > MAX_FACES {
        return Err(KernelError::Invalid("mesh size"));
    }
    if mesh.vertices.iter().any(|p| !p.is_finite() || [p.x, p.y, p.z].iter().any(|v| v.abs() > MAX_COORDINATE)) {
        return Err(KernelError::Invalid("mesh coordinate"));
    }
    if mesh.triangles.iter().flatten().any(|&i| i as usize >= mesh.vertices.len()) {
        return Err(KernelError::Invalid("mesh triangle index"));
    }
    Ok(())
}

// Single canonical per-face predicate for full inspection and streaming
// defect-only diagnostics. Validated indices/tolerance are caller-owned.
fn analyze_triangle(mesh: &TriangleMesh, [a, b, c]: [u32; 3], relative_area_tolerance: f64) -> Result<MeshFaceAnalysis> {
    let p = mesh.vertices[a as usize];
    let q = mesh.vertices[b as usize];
    let r = mesh.vertices[c as usize];
    let u = q - p;
    let v = r - p;
    let w = r - q;
    let scale = u.x.hypot(u.y).hypot(u.z).max(v.x.hypot(v.y).hypot(v.z)).max(w.x.hypot(w.y).hypot(w.z));
    let normalized_cross = if scale > 0.0 { (u * (1.0 / scale)).cross(v * (1.0 / scale)) } else { Vec3::ZERO };
    let cross_length = normalized_cross.x.hypot(normalized_cross.y).hypot(normalized_cross.z);
    let area = 0.5 * cross_length * scale * scale;
    if !area.is_finite() {
        return Err(KernelError::Invalid("mesh face area overflow"));
    }
    let degenerate = a == b || b == c || a == c || cross_length <= 2.0 * relative_area_tolerance || cross_length == 0.0;
    let normal = if degenerate { None } else { Some(normalized_cross * (1.0 / cross_length)) };
    Ok(MeshFaceAnalysis { normal, area, degenerate })
}

/// Calculate each face's unit normal and area. Degenerate faces get None,
/// including triangles whose vertex indices differ but positions coincide.
///
/// `relative_area_tolerance` is a dimensionless threshold relative to the
/// square of the longest edge. It must lie in [0, 0.5).
pub fn mesh_face_analysis(mesh: &TriangleMesh, relative_area_tolerance: f64) -> Result<Vec<MeshFaceAnalysis>> {
    validate_triangle_mesh(mesh)?;
    if !relative_area_tolerance.is_finite() || !(0.0..0.5).contains(&relative_area_tolerance) {
        return Err(KernelError::Invalid("mesh relative area tolerance"));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    for &triangle in &mesh.triangles {
        output.push(analyze_triangle(mesh, triangle, relative_area_tolerance)?);
    }
    Ok(output)
}

/// Indices of degenerate triangles, in the order found.
pub fn mesh_degenerate_faces(mesh: &TriangleMesh, relative_area_tolerance: f64) -> Result<Vec<usize>> {
    validate_triangle_mesh(mesh)?;
    if !relative_area_tolerance.is_finite() || !(0.0..0.5).contains(&relative_area_tolerance) {
        return Err(KernelError::Invalid("mesh relative area tolerance"));
    }
    // Most good meshes have few/no defective faces. Avoid materializing one
    // MeshFaceAnalysis (normal + area) for every face when only indices are wanted.
    let mut indices = Vec::new();
    for (index, &face) in mesh.triangles.iter().enumerate() {
        if analyze_triangle(mesh, face, relative_area_tolerance)?.degenerate {
            indices.try_reserve(1).map_err(|_| KernelError::Budget)?;
            indices.push(index);
        }
    }
    Ok(indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            triangles: vec![[0, 1, 2], [0, 1, 3], [0, 0, 2]],
        }
    }

    #[test]
    fn normals_areas_and_degenerate_faces() {
        let analysis = mesh_face_analysis(&mesh(), 1e-12);
        assert!(analysis.is_ok());
        if let Ok(faces) = analysis {
            assert_eq!(faces.len(), 3);
            assert_eq!(faces[0].normal, Some(Vec3::Z));
            assert_eq!(faces[0].area, 2.0);
            assert!(!faces[0].degenerate);
            assert!(faces[1].degenerate && faces[1].normal.is_none());
            assert!(faces[2].degenerate && faces[2].normal.is_none());
        }
        assert_eq!(mesh_degenerate_faces(&mesh(), 1e-12), Ok(vec![1, 2]));
    }

    #[test]
    fn streamed_defect_indices_match_full_face_analysis() {
        let mesh = mesh();
        for tolerance in [0.0, 1e-12, 0.01, 0.49] {
            let full = mesh_face_analysis(&mesh, tolerance);
            let indices = mesh_degenerate_faces(&mesh, tolerance);
            assert!(full.is_ok());
            assert!(indices.is_ok());
            if let (Ok(full), Ok(indices)) = (full, indices) {
                let expected: Vec<usize> = full.iter().enumerate().filter_map(|(i, f)| f.degenerate.then_some(i)).collect();
                assert_eq!(indices, expected);
            }
        }
    }

    #[test]
    fn reversed_winding_flips_normal() {
        let mut m = mesh();
        m.triangles = vec![[0, 2, 1]];
        assert!(mesh_face_analysis(&m, 0.0).is_ok_and(|f| f[0].normal == Some(-Vec3::Z)));
    }

    #[test]
    fn malformed_input_returns_error() {
        let mut m = mesh();
        m.triangles = vec![[0, 99, 2]];
        assert!(mesh_face_analysis(&m, 0.0).is_err());
        m = mesh();
        m.vertices[0].x = f64::NAN;
        assert!(mesh_face_analysis(&m, 0.0).is_err());
        assert!(mesh_face_analysis(&mesh(), -1.0).is_err());
        assert!(mesh_face_analysis(&mesh(), f64::NAN).is_err());
    }

    #[test]
    fn empty_triangle_list_is_valid() {
        let m = TriangleMesh { vertices: vec![Vec3::ZERO], triangles: vec![] };
        assert_eq!(mesh_face_analysis(&m, 0.0), Ok(vec![]));
    }
}
