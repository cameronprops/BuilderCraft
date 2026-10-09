//! Versioned alpha project envelope: supported DXF drawing plus model organization.
use crate::{IoError, Result, dxf_read, dxf_write};
use cadcraft_doc::{
    Drawing,
    organization::{NodeKind, Organization},
};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
struct Project {
    version: u32,
    drawing_dxf: String,
    organization: Organization,
    #[serde(default)]
    geometry3d: Vec<cadcraft_doc::organization::GeometryObject>,
    #[serde(default)]
    mesh3d: Vec<cadcraft_doc::organization::PolygonGeometryObject>,
    #[serde(default)]
    production: buildercraft_kernel::ProductionModel,
    #[serde(default)]
    feature_timelines: Vec<buildercraft_kernel::FeatureTimeline>,
}
/// Bounded validation before persistence or loading into a document.
/// The project payload itself is capped separately to 128 MiB.
fn validate_polygons(objects: &[cadcraft_doc::organization::PolygonGeometryObject]) -> Result<()> {
    if objects.len() > 4096 {
        return Err(IoError::Format("too many polygon objects".into()));
    }
    let mut vertices = 0usize;
    let mut faces = 0usize;
    for object in objects {
        if object.name.trim().is_empty() || object.name.len() > 256
            || object.layer.len() > 256 || object.id == u64::MAX
        {
            return Err(IoError::Format("invalid polygon object metadata".into()));
        }
        buildercraft_kernel::polygon_mesh_validate(&object.mesh)
            .map_err(|e| IoError::Format(e.to_string()))?;
        vertices = vertices.checked_add(object.mesh.vertices.len())
            .ok_or_else(|| IoError::Format("polygon vertex budget exceeded".into()))?;
        faces = faces.checked_add(object.mesh.faces.len())
            .ok_or_else(|| IoError::Format("polygon face budget exceeded".into()))?;
        if vertices > 1_000_000 || faces > 1_000_000 {
            return Err(IoError::Format("aggregate polygon geometry limit".into()));
        }
    }
    Ok(())
}

pub fn write(d: &Drawing) -> Result<Vec<u8>> {
    d.production.validate().map_err(|e| IoError::Format(e.to_string()))?;
    d.validate_feature_histories().map_err(|e| IoError::Format(e.to_string()))?;
    validate_polygons(&d.mesh3d)?;
    let mut organization = d.organization.clone();
    for node in &mut organization.nodes {
        node.entities.retain(|h| d.entity(*h).is_some() || d.geometry3d.iter().any(|o| o.id == h.0) || d.mesh3d.iter().any(|o| o.id == h.0));
    }
    serde_json::to_vec(&Project {
        version: 1,
        drawing_dxf: dxf_write::write(d),
        organization,
        geometry3d: d.geometry3d.clone(),
        mesh3d: d.mesh3d.clone(),
        production: d.production.clone(),
        feature_timelines: d.feature_timelines.clone(),
    })
    .map_err(|e| IoError::Format(e.to_string()))
}
pub fn read(bytes: &[u8]) -> Result<Drawing> {
    if bytes.len() > 128 << 20 {
        return Err(IoError::Format("project exceeds 128 MiB alpha limit".into()));
    }
    let p: Project = serde_json::from_slice(bytes).map_err(|e| IoError::Format(e.to_string()))?;
    if p.version != 1 {
        return Err(IoError::Format("unsupported BuilderCraft project version".into()));
    }
    if p.organization.nodes.len() > 100_000 || p.geometry3d.len().checked_add(p.mesh3d.len()).is_none_or(|n| n > 4096) {
        return Err(IoError::Format("too many model items".into()));
    }
    validate_polygons(&p.mesh3d)?;
    let mut d = dxf_read::read(p.drawing_dxf.as_bytes())?;
    let mut ids = std::collections::HashSet::new();
    let mut owned = std::collections::HashSet::new();
    for n in &p.organization.nodes {
        if n.id == u64::MAX || !ids.insert(n.id) || n.name.trim().is_empty() || n.name.len() > 256 || n.entities.len() > 100_000 {
            return Err(IoError::Format("invalid model item".into()));
        }
        if n.kind != NodeKind::Body && !n.entities.is_empty() {
            return Err(IoError::Format("only bodies can own entities".into()));
        }
        for h in &n.entities {
            if (d.entity(*h).is_none() && !p.geometry3d.iter().any(|o| o.id == h.0) && !p.mesh3d.iter().any(|o| o.id == h.0)) || !owned.insert(*h) {
                return Err(IoError::Format("invalid or multiply-owned body entity".into()));
            }
        }
    }
    for n in &p.organization.nodes {
        let mut parent = n.parent;
        let mut visited = std::collections::HashSet::from([n.id]);
        while let Some(id) = parent {
            if !visited.insert(id) || visited.len() > 64 {
                return Err(IoError::Format("cyclic or excessively deep model hierarchy".into()));
            }
            let ancestor = p.organization.nodes.iter().find(|n| n.id == id).ok_or_else(|| IoError::Format("missing model parent".into()))?;
            if ancestor.kind == NodeKind::Body {
                return Err(IoError::Format("body cannot be a parent".into()));
            }
            parent = ancestor.parent;
        }
        d.bump_handseed(cadcraft_doc::Handle(n.id));
    }
    for object in &p.geometry3d {
        let valid = match &object.shape {
            cadcraft_doc::organization::Shape::Curve(c) => c.valid(),
            cadcraft_doc::organization::Shape::Surface(s) => s.valid(),
        };
        if !valid
            || object.id == u64::MAX
            || object.name.trim().is_empty()
            || object.name.len() > 256
            || d.entity(cadcraft_doc::Handle(object.id)).is_some()
            || !ids.insert(object.id)
        {
            return Err(IoError::Format("invalid 3D object".into()));
        }
        d.bump_handseed(cadcraft_doc::Handle(object.id));
    }
    for object in &p.mesh3d {
        if object.id == u64::MAX
            || d.entity(cadcraft_doc::Handle(object.id)).is_some()
            || !ids.insert(object.id)
        {
            return Err(IoError::Format("invalid or duplicate polygon object identity".into()));
        }
        d.bump_handseed(cadcraft_doc::Handle(object.id));
    }
    p.production.validate().map_err(|e| IoError::Format(e.to_string()))?;
    d.feature_timelines = p.feature_timelines;
    d.organization = p.organization;
    d.validate_feature_histories().map_err(|e| IoError::Format(e.to_string()))?;
    d.production = p.production;
    d.geometry3d = p.geometry3d;
    d.mesh3d = p.mesh3d;
    Ok(d)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_preserves_named_body_and_geometry() {
        let mut d = Drawing::default();
        let h = d
            .add(
                &cadcraft_doc::Space::Model,
                cadcraft_doc::Common::default(),
                cadcraft_doc::EntityKind::Point(cadcraft_doc::Point { p: cadcraft_geom::Vec3::ZERO, angle: 0.0 }),
            )
            .unwrap();
        d.organization.nodes.push(cadcraft_doc::organization::ModelNode {
            id: 999,
            name: "Part".into(),
            kind: NodeKind::Body,
            parent: None,
            entities: vec![h],
        });
        let reopened = read(&write(&d).unwrap()).unwrap();
        assert_eq!(reopened.organization, d.organization);
        assert!(reopened.entity(h).is_some());
        assert!(reopened.handseed > 999);
    }
    #[test]
    fn rejects_future_version_and_cycles() {
        let d = Drawing::default();
        let mut p: Project = serde_json::from_slice(&write(&d).unwrap()).unwrap();
        p.version = 2;
        assert!(read(&serde_json::to_vec(&p).unwrap()).is_err());
        p.version = 1;
        p.organization.nodes.push(cadcraft_doc::organization::ModelNode {
            id: 1,
            name: "Cycle".into(),
            kind: NodeKind::Assembly,
            parent: Some(1),
            entities: vec![],
        });
        assert!(read(&serde_json::to_vec(&p).unwrap()).is_err());
    }
    #[test]
    fn exact_curve_roundtrip_and_lossy_exports_rejected() {
        let mut d = Drawing::default();
        d.geometry3d.push(cadcraft_doc::organization::GeometryObject {
            id: 1000,
            name: "Exact curve".into(),
            layer: "0".into(),
            visible: true,
            shape: cadcraft_doc::organization::Shape::Curve(
                cadcraft_geom::nurbs3d::Curve {
                    degree: 1,
                    control: vec![cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(1., 2., 3.)],
                    weights: vec![1., 1.],
                    knots: vec![0., 0., 1., 1.],
                }
                .into(),
            ),
        });
        let reopened = read(&write(&d).unwrap()).unwrap();
        assert_eq!(reopened.geometry3d, d.geometry3d);
        assert!(crate::write(&d, "output.dxf").is_err());
        assert!(crate::write(&d, "output.pdf").is_err());
        assert!(crate::plot(&d, &cadcraft_doc::Space::Model, &serde_json::json!({})).is_err());
    }

    #[test]
    fn polygon_quads_roundtrip_in_v1_and_legacy_file_opens() {
        use std::sync::Arc;
        let mut d = Drawing::new_metric();
        d.mesh3d.push(cadcraft_doc::organization::PolygonGeometryObject {
            id: 1000,
            name: "Rockwork fascia".into(),
            layer: "0".into(),
            visible: true,
            mesh: Arc::new(buildercraft_kernel::PolygonMesh {
                vertices: vec![
                    cadcraft_geom::Vec3::new(0., 0., 0.),
                    cadcraft_geom::Vec3::new(1., 0., 0.),
                    cadcraft_geom::Vec3::new(1., 1., 0.),
                    cadcraft_geom::Vec3::new(0., 1., 0.),
                ],
                faces: vec![buildercraft_kernel::PolygonFace::Quad([0, 1, 2, 3])],
            }),
        });
        d.organization.nodes.push(cadcraft_doc::organization::ModelNode {
            id: 999,
            name: "Scenery".into(),
            kind: NodeKind::Body,
            parent: None,
            entities: vec![cadcraft_doc::Handle(1000)],
        });
        let bytes = write(&d).unwrap();
        let reopened = read(&bytes).unwrap();
        assert_eq!(reopened.mesh3d, d.mesh3d);
        assert_eq!(reopened.organization, d.organization);
        assert!(reopened.handseed > 1000);
        assert!(crate::write(&d,"out.dxf").is_err());
        assert!(crate::write(&d,"out.pdf").is_err());
        assert!(crate::plot(&d,&cadcraft_doc::Space::Model,&serde_json::json!({})).is_err());

        // Before mesh support, v1 projects had no mesh3d key. They must still load.
        let mut legacy: serde_json::Value = serde_json::from_slice(&write(&Drawing::default()).unwrap()).unwrap();
        legacy.as_object_mut().unwrap().remove("mesh3d");
        let legacy_file = serde_json::to_vec(&legacy).unwrap();
        assert!(read(&legacy_file).is_ok_and(|d| d.mesh3d.is_empty()));
    }

    #[test]
    fn corrupt_polygon_or_duplicate_identity_rejected_on_read_and_write() {
        use std::sync::Arc;
        let mut d = Drawing::default();
        d.mesh3d.push(cadcraft_doc::organization::PolygonGeometryObject {
            id: 1000,
            name: "Fascia".into(),
            layer: "0".into(),
            visible: true,
            mesh: Arc::new(buildercraft_kernel::PolygonMesh {
                vertices: vec![
                    cadcraft_geom::Vec3::ZERO,
                    cadcraft_geom::Vec3::new(1.,0.,0.),
                    cadcraft_geom::Vec3::new(0.,1.,0.),
                ],
                faces: vec![buildercraft_kernel::PolygonFace::Triangle([0,1,2])],
            }),
        });
        let original = write(&d).unwrap();
        let mut encoded: serde_json::Value = serde_json::from_slice(&original).unwrap();
        encoded["mesh3d"][0]["mesh"]["faces"][0]["triangle"][2] = serde_json::json!(99);
        assert!(read(&serde_json::to_vec(&encoded).unwrap()).is_err());
        Arc::make_mut(&mut d.mesh3d[0].mesh).faces[0] =
            buildercraft_kernel::PolygonFace::Triangle([0,1,99]);
        assert!(write(&d).is_err());
        let mut encoded: serde_json::Value = serde_json::from_slice(&original).unwrap();
        let twin = encoded["mesh3d"][0].clone();
        encoded["mesh3d"].as_array_mut().unwrap().push(twin);
        assert!(read(&serde_json::to_vec(&encoded).unwrap()).is_err());
    }

    #[test]
    fn dftba_and_legacy_bcraft_share_the_same_native_payload() {
        let mut source = Drawing::new_metric();
        source.mesh3d.push(cadcraft_doc::organization::PolygonGeometryObject {
            id: 1000,
            name: "Worldwright quad".into(),
            layer: "0".into(),
            visible: true,
            mesh: std::sync::Arc::new(buildercraft_kernel::PolygonMesh {
                vertices: vec![
                    cadcraft_geom::Vec3::new(0., 0., 0.),
                    cadcraft_geom::Vec3::new(2., 0., 0.),
                    cadcraft_geom::Vec3::new(2., 2., 0.),
                    cadcraft_geom::Vec3::new(0., 2., 0.),
                ],
                faces: vec![buildercraft_kernel::PolygonFace::Quad([0, 1, 2, 3])],
            }),
        });
        let new_bytes = crate::write(&source, "worldwright.dftba").unwrap();
        let old_bytes = crate::write(&source, "legacy.bcraft").unwrap();
        assert_eq!(new_bytes, old_bytes);
        assert_eq!(crate::read(&new_bytes, "worldwright.dftba").unwrap().mesh3d, source.mesh3d);
        assert_eq!(crate::read(&old_bytes, "legacy.bcraft").unwrap().mesh3d, source.mesh3d);
        assert_eq!(crate::read(&old_bytes, "renamed.dftba").unwrap().mesh3d, source.mesh3d);
        assert!(crate::write(&source, "unsafe.dxf").is_err());
    }

    #[test]
    fn dftba_roundtrip_retains_document_and_block_local_histories() {
        use buildercraft_kernel::{
            FeatureHistoryEdit, FeatureScope, FeatureStep, FeatureInput,
            FeatureTimeline, ToolValue, TreeMatchPolicy,
        };
        use std::collections::BTreeMap;
        let mut d = Drawing::new_metric();
        let mut block = cadcraft_doc::Block::new("Bracket");
        block.description = "Parametric bracket definition".into();
        d.blocks.insert("Bracket".into(), std::sync::Arc::new(block));
        let mut local = FeatureTimeline::new(FeatureScope::BlockDefinition("Bracket".into())).unwrap();
        local.apply(0, FeatureHistoryEdit::Append { step: FeatureStep {
            id: 17,
            name: "Parametric Midpoint".into(),
            operation: "kernel.point.midpoint".into(),
            inputs: BTreeMap::from([
                ("a".into(), FeatureInput::Constant {
                    value: ToolValue::Point(cadcraft_geom::Vec3::ZERO),
                }),
                ("b".into(), FeatureInput::Constant {
                    value: ToolValue::Point(cadcraft_geom::Vec3::new(10., 0., 0.)),
                }),
            ]),
            matching: TreeMatchPolicy::Shortest,
            suppressed: false,
        }}).unwrap();
        d.feature_timelines.push(local);
        d.feature_timelines.push(FeatureTimeline::new(FeatureScope::Document).unwrap());
        let reopened = read(&write(&d).unwrap()).unwrap();
        assert_eq!(reopened.feature_timelines, d.feature_timelines);
        assert_eq!(reopened.feature_timelines[0].evaluate().unwrap().outputs.get(&17),
            Some(&ToolValue::Point(cadcraft_geom::Vec3::new(5., 0., 0.))));
    }

    #[test]
    fn missing_timeline_scope_is_invalid_and_old_dftba_defaults_to_no_histories() {
        use buildercraft_kernel::{FeatureScope, FeatureTimeline};
        let d = Drawing::new_metric();
        let bytes = write(&d).unwrap();
        let mut old: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        old.as_object_mut().unwrap().remove("feature_timelines");
        assert!(read(&serde_json::to_vec(&old).unwrap()).is_ok_and(|d| d.feature_timelines.is_empty()));

        let mut invalid = d.clone();
        invalid.feature_timelines.push(FeatureTimeline::new(
            FeatureScope::BlockDefinition("NotFound".into())
        ).unwrap());
        assert!(write(&invalid).is_err());

        old["feature_timelines"] = serde_json::to_value(&invalid.feature_timelines).unwrap();
        assert!(read(&serde_json::to_vec(&old).unwrap()).is_err());
    }

}
