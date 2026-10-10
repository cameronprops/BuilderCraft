//! BuilderCraft organization commands shared by UI and external controllers.
use super::*;
use cadcraft_doc::{
    Handle,
    organization::{ModelNode, NodeKind},
};
use serde_json::json;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("geometry3d.transform", "Transform Exact 3D Geometry", transform3d).params("{ids:[id,...],operation:{kind:move|rotate|scale|scale1d|scale2d|scale_nu|scale_by_plane|scale_positions|shear|orient3pt|mirror,...},copy?:false}"),
        CommandSpec::new("production.model", "Production Organization", |s,_|serde_json::to_value(&s.doc()?.production).map_err(|e|error(&e.to_string()))).noundo(),
        CommandSpec::new("production.set", "Set Production Organization", |s,p|{let model:buildercraft_kernel::ProductionModel=serde_json::from_value(p.clone()).map_err(|e|error(&e.to_string()))?;model.validate().map_err(|e|error(&e.to_string()))?;s.doc_mut()?.production=model;Ok(json!({"ok":true}))}).params("{records,bindings,links}"),
        CommandSpec::new("visualization.start", "Start Live Visualization", live_visualization).params("{project_id,directory}").noundo(),
        CommandSpec::new("visualization.status", "Live Visualization Status", visualization_status).noundo(),
        CommandSpec::new("visualization.stop", "Stop Live Visualization", stop_visualization).noundo(),
        CommandSpec::new("visualization.publish", "Publish Visualization Scene", publish_visualization).params("{project_id:32 hex digits,directory}").noundo(),
        CommandSpec::new("geometry3d.preview", "Tessellate 3D Preview", preview3d).params("{id,curve_segments?:64,surface_u?:16,surface_v?:16}").noundo(),
        CommandSpec::new("kernel.manifest", "Suite Scene Manifest", kernel_manifest).params("{project_id:32 hex digits, geometry_budget_bytes?:positive bytes}").noundo(),
        CommandSpec::new("buildercraft.capabilities", "BuilderCraft API Capabilities", |_,_| Ok(json!({"apiVersion":"0.1","projectSchema":1,"kernelProtocol":1,"sceneManifest":true,"visualizationPublication":true,"productionMetadata":true,"geometry":["rationalCurve3d","controlSurface","polygonMesh","exactOcctBrep"],"nativeProject":"dftba","legacyNativeProject":"bcraft","solids":"nativeOCCTWorkerOptional","meshTools":true,"meshViewport":true,"viewportPicking":"orthographicPreviewWires","transient3dSelection":true,"changeSubscriptions":false,"constructionPlanePointInput":"orthographic","endpointSnapping":"exactCurveEndsAndSurfaceCorners","meshExchange":["stl","triangularObj"],"controlCurveFromPoints":true}))).enabled(always).noundo(),
        CommandSpec::new("nurbs.curve3d", "3D NURBS Curve", curve3d).params("{name, curve:{degree,control:[{x,y,z}],weights,knots}}"),
        CommandSpec::new("nurbs.surface", "NURBS Control Surface", surface3d).params("{name, surface:{rows:[curve,...],degree_v,knots_v}}"),
        CommandSpec::new("geometry3d.controlpoint", "Edit NURBS Control Point", controlpoint).params("{id,row?:0,index,point:[x,y,z]}"),
        CommandSpec::new("geometry3d.set", "Edit 3D Body", geometry3d_set).params("{id,name?,visible?}"),
        CommandSpec::new("geometry3d.list", "List 3D Geometry", |s, _| Ok(json!({"objects":s.doc()?.geometry3d}))).noundo(),
        CommandSpec::new("nurbs.curve", "Rational Control Curve (XY)", nurbs_curve)
            .params("{control:[[x,y],...],degree?:1..5,weights?:[positive finite,...]}"),
        CommandSpec::new("model.create", "Create Model Item", create).params("{name, kind: assembly|component|body, parent?: id, handles?: [hex] }"),
        CommandSpec::new("model.rename", "Rename Model Item", rename).params("{id, name}"),
        CommandSpec::new("model.list", "List Model Items", |s, _| Ok(json!({"nodes": s.doc()?.organization.nodes}))).noundo(),
        CommandSpec::new("model.select", "Select Model Item", select).params("{id}").noundo(),
        CommandSpec::new("model.visible", "Set Body Visibility", visible).params("{id, visible: bool}"),
    ]
}
fn visualization_status(s: &mut Session, _: &Value) -> Result<Value> {
    s.poll_visualization();
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(s.visualization_feed.as_ref().map(|f| f.status()).unwrap_or_else(|| json!({"running":false})))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(json!({"running":false,"native_only":true}))
    }
}
fn stop_visualization(s: &mut Session, _: &Value) -> Result<Value> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        s.visualization_feed = None;
    }
    let _ = s;
    Ok(json!({"running":false}))
}
fn live_visualization(s: &mut Session, p: &Value) -> Result<Value> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (s, p);
        Err(error("live visualization is native-only"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let project = p.get("project_id").and_then(Value::as_str).ok_or_else(|| error("project_id required"))?;
        let project = buildercraft_kernel::Id::try_from(project.to_string()).map_err(|e| error(&e.to_string()))?;
        let directory = p.get("directory").and_then(Value::as_str).filter(|v| !v.is_empty()).ok_or_else(|| error("directory required"))?;
        let uid = s.state()?.uid;
        if s.visualization_feed.is_some() {
            return Err(error("stop the current visualization feed before starting another"));
        }
        s.visualization_feed = Some(crate::visualization::Feed::new(uid, project, directory.into()).map_err(|e| error(&e.to_string()))?);
        visualization_status(s, p)
    }
}
fn publish_visualization(s: &mut Session, p: &Value) -> Result<Value> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (s, p);
        Err(error("local visualization publication is native-only"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let project = p.get("project_id").and_then(Value::as_str).ok_or_else(|| error("project_id required"))?;
        let project = buildercraft_kernel::Id::try_from(project.to_string()).map_err(|e| error(&e.to_string()))?;
        let directory = p.get("directory").and_then(Value::as_str).filter(|p| !p.is_empty()).ok_or_else(|| error("directory required"))?;
        let snapshot = cadcraft_io::visualization::snapshot(s.doc()?, project, s.state()?.revision, Default::default(), &Default::default())
            .map_err(|e| error(&e.to_string()))?;
        let sequence = cadcraft_io::visualization::publish(std::path::Path::new(directory), snapshot).map_err(|e| error(&e.to_string()))?;
        Ok(
            json!({"sequence":sequence.to_string(),"snapshot":std::path::Path::new(directory).join("snapshot.json"),"glb":std::path::Path::new(directory).join(format!("scene-{sequence}.glb"))}),
        )
    }
}
fn preview3d(s: &mut Session, p: &Value) -> Result<Value> {
    let object_id = id(p)?;
    let count = |key: &str, fallback: usize| -> Result<usize> {
        match p.get(key) {
            None => Ok(fallback),
            Some(v) => v.as_u64().and_then(|n| usize::try_from(n).ok()).ok_or_else(|| error("invalid preview segment count")),
        }
    };
    let options = buildercraft_kernel::TessellationOptions {
        curve_segments: count("curve_segments", 64)?,
        surface_u: count("surface_u", 16)?,
        surface_v: count("surface_v", 16)?,
    };
    let object = s.doc()?.geometry3d.iter().find(|o| o.id == object_id).ok_or_else(|| error("unknown geometry id"))?;
    let budget = buildercraft_kernel::GeometryBudget::new(8 * 1024 * 1024, 65536);
    let preview = buildercraft_kernel::tessellate(
        &object.shape,
        options,
        buildercraft_kernel::TessellationLimits::default(),
        &budget,
        &buildercraft_kernel::Cancellation::default(),
    )
    .map_err(|e| error(&e.to_string()))?;
    Ok(json!({"source_id":object_id,"source_revision":s.state()?.revision,"sampling":"uniform_parameter",
        "tolerance_certified":false,"preview":preview.data(),"estimated_geometry_bytes":preview.estimated_bytes()}))
}
fn kernel_manifest(s: &mut Session, p: &Value) -> Result<Value> {
    let project = p.get("project_id").and_then(Value::as_str).ok_or_else(|| error("project_id required"))?;
    let project = buildercraft_kernel::Id::try_from(project.to_string()).map_err(|e| error(&e.to_string()))?;
    let bytes = match p.get("geometry_budget_bytes") {
        None => 64 * 1024 * 1024,
        Some(v) => v
            .as_u64()
            .filter(|n| *n > 0 && *n <= 1024 * 1024 * 1024)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| error("invalid geometry budget"))?,
    };
    let manifest = cadcraft_doc::kernel::manifest(s.doc()?, project, s.state()?.revision, bytes).map_err(|e| error(&e.to_string()))?;
    serde_json::to_value(manifest).map_err(|e| error(&e.to_string()))
}
fn error(message: &str) -> EngineError {
    EngineError::Other(message.into())
}
fn name(p: &Value) -> Result<String> {
    let n = p.get("name").and_then(Value::as_str).ok_or_else(|| error("name is required"))?.trim();
    if n.is_empty() || n.len() > 256 {
        return Err(error("name must contain 1–256 bytes"));
    }
    Ok(n.into())
}
fn id(p: &Value) -> Result<u64> {
    p.get("id").and_then(Value::as_u64).ok_or_else(|| error("id is required"))
}
fn create(s: &mut Session, p: &Value) -> Result<Value> {
    let name = name(p)?;
    let kind = match p.get("kind").and_then(Value::as_str) {
        Some("assembly") => NodeKind::Assembly,
        Some("component") => NodeKind::Component,
        Some("body") => NodeKind::Body,
        _ => return Err(error("kind must be assembly, component or body")),
    };
    let parent = match p.get("parent") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64().ok_or_else(|| error("parent must be an id"))?),
    };
    if let Some(parent) = parent {
        let node = s.doc()?.organization.nodes.iter().find(|n| n.id == parent).ok_or_else(|| error("parent does not exist"))?;
        if node.kind == NodeKind::Body {
            return Err(error("a body cannot contain children"));
        }
        if node.kind == NodeKind::Component && kind != NodeKind::Body {
            return Err(error("components contain bodies"));
        }
    }
    let mut entities = Vec::new();
    if let Some(values) = p.get("handles") {
        let values = values.as_array().ok_or_else(|| error("handles must be an array"))?;
        if values.len() > 100_000 {
            return Err(error("too many handles"));
        }
        for value in values {
            let hex = value.as_str().ok_or_else(|| error("handles must be hex strings"))?;
            let handle = Handle(u64::from_str_radix(hex, 16).map_err(|_| error("invalid handle"))?);
            if s.doc()?.entity(handle).is_none()
                && !s.doc()?.geometry3d.iter().any(|o| o.id == handle.0)
                && !s.doc()?.mesh3d.iter().any(|o| o.id == handle.0)
            {
                return Err(error("entity does not exist"));
            }
            if !entities.contains(&handle) {
                entities.push(handle);
            }
        }
    } else if kind == NodeKind::Body {
        entities = s.selection();
    }
    if kind != NodeKind::Body && !entities.is_empty() {
        return Err(error("only bodies own entities"));
    }
    if s.doc()?.organization.nodes.len() >= 100_000 {
        return Err(error("model item limit reached"));
    }
    if entities.iter().any(|h| s.doc().is_ok_and(|d| d.organization.nodes.iter().any(|n| n.entities.contains(h)))) {
        return Err(error("an entity already belongs to a body"));
    }
    let d = s.doc_mut()?;
    if d.handseed == u64::MAX {
        return Err(error("model id space exhausted"));
    }
    let id = d.new_handle().0;
    d.organization.nodes.push(ModelNode { id, name, kind, parent, entities });
    Ok(json!({"id": id}))
}
fn rename(s: &mut Session, p: &Value) -> Result<Value> {
    let id = id(p)?;
    let name = name(p)?;
    let node = s.doc_mut()?.organization.nodes.iter_mut().find(|n| n.id == id).ok_or_else(|| error("model item does not exist"))?;
    node.name = name;
    Ok(json!({"id": id}))
}
fn members(s: &Session, p: &Value) -> Result<Vec<Handle>> {
    let id = id(p)?;
    let d = s.doc()?;
    if !d.organization.nodes.iter().any(|n| n.id == id) {
        return Err(error("model item does not exist"));
    }
    let mut ids = vec![id];
    let mut members = Vec::new();
    let mut cursor = 0;
    while cursor < ids.len() && cursor < 100_000 {
        let Some(current) = ids.get(cursor).copied() else { break };
        for n in &d.organization.nodes {
            if n.id == current {
                members.extend(
                    n.entities
                        .iter()
                        .copied()
                        .filter(|h| d.entity(*h).is_some() || d.geometry3d.iter().any(|o| o.id == h.0) || d.mesh3d.iter().any(|o| o.id == h.0)),
                );
            }
            if n.parent == Some(current) && !ids.contains(&n.id) {
                ids.push(n.id);
            }
        }
        cursor += 1;
    }
    Ok(members)
}
fn select(s: &mut Session, p: &Value) -> Result<Value> {
    let handles = members(s, p)?;
    s.set_selection(handles.clone());
    Ok(json!({"count": handles.len()}))
}
fn visible(s: &mut Session, p: &Value) -> Result<Value> {
    let visible = p.get("visible").and_then(Value::as_bool).ok_or_else(|| error("visible must be boolean"))?;
    let handles = members(s, p)?;
    for h in &handles {
        if s.doc()?.entity(*h).is_some() {
            s.doc_mut()?.modify_entity(*h, |e| e.common.visible = visible)?;
        } else if let Some(o) = s.doc_mut()?.geometry3d.iter_mut().find(|o| o.id == h.0) {
            o.visible = visible;
        } else if let Some(o) = s.doc_mut()?.mesh3d.iter_mut().find(|o| o.id == h.0) {
            o.visible = visible;
        }
    }
    Ok(json!({"count": handles.len()}))
}

fn nurbs_curve(s: &mut Session, p: &Value) -> Result<Value> {
    let values = p.get("control").and_then(Value::as_array).ok_or_else(|| error("control array required"))?;
    if !(2..=4096).contains(&values.len()) {
        return Err(error("need 2–4096 control points"));
    }
    let degree =
        p.get("degree").map(|v| v.as_u64().ok_or_else(|| error("degree must be an integer"))).transpose()?.unwrap_or(3.min(values.len() as u64 - 1));
    if degree == 0 || degree > 5 || degree as usize >= values.len() {
        return Err(error("degree must be 1–5 and less than point count"));
    }
    let mut points = Vec::new();
    for v in values {
        let a = v.as_array().ok_or_else(|| error("control point must be [x,y]"))?;
        if a.len() != 2 {
            return Err(error("this first curve command requires XY points"));
        }
        let x = a.first().and_then(Value::as_f64).filter(|v| v.is_finite()).ok_or_else(|| error("invalid x"))?;
        let y = a.get(1).and_then(Value::as_f64).filter(|v| v.is_finite()).ok_or_else(|| error("invalid y"))?;
        points.push(cadcraft_geom::Vec2::new(x, y));
    }
    let mut curve = cadcraft_geom::Spline::from_control(points, degree as usize);
    if let Some(weights) = p.get("weights") {
        let weights = weights.as_array().ok_or_else(|| error("weights must be an array"))?;
        if weights.len() != values.len() {
            return Err(error("one weight per control point required"));
        }
        for w in weights {
            curve
                .weights
                .push(w.as_f64().filter(|w| w.is_finite() && *w >= 1e-9 && *w <= 1e9).ok_or_else(|| error("weights must be between 1e-9 and 1e9"))?);
        }
    }
    let h = s.add_entity(cadcraft_doc::EntityKind::Spline(curve))?;
    Ok(json!({"handle":h.hex()}))
}

fn add3d(s: &mut Session, p: &Value, shape: cadcraft_doc::organization::Shape) -> Result<Value> {
    let name = name(p)?;
    let d = s.doc_mut()?;
    if d.geometry3d.len() >= 4096 || d.handseed == u64::MAX {
        return Err(error("3D object limit reached"));
    }
    let id = d.new_handle().0;
    d.geometry3d.push(cadcraft_doc::organization::GeometryObject { id, name, layer: d.header.str("CLAYER", "0"), visible: true, shape });
    Ok(json!({"id":id}))
}
pub(super) fn curve3d(s: &mut Session, p: &Value) -> Result<Value> {
    let curve: cadcraft_geom::nurbs3d::Curve =
        serde_json::from_value(p.get("curve").cloned().ok_or_else(|| error("curve required"))?).map_err(|e| error(&e.to_string()))?;
    if !curve.valid() {
        return Err(error("invalid NURBS curve"));
    }
    add3d(s, p, cadcraft_doc::organization::Shape::Curve(curve.into()))
}
fn surface3d(s: &mut Session, p: &Value) -> Result<Value> {
    let surface: cadcraft_geom::nurbs3d::Surface =
        serde_json::from_value(p.get("surface").cloned().ok_or_else(|| error("surface required"))?).map_err(|e| error(&e.to_string()))?;
    if !surface.valid() {
        return Err(error("invalid NURBS surface"));
    }
    add3d(s, p, cadcraft_doc::organization::Shape::Surface(surface.into()))
}

fn geometry3d_set(s: &mut Session, p: &Value) -> Result<Value> {
    let id = id(p)?;
    let name = if p.get("name").is_some() { Some(name(p)?) } else { None };
    let visible = p.get("visible").map(|v| v.as_bool().ok_or_else(|| error("visible must be boolean"))).transpose()?;
    let object = s.doc_mut()?.geometry3d.iter_mut().find(|o| o.id == id).ok_or_else(|| error("3D body does not exist"))?;
    if let Some(n) = name {
        object.name = n;
    }
    if let Some(v) = visible {
        object.visible = v;
    }
    Ok(json!({"id":id}))
}

fn controlpoint(s: &mut Session, p: &Value) -> Result<Value> {
    let id = id(p)?;
    let index = p.get("index").and_then(Value::as_u64).filter(|i| *i < 4096).ok_or_else(|| error("invalid point index"))? as usize;
    let row = p.get("row").and_then(Value::as_u64).unwrap_or(0);
    if row >= 128 {
        return Err(error("invalid row"));
    }
    let a = p.get("point").and_then(Value::as_array).filter(|a| a.len() == 3).ok_or_else(|| error("point must be [x,y,z]"))?;
    let number = |i| a.get(i).and_then(Value::as_f64).filter(|v| v.is_finite() && v.abs() <= 1e12).ok_or_else(|| error("invalid coordinate"));
    let point = cadcraft_geom::Vec3::new(number(0)?, number(1)?, number(2)?);
    let source = s.doc()?.geometry3d.iter().find(|o| o.id == id).ok_or_else(|| error("3D body does not exist"))?;
    let valid_index = match &source.shape {
        cadcraft_doc::organization::Shape::Curve(c) => row == 0 && index < c.control.len(),
        cadcraft_doc::organization::Shape::Surface(surface) => surface.rows.get(row as usize).is_some_and(|r| index < r.control.len()),
    };
    if !valid_index {
        return Err(error("control point does not exist"));
    }
    let object = s.doc_mut()?.geometry3d.iter_mut().find(|o| o.id == id).ok_or_else(|| error("3D body does not exist"))?;
    let control = match &mut object.shape {
        cadcraft_doc::organization::Shape::Curve(c) => &mut std::sync::Arc::make_mut(c).control,
        cadcraft_doc::organization::Shape::Surface(surface) => {
            &mut std::sync::Arc::make_mut(surface).rows.get_mut(row as usize).ok_or_else(|| error("row does not exist"))?.control
        }
    };
    *control.get_mut(index).ok_or_else(|| error("control point does not exist"))? = point;
    Ok(json!({"id":id,"index":index}))
}

fn transform3d(s: &mut Session, p: &Value) -> Result<Value> {
    use std::collections::BTreeSet;
    let params = p.as_object().ok_or_else(|| error("transform parameters must be an object"))?;
    if params.keys().any(|key| !["ids", "operation", "copy"].contains(&key.as_str())) {
        return Err(error("unsupported transform option"));
    }
    let ids = p
        .get("ids")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty() && v.len() <= 128)
        .ok_or_else(|| error("ids must contain 1 to 128 exact geometry IDs"))?;
    let ids = ids.iter().map(|v| v.as_u64().ok_or_else(|| error("invalid geometry ID"))).collect::<Result<BTreeSet<_>>>()?;
    if ids.len() != p["ids"].as_array().map_or(0, Vec::len) {
        return Err(error("duplicate geometry IDs"));
    }
    let operation: buildercraft_kernel::Transform =
        serde_json::from_value(p.get("operation").cloned().ok_or_else(|| error("operation required"))?).map_err(|e| error(&e.to_string()))?;
    let copy = match p.get("copy") {
        None => false,
        Some(v) => v.as_bool().ok_or_else(|| error("copy must be boolean"))?,
    };
    let source = s.doc()?;
    if copy && (source.geometry3d.len().checked_add(ids.len()).is_none_or(|n| n > 4096) || source.handseed.checked_add(ids.len() as u64).is_none()) {
        return Err(error("3D object or identity limit reached"));
    }
    if copy {
        for offset in 0..ids.len() as u64 {
            let next = source.handseed.checked_add(offset).ok_or_else(|| error("identity limit"))?;
            if source.geometry3d.iter().any(|o| o.id == next)
                || source.organization.nodes.iter().any(|n| n.id == next)
                || source.entity(Handle(next)).is_some()
            {
                return Err(error("copy identity collision"));
            }
        }
    }
    let mut bytes = 0usize;
    let mut controls = 0usize;
    let objects = ids
        .iter()
        .map(|id| source.geometry3d.iter().find(|o| o.id == *id).ok_or_else(|| error("unknown exact geometry ID")))
        .collect::<Result<Vec<_>>>()?;
    for o in &objects {
        if !o.shape.valid() {
            return Err(error("invalid source shape"));
        }
        bytes = bytes.checked_add(o.shape.estimated_bytes().map_err(|e| error(&e.to_string()))?).ok_or_else(|| error("transform byte budget"))?;
        controls = controls.checked_add(buildercraft_kernel::exact_control_count(&o.shape)).ok_or_else(|| error("transform control budget"))?;
        if bytes > 32 * 1024 * 1024 || controls > 100_000 {
            return Err(error("transform batch budget exceeded"));
        }
    }
    let cancel = buildercraft_kernel::Cancellation::default();
    let mut work = 200_000;
    let transformed = objects
        .iter()
        .map(|o| {
            buildercraft_kernel::transform_exact_with_work(&o.shape, &operation, &cancel, 16 * 1024 * 1024, &mut work)
                .map_err(|e| error(&e.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut objects = objects.into_iter().cloned().collect::<Vec<_>>();
    for (o, shape) in objects.iter_mut().zip(transformed) {
        o.shape = shape;
    }
    let drawing = s.doc_mut()?;
    let mut output = Vec::with_capacity(objects.len());
    for mut object in objects {
        let source_id = object.id;
        if copy {
            object.id = drawing.new_handle().0;
            output.push(json!({"source_id":source_id,"id":object.id}));
            drawing.geometry3d.push(object);
        } else {
            output.push(json!({"source_id":source_id,"id":source_id}));
            if let Some(target) = drawing.geometry3d.iter_mut().find(|o| o.id == source_id) {
                *target = object;
            }
        }
    }
    Ok(json!({"objects":output,"copy":copy,"scope":"exact_curves_and_control_surfaces"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curve_session() -> (Session, Value) {
        let mut s = Session::new();
        let result = s
            .execute(
                "nurbs.curve3d",
                &json!({"name":"Arch","curve":{"degree":1,"control":[{"x":0,"y":0,"z":0},{"x":1,"y":2,"z":3}],"weights":[1,1],"knots":[0,0,1,1]}}),
            )
            .unwrap();
        (s, result["id"].clone())
    }
    #[test]
    fn capabilities_match_native_format_and_viewport_scope() {
        let mut session = Session::new();
        let revision = session.state().unwrap().revision;
        let capabilities = session.execute("buildercraft.capabilities", &json!({})).unwrap();
        assert_eq!(capabilities["nativeProject"], "dftba");
        assert_eq!(capabilities["legacyNativeProject"], "bcraft");
        assert_eq!(capabilities["meshTools"], true);
        assert_eq!(capabilities["viewportPicking"], "orthographicPreviewWires");
        assert_eq!(session.state().unwrap().revision, revision);
    }
    #[test]
    fn production_updates_are_validated_and_undoable() {
        let (mut s, _) = curve_session();
        let records =
            json!({"records":[{"id":"00000000000000000000000000000064","kind":"scene","name":"Geyser","parent":null}],"bindings":[],"links":[]});
        s.execute("production.set", &records).unwrap();
        assert_eq!(s.doc().unwrap().production.records.len(), 1);
        let revision = s.state().unwrap().revision;
        assert!(s.execute("production.set",&json!({"records":[{"id":"00000000000000000000000000000064","kind":"scene","name":"Geyser","parent":"00000000000000000000000000000065"}]})).is_err());
        assert_eq!(s.state().unwrap().revision, revision);
        s.undo().unwrap();
        assert!(s.doc().unwrap().production.records.is_empty());
    }
    #[test]
    fn preview_uses_exact_source_without_mutation_and_rejects_hostile_counts() {
        let (mut s, _) = curve_session();
        let object_id = s.doc().unwrap().geometry3d[0].id;
        let before = s.state().unwrap().revision;
        let result = s.execute("geometry3d.preview", &json!({"id":object_id,"curve_segments":4})).unwrap();
        assert_eq!(result["source_revision"], before);
        assert_eq!(result["preview"]["Polyline"].as_array().unwrap().len(), 5);
        assert_eq!(s.state().unwrap().revision, before);
        for count in [json!(0), json!(4097), json!(u64::MAX), json!(-1), json!("huge")] {
            assert!(s.execute("geometry3d.preview", &json!({"id":object_id,"curve_segments":count})).is_err());
            assert_eq!(s.state().unwrap().revision, before);
        }
    }
    #[test]
    fn kernel_manifest_is_metadata_only_and_does_not_edit() {
        let (mut s, id) = curve_session();
        let revision = s.state().unwrap().revision;
        let manifest = s.execute("kernel.manifest", &json!({"project_id":"ffffffffffffffffffffffffffffffff"})).unwrap();
        assert_eq!(manifest["revision"], revision);
        assert_eq!(manifest["objects"][0]["name"], "Arch");
        assert_eq!(manifest["objects"][0]["geometry_kind"], "nurbsCurve");
        assert!(manifest["objects"][0].get("control").is_none());
        assert_eq!(s.state().unwrap().revision, revision);
        assert!(s.execute("kernel.manifest", &json!({"project_id":"ffffffffffffffffffffffffffffffff","geometry_budget_bytes":1})).is_err());
        assert_eq!(s.doc().unwrap().geometry3d[0].id, id.as_u64().unwrap());
    }
    #[test]
    fn cad_snapshots_share_geometry_until_valid_control_edit() {
        let (mut s, id) = curve_session();
        let before = s.doc().unwrap().clone();
        s.execute("geometry3d.set", &json!({"id":id,"name":"Renamed"})).unwrap();
        let (cadcraft_doc::organization::Shape::Curve(a), cadcraft_doc::organization::Shape::Curve(b)) =
            (&before.geometry3d[0].shape, &s.doc().unwrap().geometry3d[0].shape)
        else {
            panic!()
        };
        assert!(std::sync::Arc::ptr_eq(a, b));
        let revision = s.state().unwrap().revision;
        assert!(s.execute("geometry3d.controlpoint", &json!({"id":id,"index":99,"point":[4,5,6]})).is_err());
        assert_eq!(s.state().unwrap().revision, revision);
        s.execute("geometry3d.controlpoint", &json!({"id":id,"index":0,"point":[4,5,6]})).unwrap();
        let (cadcraft_doc::organization::Shape::Curve(a), cadcraft_doc::organization::Shape::Curve(b)) =
            (&before.geometry3d[0].shape, &s.doc().unwrap().geometry3d[0].shape)
        else {
            panic!()
        };
        assert!(!std::sync::Arc::ptr_eq(a, b));
        assert_eq!(a.control[0].x, 0.);
        assert_eq!(b.control[0].x, 4.);
        s.undo().unwrap();
        let cadcraft_doc::organization::Shape::Curve(b) = &s.doc().unwrap().geometry3d[0].shape else { panic!() };
        let cadcraft_doc::organization::Shape::Curve(a) = &before.geometry3d[0].shape else { panic!() };
        assert!(std::sync::Arc::ptr_eq(a, b));
    }
    #[test]
    fn organization_undo_and_selection() {
        let mut s = Session::new();
        s.execute("line", &json!({"points": [[0,0],[5,0]]})).unwrap();
        let h = s.doc().unwrap().model.iter().next().unwrap().handle.hex();
        let parent = s.execute("model.create", &json!({"kind":"assembly","name":"Machine"})).unwrap()["id"].clone();
        let body = s.execute("model.create", &json!({"kind":"body","name":"Rail","parent":parent,"handles":[h]})).unwrap()["id"].clone();
        s.execute("model.select", &json!({"id":parent})).unwrap();
        assert_eq!(s.selection().len(), 1);
        s.execute("model.rename", &json!({"id":body,"name":"Main rail"})).unwrap();
        s.undo().unwrap();
        assert_eq!(s.doc().unwrap().organization.nodes.last().unwrap().name, "Rail");
        assert!(s.execute("model.create", &json!({"kind":"assembly","name":"Bad","parent":body})).is_err());
    }
}
