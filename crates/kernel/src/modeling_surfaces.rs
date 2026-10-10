//! Curve/surface helpers for the modeling acceptance suite.
//! Sampling is bounded; untrimmed rational NURBS only. No fabricated BRep
//! intersection, trimmed edge, manifold or collision guarantee.
use crate::{KernelError, PolygonMesh, Result, polygon_mesh_triangulate};
use cadcraft_geom::{Vec3, nurbs3d::Surface};

const MAX_INPUT: usize = 512;
const GRID: usize = 24;
fn finite(p: Vec3) -> bool {
    p.is_finite() && p.x.abs().max(p.y.abs()).max(p.z.abs()) <= 1e12
}
fn normalized(v: Vec3) -> Result<Vec3> {
    if !finite(v) || v.len() < 1e-10 {
        return Err(KernelError::Invalid("invalid direction"));
    }
    Ok(v * (1. / v.len()))
}
fn validate_points(points: &[Vec3]) -> Result<()> {
    if points.is_empty() || points.len() > MAX_INPUT || points.iter().any(|p| !finite(*p)) {
        Err(KernelError::Invalid("bounded finite source geometry required"))
    } else {
        Ok(())
    }
}
/// Two-sided triangle ray intersection for the candidate line; the closest
/// hit is selected by absolute signed distance. Does not select occluded
/// geometry via camera depth or infer user-facing face orientation.
fn ray_triangle(p: Vec3, d: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f64, [f64; 3])> {
    let e1 = b - a;
    let e2 = c - a;
    let h = d.cross(e2);
    let det = e1.dot(h);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1. / det;
    let r = p - a;
    let u = r.dot(h) * inv;
    if !(-1e-10..=1. + 1e-10).contains(&u) {
        return None;
    }
    let q = r.cross(e1);
    let v = d.dot(q) * inv;
    if v < -1e-10 || u + v > 1. + 1e-10 {
        return None;
    }
    let distance = e2.dot(q) * inv;
    if !distance.is_finite() {
        return None;
    }
    Some((distance, [1. - u - v, u, v]))
}
/// Project each source vertex along either sign of the requested direction,
/// retaining the nearest line/triangle hit. Every point must hit: a partial
/// projection is an error, not silently omitted geometry.
pub fn project_onto_mesh(points: &[Vec3], target: &PolygonMesh, direction: Vec3) -> Result<Vec<Vec3>> {
    validate_points(points)?;
    let d = normalized(direction)?;
    let triangles = polygon_mesh_triangulate(target)?.mesh;
    if triangles.triangles.is_empty() || triangles.triangles.len() > 200_000 {
        return Err(KernelError::Invalid("projection target has no supported triangles"));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(points.len()).map_err(|_| KernelError::Budget)?;
    for p in points {
        let mut best = None;
        for &[i, j, k] in &triangles.triangles {
            if let Some((t, _)) = ray_triangle(*p, d, triangles.vertices[i as usize], triangles.vertices[j as usize], triangles.vertices[k as usize])
                && best.is_none_or(|prev: f64| t.abs() < prev.abs())
            {
                best = Some(t);
            }
        }
        let t = best.ok_or(KernelError::Invalid("projection ray missed target mesh"))?;
        let q = *p + d * t;
        if !finite(q) {
            return Err(KernelError::Invalid("projected coordinate overflow"));
        }
        output.push(q);
    }
    Ok(output)
}
fn at(s: &Surface, u: f64, v: f64) -> Result<Vec3> {
    s.evaluate(u.clamp(0., 1.), v.clamp(0., 1.)).filter(|p| finite(*p)).ok_or(KernelError::Invalid("surface evaluation failed"))
}
fn derivatives(s: &Surface, u: f64, v: f64) -> Result<(Vec3, Vec3)> {
    let h = 1e-4_f64;
    let u0 = (u - h).max(0.);
    let u1 = (u + h).min(1.);
    let v0 = (v - h).max(0.);
    let v1 = (v + h).min(1.);
    if u1 == u0 || v1 == v0 {
        return Err(KernelError::Invalid("invalid surface derivative domain"));
    }
    Ok(((at(s, u1, v)? - at(s, u0, v)?) * (1. / (u1 - u0)), (at(s, u, v1)? - at(s, u, v0)?) * (1. / (v1 - v0))))
}
fn solve_step(du: Vec3, dv: Vec3, delta: Vec3) -> Option<(f64, f64)> {
    let a = du.dot(du);
    let b = du.dot(dv);
    let c = dv.dot(dv);
    let det = a * c - b * b;
    if !det.is_finite() || det <= 1e-18 * a.max(c).powi(2) {
        return None;
    }
    let x = du.dot(delta);
    let y = dv.dot(delta);
    Some(((x * c - y * b) / det, (y * a - x * b) / det))
}
/// Nearest UV by coarse global seed and damped Gauss-Newton. Conservatively
/// bounded to the untrimmed unit parameter domain. Source must be regular.
fn closest_uv(surface: &Surface, p: Vec3) -> Result<(f64, f64)> {
    if !surface.valid() {
        return Err(KernelError::Invalid("invalid NURBS source"));
    }
    let mut best = (f64::INFINITY, 0., 0.);
    for i in 0..=12 {
        for j in 0..=12 {
            let u = i as f64 / 12.;
            let v = j as f64 / 12.;
            let q = at(surface, u, v)?;
            let delta = q - p;
            let d = delta.dot(delta);
            if d < best.0 {
                best = (d, u, v);
            }
        }
    }
    let (mut u, mut v) = (best.1, best.2);
    for _ in 0..24 {
        let q = at(surface, u, v)?;
        let (du, dv) = derivatives(surface, u, v)?;
        let Some((su, sv)) = solve_step(du, dv, p - q) else { break };
        let old = (p - q).dot(p - q);
        let mut progress = false;
        for strength in [1., 0.5, 0.25, 0.125, 0.0625] {
            let nu = (u + su * strength).clamp(0., 1.);
            let nv = (v + sv * strength).clamp(0., 1.);
            let dx = p - at(surface, nu, nv)?;
            let nd = dx.dot(dx);
            if nd + 1e-18 < old {
                u = nu;
                v = nv;
                progress = true;
                break;
            }
        }
        if !progress {
            break;
        }
    }
    Ok((u, v))
}
/// Preserve signed normal displacement while deforming from one untrimmed
/// rational NURBS surface to another. Closest UV and normals are numerically
/// evaluated. Does not infer trim loops, seam wrapping or Rhino rigid mode.
pub fn flow_along_nurbs(points: &[Vec3], base: &Surface, target: &Surface) -> Result<Vec<Vec3>> {
    validate_points(points)?;
    if !base.valid() || !target.valid() {
        return Err(KernelError::Invalid("invalid NURBS surface"));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(points.len()).map_err(|_| KernelError::Budget)?;
    for &p in points {
        let (u, v) = closest_uv(base, p)?;
        let b = at(base, u, v)?;
        let t = at(target, u, v)?;
        let (bu, bv) = derivatives(base, u, v)?;
        let (tu, tv) = derivatives(target, u, v)?;
        let n = normalized(bu.cross(bv))?;
        let target_n = normalized(tu.cross(tv))?;
        let result = t + target_n * (p - b).dot(n);
        if !finite(result) {
            return Err(KernelError::Invalid("flow result overflow"));
        }
        output.push(result);
    }
    Ok(output)
}
/// Directional projection onto untrimmed NURBS, initializing from an indexed
/// tessellation, then refining UV with the exact rational surface evaluator.
pub fn project_onto_nurbs(points: &[Vec3], surface: &Surface, direction: Vec3) -> Result<Vec<Vec3>> {
    validate_points(points)?;
    if !surface.valid() {
        return Err(KernelError::Invalid("invalid NURBS target"));
    }
    let d = normalized(direction)?;
    let mut samples = Vec::new();
    samples.try_reserve_exact((GRID + 1) * (GRID + 1)).map_err(|_| KernelError::Budget)?;
    for j in 0..=GRID {
        for i in 0..=GRID {
            samples.push(at(surface, i as f64 / GRID as f64, j as f64 / GRID as f64)?);
        }
    }
    let mut output = Vec::new();
    for &point in points {
        let mut best: Option<(f64, f64, f64)> = None;
        for j in 0..GRID {
            for i in 0..GRID {
                let n = j * (GRID + 1) + i;
                for (ids, uvs) in [
                    ([n, n + 1, n + GRID + 2], [(i as f64, j as f64), ((i + 1) as f64, j as f64), ((i + 1) as f64, (j + 1) as f64)]),
                    ([n, n + GRID + 2, n + GRID + 1], [(i as f64, j as f64), ((i + 1) as f64, (j + 1) as f64), (i as f64, (j + 1) as f64)]),
                ] {
                    if let Some((distance, bary)) = ray_triangle(point, d, samples[ids[0]], samples[ids[1]], samples[ids[2]])
                        && best.is_none_or(|(old, _, _)| distance.abs() < old.abs())
                    {
                        let u = bary.iter().enumerate().map(|(k, w)| w * uvs[k].0).sum::<f64>() / GRID as f64;
                        let v = bary.iter().enumerate().map(|(k, w)| w * uvs[k].1).sum::<f64>() / GRID as f64;
                        best = Some((distance, u, v));
                    }
                }
            }
        }
        let (_, mut u, mut v) = best.ok_or(KernelError::Invalid("projection line missed NURBS surface"))?;
        for _ in 0..16 {
            let q = at(surface, u, v)?;
            let (du, dv) = derivatives(surface, u, v)?;
            let r = q - point;
            let residual = r - d * r.dot(d);
            let a = du - d * du.dot(d);
            let b = dv - d * dv.dot(d);
            let Some((su, sv)) = solve_step(a, b, -residual) else { break };
            let next_u = (u + su.clamp(-0.1, 0.1)).clamp(0., 1.);
            let next_v = (v + sv.clamp(-0.1, 0.1)).clamp(0., 1.);
            if (next_u - u).abs() + (next_v - v).abs() < 1e-10 {
                break;
            }
            u = next_u;
            v = next_v;
        }
        let result = at(surface, u, v)?;
        let error = (result - point).cross(d).len();
        if error > 1e-3 {
            return Err(KernelError::Invalid("projection refinement tolerance not met"));
        }
        output.push(result);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::nurbs3d::{Curve, uniform_knots};
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }
    fn plane(z: f64) -> Surface {
        Surface {
            rows: vec![
                Curve { degree: 1, control: vec![p(0., 0., z), p(2., 0., z)], weights: vec![1., 1.], knots: uniform_knots(2, 1) },
                Curve { degree: 1, control: vec![p(0., 2., z), p(2., 2., z)], weights: vec![1., 1.], knots: uniform_knots(2, 1) },
            ],
            degree_v: 1,
            knots_v: uniform_knots(2, 1),
        }
    }
    #[test]
    fn project_onto_mesh_and_nurbs_agree_on_plane() {
        let poly = crate::pushpull_quad(&[p(0., 0., 0.), p(2., 0., 0.), p(2., 2., 0.), p(0., 2., 0.)], 0.5).unwrap();
        let m = project_onto_mesh(&[p(1., 1., 3.)], &poly, Vec3::Z).unwrap();
        assert!((m[0] - p(1., 1., 0.5)).len() < 1e-8);
        let n = project_onto_nurbs(&[p(1., 1., 3.)], &plane(0.5), Vec3::Z).unwrap();
        assert!((n[0] - p(1., 1., 0.5)).len() < 1e-6);
    }
    #[test]
    fn nurbs_flow_uses_surface_parameters_and_normal_offsets() {
        let flow = flow_along_nurbs(&[p(0.5, 1., 1.5)], &plane(0.), &plane(4.)).unwrap();
        assert!((flow[0] - p(0.5, 1., 5.5)).len() < 1e-6);
    }
    #[test]
    fn unmatched_projection_fails_without_partial_result() {
        let target = plane(0.);
        assert!(project_onto_nurbs(&[p(5., 5., 3.)], &target, Vec3::Z).is_err());
        assert!(flow_along_nurbs(&[], &target, &target).is_err());
    }
}
