//! Shared polygonal sweep construction. A geometry-preview/mesh path, *not*
//! a tolerance-certified trimmed NURBS/BRep Rhino Sweep replacement.
//! Arc-length rail stations and frames are delegated to rail_frames.rs.
use crate::rail_frames::{normalized, rail_frames as sample_rail};
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_validate};
use cadcraft_geom::Vec3;

const MAX_GRID: usize = 256;
const MAX_VERTICES: usize = 100_000;

fn ensure(p: Vec3) -> Result<Vec3> {
    if p.is_finite() && p.x.abs().max(p.y.abs()).max(p.z.abs()) <= 1e12 {
        Ok(p)
    } else {
        Err(KernelError::Invalid("nonfinite or oversized sweep coordinate"))
    }
}
fn idx(i: usize) -> Result<u32> {
    u32::try_from(i).map_err(|_| KernelError::Budget)
}
/// Canonical row-oriented ruled surface builder reused by 1-rail and 2-rail
/// sweeps. Profile row vertices share a consistent seam and winding.
pub fn loft_section_grid(rows: &[Vec<Vec3>], closed_profile: bool, cap_ends: bool) -> Result<PolygonMesh> {
    if rows.len() < 2 || rows.len() > MAX_GRID {
        return Err(KernelError::Invalid("loft needs 2-256 stations"));
    }
    let width = rows[0].len();
    if width < if closed_profile { 3 } else { 2 } || width > MAX_GRID || rows.iter().any(|r| r.len() != width) || (cap_ends && !closed_profile) {
        return Err(KernelError::Invalid("mismatched loft sections or invalid end cap"));
    }
    let face_width = if closed_profile { width } else { width - 1 };
    let n = rows.len().checked_mul(width).and_then(|n| n.checked_add(if cap_ends { 2 } else { 0 })).ok_or(KernelError::Budget)?;
    let count =
        (rows.len() - 1).checked_mul(face_width).and_then(|n| n.checked_add(if cap_ends { 2 * width } else { 0 })).ok_or(KernelError::Budget)?;
    if n > MAX_VERTICES || count > MAX_VERTICES {
        return Err(KernelError::Budget);
    }
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    vertices.try_reserve_exact(n).map_err(|_| KernelError::Budget)?;
    faces.try_reserve_exact(count).map_err(|_| KernelError::Budget)?;
    for row in rows {
        for &p in row {
            vertices.push(ensure(p)?);
        }
    }
    for i in 0..rows.len() - 1 {
        for j in 0..face_width {
            let next = if j + 1 == width { 0 } else { j + 1 };
            let a = idx(i * width + j)?;
            let b = idx(i * width + next)?;
            let c = idx((i + 1) * width + next)?;
            let d = idx((i + 1) * width + j)?;
            faces.push(PolygonFace::Quad([a, b, c, d]));
        }
    }
    if cap_ends {
        let avg = |row: &[Vec3]| row.iter().fold(Vec3::ZERO, |sum, p| sum + *p) * (1. / row.len() as f64);
        let start = idx(vertices.len())?;
        vertices.push(ensure(avg(&rows[0]))?);
        let end = idx(vertices.len())?;
        vertices.push(ensure(avg(&rows[rows.len() - 1]))?);
        let last = (rows.len() - 1) * width;
        for j in 0..width {
            let next = (j + 1) % width;
            faces.push(PolygonFace::Triangle([start, idx(next)?, idx(j)?]));
            faces.push(PolygonFace::Triangle([end, idx(last + j)?, idx(last + next)?]));
        }
    }
    let mesh = PolygonMesh { vertices, faces };
    polygon_mesh_validate(&mesh)?;
    Ok(mesh)
}
/// Initial Sweep1 mesh: section shape defined in model coordinates at rail
/// origin, transported without scale along a 3D rail. Multiple sections,
/// NURBS refit, roadlike styles, closed-rail twist and G-continuity await a
/// separate exact surface builder.
pub fn sweep1_mesh(rail: &[Vec3], guide_up: Vec3, profile: &[Vec3], stations: usize, closed_profile: bool) -> Result<PolygonMesh> {
    if profile.len() > MAX_GRID || profile.len() < if closed_profile { 3 } else { 2 } {
        return Err(KernelError::Invalid("sweep profile size"));
    }
    for &p in profile {
        ensure(p)?;
    }
    let frames = sample_rail(rail, stations, guide_up)?;
    let first = frames[0];
    let mut rows = Vec::new();
    rows.try_reserve_exact(stations).map_err(|_| KernelError::Budget)?;
    for frame in frames {
        let mut row = Vec::new();
        row.try_reserve_exact(profile.len()).map_err(|_| KernelError::Budget)?;
        for &p in profile {
            let d = p - first.origin;
            row.push(ensure(frame.map(d.dot(first.tangent), d.dot(first.side), d.dot(first.up)))?);
        }
        rows.push(row);
    }
    loft_section_grid(&rows, closed_profile, false)
}
/// Initial Sweep2 mesh: corresponding rail positions at normalized arc length,
/// with normalized profile transverse X in [0,1] and physical height in Y.
/// Profiles must anchor both rails; exact NURBS seam/continuity not supported.
pub fn sweep2_mesh(rail_a: &[Vec3], rail_b: &[Vec3], section: &[Vec3], stations: usize) -> Result<PolygonMesh> {
    if !(2..=MAX_GRID).contains(&section.len()) {
        return Err(KernelError::Invalid("sweep2 profile size"));
    }
    for &p in section {
        ensure(p)?;
        if p.x < -1e-9 || p.x > 1. + 1e-9 || p.z.abs() > 1e-9 {
            return Err(KernelError::Invalid("sweep2 profile coordinate domain"));
        }
    }
    let start = section[0];
    let end = section[section.len() - 1];
    if start.x.abs() > 1e-8 || end.x - 1. > 1e-8 || (end.x - 1.).abs() > 1e-8 || start.y.abs() > 1e-8 || end.y.abs() > 1e-8 {
        return Err(KernelError::Invalid("sweep2 section endpoints must meet the rails"));
    }
    let a = sample_rail(rail_a, stations, Vec3::Z).or_else(|_| sample_rail(rail_a, stations, Vec3::new(0., 1., 0.)))?;
    let b = sample_rail(rail_b, stations, Vec3::Z).or_else(|_| sample_rail(rail_b, stations, Vec3::new(0., 1., 0.)))?;
    let mut rows = Vec::new();
    for (left, right) in a.iter().zip(b.iter()) {
        let lateral = right.origin - left.origin;
        if lateral.len() < 1e-8 {
            return Err(KernelError::Invalid("sweep2 rails touch"));
        }
        let normal = normalized(left.tangent.cross(lateral))?;
        let mut row = Vec::new();
        for p in section {
            row.push(ensure(left.origin + lateral * p.x + normal * p.y)?);
        }
        rows.push(row);
    }
    loft_section_grid(&rows, false, false)
}
/// Circular Pipe: axial linear radius taper, optional hollow wall, and flat
/// caps. Zero wall gives a solid; positive wall produces two annular walls.
/// Round caps, unlimited intermediate radius keys and SubD are not supported.
pub fn pipe_mesh(
    rail: &[Vec3],
    guide_up: Vec3,
    start_radius: f64,
    end_radius: f64,
    wall: f64,
    stations: usize,
    sides: usize,
    flat_caps: bool,
) -> Result<PolygonMesh> {
    if !(8..=MAX_GRID).contains(&sides) || !(2..=MAX_GRID).contains(&stations) {
        return Err(KernelError::Budget);
    }
    if !start_radius.is_finite()
        || !end_radius.is_finite()
        || start_radius <= 1e-9
        || end_radius <= 1e-9
        || start_radius > 1e9
        || end_radius > 1e9
        || !wall.is_finite()
        || wall < 0.
        || wall >= start_radius.min(end_radius)
    {
        return Err(KernelError::Invalid("pipe radii / thickness"));
    }
    let frames = sample_rail(rail, stations, guide_up)?;
    let mut rows = Vec::new();
    for (i, f) in frames.iter().enumerate() {
        let t = i as f64 / (stations - 1) as f64;
        let radius = start_radius * (1. - t) + end_radius * t;
        let mut row = Vec::new();
        for j in 0..sides {
            let theta = std::f64::consts::TAU * j as f64 / sides as f64;
            row.push(ensure(f.origin + f.side * (radius * theta.cos()) + f.up * (radius * theta.sin()))?);
        }
        rows.push(row);
    }
    if wall == 0. {
        return loft_section_grid(&rows, true, flat_caps);
    }
    let n = stations.checked_mul(sides).and_then(|v| v.checked_mul(2)).ok_or(KernelError::Budget)?;
    if n > MAX_VERTICES {
        return Err(KernelError::Budget);
    }
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    vertices.try_reserve_exact(n).map_err(|_| KernelError::Budget)?;
    for (i, frame) in frames.iter().enumerate() {
        vertices.extend_from_slice(&rows[i]);
        let t = i as f64 / (stations - 1) as f64;
        let r = start_radius * (1. - t) + end_radius * t - wall;
        for j in 0..sides {
            let theta = std::f64::consts::TAU * j as f64 / sides as f64;
            vertices.push(ensure(frame.origin + frame.side * (r * theta.cos()) + frame.up * (r * theta.sin()))?);
        }
    }
    let row_width = 2 * sides;
    for i in 0..stations - 1 {
        for j in 0..sides {
            let k = (j + 1) % sides;
            let a = i * row_width;
            let b = (i + 1) * row_width;
            faces.push(PolygonFace::Quad([idx(a + j)?, idx(a + k)?, idx(b + k)?, idx(b + j)?]));
            faces.push(PolygonFace::Quad([idx(a + sides + k)?, idx(a + sides + j)?, idx(b + sides + j)?, idx(b + sides + k)?]));
        }
    }
    if flat_caps {
        let last = (stations - 1) * row_width;
        for j in 0..sides {
            let k = (j + 1) % sides;
            faces.push(PolygonFace::Quad([idx(sides + j)?, idx(sides + k)?, idx(k)?, idx(j)?]));
            faces.push(PolygonFace::Quad([idx(last + j)?, idx(last + k)?, idx(last + sides + k)?, idx(last + sides + j)?]));
        }
    }
    let mesh = PolygonMesh { vertices, faces };
    polygon_mesh_validate(&mesh)?;
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }
    #[test]
    fn pipe_has_closed_manifold_solid_and_hollow_caps() {
        let rail = [p(0., 0., 0.), p(10., 0., 0.)];
        for wall in [0., 0.2] {
            let pipe = pipe_mesh(&rail, Vec3::Z, 1., 1., wall, 4, 12, true).unwrap();
            let t = crate::polygon_mesh_topology(&pipe).unwrap();
            assert!(t.boundary_edges.is_empty());
            assert!(t.non_manifold_edges.is_empty());
            assert!(t.inconsistent_winding_edges.is_empty());
        }
        let uncapped = pipe_mesh(&rail, Vec3::Z, 1., 1., 0., 4, 12, false).unwrap();
        assert!(!crate::polygon_mesh_topology(&uncapped).unwrap().boundary_edges.is_empty());
    }
    #[test]
    fn sweep1_uses_same_transported_frames_and_retains_sections() {
        let rail = [p(0., 0., 0.), p(4., 0., 0.), p(4., 4., 0.)];
        let profile = [p(0., 1., 0.), p(0., 0., 1.), p(0., -1., 0.), p(0., 0., -1.)];
        let m = sweep1_mesh(&rail, Vec3::Z, &profile, 3, true).unwrap();
        assert_eq!(m.vertices.len(), 12);
        assert_eq!(m.faces.len(), 8);
        assert!((m.vertices[0] - profile[0]).len() < 1e-8);
    }
    #[test]
    fn sweep2_respects_both_rails_and_rejects_bad_section() {
        let a = [p(0., 0., 0.), p(6., 0., 0.)];
        let b = [p(0., 2., 0.), p(6., 2., 0.)];
        let profile = [p(0., 0., 0.), p(0.5, 1., 0.), p(1., 0., 0.)];
        let m = sweep2_mesh(&a, &b, &profile, 4).unwrap();
        assert_eq!(m.vertices.len(), 12);
        assert_eq!(m.faces.len(), 6);
        assert!((m.vertices[0] - a[0]).len() < 1e-8);
        assert!((m.vertices[2] - b[0]).len() < 1e-8);
        assert!(sweep2_mesh(&a, &a, &profile, 4).is_err());
    }
    #[test]
    fn rejects_invalid_pipe_without_partial_mesh() {
        let rail = [p(0., 0., 0.), p(1., 0., 0.)];
        assert!(pipe_mesh(&rail, Vec3::Z, 1., 1., 2., 2, 12, true).is_err());
        assert!(pipe_mesh(&rail, Vec3::new(1., 0., 0.), 1., 1., 0., 2, 12, true).is_err());
    }
}
