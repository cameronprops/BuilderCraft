//! Adapter from the current CAD document to the suite's metadata-only scene.
//! Exact geometry allocations are shared; unsupported drawing units are rejected.
use crate::Drawing;
use buildercraft_kernel::*;

pub fn manifest(drawing: &Drawing, project_id: Id, revision: u64, geometry_budget_bytes: usize) -> Result<Manifest> {
    if drawing.organization.nodes.len().checked_add(drawing.geometry3d.len()).and_then(|n| n.checked_add(drawing.mesh3d.len())).is_none_or(|n| n > 8192) {
        return Err(KernelError::Invalid("CAD manifest object limit"));
    }
    let unit = match drawing.header.i64("INSUNITS", 0) {
        1 => LengthUnit::Inch,
        2 => LengthUnit::Foot,
        4 => LengthUnit::Millimetre,
        6 => LengthUnit::Metre,
        _ => return Err(KernelError::Invalid("unsupported or unspecified CAD units")),
    };
    let budget = GeometryBudget::new(geometry_budget_bytes, 16384);
    let mut scene = Scene::new(project_id, Frame { unit, axes: Axes::RightHandedZUp }, budget.clone(), 8192);
    let convert = |id: u64| Id::new(u128::from(id) + 1);
    let mut commands = Vec::new();
    for node in &drawing.organization.nodes {
        commands.push(SceneCommand::Insert(SceneObject {
            id: convert(node.id)?,
            name: node.name.clone(),
            layer: String::new(),
            parent: node.parent.map(convert).transpose()?,
            visible: true,
            geometry: None,
        }));
    }
    for object in &drawing.geometry3d {
        let owner = drawing.organization.nodes.iter().find(|n| n.entities.iter().any(|h| h.0 == object.id)).map(|n| convert(n.id)).transpose()?;
        let lease = budget.retain(GeometryData::Exact(object.shape.clone()))?;
        commands.push(SceneCommand::Insert(SceneObject {
            id: convert(object.id)?,
            name: object.name.clone(),
            layer: object.layer.clone(),
            parent: owner,
            visible: object.visible,
            geometry: Some(lease),
        }));
    }
    for object in &drawing.mesh3d {
        let owner = drawing.organization.nodes.iter()
            .find(|node| node.entities.iter().any(|handle| handle.0 == object.id))
            .map(|node| convert(node.id)).transpose()?;
        let lease = budget.retain(GeometryData::PolygonMesh((*object.mesh).clone()))?;
        commands.push(SceneCommand::Insert(SceneObject {
            id: convert(object.id)?,
            name: object.name.clone(),
            layer: object.layer.clone(),
            parent: owner,
            visible: object.visible,
            geometry: Some(lease),
        }));
    }
    scene.apply(0, commands, &Cancellation::default())?;
    let mut manifest = scene.manifest();
    manifest.revision = revision;
    Ok(manifest)
}


fn kind_for_object(drawing: &Drawing, object_id: u64) -> Result<GeometryKind> {
    let exact = drawing.geometry3d.iter().filter(|o| o.id == object_id).collect::<Vec<_>>();
    let polygon = drawing.mesh3d.iter().filter(|o| o.id == object_id).collect::<Vec<_>>();
    if exact.len() + polygon.len() != 1 {
        return Err(KernelError::Object);
    }
    if let Some(object) = exact.first() {
        return Ok(match &object.shape {
            ExactShape::Curve(_) => GeometryKind::NurbsCurve,
            ExactShape::Surface(_) => GeometryKind::NurbsSurface,
        });
    }
    Ok(GeometryKind::PolygonMesh)
}

/// Capture a 3D CAD object reference without cloning its geometry. The same
/// caller-supplied project identity must be used on future requests. A
/// history-bound document project identity is a later native-file migration.
pub fn capture_geometry_reference(
    drawing: &Drawing, project_id: Id, document_revision: u64, object_id: u64,
) -> Result<GeometryReference> {
    let kind = kind_for_object(drawing, object_id)?;
    let logical_id = Id::new(u128::from(object_id) + 1)?;
    Ok(GeometryReference {
        project_id, object_id: logical_id,
        source_revision: document_revision, kind,
    })
}

/// Verify a persistent metadata handle against the current CAD document.
/// The result is a kind, not an independent mutable geometry copy. Rejects
/// document-wide stale revisions, unknown or representation-changed objects.
pub fn validate_geometry_reference(
    drawing: &Drawing, reference: &GeometryReference,
    project_id: Id, document_revision: u64,
) -> Result<GeometryKind> {
    if project_id != reference.project_id {
        return Err(KernelError::Invalid("foreign geometry project"));
    }
    if document_revision != reference.source_revision {
        return Err(KernelError::Conflict {
            expected: reference.source_revision, actual: document_revision,
        });
    }
    // The scene's stable identity projection is legacy CAD handle + 1.
    // Do not expose Id's internal integer or assume JSON u64 roundtrips.
    let mut found = None;
    for o in &drawing.geometry3d {
        if Id::new(u128::from(o.id) + 1)? == reference.object_id {
            if found.is_some() { return Err(KernelError::Object); }
            found = Some(o.id);
        }
    }
    for o in &drawing.mesh3d {
        if Id::new(u128::from(o.id) + 1)? == reference.object_id {
            if found.is_some() { return Err(KernelError::Object); }
            found = Some(o.id);
        }
    }
    let id = found.ok_or(KernelError::Object)?;
    let actual = kind_for_object(drawing, id)?;
    if actual != reference.kind {
        return Err(KernelError::Invalid("geometry representation changed"));
    }
    Ok(actual)
}
