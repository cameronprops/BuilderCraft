//! Bounded, one-sided NURBS-to-polyline approximation for OCCT rail admission.
//!
//! Each clamped rational subcurve has positive weights and lies in the convex
//! hull of its controls. Distance to a closed line segment is convex; therefore
//! the maximum control-to-segment distance bounds EVERY subcurve point's
//! distance to that segment. Exact rational knot insertion makes this bound
//! progressively tighter. This is NOT a Hausdorff, surface, or fabrication
//! certificate, and does not constrain the swept BRep's deviation.
use crate::{KernelError, Result, nurbs_curve_split};
use cadcraft_geom::{Vec3, nurbs3d::Curve};

#[derive(Clone, Debug, PartialEq)]
pub struct CertifiedRailApproximation {
    pub rails: Vec<Vec<Vec3>>,
    pub segment_count: usize,
    pub max_curve_to_chord_bound: f64,
}
fn distance_to_segment(point: Vec3, a: Vec3, b: Vec3) -> Result<f64> {
    let delta = b - a;
    let denom = delta.dot(delta);
    if !denom.is_finite() {
        return Err(KernelError::Invalid("rail chord has nonfinite length"));
    }
    if denom <= 1e-18 {
        // A closed rail may have coincident endpoints but a nontrivial arc.
        // Bound it against the point and let subdivision open its chords.
        let distance = (point - a).len();
        return if distance.is_finite() { Ok(distance) } else { Err(KernelError::Invalid("nonfinite rail deviation")) };
    }
    let t = ((point - a).dot(delta) / denom).clamp(0.0, 1.0);
    let distance = (point - (a + delta * t)).len();
    if distance.is_finite() { Ok(distance) } else { Err(KernelError::Invalid("nonfinite rail deviation")) }
}
fn clamped(curve: &Curve) -> bool {
    if !curve.valid() {
        return false;
    }
    let p = curve.degree;
    let n = curve.control.len();
    let start = curve.knots[p];
    let end = curve.knots[n];
    curve.knots[..=p].iter().all(|&k| k == start) && curve.knots[n..].iter().all(|&k| k == end)
}

/// Generate at most max_segments aligned chords for one or two clamped rational
/// NURBS curves. Both rails split at identical normalized parameters.
///
/// On success, all original curve points lie within chord_tolerance of the
/// corresponding segment. This is a one-sided upper bound from the positive
/// rational convex-hull property, not a sampled-point heuristic.
///
/// Invalid geometry, unmatched rail counts or segment limits fail closed.
pub fn nurbs_rails_with_chord_bound(curves: &[&Curve], chord_tolerance: f64, max_segments: usize) -> Result<CertifiedRailApproximation> {
    if !(1..=2).contains(&curves.len())
        || !chord_tolerance.is_finite()
        || !(1e-7..=1e7).contains(&chord_tolerance)
        || !(1..=127).contains(&max_segments)
    {
        return Err(KernelError::Invalid("NURBS rail tolerance or segment budget"));
    }
    if curves.iter().any(|c| !clamped(c)) {
        return Err(KernelError::Invalid("certified NURBS rail requires clamped curves"));
    }
    let mut rails = Vec::with_capacity(curves.len());
    for curve in curves {
        rails.push(vec![curve.evaluate(0.).ok_or(KernelError::Invalid("NURBS rail start"))?]);
    }
    let mut stack: Vec<(Vec<Curve>, usize)> = vec![(curves.iter().map(|&c| c.clone()).collect(), 0)];
    let mut certified = 0.0f64;
    while let Some((sections, depth)) = stack.pop() {
        let mut bound = 0.0f64;
        for section in &sections {
            let start = section.evaluate(0.).ok_or(KernelError::Invalid("NURBS subcurve start"))?;
            let finish = section.evaluate(1.).ok_or(KernelError::Invalid("NURBS subcurve finish"))?;
            for &control in &section.control {
                bound = bound.max(distance_to_segment(control, start, finish)?);
            }
        }
        // Inflate to conservatively allow for ordinary floating-point roundoff.
        let inflated = bound * (1.0 + 1e-10) + 1e-9;
        if inflated <= chord_tolerance {
            let done = rails[0].len() - 1;
            if done + 1 > max_segments {
                return Err(KernelError::Budget);
            }
            for (index, section) in sections.iter().enumerate() {
                rails[index].push(section.evaluate(1.).ok_or(KernelError::Invalid("NURBS subcurve end"))?);
            }
            certified = certified.max(inflated);
        } else {
            // Every queued branch needs a minimum of one segment; refusing
            // before the split prevents hidden growth beyond the worker budget.
            let done = rails[0].len() - 1;
            if done + stack.len() + 2 > max_segments || depth >= 24 {
                return Err(KernelError::Budget);
            }
            let mut left = Vec::with_capacity(sections.len());
            let mut right = Vec::with_capacity(sections.len());
            for section in &sections {
                let (a, b) = nurbs_curve_split(section, 0.5)?;
                left.push(a);
                right.push(b);
            }
            stack.push((right, depth + 1));
            stack.push((left, depth + 1));
        }
    }
    let segment_count = rails[0].len() - 1;
    if segment_count == 0 || rails.iter().any(|p| p.len() != segment_count + 1) {
        return Err(KernelError::Invalid("empty or misaligned NURBS rail"));
    }
    for rail in &rails {
        for pair in rail.windows(2) {
            if !pair[0].is_finite() || !pair[1].is_finite() || (pair[1] - pair[0]).len() <= 1e-7 {
                return Err(KernelError::Invalid("certified NURBS rail has degenerate segment"));
            }
        }
    }
    Ok(CertifiedRailApproximation { rails, segment_count, max_curve_to_chord_bound: certified })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn quarter() -> Curve {
        Curve {
            degree: 2,
            control: vec![Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.)],
            weights: vec![1., 2.0_f64.sqrt().recip(), 1.],
            knots: vec![0., 0., 0., 1., 1., 1.],
        }
    }
    fn line(y: f64) -> Curve {
        Curve::from_control(vec![Vec3::new(0., y, 0.), Vec3::new(4., y, 0.)], 1).unwrap()
    }
    fn point_to_chord(point: Vec3, rail: &[Vec3]) -> f64 {
        rail.windows(2).map(|w| distance_to_segment(point, w[0], w[1]).unwrap()).fold(f64::INFINITY, f64::min)
    }
    #[test]
    fn straight_rational_rail_needs_only_one_segment() {
        let rail = line(0.);
        let result = nurbs_rails_with_chord_bound(&[&rail], 0.001, 127).unwrap();
        assert_eq!(result.segment_count, 1);
        assert_eq!(result.rails[0], rail.control);
        assert!(result.max_curve_to_chord_bound <= 0.001);
    }
    #[test]
    fn quarter_circle_bound_encloses_dense_ground_truth() {
        let curve = quarter();
        let approx = nurbs_rails_with_chord_bound(&[&curve], 0.004, 127).unwrap();
        assert!(approx.segment_count > 2);
        assert!(approx.max_curve_to_chord_bound <= 0.004);
        for i in 0..=500 {
            let point = curve.evaluate(i as f64 / 500.).unwrap();
            assert!(point_to_chord(point, &approx.rails[0]) <= 0.004 + 1e-8);
        }
    }
    #[test]
    fn paired_rail_aligns_all_stations_despite_differing_curvature() {
        let curve = quarter();
        let companion = line(3.);
        let approx = nurbs_rails_with_chord_bound(&[&curve, &companion], 0.02, 127).unwrap();
        assert_eq!(approx.rails.len(), 2);
        assert_eq!(approx.rails[0].len(), approx.rails[1].len());
        assert!(approx.segment_count > 1);
        for i in 0..=500 {
            let t = i as f64 / 500.;
            assert!(point_to_chord(curve.evaluate(t).unwrap(), &approx.rails[0]) <= 0.02 + 1e-8);
            assert!(point_to_chord(companion.evaluate(t).unwrap(), &approx.rails[1]) <= 0.02 + 1e-8);
        }
    }
    #[test]
    fn closed_loop_with_coincident_endpoints_refines_instead_of_rejecting_chord() {
        let c = Curve::from_control(
            vec![
                Vec3::new(1., 0., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(-1., 1., 0.),
                Vec3::new(-1., -1., 0.),
                Vec3::new(1., -1., 0.),
                Vec3::new(1., 0., 0.),
            ],
            3,
        )
        .unwrap();
        let out = nurbs_rails_with_chord_bound(&[&c], 0.05, 127).unwrap();
        assert!(out.segment_count > 4);
        assert!((out.rails[0][0] - out.rails[0][out.segment_count]).len() < 1e-10);
        for i in 0..=300 {
            let p = c.evaluate(i as f64 / 300.).unwrap();
            assert!(point_to_chord(p, &out.rails[0]) <= 0.05 + 1e-8);
        }
    }
    #[test]
    fn tiny_budget_and_invalid_tol_fail_closed() {
        let curve = quarter();
        assert_eq!(nurbs_rails_with_chord_bound(&[&curve], 1e-7, 1), Err(KernelError::Budget));
        assert!(nurbs_rails_with_chord_bound(&[&curve], 0., 127).is_err());
        assert!(nurbs_rails_with_chord_bound(&[&curve], f64::NAN, 127).is_err());
    }
    #[test]
    fn nonclamped_curve_is_not_false_certified() {
        let c = Curve {
            degree: 2,
            control: vec![Vec3::new(0., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(2., 1., 0.), Vec3::new(3., 0., 0.)],
            weights: vec![1.; 4],
            knots: vec![0., 0., 0.1, 0.3, 0.7, 1., 1.],
        };
        assert!(c.valid());
        assert!(nurbs_rails_with_chord_bound(&[&c], 0.1, 127).is_err());
    }
}
