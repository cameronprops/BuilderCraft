//! Exact rational B-spline curve subdivision by homogeneous knot insertion.
//! This preserves the geometry of every supported open clamped degree-1..5
//! NURBS curve, including multispan curves with repeated internal knots.
//! No tessellation or control-cage approximation is used for split/trim.
use crate::{KernelError, Result};
use cadcraft_geom::{Vec3, nurbs3d::Curve};

const MAX_CONTROL: usize = 4096;

#[derive(Clone, Copy)]
struct Weighted {
    xyz: Vec3,
    weight: f64,
}
impl Weighted {
    fn mix(self, other: Self, alpha: f64) -> Self {
        Self { xyz: self.xyz * (1.0 - alpha) + other.xyz * alpha, weight: self.weight * (1.0 - alpha) + other.weight * alpha }
    }
}
fn homogeneous(curve: &Curve) -> Result<Vec<Weighted>> {
    if !curve.valid() {
        return Err(KernelError::Invalid("invalid rational NURBS curve"));
    }
    Ok(curve.control.iter().zip(&curve.weights).map(|(&xyz, &weight)| Weighted { xyz: xyz * weight, weight }).collect())
}
fn into_curve(points: &[Weighted], knots: Vec<f64>, degree: usize) -> Result<Curve> {
    let mut control = Vec::with_capacity(points.len());
    let mut weights = Vec::with_capacity(points.len());
    for item in points {
        if !item.weight.is_finite() || !(1e-9..=1e9).contains(&item.weight) {
            return Err(KernelError::Invalid("NURBS knot insertion produced invalid rational weight"));
        }
        let p = item.xyz * (1.0 / item.weight);
        if !p.is_finite() {
            return Err(KernelError::Invalid("NURBS dehomogenization overflow"));
        }
        control.push(p);
        weights.push(item.weight);
    }
    let result = Curve { degree, control, weights, knots };
    if !result.valid() {
        return Err(KernelError::Invalid("subdivided NURBS is invalid"));
    }
    Ok(result)
}
fn insert_once(knots: &mut Vec<f64>, points: &mut Vec<Weighted>, p: usize, u: f64) -> Result<()> {
    let n = points.len() - 1;
    if points.len() >= MAX_CONTROL {
        return Err(KernelError::Budget);
    }
    let k = knots.partition_point(|&knot| knot <= u).saturating_sub(1).clamp(p, n);
    let s = knots.iter().filter(|&&knot| knot == u).count();
    if s >= p {
        return Err(KernelError::Invalid("knot already at maximum interior multiplicity"));
    }
    let mut result = vec![Weighted { xyz: Vec3::ZERO, weight: 0.0 }; points.len() + 1];
    for i in 0..=(k - p) {
        result[i] = points[i];
    }
    for i in (k - s)..=n {
        result[i + 1] = points[i];
    }
    for i in (k - p + 1)..=(k - s) {
        let denominator = knots[i + p] - knots[i];
        if denominator <= 0.0 {
            return Err(KernelError::Invalid("degenerate knot span"));
        }
        let alpha = ((u - knots[i]) / denominator).clamp(0.0, 1.0);
        result[i] = points[i - 1].mix(points[i], alpha);
    }
    knots.insert(k + 1, u);
    *points = result;
    Ok(())
}
/// Exact evaluation of a native rational NURBS curve in its normalized
/// parameter domain t ∈ [0,1]. This is *not* arc-length stationing.
pub fn nurbs_curve_evaluate(curve: &Curve, t: f64) -> Result<Vec3> {
    if !curve.valid() || !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return Err(KernelError::Invalid("NURBS evaluation domain"));
    }
    curve.evaluate(t).ok_or(KernelError::Invalid("NURBS evaluation failed"))
}
/// Split a native multispan rational NURBS exactly at normalized parameter t.
/// Endpoints are excluded; original rational weights and knots are preserved
/// up to homogeneous insertion roundoff, without fitting or resampling.
pub fn nurbs_curve_split(curve: &Curve, t: f64) -> Result<(Curve, Curve)> {
    if !curve.valid() || !t.is_finite() || t <= 0.0 || t >= 1.0 {
        return Err(KernelError::Invalid("NURBS split needs interior normalized parameter"));
    }
    let p = curve.degree;
    let domain_start = curve.knots[p];
    let domain_end = curve.knots[curve.control.len()];
    let u = domain_start + t * (domain_end - domain_start);
    if u <= domain_start || u >= domain_end {
        return Err(KernelError::Invalid("split too close to domain boundary"));
    }
    let mut knots = curve.knots.clone();
    let mut points = homogeneous(curve)?;
    let multiplicity = knots.iter().filter(|&&knot| knot == u).count();
    if multiplicity > p {
        return Err(KernelError::Invalid("interior knot multiplicity exceeds degree"));
    }
    for _ in multiplicity..p {
        insert_once(&mut knots, &mut points, p, u)?;
    }
    let left_count = knots.partition_point(|&knot| knot < u);
    if left_count <= p || left_count >= points.len() {
        return Err(KernelError::Invalid("NURBS split would produce insufficient controls"));
    }
    let mut left_knots = knots[..left_count].to_vec();
    left_knots.extend(std::iter::repeat_n(u, p + 1));
    let mut right_knots = vec![u; p + 1];
    right_knots.extend_from_slice(&knots[left_count + p..]);
    let left = into_curve(&points[..left_count], left_knots, p)?;
    let right = into_curve(&points[left_count - 1..], right_knots, p)?;
    Ok((left, right))
}
/// Exact rational trim by two knot-insertion splits. No fit to a polyline.
pub fn nurbs_curve_trim(curve: &Curve, start: f64, end: f64) -> Result<Curve> {
    if !curve.valid() || !start.is_finite() || !end.is_finite() || start < 0.0 || end > 1.0 || start >= end {
        return Err(KernelError::Invalid("NURBS trim interval"));
    }
    if start == 0.0 && end == 1.0 {
        return Ok(curve.clone());
    }
    if start == 0.0 {
        return Ok(nurbs_curve_split(curve, end)?.0);
    }
    if end == 1.0 {
        return Ok(nurbs_curve_split(curve, start)?.1);
    }
    let remaining = nurbs_curve_split(curve, start)?.1;
    let normalized_end = (end - start) / (1.0 - start);
    Ok(nurbs_curve_split(&remaining, normalized_end)?.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::nurbs3d::uniform_knots;
    fn bezier() -> Curve {
        Curve {
            degree: 2,
            control: vec![Vec3::ZERO, Vec3::new(2., 4., 0.), Vec3::new(4., 0., 0.)],
            weights: vec![1., 0.4, 1.],
            knots: uniform_knots(3, 2),
        }
    }
    fn multispan() -> Curve {
        Curve {
            degree: 2,
            control: vec![Vec3::ZERO, Vec3::new(1., 2., 0.), Vec3::new(3., 3., 1.), Vec3::new(6., -1., 0.), Vec3::new(8., 0., 0.)],
            weights: vec![1., 0.5, 1.2, 0.8, 1.],
            knots: vec![0., 0., 0., 0.4, 0.7, 1., 1., 1.],
        }
    }
    fn check_split(c: Curve, t: f64) {
        let (a, b) = nurbs_curve_split(&c, t).unwrap();
        for i in 0..=25 {
            let v = i as f64 / 25.;
            let target = nurbs_curve_evaluate(&c, v).unwrap();
            let source = if v <= t { nurbs_curve_evaluate(&a, v / t).unwrap() } else { nurbs_curve_evaluate(&b, (v - t) / (1. - t)).unwrap() };
            assert!((target - source).len() < 1e-7, "at {v}: {target:?} != {source:?}");
        }
    }
    #[test]
    fn rational_bezier_splits_exactly() {
        check_split(bezier(), 0.28);
        check_split(bezier(), 0.5);
    }
    #[test]
    fn multispan_rational_splits_inside_and_at_knot() {
        check_split(multispan(), 0.2);
        check_split(multispan(), 0.4);
        check_split(multispan(), 0.85);
    }
    #[test]
    fn trim_retains_exact_rational_geometry() {
        let c = multispan();
        let a = 0.2;
        let b = 0.8;
        let cut = nurbs_curve_trim(&c, a, b).unwrap();
        for i in 0..=24 {
            let t = i as f64 / 24.;
            assert!((nurbs_curve_evaluate(&cut, t).unwrap() - nurbs_curve_evaluate(&c, a + (b - a) * t).unwrap()).len() < 1e-7);
        }
    }
    #[test]
    fn invalid_inputs_are_rejected_without_mutating_source() {
        let c = bezier();
        let original = c.clone();
        assert!(nurbs_curve_split(&c, 0.).is_err());
        assert!(nurbs_curve_split(&c, f64::NAN).is_err());
        assert!(nurbs_curve_trim(&c, 0.8, 0.2).is_err());
        assert_eq!(c, original);
    }
}
