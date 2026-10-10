//! Bounded polygon-hole patching with surface-preserving and explicit planarization modes.
//! This is a mesh operation, not a fitted NURBS surface or certified intersection repair.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_boundary_loops, polygon_mesh_topology, polygon_mesh_validate};
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};

const MAX_LOOP: usize = 256;
const MAX_FACES: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum PolygonPatchMode {
    /// Triangles retain the original 3D boundary, including nonplanarity.
    Surface,
    /// Insert bounded interior vertices and continue the surrounding face slope
    /// toward the center while preserving the original 3D boundary exactly.
    CurvatureSmooth { refinement_levels: u8, smoothing_iterations: u16, tangent_weight: f64, max_interior_offset: f64 },
    /// Least-squares total orthogonal distance fit (3x3 symmetric covariance).
    PlanarBestFit { max_displacement: f64 },
    /// Direction inferred from the adjacent polygon faces; plane through boundary centroid.
    PlanarAverageNormal { max_displacement: f64 },
    /// User-supplied plane normal, with plane anchored at boundary centroid.
    PlanarDirection { normal: Vec3, max_displacement: f64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonPatchPlane {
    pub origin: Vec3,
    pub normal: Vec3,
    /// Distances are in native mesh/document units.
    pub rms_boundary_deviation: f64,
    pub max_boundary_deviation: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonAdvancedFillResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    pub boundary_vertices: Vec<u32>,
    pub new_face_indices: Vec<u32>,
    pub plane: PolygonPatchPlane,
    /// ID and before/after position for each vertex that the planarization moved.
    /// These are shared vertices: adjacent existing faces are affected.
    pub moved_vertices: Vec<(u32, Vec3, Vec3)>,
}

fn len(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}

fn unit(v: Vec3) -> Result<Vec3> {
    let n = len(v);
    if !n.is_finite() || n <= 1e-14 { Err(KernelError::Invalid("degenerate plane direction")) } else { Ok(v * (1.0 / n)) }
}

fn covariance_normal(points: &[Vec3], center: Vec3) -> Result<Vec3> {
    let mut a = [[0.0_f64; 3]; 3];
    for p in points {
        let d = [p.x - center.x, p.y - center.y, p.z - center.z];
        for i in 0..3 {
            for j in i..3 {
                a[i][j] += d[i] * d[j];
            }
        }
    }
    for i in 0..3 {
        for j in 0..i {
            a[i][j] = a[j][i];
        }
    }
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for _ in 0..48 {
        let mut p = 0;
        let mut q = 1;
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            if a[i][j].abs() > a[p][q].abs() {
                p = i;
                q = j;
            }
        }
        let scale = a[0][0].abs().max(a[1][1].abs()).max(a[2][2].abs());
        if a[p][q].abs() <= 1e-15 * scale {
            break;
        }
        let theta = 0.5 * (2.0 * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (s, c) = theta.sin_cos();
        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];
        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for k in 0..3 {
            if k != p && k != q {
                let kp = a[k][p];
                let kq = a[k][q];
                a[k][p] = c * kp - s * kq;
                a[p][k] = a[k][p];
                a[k][q] = s * kp + c * kq;
                a[q][k] = a[k][q];
            }
            let vp = v[k][p];
            let vq = v[k][q];
            v[k][p] = c * vp - s * vq;
            v[k][q] = s * vp + c * vq;
        }
    }
    let mut order = [0_usize, 1, 2];
    order.sort_by(|&i, &j| a[i][i].total_cmp(&a[j][j]));
    if !a[order[2]][order[2]].is_finite() || a[order[2]][order[2]] <= 0.0 || a[order[1]][order[1]] <= 1e-12 * a[order[2]][order[2]] {
        return Err(KernelError::Invalid("collinear boundary cannot define a plane"));
    }
    unit(Vec3::new(v[0][order[0]], v[1][order[0]], v[2][order[0]]))
}

fn projected_coordinate(point: Vec3, center: Vec3, u: Vec3, v: Vec3) -> [f64; 2] {
    let d = point - center;
    [d.dot(u), d.dot(v)]
}

fn cross2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn strict_in_triangle(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2], epsilon: f64) -> bool {
    cross2(a, b, p) >= -epsilon && cross2(b, c, p) >= -epsilon && cross2(c, a, p) >= -epsilon
}

fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2], epsilon: f64) -> bool {
    let x = cross2(a, b, c);
    let y = cross2(a, b, d);
    let z = cross2(c, d, a);
    let w = cross2(c, d, b);
    ((x > epsilon && y < -epsilon) || (x < -epsilon && y > epsilon)) && ((z > epsilon && w < -epsilon) || (z < -epsilon && w > epsilon))
}

fn triangulate_loop(points: &[[f64; 2]], eps: f64) -> Result<Vec<[usize; 3]>> {
    let n = points.len();
    if n < 3 || n > MAX_LOOP {
        return Err(KernelError::Invalid("unsupported hole boundary size"));
    }
    for i in 0..n {
        let p = points[i];
        let q = points[(i + 1) % n];
        if (p[0] - q[0]).hypot(p[1] - q[1]) <= eps.sqrt() {
            return Err(KernelError::Invalid("collapsed projected boundary edge"));
        }
        for j in i + 1..n {
            if i == j || (i + 1) % n == j || (j + 1) % n == i {
                continue;
            }
            if segments_cross(p, q, points[j], points[(j + 1) % n], eps) {
                return Err(KernelError::Invalid("self-crossing projected boundary"));
            }
        }
    }
    let area = (0..n)
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % n];
            a[0] * b[1] - a[1] * b[0]
        })
        .sum::<f64>();
    if area <= eps {
        return Err(KernelError::Invalid("collapsed or reversed projected hole"));
    }
    let mut remaining: Vec<usize> = (0..n).collect();
    let mut result = Vec::with_capacity(n - 2);
    while remaining.len() > 3 {
        let mut ear = None;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            if cross2(points[a], points[b], points[c]) <= eps {
                continue;
            }
            if remaining
                .iter()
                .any(|&other| other != a && other != b && other != c && strict_in_triangle(points[other], points[a], points[b], points[c], eps))
            {
                continue;
            }
            ear = Some((i, [a, b, c]));
            break;
        }
        let (i, triangle) = ear.ok_or(KernelError::Invalid("hole cannot be safely triangulated"))?;
        result.push(triangle);
        remaining.remove(i);
    }
    if cross2(points[remaining[0]], points[remaining[1]], points[remaining[2]]) <= eps {
        return Err(KernelError::Invalid("degenerate final patch triangle"));
    }
    result.push([remaining[0], remaining[1], remaining[2]]);
    Ok(result)
}

/// Preview and construct a conservative patch; no source mutation.
///
/// Surface preserves ALL original vertices, including the nonplanar rim.
/// Planar modes displace ONLY the rim vertices (also changing adjacent faces),
/// subject to a caller-supplied maximum allowable displacement.
/// No distant triangle self-intersection certificate is implied.
pub fn polygon_mesh_fill_hole_advanced(
    mesh: &PolygonMesh,
    revision: u64,
    picked_revision: u64,
    loop_index: u32,
    mode: PolygonPatchMode,
) -> Result<PolygonAdvancedFillResult> {
    if revision != picked_revision {
        return Err(KernelError::Conflict { expected: picked_revision, actual: revision });
    }
    polygon_mesh_validate(mesh)?;
    let report = polygon_mesh_boundary_loops(mesh)?;
    if !report.unresolved_edges.is_empty() || !report.non_manifold_edges.is_empty() || !report.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("ambiguous mesh boundary topology"));
    }
    let boundary = report.closed_loops.get(loop_index as usize).ok_or(KernelError::Invalid("boundary loop index"))?;
    let ids = &boundary.vertices;
    if !(3..=MAX_LOOP).contains(&ids.len()) {
        return Err(KernelError::Invalid("unsupported hole boundary size"));
    }
    if mesh.faces.len().checked_add(ids.len() - 2).is_none_or(|n| n > MAX_FACES) {
        return Err(KernelError::Budget);
    }
    let new_revision = revision.checked_add(1).ok_or(KernelError::Budget)?;
    let points: Vec<Vec3> = ids.iter().map(|&id| mesh.vertices[id as usize]).collect();
    let center = points.iter().copied().fold(Vec3::ZERO, |sum, p| sum + p) * (1.0 / ids.len() as f64);
    if !center.is_finite() {
        return Err(KernelError::Invalid("nonfinite plane centroid"));
    }
    let mut signed_area = Vec3::ZERO;
    let mut extent = 0.0_f64;
    for i in 0..points.len() {
        signed_area = signed_area + (points[i] - center).cross(points[(i + 1) % points.len()] - center);
        extent = extent.max(len(points[i] - center));
    }
    if !extent.is_finite() || extent <= 1e-12 || len(signed_area) <= 1e-12 * extent * extent {
        return Err(KernelError::Invalid("degenerate boundary plane"));
    }
    let boundary_normal = unit(signed_area)?;
    let topo = polygon_mesh_topology(mesh)?;
    let mut mean_face_normal = Vec3::ZERO;
    for &half_id in &boundary.halfedges {
        let half = &topo.halfedges[half_id as usize];
        let face = mesh.faces[half.face as usize].indices();
        let p = mesh.vertices[face[0] as usize];
        let q = mesh.vertices[face[1] as usize];
        let r = mesh.vertices[face[2] as usize];
        mean_face_normal = mean_face_normal + unit((q - p).cross(r - p))?;
    }
    // The neighboring surface must face opposite the oriented inner boundary.
    // A near-tangent or ambiguous boundary requires an explicit cap workflow.
    if mean_face_normal.dot(boundary_normal) >= -0.25 * ids.len() as f64 {
        return Err(KernelError::Invalid("exterior or ambiguous boundary orientation"));
    }
    let (candidate_normal, max_displacement) = match mode {
        PolygonPatchMode::Surface => (covariance_normal(&points, center)?, None),
        PolygonPatchMode::CurvatureSmooth { refinement_levels, smoothing_iterations, tangent_weight, max_interior_offset } => {
            return crate::polygon_mesh_fill_hole_curvature(
                mesh, revision, picked_revision, loop_index, refinement_levels, smoothing_iterations, tangent_weight, max_interior_offset
            );
        },
        PolygonPatchMode::PlanarBestFit { max_displacement } => (covariance_normal(&points, center)?, Some(max_displacement)),
        PolygonPatchMode::PlanarAverageNormal { max_displacement } => (unit(mean_face_normal)?, Some(max_displacement)),
        PolygonPatchMode::PlanarDirection { normal, max_displacement } => (unit(normal)?, Some(max_displacement)),
    };
    if max_displacement.is_some_and(|limit| !limit.is_finite() || limit < 0.0) {
        return Err(KernelError::Invalid("invalid maximum planar displacement"));
    }
    let normal = if candidate_normal.dot(boundary_normal) < 0.0 { candidate_normal * -1.0 } else { candidate_normal };
    let mut u = Vec3::ZERO;
    for i in 0..points.len() {
        let edge = points[(i + 1) % points.len()] - points[i];
        let projected = edge - normal * edge.dot(normal);
        if len(projected) > len(u) {
            u = projected;
        }
    }
    u = unit(u)?;
    let v = unit(normal.cross(u))?;
    let uv: Vec<[f64; 2]> = points.iter().map(|p| projected_coordinate(*p, center, u, v)).collect();
    let epsilon = 1e-12 * extent * extent;
    let triangles = triangulate_loop(&uv, epsilon)?;
    let mut rms = 0.0_f64;
    let mut maximum = 0.0_f64;
    for point in &points {
        let distance = (*point - center).dot(normal).abs();
        rms += distance * distance;
        maximum = maximum.max(distance);
    }
    rms = (rms / points.len() as f64).sqrt();
    let mut output = mesh.clone();
    let mut moved_vertices = Vec::new();
    if let Some(limit) = max_displacement {
        if maximum > limit {
            return Err(KernelError::Invalid("planarization exceeds maximum displacement"));
        }
        for &id in ids {
            let before = mesh.vertices[id as usize];
            let after = before - normal * (before - center).dot(normal);
            if len(after - before) > 1e-12 * extent {
                moved_vertices.push((id, before, after));
            }
            output.vertices[id as usize] = after;
        }
    }
    let mut new_face_indices = Vec::with_capacity(triangles.len());
    for [a, b, c] in triangles {
        new_face_indices.push(u32::try_from(output.faces.len()).map_err(|_| KernelError::Budget)?);
        // Boundary halfedges run with neighboring faces; patch edges must oppose them.
        output.faces.push(PolygonFace::Triangle([ids[a], ids[c], ids[b]]));
    }
    polygon_mesh_validate(&output)?;
    let after_topo = polygon_mesh_topology(&output)?;
    if !after_topo.non_manifold_edges.is_empty() || !after_topo.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("invalid patch edge topology"));
    }
    Ok(PolygonAdvancedFillResult {
        mesh: output,
        revision: new_revision,
        boundary_vertices: ids.clone(),
        new_face_indices,
        plane: PolygonPatchPlane { origin: center, normal, rms_boundary_deviation: rms, max_boundary_deviation: maximum },
        moved_vertices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(4., 0., 0.),
                Vec3::new(4., 4., 0.),
                Vec3::new(0., 4., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(3., 1., 0.),
                Vec3::new(3., 3., 0.),
                Vec3::new(1., 3., 0.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        }
    }

    fn inner(mesh: &PolygonMesh) -> u32 {
        polygon_mesh_boundary_loops(mesh)
            .ok()
            .and_then(|r| r.closed_loops.iter().position(|l| l.vertices.iter().all(|&v| v >= 4)))
            .and_then(|i| u32::try_from(i).ok())
            .unwrap_or(u32::MAX)
    }

    #[test]
    fn nonplanar_surface_preserves_rim_and_closes_hole() {
        let mut source = ring();
        source.vertices[4].z = 0.2;
        assert!(polygon_mesh_validate(&source).is_ok());
        let original = source.clone();
        let filled = polygon_mesh_fill_hole_advanced(&source, 4, 4, inner(&source), PolygonPatchMode::Surface);
        assert!(filled.is_ok(), "{filled:?}");
        if let Ok(result) = filled {
            assert_eq!(source, original);
            assert!(result.moved_vertices.is_empty());
            assert_eq!(result.mesh.vertices, source.vertices);
            assert_eq!(result.new_face_indices.len(), 2);
            assert!(polygon_mesh_boundary_loops(&result.mesh).is_ok_and(|r| r.closed_loops.len() == 1));
        }
    }

    #[test]
    fn best_fit_planarizes_boundary_and_tracks_motion() {
        let mut source = ring();
        source.vertices[4].z = 0.2;
        let filled = polygon_mesh_fill_hole_advanced(&source, 0, 0, inner(&source), PolygonPatchMode::PlanarBestFit { max_displacement: 0.3 });
        assert!(filled.is_ok(), "{filled:?}");
        if let Ok(result) = filled {
            assert!(!result.moved_vertices.is_empty());
            assert!(result.plane.max_boundary_deviation > 0.0);
            for &id in &result.boundary_vertices {
                let d = (result.mesh.vertices[id as usize] - result.plane.origin).dot(result.plane.normal);
                assert!(d.abs() < 1e-9);
            }
            assert!(polygon_mesh_validate(&result.mesh).is_ok());
        }
    }

    #[test]
    fn average_normal_planarization_and_explicit_axis() {
        let mut source = ring();
        source.vertices[4].z = 0.1;
        for mode in [
            PolygonPatchMode::PlanarAverageNormal { max_displacement: 0.3 },
            PolygonPatchMode::PlanarDirection { normal: Vec3::new(0., 0., 9.), max_displacement: 0.3 },
        ] {
            let filled = polygon_mesh_fill_hole_advanced(&source, 0, 0, inner(&source), mode);
            assert!(filled.is_ok(), "{filled:?}");
        }
    }

    #[test]
    fn fills_concave_warped_rim_without_flattening() {
        let mut vertices = Vec::new();
        for i in 0..6 {
            let a = std::f64::consts::TAU * (i as f64) / 6.0;
            vertices.push(Vec3::new(5.0 * a.cos(), 5.0 * a.sin(), 0.0));
        }
        for i in 0..6 {
            let a = std::f64::consts::TAU * (i as f64) / 6.0;
            let radius = if i == 3 { 0.2 } else { 1.5 };
            vertices.push(Vec3::new(radius * a.cos(), radius * a.sin(), if i == 1 { 0.1 } else { 0.0 }));
        }
        let mut faces = Vec::new();
        for i in 0..6_u32 {
            let next = (i + 1) % 6;
            faces.push(PolygonFace::Quad([i, next, next + 6, i + 6]));
        }
        let source = PolygonMesh { vertices, faces };
        assert!(polygon_mesh_validate(&source).is_ok());
        let result = polygon_mesh_fill_hole_advanced(&source, 7, 7, inner(&source), PolygonPatchMode::Surface);
        assert!(result.is_ok(), "{result:?}");
        if let Ok(patch) = result {
            assert_eq!(patch.new_face_indices.len(), 4);
            assert_eq!(patch.mesh.vertices, source.vertices);
            assert!(polygon_mesh_boundary_loops(&patch.mesh).is_ok_and(|r| r.closed_loops.len() == 1));
        }
    }

    #[test]
    fn caps_displacement_rejects_outer_and_stale() {
        let mut source = ring();
        source.vertices[4].z = 0.2;
        let inside = inner(&source);
        assert!(polygon_mesh_fill_hole_advanced(&source, 0, 0, inside, PolygonPatchMode::PlanarBestFit { max_displacement: 0.001 }).is_err());
        assert!(polygon_mesh_fill_hole_advanced(&source, 0, 0, if inside == 0 { 1 } else { 0 }, PolygonPatchMode::Surface).is_err());
        assert_eq!(
            polygon_mesh_fill_hole_advanced(&source, 5, 4, inside, PolygonPatchMode::Surface),
            Err(KernelError::Conflict { expected: 4, actual: 5 })
        );
        assert!(
            polygon_mesh_fill_hole_advanced(&source, 0, 0, inside, PolygonPatchMode::PlanarDirection { normal: Vec3::ZERO, max_displacement: 1. })
                .is_err()
        );
    }
}
