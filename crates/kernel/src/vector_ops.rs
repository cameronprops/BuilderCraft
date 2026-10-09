//! Checked vector operations. Arithmetic is delegated to cadcraft-geom::Vec3.
//! All angles are in radians; input vectors are in drawing coordinates.
use crate::{KernelError, Result};
use cadcraft_geom::{Vec3, EPS};

fn checked_vector(v: Vec3) -> Result<Vec3> {
    if v.is_finite() && [v.x, v.y, v.z].iter().all(|n| n.abs() <= 1e12) {
        Ok(v)
    } else {
        Err(KernelError::Invalid("vector coordinate"))
    }
}

/// Unit vector. Rejects zero and near-zero inputs instead of returning zero.
pub fn vector_normalize(v: Vec3) -> Result<Vec3> {
    let v = checked_vector(v)?;
    let length = v.x.hypot(v.y).hypot(v.z);
    if length <= EPS {
        return Err(KernelError::Invalid("zero-length vector"));
    }
    let result = v * (1.0 / length);
    if result.is_finite() { Ok(result) } else { Err(KernelError::Invalid("vector overflow")) }
}

/// Scalar dot product, useful for projection and perpendicularity tests.
pub fn vector_dot(a: Vec3, b: Vec3) -> Result<f64> {
    let product = checked_vector(a)?.dot(checked_vector(b)?);
    if product.is_finite() { Ok(product) } else { Err(KernelError::Invalid("vector overflow")) }
}

/// Right-handed vector cross product, useful for surface normals and axes.
pub fn vector_cross(a: Vec3, b: Vec3) -> Result<Vec3> {
    let result = checked_vector(a)?.cross(checked_vector(b)?);
    if result.is_finite() { Ok(result) } else { Err(KernelError::Invalid("vector overflow")) }
}

/// Unsigned angle between nonzero vectors, in [0, pi] radians.
/// atan2 is numerically stable for parallel and anti-parallel directions.
pub fn vector_angle(a: Vec3, b: Vec3) -> Result<f64> {
    let a = vector_normalize(a)?;
    let b = vector_normalize(b)?;
    let cross = a.cross(b);
    Ok(cross.x.hypot(cross.y).hypot(cross.z).atan2(a.dot(b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn unit_vector_has_length_one() {
        let v = vector_normalize(Vec3::new(3.0, 4.0, 0.0));
        assert!(v.is_ok_and(|v| near(v.len(), 1.0) && near(v.x, 0.6) && near(v.y, 0.8)));
    }

    #[test]
    fn dot_and_cross_use_existing_geometry_arithmetic() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        assert_eq!(vector_dot(x, y), Ok(0.0));
        assert_eq!(vector_dot(x, x), Ok(1.0));
        assert_eq!(vector_cross(x, y), Ok(Vec3::Z));
        assert_eq!(vector_cross(y, x), Ok(-Vec3::Z));
    }

    #[test]
    fn angles_cover_parallel_perpendicular_and_opposite() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        assert!(vector_angle(x, x).is_ok_and(|angle| near(angle, 0.0)));
        assert!(vector_angle(x, y).is_ok_and(|angle| near(angle, std::f64::consts::FRAC_PI_2)));
        assert!(vector_angle(x, -x).is_ok_and(|angle| near(angle, std::f64::consts::PI)));
    }

    #[test]
    fn validation_rejects_nonfinite_and_near_zero() {
        let nan = Vec3::new(f64::NAN, 0.0, 0.0);
        let huge = Vec3::new(1e13, 0.0, 0.0);
        assert!(vector_normalize(Vec3::ZERO).is_err());
        assert!(vector_normalize(Vec3::new(1e-12, 0.0, 0.0)).is_err());
        assert!(vector_dot(nan, Vec3::Z).is_err());
        assert!(vector_cross(huge, Vec3::Z).is_err());
        assert!(vector_angle(Vec3::ZERO, Vec3::Z).is_err());
    }

    #[test]
    fn large_valid_vectors_remain_finite() {
        let a = Vec3::new(1e12, 1e12, 1e12);
        let b = Vec3::new(-1e12, 1e12, 0.0);
        assert!(vector_dot(a, b).is_ok_and(f64::is_finite));
        assert!(vector_cross(a, b).is_ok_and(Vec3::is_finite));
        assert!(vector_angle(a, b).is_ok_and(f64::is_finite));
    }
}
