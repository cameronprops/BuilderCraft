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
