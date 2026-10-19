//! Bounded native G0/G1 NURBS curve blends. No polyline tessellation.
use crate::{KernelError, Result};
use cadcraft_geom::{Vec3, nurbs3d::Curve};

/// Exact G0 (line) or geometric G1 (cubic Bezier) from A's end to B's start.
/// Reversal flags 0/1 select the other endpoints with oriented tangents.
/// Handle strengths are fractions of endpoint distance, not physical radii.
/// Does not claim C1 speed matching, G2 curvature, trims, joins or collisions.
pub fn blend_nurbs_curves(
    a: &Curve,
    b: &Curve,
    continuity: usize,
    strength_a: f64,
    strength_b: f64,
    reverse_a: usize,
    reverse_b: usize,
) -> Result<Curve> {
    if !a.valid() || !b.valid() {
        return Err(KernelError::Invalid("blend requires valid NURBS curves"));
    }
    if continuity > 1 {
        return Err(KernelError::Invalid("only G0/G1 curve blend is implemented"));
    }
    if reverse_a > 1 || reverse_b > 1 {
        return Err(KernelError::Invalid("blend reversal must be 0 or 1"));
    }
    if !strength_a.is_finite() || !strength_b.is_finite() || !(0.01..=2.0).contains(&strength_a) || !(0.01..=2.0).contains(&strength_b) {
        return Err(KernelError::Invalid("blend handle strengths must be in [0.01, 2]"));
    }
    let ta = if reverse_a == 0 { 1.0 } else { 0.0 };
    let tb = if reverse_b == 0 { 0.0 } else { 1.0 };
    let start = a.evaluate(ta).ok_or(KernelError::Invalid("invalid first blend endpoint"))?;
    let end = b.evaluate(tb).ok_or(KernelError::Invalid("invalid second blend endpoint"))?;
    let span = (end - start).len();
    if !span.is_finite() || span <= 1e-9 {
        return Err(KernelError::Invalid("blend endpoints are coincident or too close"));
    }
    let blend = if continuity == 0 {
        Curve::from_control(vec![start, end], 1)
    } else {
        let mut incoming = a.derivative(ta).ok_or(KernelError::Invalid("first curve has no derivative"))?;
        let mut outgoing = b.derivative(tb).ok_or(KernelError::Invalid("second curve has no derivative"))?;
        if reverse_a == 1 {
            incoming = incoming * -1.0;
        }
        if reverse_b == 1 {
            outgoing = outgoing * -1.0;
        }
        let la = incoming.len();
        let lb = outgoing.len();
        if !la.is_finite() || !lb.is_finite() || la <= 1e-12 || lb <= 1e-12 {
            return Err(KernelError::Invalid("G1 blend requires nonzero endpoint tangents"));
        }
        let handle_a = incoming * (span * strength_a / la);
        let handle_b = outgoing * (span * strength_b / lb);
        let control: Vec<Vec3> = vec![start, start + handle_a, end - handle_b, end];
        Curve::from_control(control, 3)
    };
    blend.ok_or(KernelError::Invalid("blend control geometry exceeds NURBS limits"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(a: Vec3, b: Vec3) -> Curve {
        Curve::from_control(vec![a, b], 1).unwrap()
    }
    #[test]
    fn g0_joins_exact_endpoints_with_linear_nurbs() {
        let a = line(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0));
        let b = line(Vec3::new(4.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 0.0));
        let result = blend_nurbs_curves(&a, &b, 0, 1.0 / 3.0, 1.0 / 3.0, 0, 0).unwrap();
        assert_eq!(result.degree, 1);
        assert_eq!(result.control, vec![Vec3::new(2.0, 0.0, 0.0), Vec3::new(4.0, 2.0, 0.0)]);
        assert_eq!(result.evaluate(0.5), Some(Vec3::new(3.0, 1.0, 0.0)));
    }
    #[test]
    fn g1_matches_analytic_tangent_directions() {
        let a = line(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0));
        let b = line(Vec3::new(4.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 0.0));
        let result = blend_nurbs_curves(&a, &b, 1, 0.25, 0.5, 0, 0).unwrap();
        assert_eq!(result.degree, 3);
        assert_eq!(result.control.len(), 4);
        assert_eq!(result.evaluate(0.0), a.evaluate(1.0));
        assert_eq!(result.evaluate(1.0), b.evaluate(0.0));
        assert!(result.tangent(0.0).unwrap().dot(a.tangent(1.0).unwrap()) > 1.0 - 1e-9);
        assert!(result.tangent(1.0).unwrap().dot(b.tangent(0.0).unwrap()) > 1.0 - 1e-9);
    }
    #[test]
    fn reversal_selects_other_endpoints_without_modifying_sources() {
        let a = line(Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0));
        let b = line(Vec3::new(4.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 0.0));
        let before = (a.clone(), b.clone());
        let result = blend_nurbs_curves(&a, &b, 1, 0.25, 0.5, 1, 1).unwrap();
        assert_eq!(result.control[0], Vec3::ZERO);
        assert_eq!(result.control[3], Vec3::new(4.0, 4.0, 0.0));
        assert!(result.tangent(0.0).unwrap().dot(Vec3::new(-1.0, 0.0, 0.0)) > 1.0 - 1e-9);
        assert!(result.tangent(1.0).unwrap().dot(Vec3::new(0.0, -1.0, 0.0)) > 1.0 - 1e-9);
        assert_eq!((a, b), before);
    }
    #[test]
    fn rejects_coincident_unsupported_or_out_of_range_inputs() {
        let a = line(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
        let b = line(Vec3::new(3.0, 1.0, 0.0), Vec3::new(4.0, 1.0, 0.0));
        assert!(blend_nurbs_curves(&a, &b, 2, 0.3, 0.3, 0, 0).is_err());
        assert!(blend_nurbs_curves(&a, &b, 1, 0.0, 0.3, 0, 0).is_err());
        assert!(blend_nurbs_curves(&a, &b, 1, 0.3, 0.3, 2, 0).is_err());
        assert!(blend_nurbs_curves(&a, &b, 1, f64::NAN, 0.3, 0, 0).is_err());
        let close = line(Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0));
        assert!(blend_nurbs_curves(&a, &close, 1, 0.3, 0.3, 0, 0).is_err());
    }
}
