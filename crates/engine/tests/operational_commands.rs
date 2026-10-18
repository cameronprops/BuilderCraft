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
