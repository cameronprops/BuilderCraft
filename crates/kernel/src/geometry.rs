use crate::{KernelError, Result};
use cadcraft_geom::{Vec3, nurbs3d};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Same serde enum representation as alpha Shape; .bcraft v1 stays compatible.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExactShape {
    Curve(Arc<nurbs3d::Curve>),
    Surface(Arc<nurbs3d::Surface>),
}
impl ExactShape {
    pub fn valid(&self) -> bool {
        match self {
            Self::Curve(c) => c.valid(),
            Self::Surface(s) => s.valid(),
        }
    }
    pub fn estimated_bytes(&self) -> Result<usize> {
        fn curve(c: &nurbs3d::Curve) -> Result<usize> {
            let controls = c.control.capacity().checked_mul(size_of::<Vec3>()).ok_or(KernelError::Budget)?;
            let scalars =
                c.weights.capacity().checked_add(c.knots.capacity()).and_then(|n| n.checked_mul(size_of::<f64>())).ok_or(KernelError::Budget)?;
            controls.checked_add(scalars).ok_or(KernelError::Budget)
        }
        let data = match self {
            Self::Curve(c) => curve(c)?.checked_add(size_of::<nurbs3d::Curve>()),
            Self::Surface(s) => {
                let rows = s.rows.capacity().checked_mul(size_of::<nurbs3d::Curve>()).ok_or(KernelError::Budget)?;
                let knots = s.knots_v.capacity().checked_mul(size_of::<f64>()).ok_or(KernelError::Budget)?;
                let base = rows.checked_add(knots).and_then(|n| n.checked_add(size_of::<nurbs3d::Surface>())).ok_or(KernelError::Budget)?;
                Some(s.rows.iter().try_fold(base, |n, c| n.checked_add(curve(c)?).ok_or(KernelError::Budget))?)
            }
        }
        .ok_or(KernelError::Budget)?;
        data.checked_add(size_of::<Self>()).and_then(|n| n.checked_add(2 * size_of::<usize>())).ok_or(KernelError::Budget)
    }
}

/// Initial triangle mesh carrier, not a watertight/repair guarantee.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TriangleMesh {
    pub vertices: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GeometryData {
    Exact(ExactShape),
    Mesh(TriangleMesh),
    PointCloud(Vec<Vec3>),
}
impl GeometryData {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Exact(ExactShape::Curve(_)) => "nurbsCurve",
            Self::Exact(ExactShape::Surface(_)) => "nurbsSurface",
            Self::Mesh(_) => "triangleMesh",
            Self::PointCloud(_) => "pointCloud",
        }
    }
    fn validate(&self, max_samples: usize) -> Result<()> {
        let valid_points = |points: &[Vec3]| {
            !points.is_empty()
                && points.len() <= max_samples
                && points.iter().all(|p| [p.x, p.y, p.z].iter().all(|v| v.is_finite() && v.abs() <= 1e12))
        };
        let valid = match self {
            Self::Exact(s) => {
                s.valid()
                    && match s {
                        ExactShape::Curve(c) => c.control.len() <= max_samples,
                        ExactShape::Surface(s) => {
                            s.rows.iter().try_fold(0usize, |n, r| n.checked_add(r.control.len())).is_some_and(|n| n <= max_samples)
                        }
                    }
            }
            Self::PointCloud(p) => valid_points(p),
            Self::Mesh(m) => {
                valid_points(&m.vertices)
                    && !m.triangles.is_empty()
                    && m.triangles.len() <= max_samples
                    && m.triangles.iter().all(|t| {
                        t.iter().all(|i| (*i as usize) < m.vertices.len()) && t.first() != t.get(1) && t.first() != t.get(2) && t.get(1) != t.get(2)
                    })
            }
        };
        if valid { Ok(()) } else { Err(KernelError::Invalid("geometry")) }
    }
    fn estimated_bytes(&self) -> Result<usize> {
        let vertices = |n: usize| n.checked_mul(size_of::<Vec3>()).ok_or(KernelError::Budget);
        let bytes = match self {
            Self::Exact(s) => s.estimated_bytes()?,
            Self::PointCloud(p) => vertices(p.capacity())?,
            Self::Mesh(m) => vertices(m.vertices.capacity())?
                .checked_add(m.triangles.capacity().checked_mul(size_of::<[u32; 3]>()).ok_or(KernelError::Budget)?)
                .ok_or(KernelError::Budget)?,
        };
        bytes.checked_add(size_of::<Resource>()).and_then(|n| n.checked_add(2 * size_of::<usize>())).ok_or(KernelError::Budget)
    }
}

#[derive(Debug)]
pub struct GeometryBudget {
    limit: usize,
    used: AtomicUsize,
    max_samples: usize,
}
impl GeometryBudget {
    pub fn new(limit: usize, max_samples: usize) -> Arc<Self> {
        Arc::new(Self { limit, used: AtomicUsize::new(0), max_samples })
    }
    pub fn used(&self) -> usize {
        self.used.load(Ordering::Acquire)
    }
    pub fn limit(&self) -> usize {
        self.limit
    }
    /// Input data already exists: callers must limit decoding/workspace separately.
    pub fn retain(self: &Arc<Self>, data: GeometryData) -> Result<GeometryLease> {
        data.validate(self.max_samples)?;
        let bytes = data.estimated_bytes()?;
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(bytes).filter(|total| *total <= self.limit))
            .map_err(|_| KernelError::Budget)?;
        Ok(GeometryLease(Arc::new(Resource { data, bytes, budget: self.clone() })))
    }
}
#[derive(Debug)]
struct Resource {
    data: GeometryData,
    bytes: usize,
    budget: Arc<GeometryBudget>,
}
impl Drop for Resource {
    fn drop(&mut self) {
        self.budget.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
#[derive(Clone, Debug)]
pub struct GeometryLease(Arc<Resource>);
impl GeometryLease {
    pub fn data(&self) -> &GeometryData {
        &self.0.data
    }
    pub fn estimated_bytes(&self) -> usize {
        self.0.bytes
    }
    pub fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn belongs_to(&self, budget: &Arc<GeometryBudget>) -> bool {
        Arc::ptr_eq(&self.0.budget, budget)
    }
}
