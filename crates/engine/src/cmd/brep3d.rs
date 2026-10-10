//! Native CAD-facing exact BRep commands.
//!
//! One canonical OCCT worker owns Boolean/topology algorithms. WorldWright
//! persists the exact BRep payload; mesh previews are derived and never baked
//! over the source. Worker execution is one-shot, bounded, and fail-closed.
use super::*;
use cadcraft_doc::organization::ExactBrepObject;
use serde_json::json;
use std::sync::Arc;

const MAX_REQUEST_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BREP_BINARY_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_BINARY_BYTES: usize = 64 * 1024 * 1024;
const MAX_SOLIDS: usize = 64;
const MAX_MODEL_OBJECTS: usize = 4096;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("brep.box", "Exact BRep Box (OCCT)", box_solid).params("{name,min:[x,y,z],max:[x,y,z],tolerance?}"),
        CommandSpec::new("brep.sphere", "Exact BRep Sphere (OCCT)", sphere).params("{name,radius,tolerance?}"),
        CommandSpec::new("brep.cylinder", "Exact BRep Cylinder (OCCT)", cylinder).params("{name,radius,height:[dx,dy,dz],tolerance?}"),
        CommandSpec::new("brep.boolean", "Exact BRep Boolean (OCCT)", boolean)
            .params("{name,left_id,right_id,operation:union|difference|intersection,tolerance?}"),
        CommandSpec::new("brep.from_step", "Import Exact Solids from STEP", from_step).params("{name,step:base64,tolerance?}"),
        CommandSpec::new("brep.to_step", "Export Exact Solids to STEP", to_step).params("{ids:[id,...],tolerance?}").noundo(),
        CommandSpec::new("brep.inspect", "Inspect Exact Solid Topology", inspect).params("{id,tolerance?}").noundo(),
        CommandSpec::new("brep.preview", "Derived BRep Shaded Preview", preview)
            .params("{id,linear_deflection,angular_deflection,tolerance?}")
            .noundo(),
        CommandSpec::new("brep.list", "List Exact BRep Solids", list).noundo(),
        CommandSpec::new("brep.set", "Rename or Show Exact Solid", set).params("{id,name?,visible?}"),
    ]
}

fn fail(message: impl Into<String>) -> EngineError {
    EngineError::Other(message.into())
}

fn name(p: &Value) -> Result<&str> {
    p.get("name")
        .and_then(Value::as_str)
        .filter(|n| !n.trim().is_empty() && n.len() <= 256)
        .ok_or_else(|| fail("BRep name must be 1..256 nonblank UTF-8 bytes"))
}

fn uid(p: &Value, key: &str) -> Result<u64> {
    p.get(key).and_then(Value::as_u64).ok_or_else(|| fail(format!("{key} must be a document object ID")))
}

fn tolerance(p: &Value) -> Result<Option<f64>> {
    p.get("tolerance")
        .map(|v| {
            v.as_f64()
                .filter(|t| t.is_finite() && *t > 0.0 && *t <= 1.0)
                .ok_or_else(|| fail("tolerance must be finite, positive and at most 1 document unit"))
        })
        .transpose()
}

fn get<'a>(s: &'a Session, id: u64) -> Result<&'a ExactBrepObject> {
    s.doc()?.exact_breps.iter().find(|o| o.id == id).ok_or_else(|| fail("unknown exact BRep object ID"))
}

fn object_count(s: &Session) -> Result<usize> {
    let d = s.doc()?;
    d.geometry3d
        .len()
        .checked_add(d.mesh3d.len())
        .and_then(|v| v.checked_add(d.exact_breps.len()))
        .ok_or_else(|| fail("CAD 3D object count overflow"))
}

fn summary(o: &ExactBrepObject) -> Value {
    json!({"id":o.id,"name":o.name,"layer":o.layer,"visible":o.visible,
           "volume":o.volume,"faces":o.faces,"edges":o.edges,"representation":"exact_occt_brep"})
}

fn list(s: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"revision":s.state()?.revision,
        "objects":s.doc()?.exact_breps.iter().map(summary).collect::<Vec<_>>() }))
}

fn set(s: &mut Session, p: &Value) -> Result<Value> {
    let id = uid(p, "id")?;
    let title = p.get("name").map(|_| name(p).map(str::to_owned)).transpose()?;
    let visible = p.get("visible").map(|v| v.as_bool().ok_or_else(|| fail("visible must be Boolean"))).transpose()?;
    let d = s.doc_mut()?;
    let o = d.exact_breps.iter_mut().find(|o| o.id == id).ok_or_else(|| fail("unknown exact BRep object ID"))?;
    if let Some(title) = title {
        o.name = title;
    }
    if let Some(visible) = visible {
        o.visible = visible;
    }
    Ok(summary(o))
}

fn worker_params(p: &Value, op: &str) -> Result<Value> {
    let mut req = p.clone();
    let map = req.as_object_mut().ok_or_else(|| fail("BRep inputs must be a JSON object"))?;
    map.remove("name");
    map.insert("op".into(), json!(op));
    if let Some(v) = tolerance(p)? {
        map.insert("tolerance".into(), json!(v));
    }
    Ok(req)
}

fn box_solid(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    for field in ["min", "max"] {
        let xyz = p
            .get(field)
            .and_then(Value::as_array)
            .filter(|v| v.len() == 3 && v.iter().all(|x| x.as_f64().is_some_and(f64::is_finite)))
            .ok_or_else(|| fail(format!("{field} must contain 3 finite coordinates")))?;
        let _ = xyz;
    }
    let req = worker_params(p, "box")?;
    add_worker_shapes(s, p, call_worker(&req)?)
}

fn sphere(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    let _ = positive(p, "radius")?;
    add_worker_shapes(s, p, call_worker(&worker_params(p, "sphere")?)?)
}

fn cylinder(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    let _ = positive(p, "radius")?;
    if !p.get("height").and_then(Value::as_array).is_some_and(|a| a.len() == 3 && a.iter().all(|v| v.as_f64().is_some_and(f64::is_finite))) {
        return Err(fail("height must be a finite 3-vector"));
    }
    add_worker_shapes(s, p, call_worker(&worker_params(p, "cylinder")?)?)
}

fn positive(p: &Value, key: &str) -> Result<f64> {
    p.get(key).and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0).ok_or_else(|| fail(format!("{key} must be finite and positive")))
}

fn boolean(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    let lhs = uid(p, "left_id")?;
    let rhs = uid(p, "right_id")?;
    let operation = p
        .get("operation")
        .and_then(Value::as_str)
        .filter(|v| matches!(*v, "union" | "difference" | "intersection"))
        .ok_or_else(|| fail("Boolean operation must be union, difference or intersection"))?;
    let req = json!({"op":"boolean","operation":operation,
        "left_brep":get(s,lhs)?.brep.as_str(),"right_brep":get(s,rhs)?.brep.as_str(),
        "tolerance":tolerance(p)?});
    // Preserves the operands. A multi-body result becomes one undoable
    // transaction; empty intersection cannot mutate the drawing.
    add_worker_shapes(s, p, call_worker(&req)?)
}

fn from_step(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    let step = p
        .get("step")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty() && v.len() < 6 * 1024 * 1024)
        .ok_or_else(|| fail("bounded Base64 STEP payload required"))?;
    let req = json!({"op":"from_step","step":step,"tolerance":tolerance(p)?});
    add_worker_shapes(s, p, call_worker(&req)?)
}

fn to_step(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = p
        .get("ids")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty() && v.len() <= MAX_SOLIDS)
        .ok_or_else(|| fail("STEP export requires 1..64 solid IDs"))?;
    let mut breps = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id.as_u64().ok_or_else(|| fail("invalid STEP solid ID"))?;
        breps.push(get(s, id)?.brep.as_str());
    }
    let req = json!({"op":"to_step","breps":breps,"tolerance":tolerance(p)?});
    let reply = call_worker(&req)?;
    let step = reply.get("step").and_then(Value::as_str).ok_or_else(|| fail("worker STEP result missing"))?;
    Ok(json!({"step":step,"encoding":"base64","format":"STEP",
              "exact":true,"source_revision":s.state()?.revision}))
}

fn inspect(s: &mut Session, p: &Value) -> Result<Value> {
    let id = uid(p, "id")?;
    let object = get(s, id)?;
    let reply = call_worker(&json!({"op":"inspect","brep":object.brep.as_str(),
        "tolerance":tolerance(p)?}))?;
    let solids = reply.get("solids").and_then(Value::as_array).filter(|v| v.len() == 1).ok_or_else(|| fail("inspect did not return one solid"))?;
    let statistics = &solids[0];
    Ok(json!({"id":id,"source_revision":s.state()?.revision,
        "exact":true,"volume":statistics["volume"],
        "faces":statistics["faces"],"edges":statistics["edges"]}))
}

fn preview(s: &mut Session, p: &Value) -> Result<Value> {
    let id = uid(p, "id")?;
    let o = get(s, id)?;
    let linear = positive(p, "linear_deflection")?;
    let angular = positive(p, "angular_deflection")?;
    let reply = call_worker(&json!({"op":"tessellate","brep":o.brep.as_str(),
        "linear_deflection":linear,"angular_deflection":angular,
        "tolerance":tolerance(p)?}))?;
    let mesh = reply.get("mesh").ok_or_else(|| fail("worker tessellation missing"))?;
    if mesh.get("exact").and_then(Value::as_bool) != Some(false) {
        return Err(fail("display proxy must be explicitly inexact"));
    }
    Ok(json!({"source_id":id,"source_revision":s.state()?.revision,
        "authoritative_geometry":"exact_occt_brep","display_only":true,"mesh":mesh}))
}

fn add_worker_shapes(s: &mut Session, p: &Value, reply: Value) -> Result<Value> {
    let label = name(p)?.to_string();
    let before = s.state()?.revision;
    let solids =
        reply.get("solids").and_then(Value::as_array).filter(|v| v.len() <= MAX_SOLIDS).ok_or_else(|| fail("bounded worker solids array missing"))?;
    if object_count(s)?.checked_add(solids.len()).is_none_or(|n| n > MAX_MODEL_OBJECTS) {
        return Err(fail("3D object budget exceeded"));
    }
    let mut total = 0usize;
    let mut staged = Vec::with_capacity(solids.len());
    let source = s.doc()?;
    for original in &source.exact_breps {
        total = total.checked_add(original.brep.len().saturating_mul(3) / 4).ok_or_else(|| fail("BRep storage budget overflow"))?;
    }
    for (index, item) in solids.iter().enumerate() {
        let payload = item.get("brep").and_then(Value::as_str).ok_or_else(|| fail("exact binary BRep missing"))?;
        if payload.len() > 6 * 1024 * 1024 {
            return Err(fail("BRep encoded length exceeds limit"));
        }
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payload).map_err(|_| fail("worker returned invalid BRep Base64"))?;
        if bytes.is_empty() || bytes.len() > MAX_BREP_BINARY_BYTES {
            return Err(fail("exact BRep payload exceeds binary budget"));
        }
        total = total.checked_add(bytes.len()).ok_or_else(|| fail("BRep storage overflow"))?;
        if total > MAX_TOTAL_BINARY_BYTES {
            return Err(fail("aggregate 64 MiB BRep cap exceeded"));
        }
        let volume = item.get("volume").and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0).ok_or_else(|| fail("invalid solid volume"))?;
        let faces = item
            .get("faces")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0)
            .ok_or_else(|| fail("invalid exact solid face count"))?;
        let edges = item
            .get("edges")
            .and_then(Value::as_u64)
            .and_then(|v| u32::try_from(v).ok())
            .filter(|v| *v > 0)
            .ok_or_else(|| fail("invalid exact solid edge count"))?;
        let object_name = if solids.len() == 1 { label.clone() } else { format!("{} {}", label, index + 1) };
        if object_name.len() > 256 {
            return Err(fail("exact BRep result name too long"));
        }
        staged.push(ExactBrepObject {
            id: 0,
            name: object_name,
            layer: source.header.str("CLAYER", "0"),
            visible: true,
            brep: Arc::new(payload.to_owned()),
            volume,
            faces,
            edges,
        });
    }
    if s.state()?.revision != before {
        return Err(fail("document revision changed during BRep execution"));
    }
    let existing_handseed = s.doc()?.handseed;
    if existing_handseed == u64::MAX || existing_handseed.checked_add(staged.len() as u64).is_none() {
        return Err(fail("object identity overflow"));
    }
    let d = s.doc_mut()?;
    let mut ids = Vec::with_capacity(staged.len());
    for mut item in staged {
        item.id = d.new_handle().0;
        ids.push(item.id);
        d.exact_breps.push(item);
    }
    Ok(json!({"ids":ids,"count":ids.len(),"representation":"exact_occt_brep",
              "source_revision":before,"undoable":true}))
}

#[cfg(target_arch = "wasm32")]
fn call_worker(_: &Value) -> Result<Value> {
    Err(fail("exact OpenCascade BRep worker is not available in WebAssembly"))
}

#[cfg(not(target_arch = "wasm32"))]
fn call_worker(request: &Value) -> Result<Value> {
    use std::io::{Read, Write};
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let payload = serde_json::to_vec(request).map_err(|e| fail(format!("encode BRep request: {e}")))?;
    if payload.len() > MAX_REQUEST_BYTES {
        return Err(fail("BRep worker request exceeds 8 MiB cap"));
    }
    // Only trusted native installation configuration or the executable
    // installed alongside CAD may select the worker. Never use a user/API
    // parameter as an executable path or search the current PATH.
    let executable = match std::env::var_os("WORLDWRIGHT_BREP_WORKER") {
        Some(explicit) => std::path::PathBuf::from(explicit),
        None => {
            let binary = if cfg!(windows) { "worldwright-brep-worker.exe" } else { "worldwright-brep-worker" };
            std::env::current_exe().map_err(|e| fail(format!("locate CAD executable: {e}")))?.with_file_name(binary)
        }
    };
    if !executable.is_absolute() {
        return Err(fail("WORLDWRIGHT_BREP_WORKER must specify an absolute executable path"));
    }
    let mut child = Command::new(&executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| fail(format!("exact BRep worker unavailable at {}: {e}", executable.display())))?;
    let mut input = child.stdin.take().ok_or_else(|| fail("worker stdin unavailable"))?;
    let stdout = child.stdout.take().ok_or_else(|| fail("worker stdout unavailable"))?;
    // Drain output while waiting. A closed/oversized output pipe cannot block
    // the caller indefinitely: timeout kills the native process.
    let reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
        let mut output = Vec::new();
        stdout.take(MAX_RESPONSE_BYTES + 1).read_to_end(&mut output)?;
        Ok(output)
    });
    // Worker initialization itself may hang before reading stdin. Put the
    // writer on a separate thread so our deadline also bounds a full pipe.
    let writer = std::thread::spawn(move || -> std::io::Result<()> { input.write_all(&payload) });
    let deadline = Instant::now() + Duration::from_secs(30);
    let finished = (|| -> Result<std::process::ExitStatus> {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
                Ok(None) => return Err(fail("exact BRep worker timed out after 30 seconds")),
                Err(e) => return Err(fail(format!("cannot wait for BRep worker: {e}"))),
            }
        }
    })();
    if finished.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let wrote = writer.join().map_err(|_| fail("worker input writer panicked"))?;
    let output = reader.join().map_err(|_| fail("worker output reader panicked"))?.map_err(|e| fail(format!("reading BRep worker output: {e}")))?;
    let status = finished?;
    wrote.map_err(|e| fail(format!("writing BRep worker input: {e}")))?;
    if !status.success() {
        return Err(fail(format!("exact BRep worker terminated ({status})")));
    }
    if output.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(fail("BRep worker response exceeds 16 MiB"));
    }
    let reply: Value = serde_json::from_slice(&output).map_err(|e| fail(format!("malformed BRep worker response: {e}")))?;
    if reply.get("ok").and_then(Value::as_bool) != Some(true) {
        return Err(fail(reply.get("error").and_then(Value::as_str).unwrap_or("exact BRep worker rejected operation")));
    }
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_worker_is_an_explicit_capability_not_a_mesh_fallback() {
        let mut s = Session::new();
        assert!(s.execute("brep.box", &json!({"name":"Invalid","min":[0.,0.],"max":[1.,1.,1.]})).is_err());
        assert_eq!(s.doc().unwrap().exact_breps.len(), 0);
        let result = s.execute("brep.boolean", &json!({"name":"cut","left_id":1,"right_id":2,"operation":"voxel"}));
        assert!(result.is_err());
        assert_eq!(s.doc().unwrap().exact_breps.len(), 0);
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn exact_worker_document_undo_and_roundtrip_when_worker_is_installed() {
        // Run in the native BRep CI matrix with WORLDWRIGHT_BREP_WORKER set.
        // Local headless jobs without the native OCCT worker skip this fixture.
        if std::env::var_os("WORLDWRIGHT_BREP_WORKER").is_none() {
            return;
        }
        let mut s = Session::new();
        let a = s.execute("brep.box", &json!({"name":"A","min":[0.,0.,0.],"max":[2.,2.,2.]})).unwrap();
        let b = s.execute("brep.box", &json!({"name":"B","min":[1.,1.,1.],"max":[3.,3.,3.]})).unwrap();
        let id_a = a["ids"][0].as_u64().unwrap();
        let id_b = b["ids"][0].as_u64().unwrap();
        let u = s.execute("brep.boolean", &json!({"name":"Union","left_id":id_a,"right_id":id_b,"operation":"union"})).unwrap();
        assert_eq!(u["count"], 1);
        assert_eq!(s.doc().unwrap().exact_breps.len(), 3);
        let cut = s.execute("brep.boolean", &json!({"name":"Cut","left_id":id_a,"right_id":id_b,"operation":"difference"})).unwrap();
        assert_eq!(cut["count"], 1);
        let exported = cadcraft_io::write(s.doc().unwrap(), "solid.dftba").unwrap();
        let reopened = cadcraft_io::read(&exported, "solid.dftba").unwrap();
        assert_eq!(reopened.exact_breps, s.doc().unwrap().exact_breps);
        assert!(cadcraft_io::write(&reopened, "solid.dxf").is_err());
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().exact_breps.len(), 3);
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().exact_breps.len(), 4);
    }
}
