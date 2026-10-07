//! BuilderCraft organization commands shared by UI and external controllers.
use super::*;
use cadcraft_doc::{
    Handle,
    organization::{ModelNode, NodeKind},
};
use serde_json::json;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("buildercraft.capabilities", "BuilderCraft API Capabilities", |_,_| Ok(json!({"apiVersion":"0.1","projectSchema":1,"geometry":["rationalCurve3d","controlSurface"],"nativeProject":"bcraft","solids":false,"meshTools":false,"changeSubscriptions":false}))).enabled(always).noundo(),
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
            if s.doc()?.entity(handle).is_none() && !s.doc()?.geometry3d.iter().any(|o| o.id == handle.0) {
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
                members.extend(n.entities.iter().copied().filter(|h| d.entity(*h).is_some() || d.geometry3d.iter().any(|o| o.id == h.0)));
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
fn curve3d(s: &mut Session, p: &Value) -> Result<Value> {
    let curve: cadcraft_geom::nurbs3d::Curve =
        serde_json::from_value(p.get("curve").cloned().ok_or_else(|| error("curve required"))?).map_err(|e| error(&e.to_string()))?;
    if !curve.valid() {
        return Err(error("invalid NURBS curve"));
    }
    add3d(s, p, cadcraft_doc::organization::Shape::Curve(curve))
}
fn surface3d(s: &mut Session, p: &Value) -> Result<Value> {
    let surface: cadcraft_geom::nurbs3d::Surface =
        serde_json::from_value(p.get("surface").cloned().ok_or_else(|| error("surface required"))?).map_err(|e| error(&e.to_string()))?;
    if !surface.valid() {
        return Err(error("invalid NURBS surface"));
    }
    add3d(s, p, cadcraft_doc::organization::Shape::Surface(surface))
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
    let object = s.doc_mut()?.geometry3d.iter_mut().find(|o| o.id == id).ok_or_else(|| error("3D body does not exist"))?;
    let control = match &mut object.shape {
        cadcraft_doc::organization::Shape::Curve(c) => &mut c.control,
        cadcraft_doc::organization::Shape::Surface(surface) => {
            &mut surface.rows.get_mut(row as usize).ok_or_else(|| error("row does not exist"))?.control
        }
    };
    *control.get_mut(index).ok_or_else(|| error("control point does not exist"))? = point;
    Ok(json!({"id":id,"index":index}))
}

#[cfg(test)]
mod tests {
    use super::*;
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
