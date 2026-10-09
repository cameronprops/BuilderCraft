//! Immutable, revision-bound geometry handles. A reference identifies a
//! project, object, representation kind and one authoritative scene revision.
//! It never embeds a second mutable geometry copy and never uses unstable
//! mesh-face/edge indices for persistent subelement identity.
use crate::{GeometryData, GeometryLease, Id, KernelError, Result, Scene};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeometryKind {
    NurbsCurve,
    NurbsSurface,
    TriangleMesh,
    PolygonMesh,
    PointCloud,
    Polyline,
}

impl GeometryData {
    pub fn reference_kind(&self) -> GeometryKind {
        match self {
            Self::Exact(crate::ExactShape::Curve(_)) => GeometryKind::NurbsCurve,
            Self::Exact(crate::ExactShape::Surface(_)) => GeometryKind::NurbsSurface,
            Self::Mesh(_) => GeometryKind::TriangleMesh,
            Self::PolygonMesh(_) => GeometryKind::PolygonMesh,
            Self::PointCloud(_) => GeometryKind::PointCloud,
            Self::Polyline(_) => GeometryKind::Polyline,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryReference {
    pub project_id: Id,
    pub object_id: Id,
    /// Scene-global revision. Conservative: unrelated edits also invalidate
    /// references; per-object revisions are a later optimization, not assumed.
    pub source_revision: u64,
    pub kind: GeometryKind,
}

impl Scene {
    /// Capture a typed handle to current authoritative geometry. Objects
    /// without geometry cannot produce a geometry reference.
    pub fn capture_geometry_reference(&self, id: Id) -> Result<GeometryReference> {
        let object = self.object(id).ok_or(KernelError::Object)?;
        let geometry = object.geometry.as_ref()
            .ok_or(KernelError::Invalid("referenced object has no geometry"))?;
        Ok(GeometryReference {
            project_id: self.project_id(),
            object_id: id,
            source_revision: self.revision(),
            kind: geometry.data().reference_kind(),
        })
    }

    /// Read a shared immutable geometry lease, never an editable alias.
    /// Caller must re-capture after any scene change including undo/redo;
    /// a stale or foreign handle is never followed silently.
    pub fn resolve_geometry_reference(&self, reference: &GeometryReference) -> Result<GeometryLease> {
        if reference.project_id != self.project_id() {
            return Err(KernelError::Invalid("foreign geometry project"));
        }
        if reference.source_revision != self.revision() {
            return Err(KernelError::Conflict {
                expected: reference.source_revision,
                actual: self.revision(),
            });
        }
        let object = self.object(reference.object_id).ok_or(KernelError::Object)?;
        let geometry = object.geometry.as_ref()
            .ok_or(KernelError::Invalid("referenced object has no geometry"))?;
        if geometry.data().reference_kind() != reference.kind {
            return Err(KernelError::Invalid("geometry representation changed"));
        }
        Ok(geometry.clone())
    }
}
