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
    production: buildercraft_kernel::ProductionModel,
}
pub fn write(d: &Drawing) -> Result<Vec<u8>> {
    d.production.validate().map_err(|e| IoError::Format(e.to_string()))?;
    let mut organization = d.organization.clone();
    for node in &mut organization.nodes {
        node.entities.retain(|h| d.entity(*h).is_some() || d.geometry3d.iter().any(|o| o.id == h.0));
    }
    serde_json::to_vec(&Project {
        version: 1,
        drawing_dxf: dxf_write::write(d),
        organization,
        geometry3d: d.geometry3d.clone(),
        production: d.production.clone(),
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
    if p.organization.nodes.len() > 100_000 || p.geometry3d.len() > 4096 {
        return Err(IoError::Format("too many model items".into()));
    }
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
            if (d.entity(*h).is_none() && !p.geometry3d.iter().any(|o| o.id == h.0)) || !owned.insert(*h) {
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
    p.production.validate().map_err(|e| IoError::Format(e.to_string()))?;
    d.production = p.production;
    d.geometry3d = p.geometry3d;
    d.organization = p.organization;
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
}
