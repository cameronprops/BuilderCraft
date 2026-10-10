//! Axis-aligned bounds for point-based 3D geometry in drawing coordinates.
//! This does not evaluate exact NURBS curve/surface extrema.
use crate::{KernelError, Result};
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};

const MAX_POINTS: usize = 1_000_000;
const MAX_COORDINATE: f64 = 1e12;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds3 {
    /// Nonnegative XYZ extents in the drawing's linear unit.
    pub fn dimensions(self) -> Vec3 {
        self.max - self.min
    }

    /// Axis-aligned box midpoint, not the mass centroid of the geometry.
    pub fn center(self) -> Vec3 {
        Vec3::new(self.min.x * 0.5 + self.max.x * 0.5, self.min.y * 0.5 + self.max.y * 0.5, self.min.z * 0.5 + self.max.z * 0.5)
    }
}

/// Enclosing axis-aligned box for a nonempty, bounded point collection.
/// Source point data is never modified.
pub fn bounds_from_points(points: &[Vec3]) -> Result<Bounds3> {
    if points.is_empty() || points.len() > MAX_POINTS {
        return Err(KernelError::Invalid("bounding box point count"));
    }
    let first = points[0];
    if !first.is_finite() || [first.x, first.y, first.z].iter().any(|n| n.abs() > MAX_COORDINATE) {
        return Err(KernelError::Invalid("bounding box coordinate"));
    }
    let mut min = first;
    let mut max = first;
    for p in &points[1..] {
        if !p.is_finite() || [p.x, p.y, p.z].iter().any(|n| n.abs() > MAX_COORDINATE) {
            return Err(KernelError::Invalid("bounding box coordinate"));
        }
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        min.z = min.z.min(p.z);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
        max.z = max.z.max(p.z);
    }
    Ok(Bounds3 { min, max })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encloses_three_dimensional_points() {
        let points = [Vec3::new(-2.0, 5.0, 3.0), Vec3::new(6.0, -1.0, 7.0), Vec3::new(2.0, 3.0, -5.0)];
        let box3 = bounds_from_points(&points);
        assert_eq!(box3, Ok(Bounds3 { min: Vec3::new(-2.0, -1.0, -5.0), max: Vec3::new(6.0, 5.0, 7.0) }));
        if let Ok(b) = box3 {
            assert_eq!(b.dimensions(), Vec3::new(8.0, 6.0, 12.0));
            assert_eq!(b.center(), Vec3::new(2.0, 2.0, 1.0));
        }
    }

    #[test]
    fn single_point_and_flat_geometry() {
        let p = Vec3::new(2.0, 3.0, 4.0);
        assert_eq!(bounds_from_points(&[p]), Ok(Bounds3 { min: p, max: p }));
        let plane = [Vec3::new(-1.0, -2.0, 0.0), Vec3::new(1.0, 2.0, 0.0)];
        assert!(bounds_from_points(&plane).is_ok_and(|b| b.dimensions().z == 0.0));
    }

    #[test]
    fn coordinates_near_limits_are_accepted() {
        let b = bounds_from_points(&[Vec3::new(-1e12, 0.0, 0.0), Vec3::new(1e12, 0.0, 0.0)]);
        assert!(b.is_ok_and(|b| b.dimensions().x == 2e12 && b.center().x == 0.0));
    }

    #[test]
    fn rejects_empty_nonfinite_and_out_of_bounds_coordinates() {
        assert!(bounds_from_points(&[]).is_err());
        assert!(bounds_from_points(&[Vec3::new(f64::NAN, 0.0, 0.0)]).is_err());
        assert!(bounds_from_points(&[Vec3::new(0.0, f64::INFINITY, 0.0)]).is_err());
        assert!(bounds_from_points(&[Vec3::new(1e13, 0.0, 0.0)]).is_err());
    }
}
