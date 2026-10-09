//! Feature-gated 2D cubic Bézier evaluator backed by Kurbo (Apache-2.0 OR MIT).
//! This is an isolated geometry comparison adapter, not a second production
//! curve engine. It cannot modify authoritative spline/curve document data.

use crate::Vec2;
use kurbo::{CubicBez, ParamCurve, Point};

/// Evaluate a cubic Bézier from four Worldwright control points.
/// Domain and input checks match the bounded shared geometry conventions.
pub fn evaluate_cubic_with_kurbo(control: [Vec2; 4], t: f64) -> Option<Vec2> {
    if !t.is_finite() || !(0.0..=1.0).contains(&t)
        || control.iter().any(|p| !p.is_finite() || p.x.abs() > 1e12 || p.y.abs() > 1e12)
    {
        return None;
    }
    let [a, b, c, d] = control;
    let into = |p: Vec2| Point::new(p.x, p.y);
    let curve = CubicBez::new(into(a), into(b), into(c), into(d));
    let p = curve.eval(t);
    let result = Vec2::new(p.x, p.y);
    result.is_finite().then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points() -> [Vec2; 4] {
        [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 4.0),
            Vec2::new(2.0, 4.0),
            Vec2::new(4.0, -1.0),
        ]
    }

    #[test]
    fn endpoints_and_cubic_bernstein_identity() {
        let c = points();
        assert_eq!(evaluate_cubic_with_kurbo(c, 0.0), Some(c[0]));
        assert_eq!(evaluate_cubic_with_kurbo(c, 1.0), Some(c[3]));
        for i in 1..20 {
            let t = f64::from(i) / 20.0;
            let u = 1.0 - t;
            let reference = c[0] * (u * u * u)
                + c[1] * (3.0 * u * u * t)
                + c[2] * (3.0 * u * t * t)
                + c[3] * (t * t * t);
            let got = evaluate_cubic_with_kurbo(c, t).unwrap();
            assert!((got - reference).len() < 1e-12, "Bézier disagreement at t={t}");
        }
    }

    #[test]
    fn reversal_and_translation_are_invariant() {
        let c = points();
        let mut reverse = c;
        reverse.reverse();
        let translate = Vec2::new(13.0, -7.0);
        let shifted = c.map(|p| p + translate);
        for t in [0.1, 0.3, 0.5, 0.9] {
            let a = evaluate_cubic_with_kurbo(c, t).unwrap();
            let b = evaluate_cubic_with_kurbo(reverse, 1.0 - t).unwrap();
            let d = evaluate_cubic_with_kurbo(shifted, t).unwrap();
            assert!((a - b).len() < 1e-12);
            assert!((d - a - translate).len() < 1e-12);
        }
    }

    #[test]
    fn rejects_bad_domain_and_coordinates() {
        let c = points();
        assert_eq!(evaluate_cubic_with_kurbo(c, f64::INFINITY), None);
        assert_eq!(evaluate_cubic_with_kurbo(c, -0.01), None);
        let mut invalid = c;
        invalid[1].y = f64::NAN;
        assert_eq!(evaluate_cubic_with_kurbo(invalid, 0.5), None);
    }
}
