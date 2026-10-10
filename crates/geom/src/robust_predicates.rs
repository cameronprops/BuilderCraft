//! Orientation predicates delegated to the MIT-licensed GeoRust robust crate.
//! Keep tolerance/acceptance rules in Worldwright; the third-party crate
//! supplies only the adaptive-precision determinant sign.

use crate::{Vec2, Vec3};
use std::cmp::Ordering;

/// Exact-sign planar orientation on finite Worldwright coordinates.
/// Positive means counter-clockwise, negative clockwise, zero collinear.
/// Invalid or out-of-range coordinates return None instead of entering
/// an adaptive floating point routine with NaN/Inf.
pub fn orientation2d(a: Vec2, b: Vec2, c: Vec2) -> Option<Ordering> {
    let finite = [a, b, c].iter().all(|p| p.is_finite() && p.x.abs() <= 1e12 && p.y.abs() <= 1e12);
    if !finite {
        return None;
    }
    let cv = |p: Vec2| robust::Coord { x: p.x, y: p.y };
    robust::orient2d(cv(a), cv(b), cv(c)).partial_cmp(&0.0)
}

/// Oriented 3D tetrahedron predicate using GeoRust robust.
/// Swapping any two input vertices reverses the sign.
/// No tolerance-based "coplanar" classification is imposed here.
pub fn orientation3d(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Option<Ordering> {
    let finite = [a, b, c, d].iter().all(|p| p.is_finite() && [p.x, p.y, p.z].iter().all(|v| v.abs() <= 1e12));
    if !finite {
        return None;
    }
    let cv = |p: Vec3| robust::Coord3D { x: p.x, y: p.y, z: p.z };
    robust::orient3d(cv(a), cv(b), cv(c), cv(d)).partial_cmp(&0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_fallback_preserves_cancelled_orientation() {
        let a = Vec2::ZERO;
        let b = Vec2::new(134_217_729., 134_217_728.);
        let c = Vec2::new(134_217_728., 134_217_727.);
        assert_eq!((b - a).cross(c - a), 0.);
        assert_eq!(orientation2d(a, b, c), Some(Ordering::Less));
        assert_eq!(orientation2d(a, c, b), Some(Ordering::Greater));
    }

    #[test]
    fn planar_winding_and_exact_degeneracy() {
        let a = Vec2::new(0., 0.);
        let b = Vec2::new(1., 0.);
        let c = Vec2::new(0., 1.);
        assert_eq!(orientation2d(a, b, c), Some(Ordering::Greater));
        assert_eq!(orientation2d(a, c, b), Some(Ordering::Less));
        assert_eq!(orientation2d(a, b, Vec2::new(0.5, 0.)), Some(Ordering::Equal));
    }

    #[test]
    fn nearly_collinear_large_coordinates_do_not_cancel_sign() {
        let a = Vec2::new(1e8, 1e8);
        let b = Vec2::new(1e8 + 3.0, 1e8 + 3.0);
        let c = Vec2::new(1e8 + 6.0, 1e8 + 6.0 + 1e-7);
        assert_eq!(orientation2d(a, b, c), Some(Ordering::Greater));
        assert_eq!(orientation2d(a, c, b), Some(Ordering::Less));
    }

    #[test]
    fn tetrahedron_sign_reverses_and_coplanar_is_zero() {
        let a = Vec3::ZERO;
        let b = Vec3::new(1., 0., 0.);
        let c = Vec3::new(0., 1., 0.);
        let d = Vec3::new(0., 0., 1.);
        let sign = orientation3d(a, b, c, d);
        assert!(matches!(sign, Some(Ordering::Less | Ordering::Greater)));
        assert_eq!(orientation3d(a, c, b, d), sign.map(Ordering::reverse));
        assert_eq!(orientation3d(a, b, c, Vec3::new(0.25, 0.25, 0.)), Some(Ordering::Equal));
    }

    #[test]
    fn rejects_invalid_coordinates_without_panics() {
        assert_eq!(orientation2d(Vec2::new(f64::NAN, 0.), Vec2::ZERO, Vec2::X), None);
        assert_eq!(orientation2d(Vec2::new(1e13, 0.), Vec2::ZERO, Vec2::Y), None);
        assert_eq!(orientation3d(Vec3::new(f64::INFINITY, 0., 0.), Vec3::ZERO, Vec3::Z, Vec3::new(1., 0., 0.)), None);
    }
}
