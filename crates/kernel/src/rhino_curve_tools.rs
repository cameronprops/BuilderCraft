//! Rhino-inspired, bounded native curve tools for WorldBuilder.
//! Explicit alpha scope: linear polylines for Join/Split/Trim/Extend/Offset,
//! control-fitted NURBS Rebuild, and polygonal Pipe/Sweep previews.
//! Output mesh is not an exact BRep and is not a fabrication tolerance guarantee.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_validate, polyline_length, polyline_offset_xy, polyline_point_at_station};
use cadcraft_geom::{Vec3, nurbs3d::Curve};

const MAX_PATH: usize = 4_096;
const MAX_SECTION: usize = 128;
const EPS: f64 = 1e-9;

fn valid_path(points: &[Vec3]) -> Result<()> {
    let length = polyline_length(points)?;
    if points.len() > MAX_PATH {
        return Err(KernelError::Budget);
    }
    if length <= EPS {
        return Err(KernelError::Invalid("zero length path"));
    }
    if points.windows(2).any(|p| (p[1] - p[0]).len() <= EPS) {
        return Err(KernelError::Invalid("duplicate path station"));
    }
    Ok(())
}
fn length_ok(value: f64) -> bool {
    value.is_finite() && (0.0..=1e12).contains(&value)
}

/// Connect two open polylines by their nearest matching endpoints.
/// Exact end is retained; second start is snapped within tolerance.
pub fn curve_join(a: &[Vec3], b: &[Vec3], tolerance: f64) -> Result<Vec<Vec3>> {
    valid_path(a)?;
    valid_path(b)?;
    if !length_ok(tolerance) {
        return Err(KernelError::Invalid("join tolerance"));
    }
    let aa = [a[0], a[a.len() - 1]];
    let bb = [b[0], b[b.len() - 1]];
    let choices = [
        ((aa[1] - bb[0]).len(), false, false),
        ((aa[1] - bb[1]).len(), false, true),
        ((aa[0] - bb[0]).len(), true, false),
        ((aa[0] - bb[1]).len(), true, true),
    ];
    let &(best, ra, rb) = choices.iter().min_by(|x, y| x.0.total_cmp(&y.0)).ok_or(KernelError::Invalid("join choices"))?;
    if best > tolerance {
        return Err(KernelError::Invalid("join endpoints outside tolerance"));
    }
    if a.len().checked_add(b.len()).is_none_or(|n| n - 1 > MAX_PATH) {
        return Err(KernelError::Budget);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(a.len() + b.len() - 1).map_err(|_| KernelError::Budget)?;
    if ra {
        out.extend(a.iter().rev().copied());
    } else {
        out.extend_from_slice(a);
    }
    if rb {
        out.extend(b.iter().rev().skip(1).copied());
    } else {
        out.extend(b.iter().skip(1).copied());
    }
    Ok(out)
}
/// Two retained path fragments including the interpolated shared split point.
pub fn curve_split(points: &[Vec3], station: f64) -> Result<(Vec<Vec3>, Vec<Vec3>)> {
    valid_path(points)?;
    let total = polyline_length(points)?;
    if !station.is_finite() || station <= EPS || station >= total - EPS {
        return Err(KernelError::Invalid("split interior station required"));
    }
    let pivot = polyline_point_at_station(points, station)?;
    let mut left = vec![points[0]];
    let mut right = vec![pivot];
    let mut traveled = 0.0;
    let mut located = false;
    for pair in points.windows(2) {
        let next = traveled + (pair[1] - pair[0]).len();
        if !located {
            if next < station - EPS {
                left.push(pair[1]);
            } else {
                if (left[left.len() - 1] - pivot).len() > EPS {
                    left.push(pivot);
                }
                if (pair[1] - pivot).len() > EPS {
                    right.push(pair[1]);
                }
                located = true;
            }
        } else {
            right.push(pair[1]);
        }
        traveled = next;
    }
    if left.len() < 2 || right.len() < 2 {
        return Err(KernelError::Invalid("degenerate split"));
    }
    Ok((left, right))
}
/// Keep the portion of a linear polyline between two absolute stations.
pub fn curve_trim(points: &[Vec3], start: f64, end: f64) -> Result<Vec<Vec3>> {
    valid_path(points)?;
    let total = polyline_length(points)?;
    if !start.is_finite() || !end.is_finite() || start < 0.0 || end > total || end - start <= EPS {
        return Err(KernelError::Invalid("trim station interval"));
    }
    let mut out = vec![polyline_point_at_station(points, start)?];
    let mut covered = 0.0;
    for pair in points.windows(2) {
        covered += (pair[1] - pair[0]).len();
        if covered > start + EPS && covered < end - EPS {
            out.push(pair[1]);
        }
    }
    let finish = polyline_point_at_station(points, end)?;
    if (finish - out[out.len() - 1]).len() > EPS {
        out.push(finish);
    }
    if out.len() < 2 {
        return Err(KernelError::Invalid("empty trim"));
    }
    Ok(out)
}
/// Extend an open polyline by the tangent of its first/last segment.
pub fn curve_extend(points: &[Vec3], distance: f64, at_start: bool) -> Result<Vec<Vec3>> {
    valid_path(points)?;
    if !distance.is_finite() || distance <= EPS || distance > 1e12 {
        return Err(KernelError::Invalid("extend length"));
    }
    if points.len() >= MAX_PATH {
        return Err(KernelError::Budget);
    }
    let tangent = if at_start { (points[0] - points[1]).normalized() } else { (points[points.len() - 1] - points[points.len() - 2]).normalized() };
    let tip = if at_start { points[0] } else { points[points.len() - 1] };
    let new_tip = tip + tangent * distance;
    if !new_tip.is_finite() {
        return Err(KernelError::Invalid("extend overflow"));
    }
    let mut out = Vec::with_capacity(points.len() + 1);
    if at_start {
        out.push(new_tip);
    }
    out.extend_from_slice(points);
    if !at_start {
        out.push(new_tip);
    }
    Ok(out)
}
/// Construct a genuine, editable nonrational clamped NURBS curve from
/// arc-length sampled polyline control locations. This is control fitting,
/// not a least-squares/interpolating Rebuild with guaranteed deviation.
pub fn curve_rebuild(points: &[Vec3], controls: usize, degree: usize) -> Result<Curve> {
    valid_path(points)?;
    if !(2..=4_096).contains(&controls) || degree == 0 || degree > 5 || degree >= controls {
        return Err(KernelError::Invalid("NURBS rebuild controls/degree"));
    }
    let total = polyline_length(points)?;
    let control =
        (0..controls).map(|i| polyline_point_at_station(points, total * (i as f64) / ((controls - 1) as f64))).collect::<Result<Vec<_>>>()?;
    Curve::from_control(control, degree).ok_or(KernelError::Invalid("NURBS reconstruction"))
}
/// Conservative XY offset adapter; no trimming or self-intersection resolution.
pub fn curve_offset(points: &[Vec3], distance: f64, miter_limit: f64) -> Result<Vec<Vec3>> {
    valid_path(points)?;
    polyline_offset_xy(points, distance, miter_limit)
}

fn frame(tangent: Vec3, previous_u: Option<Vec3>) -> Result<(Vec3, Vec3)> {
    let t = tangent.normalized();
    if t.len() < 0.9 {
        return Err(KernelError::Invalid("invalid sweep tangent"));
    }
    let guess = previous_u.unwrap_or_else(|| if t.z.abs() < 0.9 { Vec3::Z.cross(t) } else { Vec3::new(1., 0., 0.).cross(t) });
    let mut u = guess - t * guess.dot(t);
    if u.len() < EPS {
        let backup = if t.x.abs() < 0.9 { Vec3::new(1., 0., 0.) } else { Vec3::new(0., 1., 0.) };
        u = backup - t * backup.dot(t);
    }
    u = u.normalized();
    let v = t.cross(u).normalized();
    if v.len() < 0.9 {
        return Err(KernelError::Invalid("degenerate sweep frame"));
    }
    Ok((u, v))
}
fn tangent_at_vertices(path: &[Vec3], i: usize) -> Result<Vec3> {
    let t = if i == 0 {
        (path[1] - path[0]).normalized()
    } else if i == path.len() - 1 {
        (path[i] - path[i - 1]).normalized()
    } else {
        (path[i] - path[i - 1]).normalized() + (path[i + 1] - path[i]).normalized()
    };
    if t.len() <= EPS {
        return Err(KernelError::Invalid("sweep hairpin"));
    }
    Ok(t.normalized())
}
fn profile(section: &[Vec3], normalized_width: bool) -> Result<Vec<(f64, f64)>> {
    if section.len() < 3 || section.len() > MAX_SECTION + 1 {
        return Err(KernelError::Invalid("sweep section point count"));
    }
    let limit = if (section[0] - section[section.len() - 1]).len() <= EPS { section.len() - 1 } else { section.len() };
    if !(3..=MAX_SECTION).contains(&limit) {
        return Err(KernelError::Invalid("sweep section count"));
    }
    let mut ring = Vec::with_capacity(limit);
    for p in &section[..limit] {
        if !p.is_finite() || p.z.abs() > EPS || (normalized_width && (p.x < -EPS || p.x > 1.0 + EPS)) {
            return Err(KernelError::Invalid("section must lie in XY and normalized X for Sweep2"));
        }
        ring.push((p.x, p.y));
    }
    // Strictly convex profile only; guarantees valid triangular caps.
    let mut sign = 0.0f64;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        let c = ring[(i + 2) % ring.len()];
        let turn = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        if !turn.is_finite() || turn.abs() <= EPS {
            return Err(KernelError::Invalid("degenerate or non-strictly-convex section"));
        }
        if sign == 0.0 {
            sign = turn.signum();
        }
        if turn * sign < 0.0 {
            return Err(KernelError::Invalid("concave sweep section unsupported"));
        }
    }
    if sign < 0.0 {
        ring.reverse();
    }
    Ok(ring)
}
/// Join ordered polygonal rings with side triangles and end caps.
/// Manifold face edges are shared by indexed vertices; global self intersections
/// are not checked. Caller decides whether local profiles overlap elsewhere.
fn ring_mesh(rings: Vec<Vec<Vec3>>) -> Result<PolygonMesh> {
    let n = rings.len();
    let m = rings.first().map(Vec::len).unwrap_or(0);
    if n < 2 || m < 3 || rings.iter().any(|r| r.len() != m) {
        return Err(KernelError::Invalid("sweep ring topology"));
    }
    let count = n.checked_mul(m).ok_or(KernelError::Budget)?;
    if count.checked_mul(3).is_none_or(|f| f > 200_000) {
        return Err(KernelError::Budget);
    }
    let mut vertices = Vec::with_capacity(count + 2);
    for ring in &rings {
        vertices.extend_from_slice(ring);
    }
    let avg = |ring: &[Vec3]| ring.iter().fold(Vec3::ZERO, |a, &b| a + b) * (1. / (m as f64));
    let start_id = u32::try_from(vertices.len()).map_err(|_| KernelError::Budget)?;
    vertices.push(avg(&rings[0]));
    let end_id = u32::try_from(vertices.len()).map_err(|_| KernelError::Budget)?;
    vertices.push(avg(&rings[n - 1]));
    let mut faces = Vec::with_capacity((n - 1) * m * 2 + m * 2);
    for i in 0..n - 1 {
        for j in 0..m {
            let next = (j + 1) % m;
            let a = u32::try_from(i * m + j).map_err(|_| KernelError::Budget)?;
            let b = u32::try_from(i * m + next).map_err(|_| KernelError::Budget)?;
            let c = u32::try_from((i + 1) * m + j).map_err(|_| KernelError::Budget)?;
            let d = u32::try_from((i + 1) * m + next).map_err(|_| KernelError::Budget)?;
            faces.push(PolygonFace::Triangle([a, b, c]));
            faces.push(PolygonFace::Triangle([b, d, c]));
        }
    }
    for j in 0..m {
        let next = (j + 1) % m;
        faces.push(PolygonFace::Triangle([start_id, next as u32, j as u32]));
        faces.push(PolygonFace::Triangle([end_id, ((n - 1) * m + j) as u32, ((n - 1) * m + next) as u32]));
    }
    let mesh = PolygonMesh { vertices, faces };
    polygon_mesh_validate(&mesh)?;
    Ok(mesh)
}
/// Faceted closed pipe on a 3D polyline rail, with bounded parallel-transport frames.
pub fn curve_pipe(rail: &[Vec3], radius: f64, sides: usize) -> Result<PolygonMesh> {
    valid_path(rail)?;
    if !radius.is_finite() || radius <= EPS || radius > 1e12 || !(6..=96).contains(&sides) {
        return Err(KernelError::Invalid("pipe radius/sides"));
    }
    let section = (0..sides)
        .map(|i| {
            let a = (i as f64) * std::f64::consts::TAU / (sides as f64);
            Vec3::new(radius * a.cos(), radius * a.sin(), 0.)
        })
        .collect::<Vec<_>>();
    curve_sweep1(rail, &section)
}
/// One rail, convex planar section; returns a capped **mesh**, not an exact BRep.
pub fn curve_sweep1(rail: &[Vec3], section: &[Vec3]) -> Result<PolygonMesh> {
    valid_path(rail)?;
    let section = profile(section, false)?;
    let mut rings = Vec::with_capacity(rail.len());
    let mut prev = None;
    for (i, &center) in rail.iter().enumerate() {
        let (u, v) = frame(tangent_at_vertices(rail, i)?, prev)?;
        let ring = section.iter().map(|&(x, y)| center + u * x + v * y).collect();
        rings.push(ring);
        prev = Some(u);
    }
    ring_mesh(rings)
}
/// Two-rail mesh sweep: the local section x=0 and x=1 match the rails.
/// Both rails must share station indices, direction and non-degenerate spacing.
/// No automatic reparameterization, frame-twist, or OCCT BRep surface here.
pub fn curve_sweep2(rail_a: &[Vec3], rail_b: &[Vec3], section: &[Vec3]) -> Result<PolygonMesh> {
    valid_path(rail_a)?;
    valid_path(rail_b)?;
    if rail_a.len() != rail_b.len() {
        return Err(KernelError::Invalid("Sweep2 paired rail stations"));
    }
    let section = profile(section, true)?;
    let centers = rail_a.iter().zip(rail_b).map(|(&a, &b)| (a + b) * 0.5).collect::<Vec<_>>();
    valid_path(&centers)?;
    let mut rings = Vec::with_capacity(rail_a.len());
    for i in 0..rail_a.len() {
        let span = rail_b[i] - rail_a[i];
        if span.len() <= EPS {
            return Err(KernelError::Invalid("Sweep2 collapsed rails"));
        }
        let t = tangent_at_vertices(&centers, i)?;
        let v = t.cross(span).normalized();
        if v.len() <= EPS {
            return Err(KernelError::Invalid("Sweep2 rails parallel to direction"));
        }
        let ring = section.iter().map(|&(x, y)| rail_a[i] + span * x + v * y).collect();
        rings.push(ring);
    }
    ring_mesh(rings)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }
    fn straight() -> Vec<Vec3> {
        vec![p(0., 0., 0.), p(3., 0., 0.), p(6., 0., 0.)]
    }
    fn square() -> Vec<Vec3> {
        vec![p(-1., -1., 0.), p(1., -1., 0.), p(1., 1., 0.), p(-1., 1., 0.)]
    }
    #[test]
    fn joins_reverse_and_checks_tolerance() {
        let a = vec![p(0., 0., 0.), p(1., 0., 0.)];
        let b = vec![p(3., 0., 0.), p(1., 0., 0.)];
        assert_eq!(curve_join(&a, &b, 0.), Ok(vec![p(0., 0., 0.), p(1., 0., 0.), p(3., 0., 0.)]));
        assert!(curve_join(&a, &[p(5., 0., 0.), p(6., 0., 0.)], 1.).is_err());
    }
    #[test]
    fn split_trim_extend_work_at_vertex_and_interior() {
        let (a, b) = curve_split(&straight(), 4.).unwrap();
        assert_eq!(a, vec![p(0., 0., 0.), p(3., 0., 0.), p(4., 0., 0.)]);
        assert_eq!(b, vec![p(4., 0., 0.), p(6., 0., 0.)]);
        assert_eq!(curve_trim(&straight(), 1., 5.), Ok(vec![p(1., 0., 0.), p(3., 0., 0.), p(5., 0., 0.)]));
        assert_eq!(curve_extend(&straight(), 2., true).unwrap()[0], p(-2., 0., 0.));
        assert!(curve_split(&straight(), 0.).is_err());
    }
    #[test]
    fn rebuild_is_true_valid_clamped_nurbs_and_offset_uses_shared_solver() {
        let rebuilt = curve_rebuild(&straight(), 6, 3).unwrap();
        assert!(rebuilt.valid());
        assert_eq!(rebuilt.evaluate(0.), Some(p(0., 0., 0.)));
        assert_eq!(rebuilt.evaluate(1.), Some(p(6., 0., 0.)));
        assert_eq!(curve_offset(&[p(0., 0., 0.), p(4., 0., 0.)], 1., 4.), Ok(vec![p(0., 1., 0.), p(4., 1., 0.)]));
    }
    #[test]
    fn pipe_and_sweeps_are_valid_closed_meshes() {
        let pipe = curve_pipe(&straight(), 0.3, 16).unwrap();
        let one = curve_sweep1(&straight(), &square()).unwrap();
        let other = vec![p(0., 2., 0.), p(3., 2., 0.), p(6., 2., 0.)];
        let two = curve_sweep2(&straight(), &other, &[p(0., -1., 0.), p(1., -1., 0.), p(1., 1., 0.), p(0., 1., 0.)]).unwrap();
        for mesh in [pipe, one, two] {
            assert!(polygon_mesh_validate(&mesh).is_ok());
            let topology = crate::polygon_mesh_topology(&mesh).unwrap();
            assert!(topology.boundary_edges.is_empty());
            assert!(topology.non_manifold_edges.is_empty());
        }
    }
    #[test]
    fn invalid_sections_and_rails_fail() {
        assert!(curve_pipe(&straight(), 0., 8).is_err());
        assert!(curve_sweep1(&straight(), &[p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 1.)]).is_err());
        assert!(curve_sweep1(&straight(), &[p(0., 0., 0.), p(2., 0., 0.), p(1., 0.5, 0.), p(2., 2., 0.), p(0., 2., 0.)]).is_err());
        assert!(curve_sweep2(&straight(), &[p(0., 0., 0.), p(3., 0., 0.), p(6., 0., 0.)], &square()).is_err());
    }
}
