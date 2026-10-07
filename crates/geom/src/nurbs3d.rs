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
fn basis(count: usize, degree: usize, knots: &[f64], t: f64) -> Option<Vec<f64>> {
    if !valid_axis(count, degree, knots) || !t.is_finite() || !(0.0..=1.0).contains(&t) {
        return None;
    }
    let a = *knots.get(degree)?;
    let b = *knots.get(count)?;
    let t = if t == 1.0 { b - (b - a) * 1e-12 } else { a + t * (b - a) };
    let mut values = vec![0.0; knots.len() - 1];
    for (i, w) in knots.windows(2).enumerate() {
        if *w.first()? <= t && t < *w.get(1)? {
            *values.get_mut(i)? = 1.0;
        }
    }
    for d in 1..=degree {
        let mut next = vec![0.0; values.len()];
        for i in 0..count {
            let left = knots.get(i + d)? - knots.get(i)?;
            let right = knots.get(i + d + 1)? - knots.get(i + 1)?;
            let l = if left > 0.0 { (t - knots.get(i)?) / left * values.get(i)? } else { 0.0 };
            let r = if right > 0.0 { (knots.get(i + d + 1)? - t) / right * values.get(i + 1)? } else { 0.0 };
            *next.get_mut(i)? = l + r;
        }
        values = next;
    }
    values.truncate(count);
    Some(values)
}
impl Curve {
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
        let b = basis(self.control.len(), self.degree, &self.knots, t)?;
        let mut point = Vec3::ZERO;
        let mut weight = 0.0;
        for ((p, w), b) in self.control.iter().zip(&self.weights).zip(b) {
            point = point + *p * (*w * b);
            weight += *w * b;
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
        let b = basis(self.rows.len(), self.degree_v, &self.knots_v, v)?;
        let mut p = Vec3::ZERO;
        let mut w = 0.0;
        for (row, b) in self.rows.iter().zip(b) {
            let (rp, rw) = row.homogeneous(u)?;
            p = p + rp * b;
            w += rw * b;
        }
        if w > 0.0 { Some(p * (1.0 / w)) } else { None }
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
}
