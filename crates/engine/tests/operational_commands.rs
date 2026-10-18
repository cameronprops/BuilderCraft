//! Positive command acceptance cases. Registration or a non-panic error is not a pass.
//!
//! The ignored release gate deliberately fails until every registered command
//! has an independently asserted, realistic success path.

use cadcraft_doc::EntityKind;
use cadcraft_engine::{Session, command_specs, find_command};
use serde_json::json;
use std::collections::HashSet;

type Case = (&'static str, fn());

const CASES: &[Case] = &[
    ("line", line_creates_geometry),
    ("circle", circle_creates_geometry),
    ("rectang", rectangle_creates_geometry),
    ("layer.new", layer_new_assigns_geometry),
    ("move", move_changes_coordinates),
    ("copy", copy_adds_objects),
    ("rotate", rotate_changes_coordinates),
    ("scale", scale_changes_coordinates),
    ("mirror", mirror_preserves_source),
    ("undo", undo_restores_document),
    ("redo", redo_restores_document),
    ("zoom.extents", zoom_fits_drawing),
    ("worldwright.point.distance", native_distance_is_correct),
    ("worldwright.point.interpolate", native_interpolation_is_correct),
    ("worldwright.tool.run", generic_dispatch_matches_direct_command),
    ("mesh3d.create", mesh_creation_is_persistent),
    ("mesh3d.list", mesh_listing_finds_created_object),
    ("pline", polyline_maintains_closed_shape),
    ("arc", arc_creates_expected_radius),
    ("polygon", polygon_creates_six_sides),
    ("ellipse", ellipse_preserves_axes),
    ("point", single_point_uses_requested_coordinates),
    ("point.multiple", multiple_points_are_independent),
    ("ray", ray_keeps_normalized_direction),
    ("dist", distance_query_computes_three_four_five),
    ("area", area_query_finds_rectangle_measurements),
    ("id", id_query_updates_last_point),
    ("layer", layer_list_includes_new_layer),
    ("layer.set", layer_set_changes_lock_state),
    ("layer.current", layer_current_sets_active_layer),
    ("properties", properties_query_reports_selected_line),
    ("properties.set", properties_set_changes_entity_color),
    ("drawing.inspect", drawing_inspection_counts_entities),
    ("entities", entity_filter_excludes_other_shapes),
    ("view.set", view_set_uses_exact_coordinates),
    ("view.get", view_get_reflects_new_view),
    ("pan", pan_changes_camera_not_geometry),
    ("zoom.in", zoom_in_halves_view_height),
    ("zoom.out", zoom_out_doubles_view_height),
    ("regen", regen_invalidates_view_without_geometry_change),
    ("redraw", redraw_does_not_edit_entities),
    ("time", time_query_reports_drawing_revision),
    ("status", status_query_reports_current_layer),
    ("count", count_query_on_empty_blocks),
];

fn first_line(s: &Session) -> (f64, f64, f64, f64) {
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Line(line) => (line.a.x, line.a.y, line.b.x, line.b.y),
        other => panic!("expected Line, got {other:?}"),
    }
}

fn line_handle(s: &mut Session) -> String {
    let created = s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap();
    created["handles"][0].as_str().unwrap().to_owned()
}

fn line_creates_geometry() {
    let mut s = Session::new();
    line_handle(&mut s);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    assert_eq!(first_line(&s), (0.0, 0.0, 1.0, 0.0));
}

fn circle_creates_geometry() {
    let mut s = Session::new();
    let r = s.execute("circle", &json!({"center": [2, 3], "radius": 5})).unwrap();
    assert!(r["handle"].is_string());
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Circle(c) => {
            assert_eq!(c.center.x, 2.0);
            assert_eq!(c.center.y, 3.0);
            assert_eq!(c.radius, 5.0);
        }
        other => panic!("expected Circle, got {other:?}"),
    }
}

fn rectangle_creates_geometry() {
    let mut s = Session::new();
    s.execute("rectang", &json!({"p1": [0, 0], "p2": [4, 3]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => assert_eq!(poly.vertices.len(), 4),
        other => panic!("expected LwPolyline, got {other:?}"),
    }
}

fn layer_new_assigns_geometry() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Fixtures", "current": true})).unwrap();
    let h = line_handle(&mut s);
    let handle = cadcraft_doc::Handle::parse_hex(&h).unwrap();
    assert_eq!(s.doc().unwrap().entity(handle).unwrap().common.layer, "Fixtures");
}

fn move_changes_coordinates() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("move", &json!({"handles": [h], "delta": [3, 4]})).unwrap();
    assert_eq!(first_line(&s), (3.0, 4.0, 4.0, 4.0));
}

fn copy_adds_objects() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("copy", &json!({"handles": [h], "delta": [0, 2], "count": 2})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

fn rotate_changes_coordinates() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("rotate", &json!({"handles": [h], "base": [0, 0], "angle": 90})).unwrap();
    let (ax, ay, bx, by) = first_line(&s);
    assert!(ax.abs() < 1e-9 && ay.abs() < 1e-9 && bx.abs() < 1e-9 && (by - 1.0).abs() < 1e-9);
}

fn scale_changes_coordinates() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("scale", &json!({"handles": [h], "base": [0, 0], "factor": 3})).unwrap();
    assert_eq!(first_line(&s), (0.0, 0.0, 3.0, 0.0));
}

fn mirror_preserves_source() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("mirror", &json!({"handles": [h], "p1": [0, 0], "p2": [0, 1]})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
    assert_eq!(first_line(&s), (0.0, 0.0, 1.0, 0.0));
}

fn undo_restores_document() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn redo_restores_document() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("undo", &json!({})).unwrap();
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1);
}

fn zoom_fits_drawing() {
    let mut s = Session::new();
    s.viewport_px = (1000.0, 500.0);
    s.execute("line", &json!({"points": [[0, 0], [100, 10]]})).unwrap();
    s.execute("zoom.extents", &json!({})).unwrap();
    let view = s.state().unwrap().view();
    assert!(view.center.x.is_finite() && view.height > 0.0);
    assert!(view.height > 10.0);
}

fn point_inputs() -> serde_json::Value {
    json!({
        "a": {"kind": "point", "value": {"x": 0.0, "y": 0.0, "z": 0.0}},
        "b": {"kind": "point", "value": {"x": 3.0, "y": 4.0, "z": 0.0}}
    })
}

fn native_distance_is_correct() {
    let mut s = Session::new();
    let revision = s.state().unwrap().revision;
    let r = s.execute("worldwright.point.distance", &json!({"inputs": point_inputs()})).unwrap();
    assert_eq!(r["output"]["kind"], "number");
    assert_eq!(r["output"]["value"], 5.0);
    assert_eq!(s.state().unwrap().revision, revision);
}

fn native_interpolation_is_correct() {
    let mut s = Session::new();
    let mut inputs = point_inputs();
    inputs["t"] = json!({"kind": "number", "value": 0.5});
    let r = s.execute("worldwright.point.interpolate", &json!({"inputs": inputs})).unwrap();
    assert_eq!(r["output"]["value"]["x"], 1.5);
    assert_eq!(r["output"]["value"]["y"], 2.0);
}

fn generic_dispatch_matches_direct_command() {
    let mut s = Session::new();
    let inputs = point_inputs();
    let direct = s.execute("worldwright.point.distance", &json!({"inputs": inputs.clone()})).unwrap();
    let generic = s.execute("worldwright.tool.run", &json!({"operation": "kernel.point.distance", "inputs": inputs})).unwrap();
    assert_eq!(direct, generic);
}

fn quad_mesh() -> serde_json::Value {
    json!({
        "vertices": [
            {"x": 0.0, "y": 0.0, "z": 0.0},
            {"x": 2.0, "y": 0.0, "z": 0.0},
            {"x": 2.0, "y": 2.0, "z": 0.0},
            {"x": 0.0, "y": 2.0, "z": 0.0}
        ],
        "faces": [{"quad": [0, 1, 2, 3]}]
    })
}

fn mesh_creation_is_persistent() {
    let mut s = Session::new();
    let r = s.execute("mesh3d.create", &json!({"name": "Tile", "mesh": quad_mesh()})).unwrap();
    assert!(r["id"].as_u64().is_some());
    assert_eq!(s.doc().unwrap().mesh3d.len(), 1);
    let bytes = cadcraft_io::write(s.doc().unwrap(), "operational.dftba").unwrap();
    let reopened = cadcraft_io::read(&bytes, "operational.dftba").unwrap();
    assert_eq!(reopened.mesh3d.len(), 1);
    assert_eq!(reopened.mesh3d[0].name, "Tile");
}

fn mesh_listing_finds_created_object() {
    let mut s = Session::new();
    let r = s.execute("mesh3d.create", &json!({"name": "Tile", "mesh": quad_mesh()})).unwrap();
    let listed = s.execute("mesh3d.list", &json!({})).unwrap();
    assert_eq!(listed["objects"][0]["id"], r["id"]);
}

fn polyline_maintains_closed_shape() {
    let mut s = Session::new();
    s.execute("pline", &json!({"vertices": [[0, 0], [4, 0], [4, 3]], "closed": true})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert_eq!(poly.vertices.len(), 3);
            assert!(poly.closed);
        }
        other => panic!("expected LwPolyline, got {other:?}"),
    }
}

fn arc_creates_expected_radius() {
    let mut s = Session::new();
    s.execute("arc", &json!({"center": [0, 0], "radius": 7, "start": 0, "end": 90})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Arc(arc) => assert_eq!(arc.radius, 7.0),
        other => panic!("expected Arc, got {other:?}"),
    }
}

fn polygon_creates_six_sides() {
    let mut s = Session::new();
    s.execute("polygon", &json!({"sides": 6, "center": [0, 0], "radius": 5})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert_eq!(poly.vertices.len(), 6);
            assert!(poly.closed);
        }
        other => panic!("expected LwPolyline, got {other:?}"),
    }
}

fn ellipse_preserves_axes() {
    let mut s = Session::new();
    s.execute("ellipse", &json!({"center": [1, 2], "major": [5, 0], "ratio": 0.5})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Ellipse(el) => {
            assert_eq!(el.center.x, 1.0);
            assert_eq!(el.center.y, 2.0);
            assert_eq!(el.major.x, 5.0);
            assert_eq!(el.ratio, 0.5);
        }
        other => panic!("expected Ellipse, got {other:?}"),
    }
}

fn single_point_uses_requested_coordinates() {
    let mut s = Session::new();
    let result = s.execute("point", &json!({"at": [6, 9]})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Point(p) => {
            assert_eq!(p.p.x, 6.0);
            assert_eq!(p.p.y, 9.0);
        }
        other => panic!("expected Point, got {other:?}"),
    }
}

fn multiple_points_are_independent() {
    let mut s = Session::new();
    let result = s.execute("point.multiple", &json!({"points": [[0, 0], [2, 3], [4, 5]]})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 3);
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

fn ray_keeps_normalized_direction() {
    let mut s = Session::new();
    s.execute("ray", &json!({"base": [1, 2], "through": [4, 6]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Ray(ray) => {
            assert_eq!(ray.base.x, 1.0);
            assert!((ray.dir.x - 0.6).abs() < 1e-9);
            assert!((ray.dir.y - 0.8).abs() < 1e-9);
        }
        other => panic!("expected Ray, got {other:?}"),
    }
}

fn distance_query_computes_three_four_five() {
    let mut s = Session::new();
    let before = s.state().unwrap().revision;
    let r = s.execute("dist", &json!({"p1": [0, 0], "p2": [3, 4]})).unwrap();
    assert_eq!(r["distance"], 5.0);
    assert_eq!(r["dx"], 3.0);
    assert_eq!(r["dy"], 4.0);
    assert_eq!(s.state().unwrap().revision, before);
}

fn area_query_finds_rectangle_measurements() {
    let mut s = Session::new();
    let r = s.execute("area", &json!({"points": [[0, 0], [4, 0], [4, 3], [0, 3]]})).unwrap();
    assert_eq!(r["area"], 12.0);
    assert_eq!(r["perimeter"], 14.0);
}

fn id_query_updates_last_point() {
    let mut s = Session::new();
    let r = s.execute("id", &json!({"at": [6, 8]})).unwrap();
    assert_eq!(r["x"], 6.0);
    assert_eq!(r["y"], 8.0);
    assert_eq!(s.last_point, cadcraft_geom::Vec2::new(6.0, 8.0));
}

fn layer_list_includes_new_layer() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Rockwork"})).unwrap();
    let response = s.execute("layer", &json!({})).unwrap();
    assert!(response["layers"].as_array().unwrap().iter().any(|layer| layer["name"] == "Rockwork"));
}

fn layer_set_changes_lock_state() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Rigging"})).unwrap();
    s.execute("layer.set", &json!({"name": "Rigging", "locked": true})).unwrap();
    let response = s.execute("layer", &json!({})).unwrap();
    assert!(response["layers"].as_array().unwrap().iter().any(|layer| layer["name"] == "Rigging" && layer["locked"] == true));
}

fn layer_current_sets_active_layer() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Lighting"})).unwrap();
    s.execute("layer.current", &json!({"name": "Lighting"})).unwrap();
    assert_eq!(s.doc().unwrap().header.str("CLAYER", "0"), "Lighting");
}

fn properties_query_reports_selected_line() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    let result = s.execute("properties", &json!({"handles": [h]})).unwrap();
    assert_eq!(result["count"], 1);
}

fn properties_set_changes_entity_color() {
    let mut s = Session::new();
    let h = line_handle(&mut s);
    s.execute("properties.set", &json!({"handles": [h], "color": 3})).unwrap();
    let result = s.execute("properties", &json!({"handles": [h.clone()]})).unwrap();
    assert_eq!(result["count"], 1);
    let handle = cadcraft_doc::Handle::parse_hex(&h).unwrap();
    assert_eq!(s.doc().unwrap().entity(handle).unwrap().common.color, cadcraft_color::Color::Index(3));
}

fn drawing_inspection_counts_entities() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("circle", &json!({"center": [4, 4], "radius": 2})).unwrap();
    let report = s.execute("drawing.inspect", &json!({"entities": true})).unwrap();
    assert_eq!(report["entityCount"], 2);
    assert_eq!(report["entities"].as_array().unwrap().len(), 2);
}

fn entity_filter_excludes_other_shapes() {
    let mut s = Session::new();
    line_handle(&mut s);
    s.execute("circle", &json!({"center": [4, 4], "radius": 2})).unwrap();
    let report = s.execute("entities", &json!({"type": "Line"})).unwrap();
    assert_eq!(report["count"], 1);
    assert_eq!(report["entities"][0]["type"], "Line");
}

fn view_set_uses_exact_coordinates() {
    let mut s = Session::new();
    s.execute("view.set", &json!({"center": [8, 6], "height": 20})).unwrap();
    let view = s.state().unwrap().view();
    assert_eq!(view.center.x, 8.0);
    assert_eq!(view.center.y, 6.0);
    assert_eq!(view.height, 20.0);
}

fn view_get_reflects_new_view() {
    let mut s = Session::new();
    s.execute("view.set", &json!({"center": [7, 9], "height": 25})).unwrap();
    let result = s.execute("view.get", &json!({})).unwrap();
    assert_eq!(result["center"], json!([7.0, 9.0]));
    assert_eq!(result["height"], 25.0);
}

fn pan_changes_camera_not_geometry() {
    let mut s = Session::new();
    line_handle(&mut s);
    let before = first_line(&s);
    let center = s.state().unwrap().view().center;
    s.execute("pan", &json!({"delta": [2, 3]})).unwrap();
    assert_eq!(s.state().unwrap().view().center, center - cadcraft_geom::Vec2::new(2.0, 3.0));
    assert_eq!(first_line(&s), before);
}

fn zoom_in_halves_view_height() {
    let mut s = Session::new();
    let height = s.state().unwrap().view().height;
    s.execute("zoom.in", &json!({})).unwrap();
    assert_eq!(s.state().unwrap().view().height, height / 2.0);
}

fn zoom_out_doubles_view_height() {
    let mut s = Session::new();
    let height = s.state().unwrap().view().height;
    s.execute("zoom.out", &json!({})).unwrap();
    assert_eq!(s.state().unwrap().view().height, height * 2.0);
}

fn regen_invalidates_view_without_geometry_change() {
    let mut s = Session::new();
    line_handle(&mut s);
    let before = first_line(&s);
    let revision = s.state().unwrap().revision;
    s.execute("regen", &json!({})).unwrap();
    assert!(s.state().unwrap().revision > revision);
    assert_eq!(first_line(&s), before);
}

fn redraw_does_not_edit_entities() {
    let mut s = Session::new();
    line_handle(&mut s);
    let before = first_line(&s);
    s.execute("redraw", &json!({})).unwrap();
    assert_eq!(first_line(&s), before);
}

fn time_query_reports_drawing_revision() {
    let mut s = Session::new();
    let report = s.execute("time", &json!({})).unwrap();
    assert!(report["message"].as_str().unwrap().contains("Drawing revision"));
}

fn status_query_reports_current_layer() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Scenery", "current": true})).unwrap();
    let report = s.execute("status", &json!({})).unwrap();
    assert!(report["message"].as_str().unwrap().contains("Current layer: Scenery"));
}

fn count_query_on_empty_blocks() {
    let mut s = Session::new();
    let report = s.execute("count", &json!({"block": "NotPresent"})).unwrap();
    assert_eq!(report["count"], 0);
}

#[test]
fn positive_operational_cases() {
    let registry: HashSet<_> = command_specs().iter().map(|c| c.id).collect();
    assert_eq!(registry.len(), command_specs().len(), "duplicate command IDs in the live registry");
    let mut seen = HashSet::new();
    for (id, verify) in CASES {
        assert!(registry.contains(id), "acceptance case refers to missing command {id}");
        assert!(find_command(id).is_some());
        assert!(seen.insert(*id), "duplicate operational case for {id}");
        verify();
    }
    eprintln!("Worldwright positive functional cases: {} / {} registered commands", CASES.len(), registry.len());
}

#[test]
#[ignore = "Operational alpha release gate: needs independently passing success, failure, undo and persistence cases for every command"]
fn release_gate_no_command_without_operational_case() {
    // This must not be reclassified as green by the ordinary test runner.
    // Extend CASES with genuine positive fixtures, not name-only placeholders.
    positive_operational_cases();
    let proven: HashSet<_> = CASES.iter().map(|(id, _)| *id).collect();
    let mut missing: Vec<_> = command_specs().iter().map(|spec| spec.id).filter(|id| !proven.contains(id)).collect();
    missing.sort_unstable();
    assert!(missing.is_empty(), "{} commands lack positive operational fixtures:\n{}", missing.len(), missing.join(", "));
}
