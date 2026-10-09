//! Construction-plane and exact-geometry point input for headless CAD, UI and scripts.
//! Queries are transient. Snap coordinates never change drawing history.
use super::*;
use cadcraft_doc::organization::{GeometryObject, Shape};
use cadcraft_geom::{
    Vec3,
    camera::OrthoFrame,
    nurbs3d::{Curve, uniform_knots},
    snap3d::{ConstructionPlane, ScreenRay},
};
use serde::Deserialize;
use serde_json::json;

const SNAP_OBJECT_LIMIT: usize = 512;
const SNAP_POINT_LIMIT: usize = 4096;

fn fail(message: &str) -> EngineError {
    bad("geometry3d.snap", message)
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("geometry3d.cplane.point", "3D Point on Construction Plane", cplane_point)
            .params("{pixel,viewport,center,yaw,pitch,scale,plane?:{origin,x_axis,y_axis},grid?:spacing} -> {point,plane_uv}")
            .noundo(),
        CommandSpec::new("geometry3d.snap", "3D Endpoint, Midpoint and Grid Snap", snap)
            .params("{pixel,viewport,center,yaw,pitch,scale,radius?:8,plane?,grid?,endpoints?:true,midpoints?:true} -> {point,kind,source_id?,source_revision}")
            .noundo(),
        CommandSpec::new("geometry3d.line", "Create 3D Line from Snapped Points", line)
            .params("{name,start:{3D snap query},end:{3D snap query}} -> {id,start,end}"),
    ]
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlaneInput {
    origin: [f64; 3],
    x_axis: [f64; 3],
    y_axis: [f64; 3],
}

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Query {
    pixel: [f64; 2],
    viewport: [f64; 2],
    center: [f64; 3],
    yaw: f64,
    pitch: f64,
    scale: f64,
    #[serde(default)]
    plane: Option<PlaneInput>,
    #[serde(default)]
    grid: Option<f64>,
    #[serde(default = "default_radius")]
    radius: f64,
    #[serde(default = "default_true")]
    endpoints: bool,
    #[serde(default = "default_true")]
    midpoints: bool,
}

fn default_radius() -> f64 {
    8.
}
fn default_true() -> bool {
    true
}
fn v3(p: [f64; 3]) -> Vec3 {
    Vec3::new(p[0], p[1], p[2])
}
fn array(p: Vec3) -> [f64; 3] {
    [p.x, p.y, p.z]
}

impl Query {
    fn geometry(&self) -> Result<(ScreenRay, ConstructionPlane)> {
        let ray = ScreenRay {
            frame: OrthoFrame { yaw: self.yaw, pitch: self.pitch },
            center: v3(self.center),
            pixel: Vec2::new(self.pixel[0], self.pixel[1]),
            viewport: Vec2::new(self.viewport[0], self.viewport[1]),
            scale: self.scale,
        };
        if !ray.valid() || !self.radius.is_finite() || !(0. ..=64.).contains(&self.radius) {
            return Err(fail("Invalid screen coordinates, camera or snap radius"));
        }
        if self.grid.is_some_and(|s| !s.is_finite() || !(1e-9..=1e9).contains(&s)) {
            return Err(fail("Grid spacing must be finite and between 1e-9 and 1e9 drawing units"));
        }
        let plane = match &self.plane {
            None => ConstructionPlane::world_xy(),
            Some(p) => ConstructionPlane::from_axes(v3(p.origin), v3(p.x_axis), v3(p.y_axis))
                .ok_or_else(|| fail("Invalid construction plane: axes must be perpendicular and nondegenerate"))?,
        };
        Ok((ray, plane))
    }
}

#[derive(Clone, Copy)]
struct SnapPoint {
    point: Vec3,
    kind: &'static str,
    source_id: Option<u64>,
    distance: Option<f64>,
    depth: Option<f64>,
}

fn resolved(s: &Session, q: &Query) -> Result<SnapPoint> {
    let (ray, plane) = q.geometry()?;
    let d = s.doc()?;
    if d.geometry3d.len() > SNAP_OBJECT_LIMIT {
        return Err(fail("Snap object budget exceeded (512); spatial acceleration required"));
    }
    let mut best: Option<(SnapPoint, f64, f64)> = None;
    let mut candidates = 0usize;
    for object in &d.geometry3d {
        if !object.visible || d.layer(&object.layer).is_some_and(|l| !l.visible() || l.locked) {
            continue;
        }
        let mut check = |point: Option<Vec3>, kind: &'static str| -> Result<()> {
            candidates = candidates.checked_add(1).ok_or_else(|| fail("Snap candidate overflow"))?;
            if candidates > SNAP_POINT_LIMIT {
                return Err(fail("Snap candidate budget exceeded"));
            }
            let point = point.ok_or_else(|| fail("Unable to evaluate an exact snap point"))?;
            if let Some((distance, depth)) = ray.hit(point, q.radius) {
                let id = object.id;
                // Closer pixels, then camera-facing depth, then lower stable object ID.
                if best.is_none_or(|(previous, old_distance, old_depth)| {
                    distance < old_distance
                        || (distance == old_distance && (depth > old_depth || (depth == old_depth && id < previous.source_id.unwrap_or(u64::MAX))))
                }) {
                    best = Some((SnapPoint { point, kind, source_id: Some(id), distance: Some(distance), depth: Some(depth) }, distance, depth));
                }
            }
            Ok(())
        };
        match &object.shape {
            Shape::Curve(c) => {
                if q.endpoints {
                    check(c.evaluate(0.), "endpoint")?;
                    check(c.evaluate(1.), "endpoint")?;
                }
                if q.midpoints {
                    check(c.evaluate(0.5), "parameter_midpoint")?;
                }
            }
            Shape::Surface(surface) => {
                if q.endpoints {
                    for (u, v) in [(0., 0.), (1., 0.), (0., 1.), (1., 1.)] {
                        check(surface.evaluate(u, v), "surface_corner")?;
                    }
                }
                if q.midpoints {
                    for (u, v) in [(0.5, 0.), (1., 0.5), (0.5, 1.), (0., 0.5)] {
                        check(surface.evaluate(u, v), "surface_edge_midpoint")?;
                    }
                    check(surface.evaluate(0.5, 0.5), "surface_parameter_center")?;
                }
            }
        }
    }
    if let Some((point, _, _)) = best {
        return Ok(point);
    }
    let point = ray.on_plane(plane).ok_or_else(|| fail("View ray is parallel to construction plane; no object snap available"))?;
    let point = match q.grid {
        Some(spacing) => plane.grid_point(point, spacing).ok_or_else(|| fail("Grid result outside drawing coordinate limits"))?,
        None => point,
    };
    Ok(SnapPoint { point, kind: if q.grid.is_some() { "grid" } else { "plane" }, source_id: None, distance: None, depth: None })
}

fn output(s: &Session, hit: SnapPoint) -> Result<Value> {
    Ok(json!({
        "point": array(hit.point),
        "kind": hit.kind,
        "source_id": hit.source_id,
        "distance_pixels": hit.distance,
        "depth": hit.depth,
        "document_uid": s.state()?.uid,
        "source_revision": s.state()?.revision,
    }))
}

fn parse(p: &Value) -> Result<Query> {
    serde_json::from_value(p.clone()).map_err(|e| fail(&format!("Invalid snap query: {e}")))
}

fn snap(s: &mut Session, p: &Value) -> Result<Value> {
    let q = parse(p)?;
    output(s, resolved(s, &q)?)
}

fn cplane_point(s: &mut Session, p: &Value) -> Result<Value> {
    let q = parse(p)?;
    let (ray, plane) = q.geometry()?;
    let point = ray.on_plane(plane).ok_or_else(|| fail("View ray is parallel to construction plane"))?;
    let point = match q.grid {
        Some(step) => plane.grid_point(point, step).ok_or_else(|| fail("Invalid grid result"))?,
        None => point,
    };
    Ok(json!({
        "point": array(point),
        "plane_uv": plane.coordinates(point).map(|uv| [uv.x, uv.y]),
        "source_revision": s.state()?.revision,
        "document_uid": s.state()?.uid,
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LineInput {
    name: String,
    start: Query,
    end: Query,
}
fn line(s: &mut Session, p: &Value) -> Result<Value> {
    let input: LineInput = serde_json::from_value(p.clone()).map_err(|e| fail(&format!("Invalid 3D line parameters: {e}")))?;
    let name = input.name.trim();
    if name.is_empty() || name.len() > 256 {
        return Err(fail("Line name must contain 1 to 256 bytes"));
    }
    // Resolve both endpoints against the same unmodified document first.
    let start = resolved(s, &input.start)?;
    let end = resolved(s, &input.end)?;
    if (start.point - end.point).len() <= 1e-9 {
        return Err(fail("3D line requires distinct endpoints"));
    }
    let curve = Curve { degree: 1, control: vec![start.point, end.point], weights: vec![1., 1.], knots: uniform_knots(2, 1) };
    if !curve.valid() {
        return Err(fail("Invalid line geometry"));
    }
    let d = s.doc_mut()?;
    if d.geometry3d.len() >= 4096 || d.handseed == u64::MAX {
        return Err(fail("3D object limit reached"));
    }
    let id = d.new_handle().0;
    let layer = d.header.str("CLAYER", "0");
    d.geometry3d.push(GeometryObject { id, name: name.to_owned(), layer, visible: true, shape: Shape::Curve(curve.into()) });
    Ok(json!({"id":id,"start":array(start.point),"end":array(end.point),
        "start_kind":start.kind,"end_kind":end.kind}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(x: f64, y: f64) -> Value {
        json!({
            "pixel":[x,y],"viewport":[400.,400.],"center":[0.,0.,0.],
            "yaw":0.,"pitch":-std::f64::consts::FRAC_PI_2,"scale":10.,
        })
    }

    #[test]
    fn plane_grid_and_screen_parallel_failure() {
        let mut s = Session::new();
        let revision = s.state().unwrap().revision;
        let p = s.execute("geometry3d.cplane.point", &q(227., 174.)).unwrap();
        assert!((p["point"][0].as_f64().unwrap() - 2.7).abs() < 1e-9);
        assert!((p["point"][1].as_f64().unwrap() - 2.6).abs() < 1e-9);
        let mut grid = q(227., 174.);
        grid["grid"] = json!(1.);
        assert_eq!(s.execute("geometry3d.snap", &grid).unwrap()["point"], json!([3., 3., 0.]));
        assert_eq!(s.execute("geometry3d.snap", &grid).unwrap()["kind"], "grid");
        grid["pitch"] = json!(0.);
        assert!(s.execute("geometry3d.cplane.point", &grid).is_err());
        assert!(s.execute("geometry3d.snap", &grid).is_err());
        assert_eq!(s.state().unwrap().revision, revision);
    }

    #[test]
    fn exact_curve_snap_visibility_and_replayable_line() {
        let mut s = Session::new();
        let id = s
            .execute(
                "nurbs.curve3d",
                &json!({
                    "name":"Reference",
                    "curve":{"degree":1,
                        "control":[{"x":-2.,"y":0.,"z":0.},{"x":2.,"y":0.,"z":0.}],
                        "weights":[1.,1.],"knots":[0.,0.,1.,1.]
                    }
                }),
            )
            .unwrap()["id"]
            .as_u64()
            .unwrap();
        let revision = s.state().unwrap().revision;
        let result = s.execute("geometry3d.snap", &q(181., 201.)).unwrap();
        assert_eq!(result["source_id"], id);
        assert_eq!(result["kind"], "endpoint");
        assert_eq!(result["source_revision"], revision);
        assert_eq!(s.state().unwrap().revision, revision);
        let midpoint = s.execute("geometry3d.snap", &q(201., 201.)).unwrap();
        assert_eq!(midpoint["kind"], "parameter_midpoint");
        let line = s
            .execute(
                "geometry3d.line",
                &json!({
                    "name":"Snapped segment","start":q(181., 201.),"end":q(220., 200.)
                }),
            )
            .unwrap();
        assert_eq!(line["start_kind"], "endpoint");
        assert_eq!(line["end_kind"], "endpoint");
        assert_eq!(s.doc().unwrap().geometry3d.len(), 2);
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().geometry3d.len(), 1);
        s.execute("geometry3d.set", &json!({"id":id,"visible":false})).unwrap();
        assert!(s.execute("geometry3d.snap", &q(181., 201.)).unwrap()["source_id"].is_null());
        let layer = s.doc().unwrap().geometry3d[0].layer.clone();
        s.doc_mut().unwrap().layer_mut(&layer).unwrap().locked = true;
        assert!(s.execute("geometry3d.snap", &q(201., 201.)).unwrap()["source_id"].is_null());
    }

    #[test]
    fn bad_requests_and_degenerate_line_are_atomic() {
        let mut s = Session::new();
        let mut bad_q = q(200., 200.);
        bad_q["grid"] = json!(0.);
        assert!(s.execute("geometry3d.snap", &bad_q).is_err());
        bad_q = q(200., 200.);
        bad_q["viewport"] = json!([0., 400.]);
        assert!(s.execute("geometry3d.snap", &bad_q).is_err());
        bad_q = q(200., 200.);
        bad_q["plane"] = json!({"origin":[0.,0.,0.],"x_axis":[1.,0.,0.],"y_axis":[1.,0.,0.]});
        assert!(s.execute("geometry3d.snap", &bad_q).is_err());
        assert!(
            s.execute(
                "geometry3d.line",
                &json!({
                    "name":"Zero","start":q(200.,200.),"end":q(200.,200.)
                })
            )
            .is_err()
        );
        assert!(s.doc().unwrap().geometry3d.is_empty());
    }
}
