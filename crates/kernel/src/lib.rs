//! Shared, UI-independent suite foundation. Budgets measure retained geometry,
//! not total process RSS; callers must budget input decoding and job workspace.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod bounds_ops;
mod geometry;
mod mesh_analysis;
mod mesh_edges;
mod mesh_weld_map;
mod mesh_weld;
mod production;
mod point_ops;
mod polyline_ops;
mod registry;
mod scene;
mod tessellation;
mod transform;
mod vector_ops;
pub use bounds_ops::*;
pub use geometry::*;
pub use mesh_analysis::*;
pub use mesh_edges::*;
pub use mesh_weld_map::*;
pub use mesh_weld::*;
pub use production::*;
pub use point_ops::*;
pub use polyline_ops::*;
pub use registry::*;
pub use scene::*;
pub use tessellation::*;
pub use transform::*;
pub use vector_ops::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum KernelError {
    #[error("invalid identity")]
    Identity,
    #[error("invalid geometry or metadata: {0}")]
    Invalid(&'static str),
    #[error("geometry budget exceeded")]
    Budget,
    #[error("revision conflict: expected {expected}, actual {actual}")]
    Conflict { expected: u64, actual: u64 },
    #[error("object missing or duplicate")]
    Object,
    #[error("operation cancelled")]
    Cancelled,
    #[error("snapshot belongs to another scene")]
    ForeignSnapshot,
}
pub type Result<T> = std::result::Result<T, KernelError>;

/// Supplied by the caller (e.g. a UUID). Strings avoid JSON integer precision loss.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Id(u128);

impl Id {
    pub fn new(value: u128) -> Result<Self> {
        if value == 0 { Err(KernelError::Identity) } else { Ok(Self(value)) }
    }
}
impl TryFrom<String> for Id {
    type Error = KernelError;
    fn try_from(value: String) -> Result<Self> {
        if value.len() != 32 || !value.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(KernelError::Identity);
        }
        Self::new(u128::from_str_radix(&value, 16).map_err(|_| KernelError::Identity)?)
    }
}
impl From<Id> for String {
    fn from(value: Id) -> Self {
        format!("{:032x}", value.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LengthUnit {
    Millimetre,
    Metre,
    Inch,
    Foot,
}
impl LengthUnit {
    pub fn metres(self) -> f64 {
        match self {
            Self::Millimetre => 0.001,
            Self::Metre => 1.0,
            Self::Inch => 0.0254,
            Self::Foot => 0.3048,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Axes {
    RightHandedZUp,
    LeftHandedZUp,
    RightHandedYUp,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    pub unit: LengthUnit,
    pub axes: Axes,
}
impl Frame {
    pub fn convert_point(self, target: Self, p: cadcraft_geom::Vec3) -> Result<cadcraft_geom::Vec3> {
        if ![p.x, p.y, p.z].iter().all(|v| v.is_finite()) {
            return Err(KernelError::Invalid("coordinate"));
        }
        let canonical = match self.axes {
            Axes::RightHandedZUp => p,
            Axes::LeftHandedZUp => cadcraft_geom::Vec3::new(p.x, -p.y, p.z),
            Axes::RightHandedYUp => cadcraft_geom::Vec3::new(p.x, -p.z, p.y),
        } * (self.unit.metres() / target.unit.metres());
        let out = match target.axes {
            Axes::RightHandedZUp => canonical,
            Axes::LeftHandedZUp => cadcraft_geom::Vec3::new(canonical.x, -canonical.y, canonical.z),
            Axes::RightHandedYUp => cadcraft_geom::Vec3::new(canonical.x, canonical.z, -canonical.y),
        };
        if [out.x, out.y, out.z].iter().all(|v| v.is_finite()) { Ok(out) } else { Err(KernelError::Invalid("coordinate overflow")) }
    }
}
