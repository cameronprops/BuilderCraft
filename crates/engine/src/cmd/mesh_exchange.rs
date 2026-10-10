//! Shared IO adapters, usable from CAD scripts, CLI and future Scan/Graph nodes.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use cadcraft_io::mesh_exchange::{MAX_BYTES, MAX_SAMPLES, read_mesh, write_mesh};
use serde::Deserialize;
use serde_json::json;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("worldwright.mesh.decode", "Decode STL or Triangular OBJ", decode)
            .params("{format:stl|obj,data:string,encoding?:utf8|base64} -> {mesh,losses}; bounded read-only exchange, no external files")
            .enabled(always)
            .noundo(),
        CommandSpec::new("worldwright.mesh.encode", "Encode STL or OBJ", encode)
            .params("{format:stl|obj,mesh:{vertices:[{x,y,z}],triangles:[[a,b,c]]}} -> {data,encoding,losses,max_coordinate_error}")
            .enabled(always)
            .noundo(),
    ]
}
fn fail(message: impl Into<String>) -> EngineError {
    bad("worldwright.mesh.exchange", message)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Decode {
    format: String,
    data: String,
    encoding: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Encode {
    format: String,
    mesh: buildercraft_kernel::TriangleMesh,
}
fn decode(_: &mut Session, p: &Value) -> Result<Value> {
    if p.get("data").and_then(Value::as_str).is_none_or(|s| s.len() > MAX_BYTES * 4 / 3 + 4) {
        return Err(fail("Encoded mesh input budget exceeded"));
    }
    let input: Decode = serde_json::from_value(p.clone()).map_err(|e| fail(e.to_string()))?;
    let bytes = match input.encoding.as_deref().unwrap_or("utf8") {
        "utf8" => input.data.into_bytes(),
        "base64" => STANDARD.decode(input.data).map_err(|e| fail(e.to_string()))?,
        _ => return Err(fail("Encoding must be utf8 or base64")),
    };
    let decoded = read_mesh(&bytes, &input.format).map_err(|e| fail(e.to_string()))?;
    Ok(json!({"mesh":decoded.mesh,"losses":decoded.losses,"document_mutated":false,"units":"caller_owned"}))
}
fn encode(_: &mut Session, p: &Value) -> Result<Value> {
    for key in ["vertices", "triangles"] {
        if p.get("mesh").and_then(|m| m.get(key)).and_then(Value::as_array).is_none_or(|a| a.is_empty() || a.len() > MAX_SAMPLES) {
            return Err(fail("Mesh exchange geometry budget exceeded"));
        }
    }
    let input: Encode = serde_json::from_value(p.clone()).map_err(|e| fail(e.to_string()))?;
    let output = write_mesh(&input.mesh, &input.format).map_err(|e| fail(e.to_string()))?;
    let (data, encoding) = if input.format == "obj" {
        (String::from_utf8(output.bytes).map_err(|e| fail(e.to_string()))?, "utf8")
    } else {
        (STANDARD.encode(output.bytes), "base64")
    };
    Ok(json!({"data":data,"encoding":encoding,"losses":output.losses,"max_coordinate_error":output.max_coordinate_error,"source_preserved":true}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exchange_commands_round_trip_without_revision_or_undo_mutation() {
        let mut session = Session::new();
        let before = session.state().unwrap().doc.clone();
        let revision = session.state().unwrap().revision;
        let mesh = json!({"vertices":[{"x":0.,"y":0.,"z":0.},{"x":2.,"y":0.,"z":0.},{"x":0.,"y":2.,"z":0.}],"triangles":[[0,1,2]]});
        for format in ["obj", "stl"] {
            let encoded = session.execute("worldwright.mesh.encode", &json!({"format":format,"mesh":mesh})).unwrap();
            let decoded =
                session.execute("worldwright.mesh.decode", &json!({"format":format,"encoding":encoded["encoding"],"data":encoded["data"]})).unwrap();
            assert_eq!(decoded["mesh"], mesh);
            let result = session.execute("worldwright.mesh.closest_point", &json!({"mesh":decoded["mesh"],"query":[0.5,0.5,3.]})).unwrap();
            assert_eq!(result["hit"]["distance"], 3.);
        }
        assert_eq!(session.state().unwrap().revision, revision);
        assert!(std::sync::Arc::ptr_eq(&before, &session.state().unwrap().doc));
        assert!(session.execute("worldwright.mesh.decode", &json!({"format":"stl","encoding":"base64","data":"***"})).is_err());
    }
}
