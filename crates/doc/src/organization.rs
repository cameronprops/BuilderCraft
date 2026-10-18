//! Named model organization, independent of display layers.
use crate::Handle;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    pub nodes: Vec<ModelNode>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelNode {
    pub id: u64,
    pub name: String,
    pub kind: NodeKind,
    pub parent: Option<u64>,
    pub entities: Vec<Handle>,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Assembly,
    Component,
    Body,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometryObject {
    pub id: u64,
    pub name: String,
    pub layer: String,
    pub visible: bool,
    pub shape: Shape,
}
pub use buildercraft_kernel::ExactShape as Shape;

/// Persistent native triangle/quad object. Distinct from NURBS exact shapes:
/// polygon face identity survives round-trips and edits copy on write.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonGeometryObject {
    pub id: u64,
    pub name: String,
    pub layer: String,
    pub visible: bool,
    pub mesh: std::sync::Arc<buildercraft_kernel::PolygonMesh>,
}

/// Owned OCCT exact solid. This is NOT a tessellated polygon mesh.
/// The Base64 payload is a bounded, opaque native OpenCascade BRep archive.
/// CPU/GPU display proxies are derived and must never replace this source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExactBrepObject {
    pub id: u64,
    pub name: String,
    pub layer: String,
    pub visible: bool,
    /// Exact native BRep bytes in Base64; Arc avoids N-megabyte undo copies.
    pub brep: std::sync::Arc<String>,
    /// Measurements returned by the exact kernel, not independently recomputed by the host.
    pub volume: f64,
    pub faces: u32,
    pub edges: u32,
}

/// An explicit, manually rebuildable BRep dependency recipe. Source curve
/// snapshots allow stale detection after edits without storing unbounded BReps.
/// No automatic expensive native-worker execution occurs during a curve edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrepDependencyRecipe {
    Pipe {
        radius: f64,
        up: [f64; 3],
        chord_tolerance: f64,
        max_segments: usize,
        tolerance: Option<f64>,
    },
    Sweep1 {
        section: Vec<[f64; 2]>,
        up: [f64; 3],
        chord_tolerance: f64,
        max_segments: usize,
        tolerance: Option<f64>,
    },
    Sweep2 {
        section: Vec<[f64; 2]>,
        chord_tolerance: f64,
        max_segments: usize,
        tolerance: Option<f64>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrepDependency {
    pub target_id: u64,
    pub source_ids: Vec<u64>,
    pub source_snapshots: Vec<cadcraft_geom::nurbs3d::Curve>,
    pub recipe: BrepDependencyRecipe,
}
