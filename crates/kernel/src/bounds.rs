//! Bounded rational Bezier subdivision of NURBS spans. No display-mesh bounds.
use crate::{Cancellation, ExactShape, KernelError, Result};
use cadcraft_geom::Vec3;
type H = [f64; 4];
fn invalid() -> KernelError {
    KernelError::Invalid("NURBS bounds subdivision")
}
fn mix(a: H, b: H, t: f64) -> H {
    std::array::from_fn(|i| a[i] * (1. - t) + b[i] * t)
}
fn xyz(p: H) -> Result<[f64; 3]> {
    if !p[3].is_finite() || p[3] <= 0. {
        return Err(invalid());
    }
    let out = [p[0] / p[3], p[1] / p[3], p[2] / p[3]];
    if out.iter().any(|x| !x.is_finite()) {
        return Err(invalid());
    }
    Ok(out)
}
/// Polar de Boor evaluation extracts one Bezier span without sampled fitting.
fn span(points: &[H], knots: &[f64], degree: usize, k: usize) -> Result<Vec<H>> {
    let a = *knots.get(k).ok_or_else(invalid)?;
    let b = *knots.get(k + 1).ok_or_else(invalid)?;
    let mut out = Vec::with_capacity(degree + 1);
    for control in 0..=degree {
        let mut d = points.get(k - degree..=k).ok_or_else(invalid)?.to_vec();
        for r in 1..=degree {
            let t = if r <= degree - control { a } else { b };
            for j in (r..=degree).rev() {
                let i = k - degree + j;
                let lo = *knots.get(i).ok_or_else(invalid)?;
                let hi = *knots.get(i + degree - r + 1).ok_or_else(invalid)?;
                if hi <= lo {
                    return Err(invalid());
                }
                let t = ((t - lo) / (hi - lo)).clamp(0., 1.);
                let p = mix(*d.get(j - 1).ok_or_else(invalid)?, *d.get(j).ok_or_else(invalid)?, t);
                *d.get_mut(j).ok_or_else(invalid)? = p;
            }
        }
        out.push(*d.get(degree).ok_or_else(invalid)?);
    }
    Ok(out)
}
fn split_line(points: &[H]) -> Result<(Vec<H>, Vec<H>)> {
    let mut d = points.to_vec();
    let mut left = vec![*d.first().ok_or_else(invalid)?];
    let mut right = vec![*d.last().ok_or_else(invalid)?];
    while d.len() > 1 {
        d = d.windows(2).map(|w| mix(w[0], w[1], 0.5)).collect();
        left.push(*d.first().ok_or_else(invalid)?);
        right.push(*d.last().ok_or_else(invalid)?);
    }
    right.reverse();
    Ok((left, right))
}
#[derive(Clone)]
struct Patch {
    rows: Vec<Vec<H>>,
    depth: usize,
}
impl Patch {
    fn split(self) -> Result<(Self, Self)> {
        let mut a = Vec::new();
        let mut b = Vec::new();
        if self.rows.len() == 1 || self.depth.is_multiple_of(2) {
            for row in &self.rows {
                let (l, r) = split_line(row)?;
                a.push(l);
                b.push(r);
            }
        } else {
            let n = self.rows.first().ok_or_else(invalid)?.len();
            a = vec![Vec::with_capacity(n); self.rows.len()];
            b = a.clone();
            for i in 0..n {
                let col = self.rows.iter().map(|r| r.get(i).copied().ok_or_else(invalid)).collect::<Result<Vec<_>>>()?;
                let (l, r) = split_line(&col)?;
                for (row, p) in a.iter_mut().zip(l) {
                    row.push(p);
                }
                for (row, p) in b.iter_mut().zip(r) {
                    row.push(p);
                }
            }
        }
        Ok((Self { rows: a, depth: self.depth + 1 }, Self { rows: b, depth: self.depth + 1 }))
    }
}
/// Absolute world-axis bounds tolerance in model units, plus f64 rounding guard.
/// Budget is shared by all objects in an operation; failure never returns loose bounds.
pub fn exact_bounds_center(shape: &ExactShape, tolerance: f64, cancel: &Cancellation, work: &mut usize) -> Result<Vec3> {
    cancel.check()?;
    if !shape.valid() || !tolerance.is_finite() || tolerance <= 0. {
        return Err(invalid());
    }
    let (rows, dv, kv) = match shape {
        ExactShape::Curve(c) => (vec![c.as_ref()], 0, None),
        ExactShape::Surface(s) => (s.rows.iter().collect(), s.degree_v, Some(&s.knots_v)),
    };
    let first = rows.first().ok_or_else(invalid)?;
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    let mut visit = |patch: Patch, work: &mut usize| -> Result<()> {
        let mut stack = vec![patch];
        while let Some(patch) = stack.pop() {
            cancel.check()?;
            *work = work.checked_sub(1).ok_or(KernelError::Budget)?;
            let mut lo = [f64::INFINITY; 3];
            let mut hi = [f64::NEG_INFINITY; 3];
            for p in patch.rows.iter().flatten() {
                let p = xyz(*p)?;
                for i in 0..3 {
                    lo[i] = lo[i].min(p[i]);
                    hi[i] = hi[i].max(p[i]);
                }
            }
            for row in [patch.rows.first(), patch.rows.last()].into_iter().flatten() {
                for p in [row.first(), row.last()].into_iter().flatten() {
                    let p = xyz(*p)?;
                    for i in 0..3 {
                        low[i] = low[i].min(p[i]);
                        high[i] = high[i].max(p[i]);
                    }
                }
            }
            let resolved = (0..3).all(|i| {
                let eps = tolerance + 128. * f64::EPSILON * lo[i].abs().max(hi[i].abs()).max(1.);
                lo[i] >= low[i] - eps && hi[i] <= high[i] + eps
            });
            if !resolved {
                if patch.depth >= 48 {
                    return Err(KernelError::Budget);
                }
                let (a, b) = patch.split()?;
                stack.push(b);
                stack.push(a);
            }
        }
        Ok(())
    };
    for ku in first.degree..first.control.len() {
        if first.knots.get(ku) >= first.knots.get(ku + 1) {
            continue;
        }
        let mut urows = Vec::with_capacity(rows.len());
        for row in &rows {
            cancel.check()?;
            *work = work.checked_sub(row.control.len()).ok_or(KernelError::Budget)?;
            let points = row.control.iter().zip(&row.weights).map(|(p, w)| [p.x * w, p.y * w, p.z * w, *w]).collect::<Vec<_>>();
            urows.push(span(&points, &row.knots, row.degree, ku)?);
        }
        if let Some(knots) = kv {
            for k in dv..rows.len() {
                if knots.get(k) >= knots.get(k + 1) {
                    continue;
                }
                let mut net = vec![Vec::with_capacity(first.degree + 1); dv + 1];
                for i in 0..=first.degree {
                    let col = urows.iter().map(|r| r.get(i).copied().ok_or_else(invalid)).collect::<Result<Vec<_>>>()?;
                    for (row, p) in net.iter_mut().zip(span(&col, knots, dv, k)?) {
                        row.push(p);
                    }
                }
                visit(Patch { rows: net, depth: 0 }, work)?;
            }
        } else {
            visit(Patch { rows: urows, depth: 0 }, work)?;
        }
    }
    let center = std::array::from_fn::<_, 3, _>(|i| low[i] * 0.5 + high[i] * 0.5);
    if center.iter().any(|x| !x.is_finite()) {
        return Err(invalid());
    }
    Ok(Vec3::new(center[0], center[1], center[2]))
}
