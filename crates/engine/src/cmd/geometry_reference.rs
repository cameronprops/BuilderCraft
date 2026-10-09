//! Metadata-only, revision-checked geometry references shared with OrbWeaver.
//! This is not a topology subelement naming engine or geometry upload API.
use super::*;
use buildercraft_kernel::{GeometryReference, Id, KernelError};
use serde_json::{json, Value};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("worldwright.geometry.ref.capture",
            "Capture Versioned 3D Geometry Reference", capture)
            .params("{project_id:hex32,object_id:uint,expected_revision:uint}")
            .noundo(),
        CommandSpec::new("worldwright.geometry.ref.resolve",
            "Validate Versioned 3D Geometry Reference", resolve)
            .params("{project_id:hex32,reference:{project_id,object_id,source_revision,kind},expected_revision:uint}")
            .noundo(),
    ]
}
fn project(p: &Value) -> Result<Id> {
    let raw = p.get("project_id").and_then(Value::as_str)
        .ok_or_else(|| bad("worldwright.geometry.ref", "project_id string required"))?;
    Id::try_from(raw.to_owned())
        .map_err(|e| bad("worldwright.geometry.ref", e.to_string()))
}
fn revision(session: &Session, p: &Value) -> Result<u64> {
    let expected = p.get("expected_revision").and_then(Value::as_u64)
        .ok_or_else(|| bad("worldwright.geometry.ref", "expected_revision required"))?;
    let actual = session.state()?.revision;
    if expected != actual {
        return Err(bad("worldwright.geometry.ref",
            KernelError::Conflict {expected, actual}.to_string()));
    }
    Ok(actual)
}
fn capture(session: &mut Session, p: &Value) -> Result<Value> {
    let project_id = project(p)?;
    let rev = revision(session, p)?;
    let object_id = p.get("object_id").and_then(Value::as_u64)
        .ok_or_else(|| bad("worldwright.geometry.ref.capture", "object_id required"))?;
    let reference = cadcraft_doc::kernel::capture_geometry_reference(
        session.doc()?, project_id, rev, object_id,
    ).map_err(|e| bad("worldwright.geometry.ref.capture", e.to_string()))?;
    Ok(json!({"reference":reference}))
}
fn resolve(session: &mut Session, p: &Value) -> Result<Value> {
    let project_id = project(p)?;
    let rev = revision(session, p)?;
    let reference: GeometryReference = serde_json::from_value(
        p.get("reference").cloned().ok_or_else(|| bad(
            "worldwright.geometry.ref.resolve", "reference required"
        ))?
    ).map_err(|e| bad("worldwright.geometry.ref.resolve", e.to_string()))?;
    let kind = cadcraft_doc::kernel::validate_geometry_reference(
        session.doc()?, &reference, project_id, rev,
    ).map_err(|e| bad("worldwright.geometry.ref.resolve", e.to_string()))?;
    Ok(json!({"valid":true,"kind":kind}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use cadcraft_geom::Vec3;
    use cadcraft_doc::organization::PolygonGeometryObject;
    use buildercraft_kernel::{PolygonMesh, PolygonFace};

    #[test]
    fn capture_and_validate_preserves_typed_reference_without_geometry_copy() {
        let mut session = Session::new();
        session.doc_mut().unwrap().mesh3d.push(PolygonGeometryObject {
            id: 400,
            name: "Panel".into(), layer: "0".into(), visible: true,
            mesh: Arc::new(PolygonMesh {
                vertices: vec![
                    Vec3::new(0., 0., 0.), Vec3::new(2., 0., 0.),
                    Vec3::new(2., 2., 0.), Vec3::new(0., 2., 0.),
                ],
                faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
            }),
        });
        let revision = session.state().unwrap().revision;
        let project_id = "00000000000000000000000000000123";
        let reference = session.execute("worldwright.geometry.ref.capture", &json!({
            "project_id":project_id, "object_id":400, "expected_revision":revision
        })).unwrap()["reference"].clone();
        assert_eq!(reference["kind"], "polygon_mesh");
        let validated = session.execute("worldwright.geometry.ref.resolve", &json!({
            "project_id":project_id, "expected_revision":revision, "reference":reference
        })).unwrap();
        assert_eq!(validated["valid"], true);
        assert_eq!(validated["kind"], "polygon_mesh");
        session.touch();
        let new_revision = session.state().unwrap().revision;
        assert!(session.execute("worldwright.geometry.ref.resolve", &json!({
            "project_id":project_id,"expected_revision":new_revision,
            "reference":reference
        })).is_err());
    }
    #[test]
    fn geometry_reference_rejects_foreign_project_identity() {
        let mut session = Session::new();
        let revision = session.state().unwrap().revision;
        assert!(session.execute("worldwright.geometry.ref.capture", &json!({
            "project_id":"not-a-valid-id","object_id":40,"expected_revision":revision
        })).is_err());
    }
}
