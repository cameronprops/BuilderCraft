//! Original rational 3D B-spline evaluation; not a solid topology kernel.
use crate::Vec3;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Curve {
    pub degree: usize,
    pub control: Vec<Vec3>,
    pub weights: Vec<f64>,
    pub knots: Vec<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    pub rows: Vec<Curve>,
    pub degree_v: usize,
    pub knots_v: Vec<f64>,
}
pub fn uniform_knots(count: usize, degree: usize) -> Vec<f64> {
    if !(2..=4096).contains(&count) || degree == 0 || degree > 5 || degree >= count {
        return Vec::new();
    }
    let mut knots = vec![0.0; degree + 1];
    for i in 1..count - degree {
        knots.push(i as f64 / (count - degree) as f64);
    }
    knots.extend(vec![1.0; degree + 1]);
    knots
}
fn valid_axis(count: usize, degree: usize, knots: &[f64]) -> bool {
    (2..=4096).contains(&count)
        && degree > 0
        && degree <= 5
        && degree < count
        && knots.len() == count + degree + 1
        && knots.iter().all(|k| k.is_finite() && k.abs() <= 1e12)
        && knots.windows(2).all(|w| w.first() <= w.get(1))
        && knots.get(degree).zip(knots.get(count)).is_some_and(|(a, b)| a < b)
}
/// Nonzero B-spline basis functions on a single knot span.
/// Degree is bounded by validation to <=5, so evaluating a point needs no
/// heap allocation and touches only degree + 1 control points. The right
/// endpoint is evaluated exactly instead of backing off by 1e-12.
fn local_basis(count: usize, degree: usize, knots: &[f64], t: f64) -> Option<(usize, [f64; 6])> {
    if !valid_axis(count, degree, knots) || !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return None;
    }
    let a = *knots.get(degree)?;
    let b = *knots.get(count)?;
    let u = if t == 1.0 { b } else { a + t * (b - a) };
    let span = if t == 1.0 { count - 1 } else { knots.partition_point(|k| *k <= u).saturating_sub(1).clamp(degree, count - 1) };
    let mut values = [0.0; 6];
    let mut left = [0.0; 6];
    let mut right = [0.0; 6];
    values[0] = 1.0;
    for j in 1..=degree {
        left[j] = u - knots[span + 1 - j];
        right[j] = knots[span + j] - u;
        let mut saved = 0.0;
        for r in 0..j {
            let denominator = right[r + 1] + left[j - r];
            let temp = if denominator > 0.0 { values[r] / denominator } else { 0.0 };
            values[r] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        values[j] = saved;
    }
    Some((span - degree, values))
}
impl Curve {
    /// Bounded non-rational control curve; preview, CAD commands and nodes share construction.
    pub fn from_control(control: Vec<Vec3>, degree: usize) -> Option<Self> {
        if !(2..=4096).contains(&control.len()) || !(1..=5).contains(&degree) || degree >= control.len() {
            return None;
        }
        let curve = Self { degree, knots: uniform_knots(control.len(), degree), weights: vec![1.; control.len()], control };
        curve.valid().then_some(curve)
    }
    pub fn valid(&self) -> bool {
        valid_axis(self.control.len(), self.degree, &self.knots)
            && self
                .control
                .iter()
                .all(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite() && p.x.abs() <= 1e12 && p.y.abs() <= 1e12 && p.z.abs() <= 1e12)
            && self.weights.len() == self.control.len()
            && self.weights.iter().all(|w| w.is_finite() && *w >= 1e-9 && *w <= 1e9)
    }
    fn homogeneous(&self, t: f64) -> Option<(Vec3, f64)> {
        if !self.valid() {
            return None;
        }
        let (first, basis) = local_basis(self.control.len(), self.degree, &self.knots, t)?;
        let mut point = Vec3::ZERO;
        let mut weight = 0.0;
        for (offset, coefficient) in basis.iter().take(self.degree + 1).enumerate() {
            let index = first + offset;
            let weighted_basis = self.weights[index] * coefficient;
            point = point + self.control[index] * weighted_basis;
            weight += weighted_basis;
        }
        Some((point, weight))
    }
    pub fn evaluate(&self, t: f64) -> Option<Vec3> {
        let (p, w) = self.homogeneous(t)?;
        if w > 0.0 { Some(p * (1.0 / w)) } else { None }
    }
}
impl Surface {
    pub fn valid(&self) -> bool {
        valid_axis(self.rows.len(), self.degree_v, &self.knots_v)
            && self.rows.len() <= 128
            && self.rows.iter().all(Curve::valid)
            && self.rows.first().is_some_and(|first| {
                first.control.len() <= 128
                    && self.rows.iter().all(|r| r.degree == first.degree && r.knots == first.knots && r.control.len() == first.control.len())
            })
    }
    pub fn evaluate(&self, u: f64, v: f64) -> Option<Vec3> {
        if !self.valid() {
            return None;
        }
        let first_row = self.rows.first()?;
        let (u_first, u_basis) = local_basis(first_row.control.len(), first_row.degree, &first_row.knots, u)?;
        let (v_first, v_basis) = local_basis(self.rows.len(), self.degree_v, &self.knots_v, v)?;
        let mut point = Vec3::ZERO;
        let mut weight = 0.0;
        // Tensor-product rational NURBS. Only the local (degree_u + 1) *
        // (degree_v + 1) active control points contribute, even for a large
        // freeform patch. Reuse the U basis across all active V rows.
        for (v_offset, v_coefficient) in v_basis.iter().take(self.degree_v + 1).enumerate() {
            let row = &self.rows[v_first + v_offset];
            for (u_offset, u_coefficient) in u_basis.iter().take(row.degree + 1).enumerate() {
                let index = u_first + u_offset;
                let weighted_basis = row.weights[index] * u_coefficient * v_coefficient;
                point = point + row.control[index] * weighted_basis;
                weight += weighted_basis;
            }
        }
        if weight > 0.0 { Some(point * (1.0 / weight)) } else { None }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_quarter_circle() {
        let c = Curve {
            degree: 2,
            control: vec![Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.)],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            knots: uniform_knots(3, 2),
        };
        let p = c.evaluate(0.5).unwrap();
        assert!((p.x - 0.5_f64.sqrt()).abs() < 1e-12);
        assert!((p.x * p.x + p.y * p.y - 1.).abs() < 1e-12);
        assert!(c.evaluate(f64::NAN).is_none());
    }
    #[test]
    fn bilinear_patch() {
        let row =
            |z| Curve { degree: 1, control: vec![Vec3::new(0., z, 0.), Vec3::new(10., z, 5.)], weights: vec![1., 1.], knots: uniform_knots(2, 1) };
        let s = Surface { rows: vec![row(0.), row(10.)], degree_v: 1, knots_v: uniform_knots(2, 1) };
        let p = s.evaluate(0.5, 0.5).unwrap();
        assert!((p.x - 5.).abs() < 1e-10 && (p.y - 5.).abs() < 1e-10 && (p.z - 2.5).abs() < 1e-10);
    }
    #[test]
    fn exact_clamped_endpoints_and_repeated_knots() {
        let control = (0..7).map(|x| Vec3::new(f64::from(x), 0., 0.)).collect();
        let curve = Curve { degree: 3, control, weights: vec![1.; 7], knots: vec![0., 0., 0., 0., 0.5, 0.5, 0.5, 1., 1., 1., 1.] };
        assert!(curve.valid());
        assert_eq!(curve.evaluate(0.), Some(Vec3::new(0., 0., 0.)));
        assert_eq!(curve.evaluate(1.), Some(Vec3::new(6., 0., 0.)));
        assert_eq!(curve.evaluate(0.5), Some(Vec3::new(3., 0., 0.)));
        assert!(curve.evaluate(-0.01).is_none());
        assert!(curve.evaluate(1.01).is_none());
        assert!(curve.evaluate(f64::INFINITY).is_none());
    }

    #[test]
    fn rational_cylinder_patch_preserves_radius_and_endpoints() {
        let circle_row = |z| Curve {
            degree: 2,
            control: vec![Vec3::new(1., 0., z), Vec3::new(1., 1., z), Vec3::new(0., 1., z)],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            knots: uniform_knots(3, 2),
        };
        let surface = Surface { rows: vec![circle_row(0.), circle_row(10.)], degree_v: 1, knots_v: uniform_knots(2, 1) };
        for &u in &[0., 0.125, 0.5, 0.875, 1.] {
            for &v in &[0., 0.5, 1.] {
                let p = surface.evaluate(u, v).unwrap();
                assert!((p.x * p.x + p.y * p.y - 1.).abs() < 1e-12);
                assert!((p.z - 10. * v).abs() < 1e-12);
            }
        }
        assert_eq!(surface.evaluate(1., 1.), Some(Vec3::new(0., 1., 10.)));
    }

    #[test]
    fn local_basis_is_partition_of_unity_with_repeated_knots() {
        let knots = vec![0., 0., 0., 0., 0.2, 0.5, 0.5, 0.8, 1., 1., 1., 1.];
        // 8 control points, cubic curve, 12 knots; multiplicity 2 at 0.5.
        for &t in &[0., 0.0000001, 0.2, 0.499999999, 0.5, 0.500000001, 0.8, 0.9999999, 1.] {
            let (start, values) = local_basis(8, 3, &knots, t).unwrap();
            assert!(start + 3 < 8);
            let sum: f64 = values[..4].iter().sum();
            assert!((sum - 1.).abs() < 1e-12, "partition at {t}: {sum}");
            assert!(values[..4].iter().all(|w| *w >= -1e-14 && w.is_finite()));
        }
    }
}

/// Return normalized-domain rational B-spline basis and its exact first
/// derivative. The derivative is analytic in every knot span. At a repeated
/// knot it takes the span selected by the regular evaluator, so consumers
/// must inspect continuity rather than assuming a unique tangent.
///
/// The external curve domain is [0,1]; the knot domain may be elsewhere.
fn basis_with_derivative(count: usize, degree: usize, knots: &[f64], t: f64) -> Option<(Vec<f64>, Vec<f64>)> {
    let primary = basis(count, degree, knots, t)?;
    let lo = *knots.get(degree)?;
    let hi = *knots.get(count)?;
    let span = hi - lo;
    let parameter = if t == 1.0 { hi - span * 1e-12 } else { lo + t * span };

    // Cox-de Boor at degree p-1, using count+1 functions on the SAME knot
    // vector. This also covers linear curves (p-1 == 0).
    let mut lower = vec![0.0; knots.len() - 1];
    for i in 0..lower.len() {
        if knots[i] <= parameter && parameter < knots[i + 1] {
            lower[i] = 1.0;
        }
    }
    for d in 1..degree {
        let mut next = vec![0.0; lower.len()];
        for i in 0..=count {
            let a = knots[i + d] - knots[i];
            let b = knots[i + d + 1] - knots[i + 1];
            let left = if a > 0.0 { (parameter - knots[i]) / a * lower[i] } else { 0.0 };
            let right = if b > 0.0 { (knots[i + d + 1] - parameter) / b * lower[i + 1] } else { 0.0 };
            next[i] = left + right;
        }
        lower = next;
    }

    let mut derivative = vec![0.0; count];
    for i in 0..count {
        let a = knots[i + degree] - knots[i];
        let b = knots[i + degree + 1] - knots[i + 1];
        let left = if a > 0.0 { lower[i] / a } else { 0.0 };
        let right = if b > 0.0 { lower[i + 1] / b } else { 0.0 };
        derivative[i] = degree as f64 * (left - right) * span;
    }
    derivative.iter().all(|v| v.is_finite()).then_some((primary, derivative))
}

impl Curve {
    /// Analytical derivative dC/dt on the public normalized [0,1] parameter
    /// range, accounting for rational weights and physical knot-domain scale.
    /// At kinks/repeated knots the selected one-sided derivative is returned.
    pub fn derivative(&self, t: f64) -> Option<Vec3> {
        if !self.valid() {
            return None;
        }
        let (basis, first) = basis_with_derivative(self.control.len(), self.degree, &self.knots, t)?;
        let mut numerator = Vec3::ZERO;
        let mut numerator_derivative = Vec3::ZERO;
        let mut denominator = 0.0;
        let mut denominator_derivative = 0.0;
        for (((p, &weight), &b), &d) in self.control.iter().zip(&self.weights).zip(&basis).zip(&first) {
            numerator = numerator + *p * (weight * b);
            numerator_derivative = numerator_derivative + *p * (weight * d);
            denominator += weight * b;
            denominator_derivative += weight * d;
        }
        if denominator <= 0.0 || !denominator.is_finite() {
            return None;
        }
        let derivative = (numerator_derivative - numerator * (denominator_derivative / denominator)) * (1.0 / denominator);
        derivative.is_finite().then_some(derivative)
    }

    /// Unit tangent only where the first derivative is nonzero. At cusps a
    /// directional tangent may be undefined; callers get None, not zero.
    pub fn tangent(&self, t: f64) -> Option<Vec3> {
        let derivative = self.derivative(t)?;
        let length = derivative.len();
        (length.is_finite() && length > 0.0).then_some(derivative * (1.0 / length))
    }
}

impl Surface {
    /// Analytical rational partial derivatives dS/du, dS/dv in the normalized
    /// public UV domains. They are evaluated against the same control net as
    /// Surface::evaluate; no finite-difference or display mesh is involved.
    pub fn derivatives(&self, u: f64, v: f64) -> Option<(Vec3, Vec3)> {
        if !self.valid() {
            return None;
        }
        let first = self.rows.first()?;
        let (bu, du) = basis_with_derivative(first.control.len(), first.degree, &first.knots, u)?;
        let (bv, dv) = basis_with_derivative(self.rows.len(), self.degree_v, &self.knots_v, v)?;
        let mut point_sum = Vec3::ZERO;
        let mut u_sum = Vec3::ZERO;
        let mut v_sum = Vec3::ZERO;
        let mut weight_sum = 0.0;
        let mut u_weight_sum = 0.0;
        let mut v_weight_sum = 0.0;
        for (row_index, row) in self.rows.iter().enumerate() {
            for (col_index, control) in row.control.iter().enumerate() {
                let weight = row.weights[col_index];
                let w = weight * bu[col_index] * bv[row_index];
                let wu = weight * du[col_index] * bv[row_index];
                let wv = weight * bu[col_index] * dv[row_index];
                point_sum = point_sum + *control * w;
                u_sum = u_sum + *control * wu;
                v_sum = v_sum + *control * wv;
                weight_sum += w;
                u_weight_sum += wu;
                v_weight_sum += wv;
            }
        }
        if weight_sum <= 0.0 || !weight_sum.is_finite() {
            return None;
        }
        let inv = 1.0 / weight_sum;
        let du = (u_sum - point_sum * (u_weight_sum * inv)) * inv;
        let dv = (v_sum - point_sum * (v_weight_sum * inv)) * inv;
        (du.is_finite() && dv.is_finite()).then_some((du, dv))
    }

    /// Normal of the untrimmed parametric surface. A degenerate Jacobian has
    /// no well-defined normal and must not quietly produce Vec3::ZERO.
    pub fn normal(&self, u: f64, v: f64) -> Option<Vec3> {
        let (du, dv) = self.derivatives(u, v)?;
        let cross = du.cross(dv);
        let length = cross.len();
        (length.is_finite() && length > 0.0).then_some(cross * (1.0 / length))
    }
}

#[cfg(test)]
mod analytic_derivative_tests {
    use super::*;

    fn close(a: Vec3, b: Vec3, tolerance: f64) {
        assert!((a - b).len() <= tolerance, "expected {b:?}, found {a:?}");
    }

    #[test]
    fn straight_line_exact_tangents_at_endpoints_and_center() {
        let c = Curve::from_control(vec![Vec3::ZERO, Vec3::new(3.0, 4.0, 0.0)], 1).unwrap();
        for t in [0.0, 0.125, 0.5, 0.875, 1.0] {
            close(c.derivative(t).unwrap(), Vec3::new(3.0, 4.0, 0.0), 1e-8);
            close(c.tangent(t).unwrap(), Vec3::new(0.6, 0.8, 0.0), 1e-8);
        }
        assert!(c.derivative(f64::NAN).is_none());
        assert!(c.derivative(1.001).is_none());
    }

    #[test]
    fn rational_circle_tangent_perpendicular_to_radius() {
        let c = Curve {
            degree: 2,
            control: vec![Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.)],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            knots: uniform_knots(3, 2),
        };
        for t in [0.0, 0.1, 0.5, 0.9, 1.0] {
            let point = c.evaluate(t).unwrap();
            let tangent = c.derivative(t).unwrap();
            assert!(point.dot(tangent).abs() < 1e-8, "incorrect rational tangent at {t}");
            assert!(tangent.len() > 0.1);
        }
    }

    #[test]
    fn analytic_curve_derivative_agrees_with_symmetric_difference() {
        let mut c =
            Curve::from_control(vec![Vec3::new(-2., 1., 0.), Vec3::new(1., 3., 1.), Vec3::new(3., -1., 4.), Vec3::new(5., 0., 2.)], 3).unwrap();
        c.weights = vec![1.0, 0.55, 1.8, 0.9];
        let delta = 1e-6;
        for t in [0.11, 0.3, 0.61, 0.87] {
            let diff = (c.evaluate(t + delta).unwrap() - c.evaluate(t - delta).unwrap()) * (0.5 / delta);
            close(c.derivative(t).unwrap(), diff, 2e-6);
        }
    }

    #[test]
    fn bilinear_surface_has_exact_partials_and_normal() {
        let row =
            |y| Curve { degree: 1, control: vec![Vec3::new(0., y, 0.), Vec3::new(10., y, 5.)], weights: vec![1., 1.], knots: uniform_knots(2, 1) };
        let surface = Surface { rows: vec![row(0.), row(10.)], degree_v: 1, knots_v: uniform_knots(2, 1) };
        for u in [0.0, 0.3, 1.0] {
            for v in [0.0, 0.6, 1.0] {
                let (du, dv) = surface.derivatives(u, v).unwrap();
                close(du, Vec3::new(10., 0., 5.), 1e-8);
                close(dv, Vec3::new(0., 10., 0.), 1e-8);
                let expected = Vec3::new(-0.5, 0., 1.).normalized();
                close(surface.normal(u, v).unwrap(), expected, 1e-8);
            }
        }
        assert!(surface.derivatives(0.5, f64::NAN).is_none());
    }

    #[test]
    fn singular_surface_reports_undefined_normal_not_zero() {
        let row =
            |y| Curve { degree: 1, control: vec![Vec3::new(0., y, 0.), Vec3::new(0., y, 0.)], weights: vec![1., 1.], knots: uniform_knots(2, 1) };
        let surface = Surface { rows: vec![row(0.), row(1.)], degree_v: 1, knots_v: uniform_knots(2, 1) };
        assert!(surface.derivatives(0.5, 0.5).is_some());
        assert!(surface.normal(0.5, 0.5).is_none());
    }
}
