//! Experimental Curvo NURBS differential adapter, not an additional
//! production evaluation engine. Feature gated until acceptance fixtures
//! and performance comparisons support switching canonical evaluators.

use crate::{Vec3, nurbs3d::Curve};
use curvo::curve::NurbsCurve3D;
use nalgebra::Point4;

/// Evaluate a Worldwright rational curve using Curvo's f64 NURBS.
/// Input parameter is normalized in [0,1], as in the legacy serialized curve.
pub fn evaluate_with_curvo(curve: &Curve, normalized_t: f64) -> Option<Vec3> {
    if !curve.valid() || !normalized_t.is_finite() || !(0.0..=1.0).contains(&normalized_t) {
        return None;
    }
    // Curvo control points are weighted homogeneous coordinates.
    let controls = curve.control.iter().zip(&curve.weights).map(|(p, w)| Point4::new(p.x * w, p.y * w, p.z * w, *w)).collect();
    let candidate = NurbsCurve3D::<f64>::try_new(curve.degree, controls, curve.knots.clone()).ok()?;
    let (a, b) = candidate.knots_domain();
    let point = candidate.point_at(a + (b - a) * normalized_t);
    let result = Vec3::new(point.x, point.y, point.z);
    result.is_finite().then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_curve(weights: Vec<f64>) -> Curve {
        Curve {
            degree: 2,
            control: vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 3.0, 2.0), Vec3::new(5.0, -2.0, 4.0)],
            weights,
            knots: nurbs3d::uniform_knots(3, 2),
        }
    }

    #[test]
    fn curvo_matches_existing_curve_for_normal_and_rational_weights() {
        for weights in [vec![1.0; 3], vec![1.0, 0.7071067811865476, 1.0]] {
            let curve = test_curve(weights);
            for i in 0..=20 {
                let t = f64::from(i) / 20.0;
                let expected = curve.evaluate(t).unwrap();
                let actual = evaluate_with_curvo(&curve, t).unwrap();
                let tolerance = if t == 1.0 { 1e-8 } else { 1e-10 };
                assert!((expected - actual).len() <= tolerance, "Curvo parity deviation at t={t}: {:?} vs {:?}", expected, actual);
            }
        }
    }

    #[test]
    fn curvo_adapter_keeps_serialized_shape_immutable() {
        let source = test_curve(vec![1.0, 0.8, 1.0]);
        let original = source.clone();
        let _ = evaluate_with_curvo(&source, 0.333);
        assert_eq!(source, original);
    }

    #[test]
    fn curvo_adapter_rejects_bad_inputs_instead_of_panicking() {
        let curve = test_curve(vec![1.0, 1.0, 1.0]);
        assert_eq!(evaluate_with_curvo(&curve, f64::NAN), None);
        assert_eq!(evaluate_with_curvo(&curve, 1.01), None);
        let mut invalid = curve;
        invalid.weights[0] = -1.0;
        assert_eq!(evaluate_with_curvo(&invalid, 0.5), None);
    }
}
