//! Document-backed, undoable Project and FlowAlongSrf operations on native
//! polygon geometry. All geometry is evaluated before any document mutation.
//! Underlying kernel algorithms are shared with OrbWeaver, not duplicated.
use super::*;
use cadcraft_doc::organization::{PolygonGeometryObject, Shape};
use cadcraft_geom::Vec3;
use serde_json::json;
use std::sync::Arc;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("mesh3d.project", "Project Native Mesh Onto Mesh or NURBS", project)
            .params("{id,target_id,direction:{x,y,z},copy?:false,selected_revision?:number}"),
        CommandSpec::new("mesh3d.flow_along_srf", "Flow Native Mesh Between NURBS Surfaces", flow)
            .params("{id,base_id,target_id,copy?:false,selected_revision?:number}"),
    ]
}

fn invalid(message: &str) -> crate::EngineError {
    crate::EngineError::Other(message.into())
}
fn required_id(p: &Value, name: &str) -> Result<u64> {
    p.get(name).and_then(Value::as_u64).filter(|v| *v != 0).ok_or_else(|| invalid(&format!("{name} must be an object ID")))
}
fn copy_and_revision(s: &Session, p: &Value, allowed: &[&str]) -> Result<bool> {
    let obj = p.as_object().ok_or_else(|| invalid("modeling parameters must be an object"))?;
    if obj.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid("unsupported modeling option"));
    }
    let copy = p.get("copy").map(|v| v.as_bool().ok_or_else(|| invalid("copy must be boolean"))).transpose()?.unwrap_or(false);
    let selected_revision = p.get("selected_revision").map(|v| v.as_u64().ok_or_else(|| invalid("selected_revision must be numeric"))).transpose()?;
    if let Some(rev) = selected_revision {
        if rev != s.state()?.revision {
            return Err(invalid("selected geometry revision is stale"));
        }
    }
    Ok(copy)
}
fn mesh<'a>(s: &'a Session, id: u64) -> Result<&'a PolygonGeometryObject> {
    s.doc()?.mesh3d.iter().find(|x| x.id == id).ok_or_else(|| invalid("source polygon object does not exist"))
}
fn nurbs<'a>(s: &'a Session, id: u64) -> Result<&'a cadcraft_geom::nurbs3d::Surface> {
    let object = s.doc()?.geometry3d.iter().find(|x| x.id == id).ok_or_else(|| invalid("target NURBS object does not exist"))?;
    match &object.shape {
        Shape::Surface(surface) => Ok(surface.as_ref()),
        Shape::Curve(_) => Err(invalid("target object must be a rational NURBS surface")),
    }
}
fn publish(s: &mut Session, source_id: u64, copied: bool, result: buildercraft_kernel::PolygonMesh, suffix: &str) -> Result<Value> {
    if result.vertices.len() > 100_000 || result.faces.len() > 100_000 {
        return Err(invalid("document polygon capacity exceeded"));
    }
    buildercraft_kernel::polygon_mesh_validate(&result).map_err(|e| invalid(&e.to_string()))?;
    let source = mesh(s, source_id)?.clone();
    if copied {
        let d = s.doc()?;
        if d.geometry3d.len().checked_add(d.mesh3d.len()).is_none_or(|n| n >= 4096) || d.handseed == u64::MAX {
            return Err(invalid("native mesh object budget exceeded"));
        }
        let name = format!("{} {suffix}", source.name);
        if name.len() > 256 {
            return Err(invalid("new mesh name exceeds limit"));
        }
        let d = s.doc_mut()?;
        let id = d.new_handle().0;
        d.mesh3d.push(PolygonGeometryObject { id, name, layer: source.layer, visible: source.visible, mesh: Arc::new(result) });
        Ok(json!({"id":id,"source_id":source_id,"copy":true}))
    } else {
        let entry = s.doc_mut()?.mesh3d.iter_mut().find(|x| x.id == source_id).ok_or_else(|| invalid("source mesh removed"))?;
        entry.mesh = Arc::new(result);
        Ok(json!({"id":source_id,"copy":false}))
    }
}

/// Directional infinite-line projection of every source mesh vertex.
/// Target may be a native polygon mesh or an untrimmed rational NURBS surface.
/// Rejects face collapse and partial intersections rather than outputting a
/// silently invalid mesh. The source is updated/copy-created only on success.
fn project(s: &mut Session, p: &Value) -> Result<Value> {
    let copy = copy_and_revision(s, p, &["id", "target_id", "direction", "copy", "selected_revision"])?;
    let source_id = required_id(p, "id")?;
    let target_id = required_id(p, "target_id")?;
    if source_id == target_id {
        return Err(invalid("project target must differ from source"));
    }
    let direction: Vec3 =
        serde_json::from_value(p.get("direction").cloned().ok_or_else(|| invalid("direction required"))?).map_err(|e| invalid(&e.to_string()))?;
    let original = mesh(s, source_id)?.mesh.clone();
    let d = s.doc()?;
    let positions = if let Some(target) = d.mesh3d.iter().find(|x| x.id == target_id) {
        buildercraft_kernel::project_onto_mesh(&original.vertices, &target.mesh, direction)
    } else {
        let target = nurbs(s, target_id)?;
        buildercraft_kernel::project_onto_nurbs(&original.vertices, target, direction)
    }
    .map_err(|e| invalid(&e.to_string()))?;
    let next = buildercraft_kernel::PolygonMesh { vertices: positions, faces: original.faces.clone() };
    publish(s, source_id, copy, next, "projected")
}

/// Maps every vertex to the corresponding NURBS UV and preserves signed
/// source-normal offsets. Supports arbitrary untrimmed rational surfaces;
/// trimmed faces and user-defined seams are not represented by this kernel.
fn flow(s: &mut Session, p: &Value) -> Result<Value> {
    let copy = copy_and_revision(s, p, &["id", "base_id", "target_id", "copy", "selected_revision"])?;
    let source_id = required_id(p, "id")?;
    let base_id = required_id(p, "base_id")?;
    let target_id = required_id(p, "target_id")?;
    if base_id == target_id {
        return Err(invalid("source and target NURBS must be distinct objects"));
    }
    let original = mesh(s, source_id)?.mesh.clone();
    let base = nurbs(s, base_id)?;
    let target = nurbs(s, target_id)?;
    let positions = buildercraft_kernel::flow_along_nurbs(&original.vertices, base, target).map_err(|e| invalid(&e.to_string()))?;
    let next = buildercraft_kernel::PolygonMesh { vertices: positions, faces: original.faces.clone() };
    publish(s, source_id, copy, next, "flowed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::nurbs3d::{Curve, Surface, uniform_knots};
    fn p(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }
    fn face(z: f64) -> buildercraft_kernel::PolygonMesh {
        buildercraft_kernel::PolygonMesh {
            vertices: vec![p(0., 0., z), p(2., 0., z), p(2., 2., z), p(0., 2., z)],
            faces: vec![buildercraft_kernel::PolygonFace::Quad([0, 1, 2, 3])],
        }
    }
    fn plane(z: f64) -> Surface {
        let row = |y| Curve { degree: 1, control: vec![p(0., y, z), p(2., y, z)], weights: vec![1., 1.], knots: uniform_knots(2, 1) };
        Surface { rows: vec![row(0.), row(2.)], degree_v: 1, knots_v: uniform_knots(2, 1) }
    }
    fn create_mesh(s: &mut Session, z: f64) -> u64 {
        s.execute("mesh3d.create", &json!({"name":"test quad","mesh":face(z)})).unwrap()["id"].as_u64().unwrap()
    }
    fn create_surface(s: &mut Session, z: f64) -> u64 {
        s.execute("nurbs.surface", &json!({"name":"test surface","surface":plane(z)})).unwrap()["id"].as_u64().unwrap()
    }
    #[test]
    fn project_polygon_mesh_onto_nurbs_creates_undoable_copy() {
        let mut s = Session::new();
        let source = create_mesh(&mut s, 2.);
        let target = create_surface(&mut s, 0.);
        let old = s.doc().unwrap().mesh3d[0].mesh.clone();
        let rev = s.state().unwrap().revision;
        let result = s
            .execute(
                "mesh3d.project",
                &json!({
                    "id":source,"target_id":target,"direction":p(0.,0.,-1.),"copy":true,"selected_revision":rev
                }),
            )
            .unwrap();
        let id = result["id"].as_u64().unwrap();
        assert_ne!(id, source);
        assert_eq!(s.doc().unwrap().mesh3d.len(), 2);
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh, old);
        assert!(s.doc().unwrap().mesh3d[1].mesh.vertices.iter().all(|p| p.z.abs() < 1e-6));
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().mesh3d.len(), 1);
        assert!(
            s.execute(
                "mesh3d.project",
                &json!({
                    "id":source,"target_id":target,"direction":p(0.,0.,-1.),"selected_revision":rev
                })
            )
            .is_err()
        );
    }
    #[test]
    fn project_to_native_polygon_target_and_undo() {
        let mut s = Session::new();
        let source = create_mesh(&mut s, 3.);
        let target = create_mesh(&mut s, 0.);
        let result = s.execute(
            "mesh3d.project",
            &json!({
                "id":source,"target_id":target,"direction":p(0.,0.,1.)
            }),
        );
        assert!(result.is_ok());
        assert!(s.doc().unwrap().mesh3d[0].mesh.vertices.iter().all(|p| p.z.abs() < 1e-8));
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.vertices[0].z, 3.);
    }
    #[test]
    fn flow_between_nurbs_surfaces_preserves_topology_and_undo() {
        let mut s = Session::new();
        let source = create_mesh(&mut s, 0.);
        let base = create_surface(&mut s, 0.);
        let target = create_surface(&mut s, 5.);
        let revision = s.state().unwrap().revision;
        let moved = s.execute(
            "mesh3d.flow_along_srf",
            &json!({
                "id":source,"base_id":base,"target_id":target,"selected_revision":revision
            }),
        );
        assert!(moved.is_ok());
        let mesh = &s.doc().unwrap().mesh3d[0].mesh;
        assert_eq!(mesh.faces, face(0.).faces);
        assert!(mesh.vertices.iter().all(|p| (p.z - 5.).abs() < 1e-6));
        s.execute("undo", &json!({})).unwrap();
        assert!(s.doc().unwrap().mesh3d[0].mesh.vertices.iter().all(|p| p.z == 0.));
    }
    #[test]
    fn transformed_mesh_survives_native_file_roundtrip() {
        let mut s = Session::new();
        let source = create_mesh(&mut s, 0.);
        let base = create_surface(&mut s, 0.);
        let target = create_surface(&mut s, 5.);
        let edited = s
            .execute(
                "mesh3d.flow_along_srf",
                &json!({
                    "id":source,"base_id":base,"target_id":target,"copy":true
                }),
            )
            .unwrap();
        let transformed_id = edited["id"].as_u64().unwrap();
        let expected = s.doc().unwrap().mesh3d[1].mesh.clone();
        let bytes = cadcraft_io::write(s.doc().unwrap(), "modeling-roundtrip.bcraft").unwrap();
        let reopened = cadcraft_io::read(&bytes, "modeling-roundtrip.bcraft").unwrap();
        assert_eq!(reopened.mesh3d.len(), 2);
        assert_eq!(reopened.mesh3d[1].id, transformed_id);
        assert_eq!(reopened.mesh3d[1].mesh.as_ref(), expected.as_ref());
        assert!(reopened.mesh3d[1].mesh.vertices.iter().all(|p| (p.z - 5.).abs() < 1e-6));
    }

    #[test]
    fn invalid_geometry_and_stale_revision_do_not_edit_document() {
        let mut s = Session::new();
        let source = create_mesh(&mut s, 3.);
        let target = create_surface(&mut s, 0.);
        let previous = s.doc().unwrap().mesh3d.clone();
        assert!(
            s.execute(
                "mesh3d.project",
                &json!({
                    "id":source,"target_id":target,"direction":p(1.,0.,0.)
                })
            )
            .is_err()
        );
        assert!(
            s.execute(
                "mesh3d.flow_along_srf",
                &json!({
                    "id":source,"base_id":target,"target_id":target
                })
            )
            .is_err()
        );
        assert!(
            s.execute(
                "mesh3d.project",
                &json!({
                    "id":source,"target_id":target,"direction":p(0.,0.,-1.),"selected_revision":0
                })
            )
            .is_err()
        );
        assert_eq!(s.doc().unwrap().mesh3d, previous);
    }
}
