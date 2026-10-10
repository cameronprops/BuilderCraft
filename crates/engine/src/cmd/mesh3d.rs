//! Native polygon CAD commands: persistent quad/triangle objects, transactional
//! edits, source topology diagnostics and non-destructive preview triangulation.
use super::*;
use buildercraft_kernel::{
    PolygonMesh, PolygonSceneEdit, apply_polygon_scene_edit, polygon_mesh_boundary_loops, polygon_mesh_triangulate, polygon_mesh_validate,
};
use cadcraft_doc::organization::PolygonGeometryObject;
use serde_json::json;
use serde::Deserialize;
use cadcraft_geom::Vec3;
use std::sync::Arc;

const MAX_EDIT_VERTICES: usize = 100_000;
const MAX_EDIT_FACES: usize = 100_000;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("mesh3d.create", "Create Native Polygon Mesh", create)
            .params("{name,mesh:{vertices:[{x,y,z},...],faces:[{triangle:[...]},{quad:[...]}]}}"),
        CommandSpec::new("mesh3d.array", "Array Native Polygon Mesh Object", array_object).params("{id,mode:linear|rectangular|polar|path, ...}"),
        CommandSpec::new("mesh3d.pushpull", "PushPull Face on Native Polygon Solid", pushpull_face).params("{id,face_index,distance,selected_revision?}"),
        CommandSpec::new("mesh3d.edit", "Edit Native Polygon Mesh", edit)
            .params("{id,edit:{kind:delete_faces|add_triangle_from_edge|fill_planar_hole,selected_revision,...}}"),
        CommandSpec::new("mesh3d.list", "List Native Polygon Meshes", list).noundo(),
        CommandSpec::new("mesh3d.boundaries", "Inspect Polygon Boundaries", boundaries).params("{id}").noundo(),
        CommandSpec::new("mesh3d.preview", "Preview Triangulated Polygon Mesh", preview).params("{id}").noundo(),
        CommandSpec::new("mesh3d.set", "Set Polygon Mesh Metadata", set).params("{id,name?,visible?}"),
    ]
}

fn invalid(message: &str) -> crate::EngineError {
    crate::EngineError::Other(message.into())
}
fn id(p: &Value) -> Result<u64> {
    p.get("id").and_then(Value::as_u64).ok_or_else(|| invalid("mesh ID required"))
}
fn selected(s: &Session, object_id: u64) -> Result<&PolygonGeometryObject> {
    s.doc()?.mesh3d.iter().find(|object| object.id == object_id).ok_or_else(|| invalid("unknown polygon object ID"))
}
fn validate_size(mesh: &PolygonMesh) -> Result<()> {
    if mesh.vertices.len() > MAX_EDIT_VERTICES || mesh.faces.len() > MAX_EDIT_FACES {
        return Err(invalid("interactive polygon mesh limit (100000 vertices and faces)"));
    }
    polygon_mesh_validate(mesh).map_err(|e| invalid(&e.to_string()))
}
fn list(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({
        "revision": s.state()?.revision,
        "objects": s.doc()?.mesh3d,
    }))
}
fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let name = p
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty() && name.len() <= 256)
        .ok_or_else(|| invalid("mesh name must be 1-256 characters"))?;
    let mesh: PolygonMesh =
        serde_json::from_value(p.get("mesh").cloned().ok_or_else(|| invalid("polygon mesh required"))?).map_err(|e| invalid(&e.to_string()))?;
    validate_size(&mesh)?;
    let d = s.doc_mut()?;
    if d.geometry3d.len().checked_add(d.mesh3d.len()).is_none_or(|n| n >= 4096) || d.handseed == u64::MAX {
        return Err(invalid("3D object or identity limit"));
    }
    let object_id = d.new_handle().0;
    d.mesh3d.push(PolygonGeometryObject {
        id: object_id,
        name: name.into(),
        layer: d.header.str("CLAYER", "0"),
        visible: true,
        mesh: Arc::new(mesh),
    });
    Ok(json!({"id":object_id,"kind":"polygonMesh"}))
}

/// Persistent face extrusion through the document undo/revision transaction.
/// An explicit pick revision is required for GUI callers that hold stale picks;
/// headless clients may omit it to use the current document revision.

#[derive(Deserialize)]
#[serde(tag="mode",rename_all="snake_case",deny_unknown_fields)]
enum MeshArraySpec {
    Linear { step: Vec3, count: usize },
    Rectangular { x_step: Vec3, y_step: Vec3, z_step: Vec3, nx: usize, ny: usize, nz: usize },
    Polar { center: Vec3, axis: Vec3, sweep_degrees: f64, count: usize },
    Path { path: Vec<Vec3>, count: usize },
}

/// Populate native document objects. All geometry and size checks finish
/// before a single document mutation, preserving the Session undo snapshot.
/// This command creates explicit independent meshes, not associative instances.
fn array_object(s: &mut Session, p: &Value) -> Result<Value> {
    let source_id=id(p)?;
    let source=selected(s,source_id)?.clone();
    let mut arguments=p.clone();
    arguments.as_object_mut().ok_or_else(|| invalid("array parameters must be a JSON object"))?.remove("id");
    let args:MeshArraySpec=serde_json::from_value(arguments).map_err(|e| invalid(&e.to_string()))?;
    let copies=match args {
        MeshArraySpec::Linear{step,count} => buildercraft_kernel::array_linear(&source.mesh.vertices,step,count),
        MeshArraySpec::Rectangular{x_step,y_step,z_step,nx,ny,nz} =>
            buildercraft_kernel::array_rectangular(&source.mesh.vertices,x_step,y_step,z_step,nx,ny,nz),
        MeshArraySpec::Polar{center,axis,sweep_degrees,count} =>
            buildercraft_kernel::array_polar(&source.mesh.vertices,center,axis,sweep_degrees,count),
        MeshArraySpec::Path{path,count} => buildercraft_kernel::array_path(&source.mesh.vertices,&path,count),
    }.map_err(|e| invalid(&e.to_string()))?;
    if copies.len()<2 { return Err(invalid("array needs at least two instances to add objects")); }
    let required=copies.len()-1;
    let d=s.doc()?;
    if d.geometry3d.len().checked_add(d.mesh3d.len()).and_then(|n| n.checked_add(required)).is_none_or(|n| n>4096)
        || d.handseed > u64::MAX - required as u64 {
        return Err(invalid("3D object or identity budget exceeded"));
    }
    let mut staged=Vec::new();
    staged.try_reserve_exact(required).map_err(|_| invalid("array allocation rejected"))?;
    for (i, vertices) in copies.into_iter().enumerate().skip(1) {
        let mesh=PolygonMesh { vertices, faces: source.mesh.faces.clone() };
        validate_size(&mesh)?;
        let name=format!("{} array {}",source.name,i);
        if name.len()>256 { return Err(invalid("array copy name too long")); }
        staged.push((name,Arc::new(mesh)));
    }
    let d=s.doc_mut()?;
    let mut ids=Vec::new();
    for (name,mesh) in staged {
        let new_id=d.new_handle().0;
        d.mesh3d.push(PolygonGeometryObject {
            id:new_id, name, layer:source.layer.clone(), visible:source.visible, mesh
        });
        ids.push(new_id);
    }
    Ok(json!({"source_id":source_id,"new_ids":ids,"copy_count":ids.len()}))
}

fn pushpull_face(s: &mut Session, p: &Value) -> Result<Value> {
    let object_id = id(p)?;
    let face_index = p.get("face_index").and_then(Value::as_u64).ok_or_else(|| invalid("face_index required"))?;
    let face_index = u32::try_from(face_index).map_err(|_| invalid("face_index exceeds supported size"))?;
    let distance = p.get("distance").and_then(Value::as_f64).filter(|d| d.is_finite()).ok_or_else(|| invalid("finite distance required"))?;
    let revision = s.state()?.revision;
    let pick_revision = p.get("selected_revision").map(|v| v.as_u64().ok_or_else(|| invalid("selected_revision must be a revision number"))).transpose()?.unwrap_or(revision);
    edit(s, &json!({"id":object_id, "edit":{"kind":"push_pull_face", "selected_revision":pick_revision, "face_index":face_index, "distance":distance}}))
}

fn edit(s: &mut Session, p: &Value) -> Result<Value> {
    let object_id = id(p)?;
    let operation: PolygonSceneEdit =
        serde_json::from_value(p.get("edit").cloned().ok_or_else(|| invalid("polygon edit required"))?).map_err(|e| invalid(&e.to_string()))?;
    let revision = s.state()?.revision;
    let source = selected(s, object_id)?;
    validate_size(&source.mesh)?;
    let edited = apply_polygon_scene_edit(&source.mesh, revision, &operation).map_err(|e| invalid(&e.to_string()))?;
    validate_size(&edited)?;
    // No document mutation occurs until the entire edit validates.
    let target = s.doc_mut()?.mesh3d.iter_mut().find(|o| o.id == object_id).ok_or_else(|| invalid("unknown polygon object ID"))?;
    target.mesh = Arc::new(edited);
    Ok(json!({"id":object_id,"kind":"polygonMesh"}))
}
fn boundaries(s: &mut Session, p: &Value) -> Result<Value> {
    let object_id = id(p)?;
    let report = polygon_mesh_boundary_loops(&selected(s, object_id)?.mesh).map_err(|e| invalid(&e.to_string()))?;
    Ok(json!({
        "id": object_id,
        "source_revision": s.state()?.revision,
        "report": report,
    }))
}
fn preview(s: &mut Session, p: &Value) -> Result<Value> {
    let object_id = id(p)?;
    let polygon = &selected(s, object_id)?.mesh;
    let preview = polygon_mesh_triangulate(polygon).map_err(|e| invalid(&e.to_string()))?;
    Ok(json!({
        "id": object_id,
        "source_revision": s.state()?.revision,
        "vertices": preview.mesh.vertices,
        "triangles": preview.mesh.triangles,
        "source_face_indices": preview.source_face_indices,
    }))
}
fn set(s: &mut Session, p: &Value) -> Result<Value> {
    let object_id = id(p)?;
    let name = p
        .get("name")
        .map(|v| v.as_str().filter(|s| !s.trim().is_empty() && s.len() <= 256).ok_or_else(|| invalid("invalid polygon object name")))
        .transpose()?;
    let visible = p.get("visible").map(|v| v.as_bool().ok_or_else(|| invalid("visible must be boolean"))).transpose()?;
    if name.is_none() && visible.is_none() {
        return Err(invalid("at least one mesh metadata change required"));
    }
    let target = s.doc_mut()?.mesh3d.iter_mut().find(|o| o.id == object_id).ok_or_else(|| invalid("unknown polygon object ID"))?;
    if let Some(name) = name {
        target.name = name.into();
    }
    if let Some(visible) = visible {
        target.visible = visible;
    }
    Ok(json!({"id":object_id}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildercraft_kernel::{PolygonFace, PolygonMesh};
    use cadcraft_geom::Vec3;

    #[test]
    fn persistent_face_pushpull_is_undoable_and_revision_checked() {
        let mut s = Session::new();
        let face = vec![Vec3::new(0.,0.,0.),Vec3::new(2.,0.,0.),Vec3::new(2.,2.,0.),Vec3::new(0.,2.,0.)];
        let mesh = buildercraft_kernel::pushpull_quad(&face,2.).unwrap();
        let id=s.execute("mesh3d.create",&json!({"name":"Extrusion target","mesh":mesh})).unwrap()["id"].as_u64().unwrap();
        let before=s.doc().unwrap().mesh3d[0].mesh.clone();
        let revision=s.state().unwrap().revision;
        let result=s.execute("mesh3d.pushpull",&json!({"id":id,"face_index":1,"distance":1.5,"selected_revision":revision}));
        assert!(result.is_ok());
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.faces.len(),10);
        assert!(s.execute("mesh3d.pushpull",&json!({"id":id,"face_index":1,"distance":1.,"selected_revision":revision})).is_err());
        s.execute("undo",&json!({})).unwrap();
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.as_ref(),before.as_ref());
    }

    #[test]
    fn document_array_creates_independent_geometry_and_is_undoable() {
        let mut s=Session::new();
        let square=vec![Vec3::new(0.,0.,0.),Vec3::new(1.,0.,0.),Vec3::new(1.,1.,0.),Vec3::new(0.,1.,0.)];
        let cube=buildercraft_kernel::pushpull_quad(&square,1.).unwrap();
        let id=s.execute("mesh3d.create",&json!({"name":"Seed","mesh":cube})).unwrap()["id"].as_u64().unwrap();
        let created=s.execute("mesh3d.array",&json!({"id":id,"mode":"polar","center":{"x":0.,"y":0.,"z":0.},"axis":{"x":0.,"y":0.,"z":1.},"sweep_degrees":360.,"count":4})).unwrap();
        assert_eq!(created["copy_count"],3);
        assert_eq!(s.doc().unwrap().mesh3d.len(),4);
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.vertices[0],Vec3::ZERO);
        s.execute("undo",&json!({})).unwrap();
        assert_eq!(s.doc().unwrap().mesh3d.len(),1);
        assert!(s.execute("mesh3d.array",&json!({"id":id,"mode":"linear","step":{"x":1.,"y":0.,"z":0.},"count":usize::MAX})).is_err());
        assert_eq!(s.doc().unwrap().mesh3d.len(),1);
    }

    fn ring() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(4., 0., 0.),
                Vec3::new(4., 4., 0.),
                Vec3::new(0., 4., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(3., 1., 0.),
                Vec3::new(3., 3., 0.),
                Vec3::new(1., 3., 0.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        }
    }

    fn session() -> (Session, u64) {
        let mut s = Session::new();
        let id = s.execute("mesh3d.create", &json!({"name":"Inner wall","mesh":ring()})).unwrap()["id"].as_u64().unwrap();
        (s, id)
    }

    #[test]
    fn create_preview_edit_and_undo_keep_native_quad_faces() {
        let (mut s, id) = session();
        let original = s.doc().unwrap().mesh3d[0].mesh.clone();
        let preview = s.execute("mesh3d.preview", &json!({"id":id})).unwrap();
        assert_eq!(preview["triangles"].as_array().unwrap().len(), 8);
        assert_eq!(preview["source_face_indices"].as_array().unwrap().len(), 8);
        let report = s.execute("mesh3d.boundaries", &json!({"id":id})).unwrap();
        let loops = report["report"]["closed_loops"].as_array().unwrap();
        assert_eq!(loops.len(), 2);
        let inner_index =
            loops.iter().position(|l| l["vertices"].as_array().is_some_and(|v| v.iter().all(|id| id.as_u64().is_some_and(|n| n >= 4)))).unwrap();
        let revision = report["source_revision"].as_u64().unwrap();
        s.execute(
            "mesh3d.edit",
            &json!({"id":id,"edit":{
                "kind":"fill_planar_hole","selected_revision":revision,"loop_index":inner_index
            }}),
        )
        .unwrap();
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.faces.len(), 6);
        assert!(matches!(s.doc().unwrap().mesh3d[0].mesh.faces[0], PolygonFace::Quad(_)));
        s.undo().unwrap();
        assert!(Arc::ptr_eq(&s.doc().unwrap().mesh3d[0].mesh, &original));
    }

    #[test]
    fn stale_pick_and_invalid_face_edits_are_atomic() {
        let (mut s, id) = session();
        let rev = s.state().unwrap().revision;
        let before = s.doc().unwrap().mesh3d[0].mesh.clone();
        assert!(
            s.execute(
                "mesh3d.edit",
                &json!({"id":id,"edit":{
                    "kind":"delete_faces","selected_revision":rev-1,"selected_faces":[0]
                }})
            )
            .is_err()
        );
        assert_eq!(s.state().unwrap().revision, rev);
        assert!(
            s.execute(
                "mesh3d.edit",
                &json!({"id":id,"edit":{
                    "kind":"delete_faces","selected_revision":rev,"selected_faces":[99]
                }})
            )
            .is_err()
        );
        assert!(Arc::ptr_eq(&s.doc().unwrap().mesh3d[0].mesh, &before));
        assert_eq!(s.state().unwrap().revision, rev);
    }

    #[test]
    fn document_mesh_persists_roundtrip_with_identity_and_undo() {
        let (mut s, id) = session();
        let original = s.doc().unwrap().mesh3d[0].mesh.clone();
        let bytes = cadcraft_io::write(s.doc().unwrap(), "fixture.bcraft").unwrap();
        let reopened = cadcraft_io::read(&bytes, "fixture.bcraft").unwrap();
        assert_eq!(reopened.mesh3d.len(), 1);
        assert_eq!(reopened.mesh3d[0].id, id);
        assert_eq!(reopened.mesh3d[0].mesh.as_ref(), original.as_ref());
        assert!(matches!(reopened.mesh3d[0].mesh.faces[0], PolygonFace::Quad(_)));
        let rev = s.state().unwrap().revision;
        s.execute(
            "mesh3d.edit",
            &json!({"id":id,"edit":{
                "kind":"delete_faces","selected_revision":rev,"selected_faces":[0]
            }}),
        )
        .unwrap();
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.faces.len(), 3);
        s.undo().unwrap();
        assert_eq!(s.doc().unwrap().mesh3d[0].mesh.as_ref(), original.as_ref());
    }
}
