//! Oriented path array via WorldWright's shared RailFrame service.
//! No separate rotation/arc-length implementation is permitted in this file.
use crate::{KernelError, Result, rail_frames, MAX_RAIL_POINTS, MAX_RAIL_STATIONS};
use cadcraft_geom::Vec3;

const MAX_SEED: usize = 1024;
const MAX_ITEMS: usize = 65_536;
const LIMIT: f64 = 1e12;

/// Position and rotate copies using the same frame solver as Pipe and Sweep.
pub fn array_path_oriented(seed:&[Vec3],path:&[Vec3],count:usize,guide_up:Vec3,anchor:Vec3)->Result<Vec<Vec<Vec3>>> {
    if seed.is_empty() || seed.len()>MAX_SEED || path.len()>MAX_RAIL_POINTS || count==0 || count>MAX_RAIL_STATIONS
        || seed.len().checked_mul(count).is_none_or(|n|n>MAX_ITEMS) {
        return Err(KernelError::Budget);
    }
    if !anchor.is_finite() || anchor.x.abs().max(anchor.y.abs()).max(anchor.z.abs())>LIMIT ||
        seed.iter().any(|p| !p.is_finite() || p.x.abs().max(p.y.abs()).max(p.z.abs())>LIMIT) {
        return Err(KernelError::Invalid("invalid oriented-array input"));
    }
    let frames=rail_frames(path,count.max(2),guide_up)?;
    let first=frames[0];
    let mut out=Vec::new();
    out.try_reserve_exact(count).map_err(|_|KernelError::Budget)?;
    for (i,frame) in frames.into_iter().take(count).enumerate() {
        if i==0 {
            out.push(seed.to_vec());
            continue;
        }
        let mut copy=Vec::new();
        copy.try_reserve_exact(seed.len()).map_err(|_|KernelError::Budget)?;
        for &point in seed {
            let d=point-anchor;
            let x=d.dot(first.tangent);
            let y=d.dot(first.side);
            let z=d.dot(first.up);
            let mapped=anchor+(frame.origin-path[0])+frame.tangent*x+frame.side*y+frame.up*z;
            if !mapped.is_finite() || mapped.x.abs().max(mapped.y.abs()).max(mapped.z.abs())>LIMIT {
                return Err(KernelError::Invalid("array mapping overflow"));
            }
            copy.push(mapped);
        }
        out.push(copy);
    }
    Ok(out)
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
