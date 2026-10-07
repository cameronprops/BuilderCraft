//! Adapter from the current CAD document to the suite's metadata-only scene.
//! Exact geometry allocations are shared; unsupported drawing units are rejected.
use crate::Drawing;
use buildercraft_kernel::*;

pub fn manifest(drawing: &Drawing, project_id: Id, revision: u64, geometry_budget_bytes: usize) -> Result<Manifest> {
    if drawing.organization.nodes.len().checked_add(drawing.geometry3d.len()).is_none_or(|n| n > 8192) {
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
    scene.apply(0, commands, &Cancellation::default())?;
    let mut manifest = scene.manifest();
    manifest.revision = revision;
    Ok(manifest)
}
