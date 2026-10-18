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
        CommandSpec::new("brep.pipe", "Exact BRep Pipe Along Polyline Spine (OCCT)", pipe)
            .params("{name,rail:[[x,y,z],...],radius,up:[x,y,z],tolerance?}"),
        CommandSpec::new("brep.sweep1", "Exact Single-Rail Constant Section Sweep (OCCT)", sweep_one)
            .params("{name,rail:[[x,y,z],...],section:[[u,v],...],up:[x,y,z],tolerance?}"),
        CommandSpec::new("brep.sweep2", "Exact Two-Rail Section Loft (OCCT)", sweep_two)
            .params("{name,rail_a:[[x,y,z],...],rail_b:[[x,y,z],...],section:[[u,v],...],tolerance?}"),
        CommandSpec::new("brep.pipe_from_curve", "Create Exact BRep Pipe from Native NURBS Object", pipe_from_curve)
            .params("{name,curve_id,radius,up:[x,y,z],chord_tolerance?:0.01,max_segments?:127,tolerance?,selected_revision?}"),
        CommandSpec::new("brep.sweep1_from_curve", "Create Exact BRep Sweep1 from Native NURBS Object", sweep_one_from_curve)
            .params("{name,curve_id,section:[[u,v],...],up:[x,y,z],chord_tolerance?:0.01,max_segments?:127,tolerance?,selected_revision?}"),
        CommandSpec::new("brep.sweep2_from_curves", "Create Exact BRep Sweep2 from Two Native NURBS Objects", sweep_two_from_curves)
            .params("{name,curve_a_id,curve_b_id,section:[[u,v],...],chord_tolerance?:0.01,max_segments?:127,tolerance?,selected_revision?}"),
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

fn get(s: &Session, id: u64) -> Result<&ExactBrepObject> {
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

fn point_field(p: &Value, key: &str, dimensions: usize) -> Result<()> {
    let coords = p
        .get(key)
        .and_then(Value::as_array)
        .filter(|v| v.len() == dimensions && v.iter().all(|n| n.as_f64().is_some_and(|v| v.is_finite() && v.abs() <= 1e9)));
    if coords.is_none() {
        return Err(fail(format!("{key} must contain {dimensions} finite coordinates")));
    }
    Ok(())
}
fn rail_field(p: &Value, key: &str) -> Result<()> {
    let points = p
        .get(key)
        .and_then(Value::as_array)
        .filter(|v| (2..=128).contains(&v.len()))
        .ok_or_else(|| fail(format!("{key} rail must have 2..128 stations")))?;
    for v in points {
        let values = v
            .as_array()
            .filter(|xyz| xyz.len() == 3 && xyz.iter().all(|n| n.as_f64().is_some_and(|n| n.is_finite() && n.abs() <= 1e9)))
            .ok_or_else(|| fail(format!("{key} rail point must be [x,y,z]")))?;
        let _ = values;
    }
    Ok(())
}
fn section_field(p: &Value) -> Result<()> {
    let points = p
        .get("section")
        .and_then(Value::as_array)
        .filter(|v| (3..=32).contains(&v.len()))
        .ok_or_else(|| fail("section must have 3..32 planar coordinates"))?;
    for v in points {
        let xyz = v
            .as_array()
            .filter(|v| v.len() == 2 && v.iter().all(|n| n.as_f64().is_some_and(|n| n.is_finite() && n.abs() <= 1e8)))
            .ok_or_else(|| fail("section coordinates must be finite [u,v]"))?;
        let _ = xyz;
    }
    Ok(())
}

fn pipe(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    rail_field(p, "rail")?;
    point_field(p, "up", 3)?;
    let _ = positive(p, "radius")?;
    let req = worker_params(p, "pipe")?;
    add_worker_shapes(s, p, call_worker(&req)?)
}
fn sweep_one(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    rail_field(p, "rail")?;
    section_field(p)?;
    point_field(p, "up", 3)?;
    add_worker_shapes(s, p, call_worker(&worker_params(p, "sweep1")?)?)
}
fn sweep_two(s: &mut Session, p: &Value) -> Result<Value> {
    name(p)?;
    rail_field(p, "rail_a")?;
    rail_field(p, "rail_b")?;
    section_field(p)?;
    if p["rail_a"].as_array().map(Vec::len) != p["rail_b"].as_array().map(Vec::len) {
        return Err(fail("two-rail Sweep2 requires equal station counts"));
    }
    add_worker_shapes(s, p, call_worker(&worker_params(p, "sweep2")?)?)
}

/// Resolve a document-owned rational curve without discretizing it.
/// The adaptive kernel owns all geometric approximation and proof bounds.
fn document_curve(s: &Session, p: &Value, id_key: &str) -> Result<(u64, std::sync::Arc<cadcraft_geom::nurbs3d::Curve>)> {
    let id = uid(p, id_key)?;
    let object = s.doc()?.geometry3d.iter().find(|o| o.id == id).ok_or_else(|| fail("NURBS rail object not found"))?;
    let cadcraft_doc::organization::Shape::Curve(curve) = &object.shape else {
        return Err(fail("selected geometry must be a 3D rational NURBS curve"));
    };
    if !curve.valid() { return Err(fail("NURBS rail is invalid")); }
    Ok((id,curve.clone()))
}
fn rail_sampling_config(p: &Value) -> Result<(f64,usize)> {
    let chord_tolerance = p.get("chord_tolerance")
        .map(|v| v.as_f64().filter(|v| v.is_finite() && (1e-7..=1e7).contains(v))
            .ok_or_else(|| fail("chord_tolerance must be finite in document units, between 1e-7 and 1e7")))
        .transpose()?.unwrap_or(0.01);
    if p.get("max_segments").is_some() && p.get("segments").is_some() {
        return Err(fail("use max_segments, not both max_segments and deprecated segments"));
    }
    let budget = p.get("max_segments").or_else(|| p.get("segments"))
        .map(|v| v.as_u64().and_then(|n| usize::try_from(n).ok())
            .filter(|n| (1..=127).contains(n))
            .ok_or_else(|| fail("NURBS rail max_segments must be 1..127")))
        .transpose()?.unwrap_or(127);
    Ok((chord_tolerance,budget))
}
fn certified_worker_rails(curves:&[&cadcraft_geom::nurbs3d::Curve],tolerance:f64,budget:usize)
    -> Result<buildercraft_kernel::CertifiedRailApproximation> {
    let output=buildercraft_kernel::nurbs_rails_with_chord_bound(curves,tolerance,budget)
        .map_err(|e| fail(format!("NURBS rail chord tolerance could not be satisfied: {e}")))?;
    if output.rails.iter().flatten().any(|p| p.x.abs()>1e9 || p.y.abs()>1e9 || p.z.abs()>1e9) {
        return Err(fail("rail coordinate outside bounded native worker range"));
    }
    Ok(output)
}
fn triplets(points:&[cadcraft_geom::Vec3])->Vec<[f64;3]> {
    points.iter().map(|p| [p.x,p.y,p.z]).collect()
}
fn check_curve_revision(s: &Session, p: &Value) -> Result<()> {
    if let Some(value) = p.get("selected_revision") {
        let revision = value.as_u64().ok_or_else(|| fail("selected_revision must be an integer"))?;
        if s.state()?.revision != revision {
            return Err(fail("NURBS rail selection revision is stale"));
        }
    }
    Ok(())
}
fn relay_document_curve(s: &mut Session, p: &Value, mode: &str) -> Result<Value> {
    name(p)?;
    check_curve_revision(s,p)?;
    let (id_a,curve_a)=document_curve(s,p,if mode=="sweep2"{"curve_a_id"}else{"curve_id"})?;
    let (chord_tolerance,max_segments)=rail_sampling_config(p)?;
    let id_b;
    let (worker,approximation) = match mode {
        "pipe" => {
            id_b=None;
            let radius=positive(p,"radius")?;
            point_field(p,"up",3)?;
            let approximated=certified_worker_rails(&[curve_a.as_ref()],chord_tolerance,max_segments)?;
            let rail=triplets(&approximated.rails[0]);
            (json!({"op":"pipe","rail":rail,"up":p["up"],"radius":radius,"tolerance":tolerance(p)?}),approximated)
        }
        "sweep1" => {
            id_b=None;
            section_field(p)?;
            point_field(p,"up",3)?;
            let approximated=certified_worker_rails(&[curve_a.as_ref()],chord_tolerance,max_segments)?;
            let rail=triplets(&approximated.rails[0]);
            (json!({"op":"sweep1","rail":rail,"section":p["section"],"up":p["up"],"tolerance":tolerance(p)?}),approximated)
        }
        "sweep2" => {
            section_field(p)?;
            let (second_id,curve_b)=document_curve(s,p,"curve_b_id")?;
            if id_a==second_id { return Err(fail("Sweep2 requires distinct rails")); }
            id_b=Some(second_id);
            // Critical: subdivide both curves at the same parameter stations.
            let approximated=certified_worker_rails(
                &[curve_a.as_ref(),curve_b.as_ref()],chord_tolerance,max_segments)?;
            let rail_a=triplets(&approximated.rails[0]);
            let rail_b=triplets(&approximated.rails[1]);
            (json!({"op":"sweep2","rail_a":rail_a,"rail_b":rail_b,
                "section":p["section"],"tolerance":tolerance(p)?}),approximated)
        }
        _ => return Err(fail("unrecognized document NURBS construction")),
    };
    // All exact rail qualification and native processing precedes mutation.
    let reply=call_worker(&worker)?;
    let mut output=add_worker_shapes(s,p,reply)?;
    if let Some(map)=output.as_object_mut() {
        map.insert("source_curve_id".into(),json!(id_a));
        if let Some(second_id)=id_b { map.insert("source_curve_b_id".into(),json!(second_id)); }
        map.insert("source_representation".into(),json!("rational_nurbs"));
        map.insert("construction_spine".into(),json!("adaptive_exact_knot_insertion_polyline"));
        map.insert("spine_deviation_certified".into(),json!(true));
        map.insert("spine_bound_kind".into(),json!("one_sided_curve_to_chord_convex_hull"));
        map.insert("max_curve_to_chord_bound".into(),json!(approximation.max_curve_to_chord_bound));
        map.insert("chord_tolerance".into(),json!(chord_tolerance));
        map.insert("sample_segments".into(),json!(approximation.segment_count));
        map.insert("max_segments".into(),json!(max_segments));
        map.insert("sweep_surface_deviation_certified".into(),json!(false));
        map.insert("associative".into(),json!(false));
    }
    Ok(output)
}
fn pipe_from_curve(s: &mut Session, p: &Value) -> Result<Value> {
    relay_document_curve(s, p, "pipe")
}
fn sweep_one_from_curve(s: &mut Session, p: &Value) -> Result<Value> {
    relay_document_curve(s, p, "sweep1")
}
fn sweep_two_from_curves(s: &mut Session, p: &Value) -> Result<Value> {
    relay_document_curve(s, p, "sweep2")
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
    fn exact_sweep_rejects_malformed_inputs_without_changing_document() {
        let mut s = Session::new();
        let before = s.state().unwrap().revision;
        for cmd in [
            json!({"name":"Bad","rail":[[0.,0.,0.]],"radius":1.,"up":[0.,1.,0.]}),
            json!({"name":"Bad","rail":[[0.,0.,0.],[0.,0.,4.]],"radius":0.,"up":[0.,1.,0.]}),
        ] {
            assert!(s.execute("brep.pipe", &cmd).is_err());
        }
        assert!(
            s.execute(
                "brep.sweep2",
                &json!({"name":"Bad",
            "rail_a":[[0.,0.,0.],[0.,0.,1.]],
            "rail_b":[[2.,0.,0.]],
            "section":[[0.,0.],[1.,0.],[0.,1.]]})
            )
            .is_err()
        );
        assert_eq!(s.state().unwrap().revision, before);
        assert!(s.doc().unwrap().exact_breps.is_empty());
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn exact_sweep_commands_preserve_native_breps_when_worker_is_installed() {
        if std::env::var_os("WORLDWRIGHT_BREP_WORKER").is_none() {
            return;
        }
        let mut s = Session::new();
        let rail = json!([[0., 0., 0.], [0., 0., 5.]]);
        let section = json!([[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]]);
        let a = s.execute("brep.pipe", &json!({"name":"Exact pipe","rail":rail,"radius":0.5,"up":[0.,1.,0.]})).unwrap();
        let b = s.execute("brep.sweep1", &json!({"name":"Exact sweep","rail":rail,"section":section,"up":[0.,1.,0.]})).unwrap();
        let c = s
            .execute(
                "brep.sweep2",
                &json!({"name":"Two rail",
            "rail_a":[[0.,0.,0.],[0.,0.,5.]],
            "rail_b":[[2.,0.,0.],[2.,0.,5.]],
            "section":[[0.,-0.5],[1.,-0.5],[1.,0.5],[0.,0.5]]}),
            )
            .unwrap();
        for result in [&a, &b, &c] {
            assert_eq!(result["representation"], "exact_occt_brep");
            let id = result["ids"][0].as_u64().unwrap();
            let summary = s.execute("brep.inspect", &json!({"id":id})).unwrap();
            assert_eq!(summary["exact"], true);
            assert!(summary["volume"].as_f64().unwrap() > 0.);
        }
        assert_eq!(s.doc().unwrap().exact_breps.len(), 3);
        let bytes = cadcraft_io::write(s.doc().unwrap(), "sweeps.dftba").unwrap();
        let reopened = cadcraft_io::read(&bytes, "sweeps.dftba").unwrap();
        assert_eq!(reopened.exact_breps, s.doc().unwrap().exact_breps);
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().exact_breps.len(), 2);
    }
    #[test]
    fn document_nurbs_pipe_selection_failures_do_not_modify_scene() {
        let mut s = Session::new();
        let c =
            cadcraft_geom::nurbs3d::Curve::from_control(vec![cadcraft_geom::Vec3::new(0., 0., 0.), cadcraft_geom::Vec3::new(0., 0., 5.)], 1).unwrap();
        let id = s.execute("nurbs.curve3d", &json!({"name":"Track","curve":c})).unwrap()["id"].as_u64().unwrap();
        let before = s.state().unwrap().revision;
        for p in [
            json!({"name":"Bad","curve_id":id,"radius":0.3,"up":[0.,1.,0.],"selected_revision":before+1}),
            json!({"name":"Bad","curve_id":id,"radius":0.,"up":[0.,1.,0.]}),
            json!({"name":"Bad","curve_id":id+1000,"radius":0.3,"up":[0.,1.,0.]}),
            json!({"name":"Bad","curve_id":id,"radius":0.3,"up":[0.,1.,0.],"segments":128}),
        ] {
            assert!(s.execute("brep.pipe_from_curve", &p).is_err());
        }
        assert!(s.doc().unwrap().exact_breps.is_empty());
        assert_eq!(s.state().unwrap().revision, before);
    }
    #[test]
    fn curved_nurbs_rejects_inadequate_certification_budget_without_worker_or_mutation() {
        let mut s=Session::new();
        let curved=cadcraft_geom::nurbs3d::Curve{
            degree:2, control:vec![
                cadcraft_geom::Vec3::new(1.,0.,0.),
                cadcraft_geom::Vec3::new(1.,1.,0.),
                cadcraft_geom::Vec3::new(0.,1.,0.)],
            weights:vec![1.,2.0_f64.sqrt().recip(),1.],
            knots:vec![0.,0.,0.,1.,1.,1.]};
        let id=s.execute("nurbs.curve3d",&json!({"name":"Arc Rail","curve":curved})).unwrap()["id"].as_u64().unwrap();
        let revision=s.state().unwrap().revision;
        let args=json!({"name":"Guarded Pipe","curve_id":id,"radius":0.2,"up":[0.,0.,1.],
            "chord_tolerance":0.000001,"max_segments":2});
        let error=s.execute("brep.pipe_from_curve",&args);
        assert!(error.is_err());
        assert!(format!("{error:?}").contains("tolerance"));
        assert_eq!(s.state().unwrap().revision,revision);
        assert!(s.doc().unwrap().exact_breps.is_empty());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn document_nurbs_spine_produces_real_exact_solid_when_worker_installed() {
        if std::env::var_os("WORLDWRIGHT_BREP_WORKER").is_none() {
            return;
        }
        let mut s = Session::new();
        let control = vec![cadcraft_geom::Vec3::new(0., 0., 0.), cadcraft_geom::Vec3::new(0., 0., 5.)];
        let id = s
            .execute(
                "nurbs.curve3d",
                &json!({"name":"Show Track","curve":
            cadcraft_geom::nurbs3d::Curve::from_control(control,1).unwrap()}),
            )
            .unwrap()["id"]
            .as_u64()
            .unwrap();
        let result = s
            .execute(
                "brep.pipe_from_curve",
                &json!({"name":"Handrail","curve_id":id,
            "radius":0.5,"up":[0.,1.,0.]}),
            )
            .unwrap();
        assert_eq!(result["representation"], "exact_occt_brep");
        assert_eq!(result["construction_spine"], "adaptive_exact_knot_insertion_polyline");
        assert_eq!(result["spine_deviation_certified"], true);
        assert_eq!(result["sweep_surface_deviation_certified"], false);
        assert!(result["max_curve_to_chord_bound"].as_f64().unwrap()<=0.01);
        let solid = result["ids"][0].as_u64().unwrap();
        let summary = s.execute("brep.inspect", &json!({"id":solid})).unwrap();
        assert!((summary["volume"].as_f64().unwrap() - std::f64::consts::PI * 0.25 * 5.).abs() < 1e-3);
        let section = json!([[-0.25, -0.25], [0.25, -0.25], [0.25, 0.25], [-0.25, 0.25]]);
        let one = s
            .execute(
                "brep.sweep1_from_curve",
                &json!({"name":"Extruded scenic","curve_id":id,
            "section":section,"up":[0.,1.,0.]}),
            )
            .unwrap();
        assert_eq!(one["representation"], "exact_occt_brep");
        let other_curve = s
            .execute(
                "nurbs.curve3d",
                &json!({"name":"Rail B","curve":
            cadcraft_geom::nurbs3d::Curve::from_control(vec![
                cadcraft_geom::Vec3::new(2.,0.,0.),cadcraft_geom::Vec3::new(2.,0.,5.)
            ],1).unwrap()}),
            )
            .unwrap()["id"]
            .as_u64()
            .unwrap();
        let two = s
            .execute(
                "brep.sweep2_from_curves",
                &json!({"name":"Scenic beam",
            "curve_a_id":id,"curve_b_id":other_curve,
            "section":[[0.,-0.25],[1.,-0.25],[1.,0.25],[0.,0.25]]}),
            )
            .unwrap();
        assert_eq!(two["representation"], "exact_occt_brep");
        assert_eq!(s.doc().unwrap().exact_breps.len(), 3);
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().exact_breps.len(), 2);
    }
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
        let checked = s.execute("brep.inspect", &json!({"id":id_a})).unwrap();
        assert_eq!(checked["faces"], 6);
        assert!((checked["volume"].as_f64().unwrap() - 8.0).abs() < 1e-8);
        let proxy = s.execute("brep.preview", &json!({"id":id_a,"linear_deflection":0.1,"angular_deflection":0.4})).unwrap();
        assert_eq!(proxy["display_only"], true);
        assert_eq!(proxy["mesh"]["exact"], false);
        assert_eq!(s.doc().unwrap().exact_breps.len(), 4);
        let step = s.execute("brep.to_step", &json!({"ids":[id_a]})).unwrap();
        assert_eq!(step["exact"], true);
        let imported = s.execute("brep.from_step", &json!({"name":"STEP roundtrip","step":step["step"]})).unwrap();
        assert_eq!(imported["count"], 1);
        let exported = cadcraft_io::write(s.doc().unwrap(), "solid.dftba").unwrap();
        let reopened = cadcraft_io::read(&exported, "solid.dftba").unwrap();
        assert_eq!(reopened.exact_breps, s.doc().unwrap().exact_breps);
        assert!(cadcraft_io::write(&reopened, "solid.dxf").is_err());
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().exact_breps.len(), 4);
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().exact_breps.len(), 5);
    }
}
