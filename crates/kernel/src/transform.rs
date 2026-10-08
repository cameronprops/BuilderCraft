//! Original affine edits of exact rational curves and control surfaces.
use crate::{Cancellation, ExactShape, KernelError, Result};
use cadcraft_geom::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Transform {
    Move { delta: [f64; 3] },
    Rotate { origin: [f64; 3], axis: [f64; 3], angle_degrees: f64 },
    Scale { origin: [f64; 3], factor: f64 },
    Scale1d { origin: [f64; 3], axis: [f64; 3], factor: f64 },
    Scale2d { origin: [f64; 3], normal: [f64; 3], factor: f64 },
    ScaleNu { origin: [f64; 3], factors: [f64; 3] },
    ScaleByPlane { origin: [f64; 3], x_axis: [f64; 3], y_axis: [f64; 3], factors: [f64; 2] },
    Mirror { origin: [f64; 3], normal: [f64; 3] },
}
fn point(p: [f64; 3]) -> Result<Vec3> {
    if p.iter().any(|v| !v.is_finite() || v.abs() > 1e12) {
        return Err(KernelError::Invalid("transform coordinate"));
    }
    Ok(Vec3::new(p[0], p[1], p[2]))
}
fn direction(p: [f64; 3]) -> Result<Vec3> {
    let p = point(p)?;
    let length = p.x.hypot(p.y).hypot(p.z);
    if length < 1e-12 {
        return Err(KernelError::Invalid("zero transform axis"));
    }
    Ok(p * (1. / length))
}
fn directional_factor(factor: f64) -> Result<f64> {
    if !factor.is_finite() || !(0.0..=1e9).contains(&factor) {
        return Err(KernelError::Invalid("directional scale factor outside supported range"));
    }
    Ok(factor)
}
impl Transform {
    pub fn matrix(&self) -> Result<Mat4> {
        let mut m = Mat4::IDENTITY;
        let origin = match *self {
            Self::Move { delta } => {
                let d = point(delta)?;
                m.m[0][3] = d.x;
                m.m[1][3] = d.y;
                m.m[2][3] = d.z;
                return Ok(m);
            }
            Self::Scale { origin, factor } => {
                if !factor.is_finite() || !(1e-9..=1e9).contains(&factor) {
                    return Err(KernelError::Invalid("positive scale factor outside supported range"));
                }
                for i in 0..3 {
                    m.m[i][i] = factor;
                }
                point(origin)?
            }
            Self::Scale1d { origin, axis, factor } | Self::Scale2d { origin, normal: axis, factor } => {
                directional_factor(factor)?;
                let axis = direction(axis)?;
                let a = [axis.x, axis.y, axis.z];
                let planar = matches!(self, Self::Scale2d { .. });
                for (i, row) in m.m.iter_mut().take(3).enumerate() {
                    for (j, value) in row.iter_mut().take(3).enumerate() {
                        let projection = a[i] * a[j];
                        let identity = if i == j { 1.0 } else { 0.0 };
                        *value = if planar { factor * identity + (1.0 - factor) * projection } else { identity + (factor - 1.0) * projection };
                    }
                }
                point(origin)?
            }
            Self::ScaleNu { origin, factors } => {
                for (i, factor) in factors.into_iter().enumerate() {
                    m.m[i][i] = directional_factor(factor)?;
                }
                point(origin)?
            }
            Self::ScaleByPlane { origin, x_axis, y_axis, factors } => {
                let x = direction(x_axis)?;
                let y = direction(y_axis)?;
                let dot = x.dot(y);
                if dot.abs() > 1e-9 {
                    return Err(KernelError::Invalid("scale plane axes must be perpendicular"));
                }
                let y = y - x * dot;
                let y = direction([y.x, y.y, y.z])?;
                let x = [x.x, x.y, x.z];
                let y = [y.x, y.y, y.z];
                let fx = directional_factor(factors[0])? - 1.0;
                let fy = directional_factor(factors[1])? - 1.0;
                for (i, row) in m.m.iter_mut().take(3).enumerate() {
                    for (j, value) in row.iter_mut().take(3).enumerate() {
                        *value += fx * x[i] * x[j] + fy * y[i] * y[j];
                    }
                }
                point(origin)?
            }
            Self::Rotate { origin, axis, angle_degrees } => {
                if !angle_degrees.is_finite() || angle_degrees.abs() > 1e9 {
                    return Err(KernelError::Invalid("rotation angle"));
                }
                let a = direction(axis)?;
                let (s, c) = angle_degrees.to_radians().sin_cos();
                let q = 1. - c;
                m.m[0][0] = c + a.x * a.x * q;
                m.m[0][1] = a.x * a.y * q - a.z * s;
                m.m[0][2] = a.x * a.z * q + a.y * s;
                m.m[1][0] = a.y * a.x * q + a.z * s;
                m.m[1][1] = c + a.y * a.y * q;
                m.m[1][2] = a.y * a.z * q - a.x * s;
                m.m[2][0] = a.z * a.x * q - a.y * s;
                m.m[2][1] = a.z * a.y * q + a.x * s;
                m.m[2][2] = c + a.z * a.z * q;
                point(origin)?
            }
            Self::Mirror { origin, normal } => {
                let n = direction(normal)?;
                let n = [n.x, n.y, n.z];
                for (i, row) in m.m.iter_mut().take(3).enumerate() {
                    for (j, value) in row.iter_mut().take(3).enumerate() {
                        *value -= 2. * n[i] * n[j];
                    }
                }
                point(origin)?
            }
        };
        let shift = origin - m.apply(origin);
        m.m[0][3] = shift.x;
        m.m[1][3] = shift.y;
        m.m[2][3] = shift.z;
        if m.m.iter().flatten().any(|v| !v.is_finite()) {
            return Err(KernelError::Invalid("transform overflow"));
        }
        Ok(m)
    }
}
pub fn exact_control_count(shape: &ExactShape) -> usize {
    match shape {
        ExactShape::Curve(c) => c.control.len(),
        ExactShape::Surface(s) => s.rows.iter().map(|r| r.control.len()).sum(),
    }
}
/// Preflight before copy-on-write; source and parameterization stay intact on failure.
pub fn transform_exact(shape: &ExactShape, operation: &Transform, cancel: &Cancellation, max_bytes: usize) -> Result<ExactShape> {
    cancel.check()?;
    if !shape.valid() {
        return Err(KernelError::Invalid("source exact shape"));
    }
    if shape.estimated_bytes()? > max_bytes || exact_control_count(shape) > 100_000 {
        return Err(KernelError::Budget);
    }
    let matrix = operation.matrix()?;
    let edit = |points: &mut [Vec3]| -> Result<()> {
        for p in points {
            cancel.check()?;
            let next = matrix.apply(*p);
            point([next.x, next.y, next.z])?;
            *p = next;
        }
        Ok(())
    };
    let mut result = shape.clone();
    match &mut result {
        ExactShape::Curve(c) => edit(&mut Arc::make_mut(c).control)?,
        ExactShape::Surface(s) => {
            for row in &mut Arc::make_mut(s).rows {
                edit(&mut row.control)?;
            }
        }
    }
    cancel.check()?;
    Ok(result)
}
