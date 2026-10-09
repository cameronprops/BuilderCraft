//! Non-mutating wire hit queries and transient selection through the command API.
use super::*;
use cadcraft_geom::{Vec3, camera::OrthoFrame, picking::WirePick};
use serde::Deserialize;
use serde_json::json;

fn error(message: &str) -> EngineError {
    bad("geometry3d.selection", message)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("geometry3d.pick", "Pick 3D Preview Wire", pick)
            .params("{pixel:[x,y],viewport:[width,height],center:[x,y,z],yaw,pitch,scale,radius?:6} -> {id|null,distance_pixels?,depth?}")
            .noundo(),
        CommandSpec::new("geometry3d.select", "Select Exact 3D Geometry", select).params("{ids:[id,...],mode?:replace|toggle} -> {ids}").noundo(),
    ]
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    pixel: [f64; 2],
    viewport: [f64; 2],
    center: [f64; 3],
    yaw: f64,
    pitch: f64,
    scale: f64,
    #[serde(default = "radius")]
    radius: f64,
}
fn radius() -> f64 {
    6.
}
fn pick(s: &mut Session, p: &Value) -> Result<Value> {
    pick_with_work(s, p, 50_000_000)
}
fn pick_with_work(s: &mut Session, p: &Value, mut work: usize) -> Result<Value> {
    let q: Query = serde_json::from_value(p.clone()).map_err(|e| error(&e.to_string()))?;
    let query = WirePick {
        frame: OrthoFrame { yaw: q.yaw, pitch: q.pitch },
        center: Vec3::new(q.center[0], q.center[1], q.center[2]),
        pixel: Vec2::new(q.pixel[0], q.pixel[1]),
        viewport: Vec2::new(q.viewport[0], q.viewport[1]),
        scale: q.scale,
        radius: q.radius,
    };
    if !query.valid() {
        return Err(error("Invalid viewport pick query"));
    }
    let d = s.doc()?;
    if d.geometry3d.len() > 4096 {
        return Err(error("Pick object budget exceeded"));
    }
    let mut best: Option<(u64, f64, f64)> = None;
    for object in &d.geometry3d {
        if !object.visible || d.layer(&object.layer).is_some_and(|l| !l.visible() || l.locked) {
            continue;
        }
        buildercraft_kernel::visit_preview_wires(&object.shape, &mut work, |a, b| {
            if let Some((distance, depth)) = query.segment(a, b) {
                // Nearest wire wins; coincident wires prefer camera-facing depth, then stable ID.
                if best.is_none_or(|(id, old, old_depth)| {
                    distance < old || (distance == old && (depth > old_depth || (depth == old_depth && object.id < id)))
                }) {
                    best = Some((object.id, distance, depth));
                }
            }
        })
        .map_err(|e| error(&e.to_string()))?;
    }
    Ok(best.map_or_else(|| json!({"id":null}), |(id, distance, depth)| json!({"id":id,"distance_pixels":distance,"depth":depth})))
}
fn select(s: &mut Session, p: &Value) -> Result<Value> {
    let ids = p.get("ids").and_then(Value::as_array).ok_or_else(|| error("ids array required"))?;
    if ids.len() > 4096 {
        return Err(error("Selection budget exceeded"));
    }
    let mode = p.get("mode").map_or(Some("replace"), Value::as_str).ok_or_else(|| error("Invalid selection mode"))?;
    if mode != "replace" && mode != "toggle" {
        return Err(error("Invalid selection mode"));
    }
    let d = s.doc()?;
    let mut requested = Vec::new();
    for id in ids {
        let id = id.as_u64().ok_or_else(|| error("Object IDs must be unsigned integers"))?;
        let object = d.geometry3d.iter().find(|o| o.id == id).ok_or_else(|| error("3D object not found"))?;
        if !object.visible || d.layer(&object.layer).is_some_and(|l| !l.visible() || l.locked) {
            return Err(error("Object is hidden or locked"));
        }
        let handle = Handle(id);
        if !requested.contains(&handle) {
            requested.push(handle);
        }
    }
    let mut result = if mode == "toggle" { s.selection() } else { Vec::new() };
    for handle in requested {
        if let Some(index) = result.iter().position(|h| *h == handle) {
            result.remove(index);
        } else {
            result.push(handle);
        }
    }
    if result.len() > 4096 {
        return Err(error("Selection budget exceeded"));
    }
    s.set_selection(result.clone());
    Ok(json!({"ids":result.iter().map(|h|h.0).collect::<Vec<_>>()}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn curve(s: &mut Session, y: f64) -> u64 {
        s.execute("nurbs.curve3d",&json!({"name":"Pick fixture","curve":{"degree":1,"control":[{"x":-1.,"y":y,"z":0.},{"x":1.,"y":y,"z":0.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}})).unwrap()["id"].as_u64().unwrap()
    }
    fn query() -> Value {
        json!({"pixel":[200.,200.],"viewport":[400.,400.],"center":[0.,0.,0.],"yaw":0.,"pitch":0.,"scale":10.})
    }
    #[test]
    fn depth_visibility_locking_and_selection_failure_preservation() {
        let mut s = Session::new();
        let back = curve(&mut s, 2.);
        let front = curve(&mut s, -2.);
        let revision = s.state().unwrap().revision;
        assert_eq!(s.execute("geometry3d.pick", &query()).unwrap()["id"], front);
        s.execute("geometry3d.select", &json!({"ids":[front,front]})).unwrap();
        assert_eq!(s.selection(), &[Handle(front)]);
        s.execute("geometry3d.select", &json!({"ids":[back],"mode":"toggle"})).unwrap();
        assert_eq!(s.selection().len(), 2);
        assert_eq!(s.state().unwrap().revision, revision);
        assert!(s.execute("geometry3d.select", &json!({"ids":[front, u64::MAX]})).is_err());
        assert_eq!(s.selection().len(), 2);
        s.execute("geometry3d.set", &json!({"id":front,"visible":false})).unwrap();
        assert_eq!(s.execute("geometry3d.pick", &query()).unwrap()["id"], back);
        let layer = s.doc().unwrap().geometry3d[0].layer.clone();
        s.doc_mut().unwrap().layer_mut(&layer).unwrap().locked = true;
        assert!(s.execute("geometry3d.pick", &query()).unwrap()["id"].is_null());
        let mut ties = Session::new();
        let first = curve(&mut ties, 0.);
        curve(&mut ties, 0.);
        ties.doc_mut().unwrap().geometry3d.reverse();
        assert_eq!(ties.execute("geometry3d.pick", &query()).unwrap()["id"], first);
        let layer = s.doc_mut().unwrap().layer_mut(&layer).unwrap();
        layer.locked = false;
        layer.on = false;
        assert!(s.execute("geometry3d.pick", &query()).unwrap()["id"].is_null());
    }
    #[test]
    fn miss_and_invalid_query_do_not_edit_or_consume_undo() {
        let mut s = Session::new();
        curve(&mut s, 0.);
        let mut p = query();
        p["pixel"] = json!([0., 0.]);
        assert!(s.execute("geometry3d.pick", &p).unwrap()["id"].is_null());
        p["scale"] = json!(0.);
        assert!(s.execute("geometry3d.pick", &p).is_err());
        s.execute("geometry3d.select", &json!({"ids":[]})).unwrap();
        s.execute("undo", &json!({})).unwrap();
        assert!(s.doc().unwrap().geometry3d.is_empty());
    }
    #[test]
    fn surface_wire_and_scene_budget_are_explicit() {
        let mut s = Session::new();
        let row = |y: f64| json!({"degree":1,"control":[{"x":-1.,"y":y,"z":0.},{"x":1.,"y":y,"z":0.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]});
        let id = s
            .execute("nurbs.surface", &json!({"name":"Wire patch","surface":{"rows":[row(-1.),row(1.)],"degree_v":1,"knots_v":[0.,0.,1.,1.]}}))
            .unwrap()["id"]
            .as_u64()
            .unwrap();
        let mut p = query();
        p["pitch"] = json!(-std::f64::consts::FRAC_PI_2);
        assert_eq!(s.execute("geometry3d.pick", &p).unwrap()["id"], id);
        s.execute("geometry3d.select", &json!({"ids":[id]})).unwrap();
        let object = s.doc().unwrap().geometry3d[0].clone();
        // Admit one patch but reject the next before returning any partial hit.
        s.doc_mut().unwrap().geometry3d = vec![object; 2];
        assert!(pick_with_work(&mut s, &p, 40_000).is_err());
        assert_eq!(s.selection(), vec![Handle(id)]);
    }
}
