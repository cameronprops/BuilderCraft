//! Numerically bounded 3D closest-point primitives used by CAD and metrology.
//! Triangle queries preserve barycentric coordinates and do not modify geometry.
use crate::{Vec2, Vec3};

fn projected_inside(point: Vec3, a: Vec3, b: Vec3, c: Vec3, normal: Vec3) -> bool {
    let project = |p: Vec3| {
        if normal.x.abs() >= normal.y.abs().max(normal.z.abs()) {
            Vec2::new(p.y, p.z)
        } else if normal.y.abs() >= normal.z.abs() {
            Vec2::new(p.x, p.z)
        } else {
            Vec2::new(p.x, p.y)
        }
    };
    let p = project(point);
    let signs = [(a, b), (b, c), (c, a)].map(|(a, b)| crate::robust_predicates::orientation2d(project(a), project(b), p));
    signs.iter().all(|s| s.is_some_and(|s| s != std::cmp::Ordering::Less))
        || signs.iter().all(|s| s.is_some_and(|s| s != std::cmp::Ordering::Greater))
}

/// Nearest position on one triangle. Degenerate faces fall back to their
/// segments or vertices rather than dividing by zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleClosest {
    pub point: Vec3,
    pub barycentric: [f64; 3],
    pub distance: f64,
    pub degenerate: bool,
}

fn valid(p: Vec3) -> bool {
    p.is_finite() && p.x.abs().max(p.y.abs()).max(p.z.abs()) <= 1e12
}

fn edge_closest(query: Vec3, a: Vec3, b: Vec3) -> (Vec3, f64) {
    let edge = b - a;
    let length2 = edge.dot(edge);
    let t = if length2 > 0.0 { ((query - a).dot(edge) / length2).clamp(0.0, 1.0) } else { 0.0 };
    (a + edge * t, t)
}

/// Closest Euclidean point on the closed triangular region.
///
/// Coordinates are in drawing units. This is an exact planar triangle
/// primitive, not an exact closest-point solver on a curved NURBS surface.
/// Returns None on nonfinite or out-of-bounds coordinates.
pub fn closest_point_triangle(query: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<TriangleClosest> {
    if ![query, a, b, c].into_iter().all(valid) {
        return None;
    }

    // Consider all boundary segments first. Ties retain the earlier edge.
    let edges = [(a, b, 0usize, 1usize), (b, c, 1usize, 2usize), (c, a, 2usize, 0usize)];
    let mut best_point = a;
    let mut best_weights = [1.0, 0.0, 0.0];
    let mut best_distance2 = (query - a).dot(query - a);
    for (start, end, i, j) in edges {
        let (point, t) = edge_closest(query, start, end);
        let difference = query - point;
        let distance2 = difference.dot(difference);
        if distance2 < best_distance2 {
            best_distance2 = distance2;
            best_point = point;
            let mut weights = [0.0; 3];
            weights[i] = 1.0 - t;
            weights[j] = t;
            best_weights = weights;
        }
    }

    let ab = b - a;
    let ac = c - a;
    let bc = c - b;
    let scale2 = ab.dot(ab).max(ac.dot(ac)).max(bc.dot(bc));
    let normal = ab.cross(ac);
    let normal2 = normal.dot(normal);
    // Relative tolerance avoids mistaking very long, very thin triangles
    // for well-conditioned planar areas.
    let degenerate = scale2 == 0.0 || normal2 <= 1e-24 * scale2 * scale2;
    if !degenerate {
        let t = (query - a).dot(normal) / normal2;
        let projected = query - normal * t;
        let mut weights = [
            (b - projected).cross(c - projected).dot(normal) / normal2,
            (c - projected).cross(a - projected).dot(normal) / normal2,
            (a - projected).cross(b - projected).dot(normal) / normal2,
        ];
        if projected_inside(projected, a, b, c, normal) && weights.iter().all(|w| w.is_finite()) {
            for weight in &mut weights {
                *weight = weight.max(0.0);
            }
            let sum: f64 = weights.iter().sum();
            if sum > 0.0 && sum.is_finite() {
                let point = a * (weights[0] / sum) + b * (weights[1] / sum) + c * (weights[2] / sum);
                let difference = query - point;
                let distance2 = difference.dot(difference);
                if distance2 < best_distance2 {
                    best_distance2 = distance2;
                    best_point = point;
                    best_weights = weights.map(|w| w / sum);
                }
            }
        }
    }
    let distance = best_distance2.sqrt();
    (valid(best_point) && distance.is_finite()).then_some(TriangleClosest { point: best_point, barycentric: best_weights, distance, degenerate })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> (Vec3, Vec3, Vec3) {
        (Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0), Vec3::new(0.0, 2.0, 0.0))
    }

    #[test]
    fn interior_projection_has_barycentric_coordinates() {
        let (a, b, c) = triangle();
        let p = closest_point_triangle(Vec3::new(0.5, 0.5, 3.0), a, b, c).unwrap();
        assert_eq!(p.point, Vec3::new(0.5, 0.5, 0.0));
        assert!((p.distance - 3.0).abs() < 1e-12);
        assert!((p.barycentric[0] - 0.5).abs() < 1e-12);
        assert!((p.barycentric[1] - 0.25).abs() < 1e-12);
        assert!((p.barycentric[2] - 0.25).abs() < 1e-12);
        assert!(!p.degenerate);
    }

    #[test]
    fn edge_vertex_and_degenerate_inputs_are_safe() {
        let (a, b, c) = triangle();
        let edge = closest_point_triangle(Vec3::new(1.0, -1.0, 0.0), a, b, c).unwrap();
        assert_eq!(edge.point, Vec3::new(1.0, 0.0, 0.0));
        let vertex = closest_point_triangle(Vec3::new(-2.0, -2.0, 0.0), a, b, c).unwrap();
        assert_eq!(vertex.point, a);
        let degenerate = closest_point_triangle(Vec3::new(0.5, 1.0, 0.0), a, Vec3::new(1.0, 0.0, 0.0), b).unwrap();
        assert!(degenerate.degenerate);
        assert_eq!(degenerate.point, Vec3::new(0.5, 0.0, 0.0));
        let repeated = closest_point_triangle(Vec3::new(4.0, 0.0, 0.0), a, a, a).unwrap();
        assert!(repeated.degenerate);
        assert_eq!(repeated.point, a);
    }

    #[test]
    fn invalid_values_rejected_without_panic() {
        let (a, b, c) = triangle();
        assert!(closest_point_triangle(Vec3::new(f64::NAN, 0.0, 0.0), a, b, c).is_none());
        assert!(closest_point_triangle(Vec3::ZERO, a, Vec3::new(1e13, 0.0, 0.0), c).is_none());
        assert!(closest_point_triangle(Vec3::new(f64::INFINITY, 0.0, 0.0), a, b, c).is_none());
    }
}
