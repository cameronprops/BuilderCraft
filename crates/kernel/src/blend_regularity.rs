//! Analytic cubic Bézier interior stationary-point preflight.
//! This detects zero velocity, not generic self-intersections or collisions.
use cadcraft_geom::Vec3;

/// A zero-speed interior parameter of a nonrational cubic Bézier curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlendStationaryPoint {
    pub parameter: f64,
    pub position: Vec3,
}

/// Find an interior root of the *full vector* derivative of a cubic Bézier.
///
/// C'(t)/3 is a quadratic vector polynomial. Roots are generated from its
/// strongest scalar component and accepted only when the complete 3D velocity
/// vanishes relative to the control polygon's local scale. This avoids
/// classifying an ordinary x- or y-extremum as a stationary point.
///
/// The routine uses floating-point root finding, not a proof of global curve
/// injectivity, self-intersection avoidance or clearance.
pub fn cubic_bezier_stationary_point(control: [Vec3; 4]) -> Option<BlendStationaryPoint> {
    if !control.iter().all(|point| point.is_finite()) {
        return None;
    }
    let [p0, p1, p2, p3] = control;
    let d0 = p1 - p0;
    let d1 = p2 - p1;
    let d2 = p3 - p2;
    let scale = d0.len().max(d1.len()).max(d2.len());
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    // Quadratic Bernstein-to-power conversion, omitting the common factor 3.
    let a = d0 - d1 * 2.0 + d2;
    let b = (d1 - d0) * 2.0;
    let c = d0;
    let components = [(a.x, b.x, c.x), (a.y, b.y, c.y), (a.z, b.z, c.z)];
    let (aa, bb, cc) = components.into_iter().max_by(|lhs, rhs| {
        let magnitude = |v: &(f64, f64, f64)| v.0.abs().max(v.1.abs()).max(v.2.abs());
        magnitude(lhs).total_cmp(&magnitude(rhs))
    })?;
    let magnitude = aa.abs().max(bb.abs()).max(cc.abs());
    if !magnitude.is_finite() || magnitude <= 0.0 {
        return None;
    }
    // Relative coefficient bound only determines whether the scalar
    // polynomial is linear; the complete 3D velocity is checked below.
    let tiny = magnitude * 1e-14;
    let mut roots = [f64::NAN; 2];
    if aa.abs() <= tiny {
        if bb.abs() <= tiny {
            return None;
        }
        roots[0] = -cc / bb;
    } else {
        let discriminant = bb * bb - 4.0 * aa * cc;
        if !discriminant.is_finite() || discriminant < 0.0 {
            return None;
        }
        let sqrt_d = discriminant.sqrt();
        // Stable quadratic formula avoids cancellation for one of the roots.
        let q = -0.5 * (bb + sqrt_d.copysign(bb));
        if q != 0.0 {
            roots = [q / aa, cc / q];
        } else {
            roots[0] = -bb / (2.0 * aa);
        }
    }
    roots.sort_by(f64::total_cmp);
    for t in roots {
        if !(t.is_finite() && t > 0.0 && t < 1.0) {
            continue;
        }
        let s = 1.0 - t;
        let velocity_third = d0 * (s * s) + d1 * (2.0 * s * t) + d2 * (t * t);
        if !velocity_third.is_finite() || velocity_third.len() > scale * 1e-11 {
            continue;
        }
        let position = p0 * (s * s * s) + p1 * (3.0 * s * s * t) + p2 * (3.0 * s * t * t) + p3 * (t * t * t);
        if position.is_finite() {
            return Some(BlendStationaryPoint { parameter: t, position });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn witness(scale: f64) -> [Vec3; 4] {
        [
            Vec3::new(1.0 * scale, 0.0, 0.0),
            Vec3::new(2.0 * scale, 0.0, 0.0),
            Vec3::new(2.5 * scale, 0.0, 0.0),
            Vec3::new(1.5 * scale, 0.0, 0.0),
        ]
    }

    #[test]
    fn wolfram_exact_witness_has_golden_ratio_stationary_parameter() {
        for factor in [0.01, 1.0, 100.0] {
            let hit = cubic_bezier_stationary_point(witness(factor)).expect("stationary point");
            let t = (5.0_f64.sqrt() - 1.0) / 2.0;
            let x = (5.0_f64.sqrt() * 5.0 - 3.0) / 4.0 * factor;
            assert!((hit.parameter - t).abs() < 1e-12);
            assert!((hit.position - Vec3::new(x, 0.0, 0.0)).len() < 1e-11 * factor.max(1.0));
        }
    }

    #[test]
    fn coordinate_extremum_is_not_false_positive_when_other_axes_move() {
        let curve = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(2.5, 1.0, 0.0),
            Vec3::new(1.5, 2.0, 0.0),
        ];
        assert!(cubic_bezier_stationary_point(curve).is_none());
    }

    #[test]
    fn regular_monotone_cubic_has_no_stationary_point() {
        let curve = [
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(2.0, 1.0, 0.0),
            Vec3::new(3.0, 1.0, 0.0),
        ];
        assert!(cubic_bezier_stationary_point(curve).is_none());
    }

    #[test]
    fn rotated_and_translated_witness_has_same_parameter() {
        let transformed = witness(1.0).map(|p| Vec3::new(10.0 - p.y, p.x + 20.0, 7.0));
        let hit = cubic_bezier_stationary_point(transformed).expect("stationary point");
        assert!((hit.parameter - (5.0_f64.sqrt() - 1.0) / 2.0).abs() < 1e-12);
        assert!((hit.position - Vec3::new(10.0, 22.045084971874737, 7.0)).len() < 1e-11);
    }
}
