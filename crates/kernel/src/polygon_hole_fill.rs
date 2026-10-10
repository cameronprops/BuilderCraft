//! Conservative, quality-aware triangulation of simple planar INNER boundaries of polygon meshes.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_boundary_loops, polygon_mesh_topology, polygon_mesh_validate};
use cadcraft_geom::{Vec2, Vec3, robust_predicates::orientation2d};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonFillResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    pub boundary_vertices: Vec<u32>,
    pub new_face_indices: Vec<u32>,
}
fn norm(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}


fn project_loop(mesh: &PolygonMesh, ids: &[u32], normal: Vec3, origin: Vec3) -> Vec<Vec2> {
    // Pick a signed dominant-axis plane, preserving the orientation of
    // the boundary in the 2D projection. Relative positions reduce
    // catastrophic cancellation for drawings far from the world origin.
    ids.iter().map(|&id| {
        let d = mesh.vertices[id as usize] - origin;
        if normal.x.abs() >= normal.y.abs() && normal.x.abs() >= normal.z.abs() {
            if normal.x >= 0. { Vec2::new(d.y, d.z) } else { Vec2::new(d.z, d.y) }
        } else if normal.y.abs() >= normal.z.abs() {
            if normal.y >= 0. { Vec2::new(d.z, d.x) } else { Vec2::new(d.x, d.z) }
        } else if normal.z >= 0. {
            Vec2::new(d.x, d.y)
        } else {
            Vec2::new(d.y, d.x)
        }
    }).collect()
}

fn on_segment(a: Vec2, b: Vec2, point: Vec2) -> bool {
    point.x >= a.x.min(b.x) && point.x <= a.x.max(b.x)
        && point.y >= a.y.min(b.y) && point.y <= a.y.max(b.y)
}

fn intersects(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> Result<bool> {
    let ab_c = orientation2d(a, b, c).ok_or(KernelError::Invalid("invalid boundary orientation"))?;
    let ab_d = orientation2d(a, b, d).ok_or(KernelError::Invalid("invalid boundary orientation"))?;
    let cd_a = orientation2d(c, d, a).ok_or(KernelError::Invalid("invalid boundary orientation"))?;
    let cd_b = orientation2d(c, d, b).ok_or(KernelError::Invalid("invalid boundary orientation"))?;
    if (ab_c == Ordering::Equal && on_segment(a, b, c))
        || (ab_d == Ordering::Equal && on_segment(a, b, d))
        || (cd_a == Ordering::Equal && on_segment(c, d, a))
        || (cd_b == Ordering::Equal && on_segment(c, d, b)) {
        return Ok(true);
    }
    Ok(ab_c != ab_d && cd_a != cd_b)
}

fn validate_simple_loop(points: &[Vec2], extent: f64) -> Result<()> {
    let n = points.len();
    let min_separation = extent * 1e-9;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        if !a.is_finite() || !b.is_finite() || (b - a).len() <= min_separation {
            return Err(KernelError::Invalid("collapsed boundary edge"));
        }
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) { continue; }
            if intersects(a, b, points[j], points[(j + 1) % n])? {
                return Err(KernelError::Invalid("self-intersecting planar hole boundary"));
            }
        }
    }
    Ok(())
}

fn triangle_contains_or_touches(a: Vec2, b: Vec2, c: Vec2, point: Vec2) -> Result<bool> {
    Ok(orientation2d(a, b, point).ok_or(KernelError::Invalid("invalid triangulation coordinate"))? != Ordering::Less
        && orientation2d(b, c, point).ok_or(KernelError::Invalid("invalid triangulation coordinate"))? != Ordering::Less
        && orientation2d(c, a, point).ok_or(KernelError::Invalid("invalid triangulation coordinate"))? != Ordering::Less)
}

/// Quality-aware ear clipping over an already validated simple, CCW loop.
/// Bound at 256 vertices. Triangles are reversed for the new patch because
/// the existing polygon half-edges around the hole have opposite winding.
fn triangulate_loop(ids: &[u32], points: &[Vec2], extent: f64) -> Result<Vec<[u32; 3]>> {
    validate_simple_loop(points, extent)?;
    let original_twice_area: f64 = (0..ids.len()).map(|i| points[i].cross(points[(i + 1) % ids.len()])).sum();
    let area_tol = 1e-12 * extent * extent;
    if !original_twice_area.is_finite() || original_twice_area <= area_tol {
        return Err(KernelError::Invalid("zero or inverted projected hole area"));
    }
    let mut remaining: Vec<usize> = (0..ids.len()).collect();
    let mut triangles = Vec::with_capacity(ids.len() - 2);
    while remaining.len() > 3 {
        let n = remaining.len();
        let mut candidate: Option<(usize, f64)> = None;
        for slot in 0..n {
            let ia = remaining[(slot + n - 1) % n];
            let ib = remaining[slot];
            let ic = remaining[(slot + 1) % n];
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            if orientation2d(a, b, c) != Some(Ordering::Greater) { continue; }
            let twice_area = (b - a).cross(c - a);
            if !twice_area.is_finite() || twice_area <= area_tol { continue; }
            let mut blocked = false;
            for &other in &remaining {
                if other != ia && other != ib && other != ic && triangle_contains_or_touches(a, b, c, points[other])? {
                    blocked = true;
                    break;
                }
            }
            if blocked { continue; }
            // Compact triangle quality proxy: avoids repeatedly choosing
            // near-zero-angle ears when a better ear exists.
            let edge_squares = (b - a).len2() + (c - b).len2() + (a - c).len2();
            let quality = twice_area / edge_squares;
            if candidate.is_none_or(|(_, previous)| quality > previous) {
                candidate = Some((slot, quality));
            }
        }
        let (slot, _) = candidate.ok_or(KernelError::Invalid("no valid ear for planar hole"))?;
        let n = remaining.len();
        let (a, b, c) = (remaining[(slot + n - 1) % n], remaining[slot], remaining[(slot + 1) % n]);
        triangles.push([ids[a], ids[c], ids[b]]);
        remaining.remove(slot);
    }
    let [a, b, c] = [remaining[0], remaining[1], remaining[2]];
    if orientation2d(points[a], points[b], points[c]) != Some(Ordering::Greater)
        || (points[b] - points[a]).cross(points[c] - points[a]) <= area_tol {
        return Err(KernelError::Invalid("degenerate final hole triangle"));
    }
    triangles.push([ids[a], ids[c], ids[b]]);
    // Area conservation independently guards all ear selections, including
    // accidental omission from a near-degenerate or numerically fragile loop.
    let covered: f64 = triangles.iter().map(|t| {
        let ia = ids.iter().position(|id| *id == t[0]).unwrap_or(0);
        let ib = ids.iter().position(|id| *id == t[1]).unwrap_or(0);
        let ic = ids.iter().position(|id| *id == t[2]).unwrap_or(0);
        (points[ib] - points[ia]).cross(points[ic] - points[ia]).abs()
    }).sum();
    if !covered.is_finite() || (covered - original_twice_area).abs() > 1e-8 * original_twice_area.max(extent * extent * 1e-12) {
        return Err(KernelError::Invalid("triangulated patch does not conserve area"));
    }
    Ok(triangles)
}

/// Triangulates a simple planar convex OR concave inner loop. Outer boundaries
/// are rejected by comparing loop winding with adjacent polygon orientation.
/// Does not support nonplanar holes, global 3D intersection repair or fairing.
pub fn polygon_mesh_fill_hole(mesh: &PolygonMesh, revision: u64, picked_revision: u64, loop_index: u32) -> Result<PolygonFillResult> {
    if revision != picked_revision {
        return Err(KernelError::Conflict { expected: picked_revision, actual: revision });
    }
    polygon_mesh_validate(mesh)?;
    let report = polygon_mesh_boundary_loops(mesh)?;
    if !report.unresolved_edges.is_empty() || !report.non_manifold_edges.is_empty() || !report.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("ambiguous source topology"));
    }
    let loop_data = report.closed_loops.get(loop_index as usize).ok_or(KernelError::Invalid("boundary loop index"))?;
    let ids = &loop_data.vertices;
    if ids.len() < 3 || ids.len() > 256 {
        return Err(KernelError::Invalid("unsupported boundary size"));
    }
    if mesh.faces.len().checked_add(ids.len() - 2).is_none_or(|n| n > 1_000_000) {
        return Err(KernelError::Budget);
    }
    let next_revision = revision.checked_add(1).ok_or(KernelError::Budget)?;
    let origin = mesh.vertices[ids[0] as usize];
    let mut extent = 0.0_f64;
    let mut area = Vec3::ZERO;
    for i in 0..ids.len() {
        let a = mesh.vertices[ids[i] as usize] - origin;
        let b = mesh.vertices[ids[(i + 1) % ids.len()] as usize] - origin;
        extent = extent.max(norm(a));
        area = area + a.cross(b);
    }
    if !extent.is_finite() || extent <= f64::EPSILON || norm(area) <= 1e-12 * extent * extent {
        return Err(KernelError::Invalid("degenerate boundary"));
    }
    let normal = area * (1.0 / norm(area));
    for &id in ids {
        if (mesh.vertices[id as usize] - origin).dot(normal).abs() > 1e-7 * extent {
            return Err(KernelError::Invalid("nonplanar boundary"));
        }
    }
    let points = project_loop(mesh, ids, normal, origin);
    let patch_triangles = triangulate_loop(ids, &points, extent)?;
    let topo = polygon_mesh_topology(mesh)?;
    for &half_id in &loop_data.halfedges {
        let h = &topo.halfedges[half_id as usize];
        let face = mesh.faces[h.face as usize].indices();
        let a = mesh.vertices[face[0] as usize];
        let b = mesh.vertices[face[1] as usize];
        let c = mesh.vertices[face[2] as usize];
        let facing = (b - a).cross(c - a);
        if norm(facing) <= f64::EPSILON || facing.dot(normal) / norm(facing) >= -0.9 {
            return Err(KernelError::Invalid("exterior or nonplanar surrounding face"));
        }
    }
    let mut output = mesh.clone();
    let mut new_face_indices = Vec::new();
    for triangle in patch_triangles {
        let index = u32::try_from(output.faces.len()).map_err(|_| KernelError::Budget)?;
        new_face_indices.push(index);
        output.faces.push(PolygonFace::Triangle(triangle));
    }
    polygon_mesh_validate(&output)?;
    let after = polygon_mesh_topology(&output)?;
    if !after.non_manifold_edges.is_empty() || !after.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("invalid patch topology"));
    }
    Ok(PolygonFillResult { mesh: output, revision: next_revision, boundary_vertices: ids.clone(), new_face_indices })
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
        match polygon_mesh_boundary_loops(mesh) {
            Ok(r) => r.closed_loops.iter().position(|l| l.vertices.iter().all(|&v| v >= 4)).map_or(u32::MAX, |i| i as u32),
            Err(_) => u32::MAX,
        }
    }
    #[test]
    fn fills_inner_hole_and_preserves_original() {
        let source = ring();
        let snapshot = source.clone();
        let result = polygon_mesh_fill_hole(&source, 8, 8, inner(&source));
        assert!(result.is_ok());
        if let Ok(r) = result {
            assert_eq!(r.revision, 9);
            assert_eq!(r.new_face_indices, vec![4, 5]);
            assert_eq!(r.mesh.faces.len(), 6);
            assert!(polygon_mesh_boundary_loops(&r.mesh).is_ok_and(|b| b.closed_loops.len() == 1));
        }
        assert_eq!(source, snapshot);
    }
    #[test]
    fn rejects_outer_perimeter() {
        let source = ring();
        let outer = if inner(&source) == 0 { 1 } else { 0 };
        assert!(polygon_mesh_fill_hole(&source, 0, 0, outer).is_err());
    }
    #[test]
    fn rejects_stale_selection() {
        assert_eq!(polygon_mesh_fill_hole(&ring(), 3, 2, 0), Err(KernelError::Conflict { expected: 2, actual: 3 }));
    }
    #[test]
    fn rejects_nonplanar_hole_loops() {
        let mut source = ring();
        source.vertices[4].z = 0.1;
        assert!(polygon_mesh_fill_hole(&source, 0, 0, inner(&source)).is_err());
    }
    #[test]
    fn rejects_invalid_index_and_revision_overflow() {
        let source = ring();
        assert!(polygon_mesh_fill_hole(&source, 0, 0, 99).is_err());
        assert_eq!(polygon_mesh_fill_hole(&source, u64::MAX, u64::MAX, inner(&source)), Err(KernelError::Budget));
    }

    fn concave_ring() -> PolygonMesh {
        let mut mesh = ring();
        mesh.vertices.push(Vec3::new(2., 1.6, 0.));
        mesh.faces.remove(0);
        mesh.faces.splice(0..0, [
            PolygonFace::Triangle([0, 1, 5]),
            PolygonFace::Triangle([0, 5, 8]),
            PolygonFace::Triangle([0, 8, 4]),
        ]);
        mesh
    }

    #[test]
    fn fills_concave_planar_hole_without_stepping_outside_loop() {
        let original = concave_ring();
        let pick = inner(&original);
        assert!(polygon_mesh_validate(&original).is_ok());
        let filled = polygon_mesh_fill_hole(&original, 3, 3, pick).unwrap();
        assert_eq!(filled.revision, 4);
        assert_eq!(filled.new_face_indices.len(), 3);
        assert_eq!(filled.boundary_vertices.len(), 5);
        assert_eq!(filled.mesh.faces.len(), original.faces.len() + 3);
        assert_eq!(&filled.mesh.faces[..original.faces.len()], &original.faces);
        let after = polygon_mesh_boundary_loops(&filled.mesh).unwrap();
        assert_eq!(after.closed_loops.len(), 1);
        assert!(after.non_manifold_edges.is_empty());
        assert!(after.inconsistent_winding_edges.is_empty());
        assert_eq!(original, concave_ring(), "operation must be atomic");
    }

    #[test]
    fn robust_simple_loop_validation_rejects_crossings_and_collapsed_edges() {
        let crossing = [
            Vec2::new(0., 0.), Vec2::new(4., 3.),
            Vec2::new(0., 4.), Vec2::new(4., 0.),
        ];
        assert!(validate_simple_loop(&crossing, 5.).is_err());
        let collapsed = [
            Vec2::new(0., 0.), Vec2::new(4., 0.),
            Vec2::new(4., 0.), Vec2::new(0., 4.),
        ];
        assert!(validate_simple_loop(&collapsed, 5.).is_err());
    }

    #[test]
    fn concave_rejects_outer_boundary_and_stale_picks() {
        let source = concave_ring();
        let inner_index = inner(&source);
        let outer = if inner_index == 0 { 1 } else { 0 };
        assert!(polygon_mesh_fill_hole(&source, 1, 1, outer).is_err());
        assert_eq!(polygon_mesh_fill_hole(&source, 12, 11, inner_index), Err(KernelError::Conflict { expected: 11, actual: 12 }));
    }
}
