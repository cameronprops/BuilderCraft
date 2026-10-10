//! Streaming preview wires shared by rendering and object picking.
//! Fixed sampling is a display approximation, not exact intersection geometry.
use crate::{ExactShape, KernelError, Result};
use cadcraft_geom::Vec3;

/// Conservative shared exact-evaluation admission for preview and point snaps.
/// Count buffers before full validation; callers validate after charging admission.
pub fn exact_evaluation_work(shape: &ExactShape, samples: usize) -> Result<usize> {
    let curve_cost = |c: &cadcraft_geom::nurbs3d::Curve| {
        if !(2..=4096).contains(&c.control.len()) {
            return None;
        }
        c.degree.checked_add(4).and_then(|n| c.control.len().checked_mul(n))
    };
    let cost = match shape {
        ExactShape::Curve(c) => curve_cost(c),
        ExactShape::Surface(s) => {
            if !(2..=4096).contains(&s.rows.len()) {
                return Err(KernelError::Invalid("surface row count"));
            }
            s.rows.iter().try_fold(0usize, |total, row| {
                total.checked_add(curve_cost(row)?).and_then(|n| s.degree_v.checked_add(4).and_then(|v| n.checked_add(v)))
            })
        }
    };
    cost.and_then(|n| n.checked_mul(samples)).ok_or(KernelError::Budget)
}
/// Charge the whole shape before evaluation; callers share a work budget across a scene.
pub fn visit_preview_wires(shape: &ExactShape, work_left: &mut usize, mut visit: impl FnMut(Vec3, Vec3)) -> Result<()> {
    let samples = match shape {
        ExactShape::Curve(_) => 192,
        ExactShape::Surface(_) => 1248,
    };
    let work = exact_evaluation_work(shape, samples)?;
    *work_left = work_left.checked_sub(work).ok_or(KernelError::Budget)?;
    if !shape.valid() {
        return Err(KernelError::Invalid("preview wire source"));
    }
    match shape {
        ExactShape::Curve(c) => {
            for i in 0..96 {
                let a = c.evaluate(f64::from(i) / 96.).ok_or(KernelError::Invalid("curve preview sample"))?;
                let b = c.evaluate(f64::from(i + 1) / 96.).ok_or(KernelError::Invalid("curve preview sample"))?;
                visit(a, b);
            }
        }
        ExactShape::Surface(s) => {
            for i in 0..=12 {
                for j in 0..24 {
                    let a = f64::from(i) / 12.;
                    let b = f64::from(j) / 24.;
                    let c = f64::from(j + 1) / 24.;
                    for ((u0, v0), (u1, v1)) in [((a, b), (a, c)), ((b, a), (c, a))] {
                        let p = s.evaluate(u0, v0).ok_or(KernelError::Invalid("surface preview sample"))?;
                        let q = s.evaluate(u1, v1).ok_or(KernelError::Invalid("surface preview sample"))?;
                        visit(p, q);
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn budget_rejection_happens_before_emitting_geometry() {
        let c = cadcraft_geom::nurbs3d::Curve {
            degree: 1,
            control: vec![Vec3::ZERO, Vec3::new(1., 0., 0.)],
            weights: vec![1., 1.],
            knots: vec![0., 0., 1., 1.],
        };
        let shape = ExactShape::Curve(std::sync::Arc::new(c));
        let mut work = 0;
        let mut wires = 0;
        assert_eq!(visit_preview_wires(&shape, &mut work, |_, _| wires += 1), Err(KernelError::Budget));
        assert_eq!(wires, 0);
        work = 50_000_000;
        visit_preview_wires(&shape, &mut work, |_, _| wires += 1).unwrap();
        assert_eq!(wires, 96);
    }
    #[test]
    fn failed_evaluation_is_not_silently_skipped() {
        let c = cadcraft_geom::nurbs3d::Curve {
            degree: 1,
            control: vec![Vec3::ZERO, Vec3::new(1., 0., 0.)],
            weights: vec![1., 1.],
            knots: vec![1e11, 1e11, 1e11 + 1., 1e11 + 1.],
        };
        assert!(c.valid());
        // Endpoint's inward epsilon is below the representable spacing at this knot origin.
        assert!(c.evaluate(1.).is_none());
        let shape = ExactShape::Curve(std::sync::Arc::new(c));
        assert_eq!(visit_preview_wires(&shape, &mut 50_000_000, |_, _| {}), Err(KernelError::Invalid("curve preview sample")));
    }
}
