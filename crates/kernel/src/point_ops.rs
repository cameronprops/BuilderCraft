//! Validated 3D point operations for CAD, procedural nodes and external adapters.
//! The underlying vector arithmetic lives in cadcraft-geom; do not duplicate it.
use crate::{KernelError, Result};
use cadcraft_geom::Vec3;

fn checked_point(p: Vec3) -> Result<Vec3> {
    if p.is_finite() && [p.x, p.y, p.z].iter().all(|v| v.abs() <= 1e12) { Ok(p) } else { Err(KernelError::Invalid("point coordinate")) }
}

/// Euclidean distance in the document's current linear unit.
pub fn point_distance(a: Vec3, b: Vec3) -> Result<f64> {
    let a = checked_point(a)?;
    let b = checked_point(b)?;
    let delta = a - b;
    Ok(delta.x.hypot(delta.y).hypot(delta.z))
}

/// Exact midpoint of a segment, without summing full-magnitude coordinates.
pub fn point_midpoint(a: Vec3, b: Vec3) -> Result<Vec3> {
    let a = checked_point(a)?;
    let b = checked_point(b)?;
    checked_point(Vec3::new(a.x * 0.5 + b.x * 0.5, a.y * 0.5 + b.y * 0.5, a.z * 0.5 + b.z * 0.5))
}

/// Interpolate from a (t=0) to b (t=1), without extrapolation.
pub fn point_interpolate(a: Vec3, b: Vec3, t: f64) -> Result<Vec3> {
    let a = checked_point(a)?;
    let b = checked_point(b)?;
    if !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return Err(KernelError::Invalid("interpolation parameter"));
    }
    if t == 0.0 {
        return Ok(a);
    }
    if t == 1.0 {
        return Ok(b);
    }
    checked_point(Vec3::new(a.x * (1.0 - t) + b.x * t, a.y * (1.0 - t) + b.y * t, a.z * (1.0 - t) + b.z * t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_dimensional_distance() {
        assert_eq!(point_distance(Vec3::ZERO, Vec3::new(3.0, 4.0, 12.0)), Ok(13.0));
        assert_eq!(point_distance(Vec3::ZERO, Vec3::ZERO), Ok(0.0));
    }

    #[test]
    fn midpoint_and_interpolation() {
        let a = Vec3::new(-2.0, 4.0, 6.0);
        let b = Vec3::new(6.0, 0.0, -2.0);
        assert_eq!(point_midpoint(a, b), Ok(Vec3::new(2.0, 2.0, 2.0)));
        assert_eq!(point_interpolate(a, b, 0.5), point_midpoint(a, b));
        assert_eq!(point_interpolate(a, b, 0.0), Ok(a));
        assert_eq!(point_interpolate(a, b, 1.0), Ok(b));
        assert_eq!(point_interpolate(a, b, 0.25), Ok(Vec3::new(0.0, 3.0, 4.0)));
    }

    #[test]
    fn rejects_nonfinite_and_out_of_range_inputs() {
        let zero = Vec3::ZERO;
        assert!(point_distance(zero, Vec3::new(f64::NAN, 0.0, 0.0)).is_err());
        assert!(point_midpoint(zero, Vec3::new(1e13, 0.0, 0.0)).is_err());
        assert!(point_interpolate(zero, zero, -0.1).is_err());
        assert!(point_interpolate(zero, zero, 1.1).is_err());
        assert!(point_interpolate(zero, zero, f64::INFINITY).is_err());
    }

    #[test]
    fn extremes_remain_finite() {
        let a = Vec3::new(-1e12, -1e12, -1e12);
        let b = Vec3::new(1e12, 1e12, 1e12);
        assert_eq!(point_midpoint(a, b), Ok(Vec3::ZERO));
        assert_eq!(point_interpolate(a, b, 0.5), Ok(Vec3::ZERO));
        assert!(point_distance(a, b).is_ok_and(|d| d.is_finite()));
    }
}
