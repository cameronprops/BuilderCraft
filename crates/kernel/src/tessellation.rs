//! Uniform preview sampling, not tolerance-certified or trimmed Brep meshing.
use crate::{Cancellation, ExactShape, GeometryBudget, GeometryData, GeometryLease, KernelError, Result, TriangleMesh};
use cadcraft_geom::Vec3;
use std::{mem::size_of, sync::Arc};

#[derive(Clone, Copy, Debug)]
pub struct TessellationOptions {
    pub curve_segments: usize,
    pub surface_u: usize,
    pub surface_v: usize,
}
impl Default for TessellationOptions {
    fn default() -> Self {
        Self { curve_segments: 64, surface_u: 16, surface_v: 16 }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TessellationLimits {
    pub max_samples: usize,
    /// Per-job output buffer capacity cap, not aggregate process/workspace RAM.
    pub max_buffer_bytes: usize,
    /// Conservative complexity admission score, not a wall-clock guarantee.
    pub max_work_units: usize,
}
impl Default for TessellationLimits {
    fn default() -> Self {
        Self { max_samples: 65536, max_buffer_bytes: 8 * 1024 * 1024, max_work_units: 50_000_000 }
    }
}
fn mul(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b).ok_or(KernelError::Budget)
}
fn add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b).ok_or(KernelError::Budget)
}
fn buffers(vertices: usize, triangles: usize) -> Result<usize> {
    add(mul(vertices, size_of::<Vec3>())?, mul(triangles, size_of::<[u32; 3]>())?)
}
/// Rejects oversized work before output allocation. Failed/cancelled jobs publish no lease.
/// Exact input remains authoritative; caller supplies source ID/revision provenance.
pub fn tessellate(
    shape: &ExactShape,
    options: TessellationOptions,
    limits: TessellationLimits,
    budget: &Arc<GeometryBudget>,
    cancellation: &Cancellation,
) -> Result<GeometryLease> {
    cancellation.check()?;
    if !shape.valid() {
        return Err(KernelError::Invalid("exact tessellation source"));
    }
    let (vertices, triangles, work) = match shape {
        ExactShape::Curve(c) => {
            if !(1..=4096).contains(&options.curve_segments) {
                return Err(KernelError::Invalid("curve segments"));
            }
            let n = add(options.curve_segments, 1)?;
            (n, 0, mul(n, mul(c.control.len(), add(c.degree, 4)?)?)?)
        }
        ExactShape::Surface(s) => {
            if !(1..=128).contains(&options.surface_u) || !(1..=128).contains(&options.surface_v) {
                return Err(KernelError::Invalid("surface segments"));
            }
            let n = mul(add(options.surface_u, 1)?, add(options.surface_v, 1)?)?;
            let count = s.rows.first().map_or(0, |r| r.control.len());
            let degree = s.rows.first().map_or(0, |r| r.degree);
            let score = mul(s.rows.len(), add(mul(count, add(degree, 4)?)?, add(s.degree_v, 4)?)?)?;
            (n, mul(mul(options.surface_u, options.surface_v)?, 2)?, mul(n, score)?)
        }
    };
    if vertices > limits.max_samples
        || triangles > limits.max_samples
        || work > limits.max_work_units
        || buffers(vertices, triangles)? > limits.max_buffer_bytes
        || !budget.preview_fits(vertices, triangles, buffers(vertices, triangles)?)
    {
        return Err(KernelError::Budget);
    }
    let mut points = Vec::new();
    let mut faces = Vec::new();
    points.try_reserve_exact(vertices).map_err(|_| KernelError::Budget)?;
    faces.try_reserve_exact(triangles).map_err(|_| KernelError::Budget)?;
    if buffers(points.capacity(), faces.capacity())? > limits.max_buffer_bytes
        || !budget.preview_fits(vertices, triangles, buffers(points.capacity(), faces.capacity())?)
    {
        return Err(KernelError::Budget);
    }
    match shape {
        ExactShape::Curve(c) => {
            for i in 0..vertices {
                cancellation.check()?;
                points.push(c.evaluate(i as f64 / options.curve_segments as f64).ok_or(KernelError::Invalid("curve sample"))?);
            }
        }
        ExactShape::Surface(s) => {
            let width = options.surface_u + 1;
            for v in 0..=options.surface_v {
                for u in 0..=options.surface_u {
                    cancellation.check()?;
                    points.push(
                        s.evaluate(u as f64 / options.surface_u as f64, v as f64 / options.surface_v as f64)
                            .ok_or(KernelError::Invalid("surface sample"))?,
                    );
                }
            }
            for v in 0..options.surface_v {
                for u in 0..options.surface_u {
                    cancellation.check()?;
                    let a = u32::try_from(v * width + u).map_err(|_| KernelError::Budget)?;
                    let b = a + 1;
                    let c = a + u32::try_from(width).map_err(|_| KernelError::Budget)?;
                    let d = c + 1;
                    faces.extend([[a, b, d], [a, d, c]]);
                }
            }
        }
    }
    cancellation.check()?;
    let data = match shape {
        ExactShape::Curve(_) => GeometryData::Polyline(points),
        ExactShape::Surface(_) => GeometryData::Mesh(TriangleMesh { vertices: points, triangles: faces }),
    };
    let lease = budget.retain(data)?;
    cancellation.check()?;
    Ok(lease)
}
