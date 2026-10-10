//! Arc-length path arrays with rotation-minimizing tangent frames.
//! First copy is unchanged. Later copies rotate with the path and preserve
//! shape scale. This is not NURBS-path, bank, or closed-loop seam parity.
use crate::{KernelError, Result, polyline_divide_count};
use cadcraft_geom::Vec3;

const MAX_SEED: usize = 1024;
const MAX_PATH: usize = 1024;
const MAX_COUNT: usize = 256;
const MAX_ITEMS: usize = 65_536;
const LIMIT: f64 = 1e12;

fn checked(v: Vec3) -> Result<Vec3> {
    if v.is_finite() && v.x.abs().max(v.y.abs()).max(v.z.abs()) <= LIMIT {
        Ok(v)
    } else {
        Err(KernelError::Invalid("nonfinite or oversized array coordinate"))
    }
}
fn unit(v: Vec3) -> Result<Vec3> {
    checked(v)?;
    let len = v.len();
    if !len.is_finite() || len < 1e-10 {
        return Err(KernelError::Invalid("degenerate path tangent or frame axis"));
    }
    Ok(v * (1. / len))
}
fn rodrigues(v: Vec3, axis: Vec3, c: f64, s: f64) -> Vec3 {
    v * c + axis.cross(v) * s + axis * (axis.dot(v) * (1. - c))
}
#[derive(Clone, Copy)]
struct Frame {
    tangent: Vec3,
    up: Vec3,
    side: Vec3,
}
fn first_frame(tangent: Vec3, guide: Vec3) -> Result<Frame> {
    let tangent = unit(tangent)?;
    let up = unit(guide - tangent * tangent.dot(guide))?;
    let side = unit(up.cross(tangent))?;
    Ok(Frame { tangent, up, side })
}
fn advance_frame(previous: Frame, next_tangent: Vec3) -> Result<Frame> {
    let next = unit(next_tangent)?;
    let axis = previous.tangent.cross(next);
    let magnitude = axis.len();
    let cosine = previous.tangent.dot(next).clamp(-1., 1.);
    let candidate_up = if magnitude < 1e-10 {
        // An exact 180-degree turn has no unique rotation axis: preserve up.
        previous.up
    } else {
        rodrigues(previous.up, axis * (1. / magnitude), cosine, magnitude)
    };
    let up = unit(candidate_up - next * candidate_up.dot(next))?;
    let side = unit(up.cross(next))?;
    Ok(Frame { tangent: next, up, side })
}
/// Position and orient repeated geometry along a bounded 3D polyline.
/// The pivot is the explicit source anchor; copy zero keeps source placement.
/// Zero-length path segments are ignored, wholly degenerate paths rejected.
pub fn array_path_oriented(seed: &[Vec3], path: &[Vec3], count: usize, guide_up: Vec3, anchor: Vec3) -> Result<Vec<Vec<Vec3>>> {
    if seed.is_empty()
        || seed.len() > MAX_SEED
        || path.len() < 2
        || path.len() > MAX_PATH
        || count == 0
        || count > MAX_COUNT
        || seed.len().checked_mul(count).is_none_or(|n| n > MAX_ITEMS)
    {
        return Err(KernelError::Budget);
    }
    checked(guide_up)?;
    checked(anchor)?;
    for &point in seed.iter().chain(path.iter()) {
        checked(point)?;
    }
    let mut segments = Vec::<(f64, Vec3)>::new();
    let mut total = 0.;
    for pair in path.windows(2) {
        let delta = pair[1] - pair[0];
        let length = delta.len();
        if !length.is_finite() {
            return Err(KernelError::Invalid("path arc-length overflow"));
        }
        if length > 1e-10 {
            let tangent = unit(delta)?;
            segments.push((length, tangent));
            total += length;
        }
    }
    let Some(&(_, first_tangent)) = segments.first() else {
        return Err(KernelError::Invalid("path has zero length"));
    };
    if !total.is_finite() || total <= 1e-10 {
        return Err(KernelError::Invalid("invalid path length"));
    }
    let first = first_frame(first_tangent, guide_up)?;
    if count == 1 {
        return Ok(vec![seed.to_vec()]);
    }
    let samples = polyline_divide_count(path, count - 1)?;
    if samples.len() != count {
        return Err(KernelError::Invalid("path sample count mismatch"));
    }
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| KernelError::Budget)?;
    let mut frame = first;
    let mut index = 0usize;
    let mut completed = 0.;
    for (i, sample) in samples.iter().enumerate() {
        let distance = total * (i as f64 / (count - 1) as f64);
        // At interior corners select outgoing tangent; at end select final.
        while index + 1 < segments.len() && distance >= completed + segments[index].0 - 1e-10 {
            completed += segments[index].0;
            index += 1;
        }
        frame = advance_frame(frame, segments[index].1)?;
        let placement = *sample - path[0];
        let mut instance = Vec::new();
        instance.try_reserve_exact(seed.len()).map_err(|_| KernelError::Budget)?;
        for &point in seed {
            let local = point - anchor;
            let along = local.dot(first.tangent);
            let sideways = local.dot(first.side);
            let upwards = local.dot(first.up);
            let mapped = anchor + placement + frame.tangent * along + frame.side * sideways + frame.up * upwards;
            instance.push(checked(mapped)?);
        }
        result.push(instance);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }
    fn near(a: Vec3, b: Vec3) {
        assert!((a - b).len() < 1e-8, "{a:?} not close to {b:?}");
    }

    #[test]
    fn copies_follow_corner_without_collapsing_scale() {
        let seed = [p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.), p(0., 0., 1.)];
        let path = [p(0., 0., 0.), p(5., 0., 0.), p(5., 5., 0.)];
        let copies = array_path_oriented(&seed, &path, 3, Vec3::Z, Vec3::ZERO).unwrap();
        assert_eq!(copies[0], seed.to_vec());
        near(copies[1][0], p(5., 0., 0.));
        near(copies[1][1], p(5., 1., 0.));
        near(copies[1][2], p(4., 0., 0.));
        near(copies[1][3], p(5., 0., 1.));
        near(copies[2][0], p(5., 5., 0.));
        near(copies[2][1], p(5., 6., 0.));
        for copy in copies {
            assert!(((copy[0] - copy[1]).len() - 1.).abs() < 1e-8);
            assert!(((copy[0] - copy[2]).len() - 1.).abs() < 1e-8);
        }
    }
    #[test]
    fn supports_vertical_paths_and_reversed_tangents() {
        let seed = [p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.)];
        let upright = [p(0., 0., 0.), p(2., 0., 0.), p(2., 0., 3.)];
        let r = array_path_oriented(&seed, &upright, 3, Vec3::Z, Vec3::ZERO).unwrap();
        near(r[2][0], p(2., 0., 3.));
        near(r[2][1] - r[2][0], Vec3::Z);
        let reverse = [p(0., 0., 0.), p(2., 0., 0.), p(0., 0., 0.)];
        let back = array_path_oriented(&seed, &reverse, 3, Vec3::Z, Vec3::ZERO).unwrap();
        near(back[2][1] - back[2][0], p(-1., 0., 0.));
        near(back[2][2] - back[2][0], p(0., -1., 0.));
    }
    #[test]
    fn rejects_invalid_up_count_or_path_without_partial_results() {
        let source = [p(0., 0., 0.), p(1., 0., 0.)];
        let path = [p(0., 0., 0.), p(2., 0., 0.)];
        assert!(array_path_oriented(&source, &path, 3, p(1., 0., 0.), Vec3::ZERO).is_err());
        assert!(array_path_oriented(&source, &path, 257, Vec3::Z, Vec3::ZERO).is_err());
        assert!(array_path_oriented(&source, &[Vec3::ZERO, Vec3::ZERO], 2, Vec3::Z, Vec3::ZERO).is_err());
        assert!(array_path_oriented(&source, &path, 2, Vec3::Z, p(f64::INFINITY, 0., 0.)).is_err());
    }
}
