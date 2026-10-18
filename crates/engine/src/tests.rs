use cadcraft_doc::EntityKind;
use cadcraft_geom::Vec2;
use serde_json::json;

use crate::*;

fn kinds(s: &Session) -> Vec<&'static str> {
    s.doc().unwrap().model.iter().map(|e| e.kind.type_name()).collect()
}

#[test]
fn line_via_command_line() {
    let mut s = Session::new();
    s.cmdline("LINE").unwrap();
    s.cmdline("0,0").unwrap();
    s.cmdline("10,0").unwrap();
    s.cmdline("@0,5").unwrap();
    s.cmdline("c").unwrap();
    assert!(s.running.is_none());
    assert_eq!(kinds(&s), vec!["Line", "Line", "Line"]);
    // One undo step for the whole command.
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0);
    s.redo().unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

#[test]
fn line_undo_option_and_aliases() {
    let mut s = Session::new();
    s.cmdline("l 0,0 5,5 10,0 u").unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1);
    s.cmdline("").unwrap(); // Enter ends
    assert!(s.running.is_none());
}

#[test]
fn polar_and_direct_distance() {
    let mut s = Session::new();
    s.cmdline("line 1,1").unwrap();
    s.cursor = Vec2::new(100.0, 1.0);
    s.cmdline("4").unwrap(); // direct distance along +X
    s.cmdline("@3<90").unwrap();
    s.cmdline("").unwrap();
    let ends: Vec<_> = s.doc().unwrap().model.iter().map(|e| e.kind.grips()).collect();
    assert!(ends[0][2].near(Vec2::new(5.0, 1.0), 1e-9));
    assert!(ends[1][2].near(Vec2::new(5.0, 4.0), 1e-9));
}

#[test]
fn circle_variants() {
    let mut s = Session::new();
    s.cmdline("circle 0,0 5").unwrap();
    s.cmdline("c 10,0 d 4").unwrap();
    s.cmdline("circle 3p 0,0 2,2 4,0").unwrap();
    s.cmdline("circle 2p 0,0 0,6").unwrap();
    let d = s.doc().unwrap();
    let radii: Vec<f64> = d.model.iter().filter_map(|e| if let EntityKind::Circle(c) = &e.kind { Some(c.radius) } else { None }).collect();
    assert_eq!(radii.len(), 4);
    assert!((radii[0] - 5.0).abs() < 1e-9);
    assert!((radii[1] - 2.0).abs() < 1e-9);
    assert!((radii[2] - 2.0).abs() < 1e-9);
    assert!((radii[3] - 3.0).abs() < 1e-9);
}

#[test]
fn rectangle_polygon_and_pline() {
    let mut s = Session::new();
    s.cmdline("rectang 0,0 4,3").unwrap();
    s.cmdline("polygon 6 10,10 i 2").unwrap();
    s.cmdline("pline 0,0 5,0 a 5,5 l 0,5 c").unwrap();
    let d = s.doc().unwrap();
    let mut it = d.model.iter();
    match &it.next().unwrap().kind {
        EntityKind::LwPolyline(p) => assert_eq!(p.vertices.len(), 4),
        _ => panic!(),
    }
    match &it.next().unwrap().kind {
        EntityKind::LwPolyline(p) => assert_eq!(p.vertices.len(), 6),
        _ => panic!(),
    }
    match &it.next().unwrap().kind {
        EntityKind::LwPolyline(p) => {
            assert!(p.closed);
            assert!(p.vertices[1].bulge.abs() > 0.1, "arc segment has a bulge");
        }
        _ => panic!(),
    }
}

#[test]
fn json_execute_and_errors() {
    let mut s = Session::new();
    let r = s.execute("circle", &json!({"center": [1, 2], "radius": 3})).unwrap();
    assert!(r["handle"].is_string());
    assert!(s.execute("circle", &json!({"center": [1, 2]})).is_err());
    assert!(s.execute("circle", &json!("not an object")).is_err());
    assert!(s.execute("nope", &json!({})).is_err());
    // Bad params don't create undo steps.
    assert_eq!(s.state().unwrap().undo.len(), 1);
}

#[test]
fn move_with_pickfirst_and_window_selection() {
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap();
    s.execute("line", &json!({"points": [[5, 5], [6, 5]]})).unwrap();
    s.viewport_px = (1000.0, 1000.0);
    s.state_mut().unwrap().set_view(View { center: Vec2::new(3.0, 3.0), height: 10.0 });
    // Window selection from left to right picks only the first line.
    s.cmdline("move").unwrap();
    s.input(Input::Point(Vec2::new(-1.0, -1.0))).unwrap();
    s.input(Input::Point(Vec2::new(2.0, 1.0))).unwrap();
    s.input(Input::Enter).unwrap();
    s.cmdline("0,0").unwrap();
    s.cmdline("10,0").unwrap();
    assert!(s.running.is_none());
    let g: Vec<_> = s.doc().unwrap().model.iter().map(|e| e.kind.grips()[0]).collect();
    assert!(g[0].near(Vec2::new(10.0, 0.0), 1e-9));
    assert!(g[1].near(Vec2::new(5.0, 5.0), 1e-9));
}

#[test]
fn erase_all_and_selection_keywords() {
    let mut s = Session::new();
    s.execute("circle", &json!({"center": [0, 0], "radius": 1})).unwrap();
    s.execute("circle", &json!({"center": [5, 0], "radius": 1})).unwrap();
    s.cmdline("erase").unwrap();
    s.cmdline("all").unwrap();
    s.cmdline("").unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

#[test]
fn trim_line_between_edges() {
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[0, 0], [10, 0]]})).unwrap();
    s.execute("line", &json!({"points": [[3, -2], [3, 2]]})).unwrap();
    s.execute("line", &json!({"points": [[7, -2], [7, 2]]})).unwrap();
    let h = s.doc().unwrap().model.iter().next().unwrap().handle;
    s.execute("trim", &json!({"handle": h.hex(), "pick": [5, 0]})).unwrap();
    let lines: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter(|e| matches!(&e.kind, EntityKind::Line(l) if l.a.y == 0.0 && l.b.y == 0.0))
        .map(|e| e.kind.grips())
        .collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn extend_line_to_boundary() {
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[0, 0], [5, 0]]})).unwrap();
    s.execute("line", &json!({"points": [[8, -2], [8, 2]]})).unwrap();
    let h = s.doc().unwrap().model.iter().next().unwrap().handle;
    s.execute("extend", &json!({"handle": h.hex(), "pick": [4.5, 0]})).unwrap();
    let g = s.doc().unwrap().entity(h).unwrap().kind.grips();
    assert!(g[2].near(Vec2::new(8.0, 0.0), 1e-9));
}

#[test]
fn fillet_two_lines() {
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[0, 0], [10, 0]]})).unwrap();
    s.execute("line", &json!({"points": [[0, 0], [0, 10]]})).unwrap();
    let hs: Vec<_> = s.doc().unwrap().model.handles();
    let r = s.execute("fillet", &json!({"h1": hs[0].hex(), "p1": [5, 0], "h2": hs[1].hex(), "p2": [0, 5], "radius": 2})).unwrap();
    assert!(r["arc"].is_string());
    let g = s.doc().unwrap().entity(hs[0]).unwrap().kind.grips();
    assert!(g[0].near(Vec2::new(10.0, 0.0), 1e-9) || g[2].near(Vec2::new(10.0, 0.0), 1e-9));
    assert!(g.iter().any(|p| p.near(Vec2::new(2.0, 0.0), 1e-9)));
}

#[test]
fn offset_circle_and_polyline() {
    let mut s = Session::new();
    let c = s.execute("circle", &json!({"center": [0, 0], "radius": 5})).unwrap();
    s.execute("offset", &json!({"handle": c["handle"], "distance": 1, "side": [0, 0]})).unwrap();
    let r = s.execute("rectang", &json!({"p1": [0, 0], "p2": [10, 10]})).unwrap();
    s.execute("offset", &json!({"handle": r["handle"], "distance": 1, "side": [5, 5]})).unwrap();
    let d = s.doc().unwrap();
    let last = d.model.last().unwrap();
    match &last.kind {
        EntityKind::LwPolyline(p) => {
            let b = cadcraft_geom::Bounds2::from_points(p.vertices.iter().map(|v| v.p));
            assert!(b.min.near(Vec2::new(1.0, 1.0), 1e-9), "{b:?}");
            assert!(b.max.near(Vec2::new(9.0, 9.0), 1e-9));
        }
        _ => panic!(),
    }
}

#[test]
fn copy_rotate_scale_mirror() {
    let mut s = Session::new();
    let l = s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap();
    let h = l["handles"][0].clone();
    s.execute("copy", &json!({"handles": [h], "delta": [0, 2], "count": 3})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 4);
    s.execute("rotate", &json!({"handles": [h], "base": [0, 0], "angle": 90})).unwrap();
    s.execute("scale", &json!({"handles": [h], "base": [0, 0], "factor": 3})).unwrap();
    let hh = cadcraft_doc::Handle::parse_hex(h.as_str().unwrap()).unwrap();
    let g = s.doc().unwrap().entity(hh).unwrap().kind.grips();
    assert!(g[2].near(Vec2::new(0.0, 3.0), 1e-9));
    s.execute("mirror", &json!({"handles": [h], "p1": [-1, 0], "p2": [-1, 1]})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 5);
}

#[test]
fn layers_and_properties() {
    let mut s = Session::new();
    s.execute("layer.new", &json!({"name": "Walls", "color": "red", "current": true})).unwrap();
    let c = s.execute("circle", &json!({"center": [0, 0], "radius": 1})).unwrap();
    let h = cadcraft_doc::Handle::parse_hex(c["handle"].as_str().unwrap()).unwrap();
    assert_eq!(s.doc().unwrap().entity(h).unwrap().common.layer, "Walls");
    assert!(s.execute("layer.set", &json!({"name": "Walls", "frozen": true})).is_err());
    s.execute("properties.set", &json!({"handles": [h.hex()], "color": 3, "radius": 4})).unwrap();
    let e = s.doc().unwrap().entity(h).unwrap();
    assert_eq!(e.common.color, cadcraft_color::Color::Index(3));
    assert!(matches!(&e.kind, EntityKind::Circle(c) if c.radius == 4.0));
    assert!(s.execute("layer.new", &json!({"name": "bad/name"})).is_err());
}

#[test]
fn zoom_extents_fits_drawing() {
    let mut s = Session::new();
    s.viewport_px = (1000.0, 500.0);
    s.execute("line", &json!({"points": [[0, 0], [100, 10]]})).unwrap();
    s.execute("zoom.extents", &json!({})).unwrap();
    let v = s.state().unwrap().view();
    assert!((v.center.x - 50.0).abs() < 1e-9);
    assert!(v.height >= 50.0);
}

#[test]
fn sysvars_roundtrip() {
    let mut s = Session::new();
    s.execute("setvar", &json!({"name": "osmode", "value": 7})).unwrap();
    assert_eq!(s.settings.osmode, 7);
    s.execute("setvar", &json!({"name": "LTSCALE", "value": 2.5})).unwrap();
    assert_eq!(s.doc().unwrap().header.f64("LTSCALE", 0.0), 2.5);
    assert!(s.execute("setvar", &json!({"name": "ORTHOMODE", "value": "x"})).is_err());
    assert_eq!(sysvars::get(&s, "orthomode"), Some(json!(0)));
}

#[test]
fn script_runs_commands() {
    let mut s = Session::new();
    s.script("LINE 0,0 10,0 10,10\n\nCIRCLE 5,5 2\nTEXT 0,-2 0.5 0 Hello world\n").unwrap();
    assert_eq!(kinds(&s), vec!["Line", "Line", "Circle", "Text"]);
    let t = s.doc().unwrap().model.last().unwrap();
    assert!(matches!(&t.kind, EntityKind::Text(t) if t.value == "Hello world"));
}

#[test]
fn enter_repeats_last_command() {
    let mut s = Session::new();
    s.cmdline("circle 0,0 1").unwrap();
    s.cmdline("").unwrap();
    assert_eq!(s.running.as_ref().map(|r| r.id.as_str()), Some("circle"));
    s.cmdline("5,5 1").unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
}

#[test]
fn cancel_keeps_partial_line_as_one_undo() {
    let mut s = Session::new();
    s.cmdline("line 0,0 1,0 2,0").unwrap();
    s.cancel();
    assert_eq!(s.doc().unwrap().model.len(), 2);
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

fn partially_writes_then_fails(s: &mut Session, _: &serde_json::Value) -> Result<serde_json::Value> {
    s.add_entity(crate::cmd::helpers::line(Vec2::new(2.0, 2.0), Vec2::new(7.0, 2.0)))?;
    s.set_selection(Vec::new());
    Err(EngineError::Other("intentional transactional test failure".into()))
}

#[test]
fn failed_registered_command_restores_document_selection_and_redo() {
    let mut s = Session::new();
    let line = s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap();
    s.execute("circle", &json!({"center": [3, 3], "radius": 2})).unwrap();
    s.undo().unwrap();
    let h = cadcraft_doc::Handle::parse_hex(line["handles"][0].as_str().unwrap()).unwrap();
    s.set_selection(vec![h]);

    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();
    let redo_count = s.state().unwrap().redo.len();
    assert!(redo_count > 0);

    let spec = CommandSpec::new("test.partial_write", "Partial Write Fixture", partially_writes_then_fails);
    assert!(matches!(s.execute_spec(&spec, &json!({})), Err(EngineError::Other(_))));
    let state = s.state().unwrap();
    assert!(std::sync::Arc::ptr_eq(&before, &state.doc), "failed command must restore the same drawing");
    assert_eq!(state.selection, vec![h], "failed command must restore selection");
    assert_eq!(state.revision, revision, "failure must not advance document revision");
    assert_eq!(state.undo.len(), undo_count, "failure must not create a false undo step");
    assert_eq!(state.redo.len(), redo_count, "failure must preserve available redo");

    s.redo().unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2, "redo must still restore the prior valid circle");
}

#[test]
fn transform_rejects_stale_or_locked_batch_without_partial_geometry() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points": [[0, 0], [5, 0]]})).unwrap();
    let first = a["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points": [[0, 2], [5, 2]]})).unwrap();
    let second = b["handles"][0].as_str().unwrap().to_string();
    let stale = format!("{:X}", u64::MAX);

    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    assert!(s.execute("move", &json!({"handles": [first.clone(), stale], "delta": [10, 0]})).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);

    // Lock only the second line's layer. The first line must not move when
    // the second line is rejected.
    s.execute("layer.new", &json!({"name": "Locked"})).unwrap();
    s.execute("laymch", &json!({"handles": [second.clone()], "layer": "Locked"})).unwrap();
    s.execute("layer.set", &json!({"name": "Locked", "locked": true})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    assert!(s.execute("move", &json!({"handles": [first.clone(), second.clone()], "delta": [10, 0]})).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);

    // A genuinely valid transform still edits the editable target.
    assert_eq!(s.execute("move", &json!({"handles": [first], "delta": [2, 0]})).unwrap()["moved"], 1);
}

#[test]
fn transform_rejects_coordinate_overflow_without_mutation() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    for params in
        [json!({"handles":[h.clone()], "from":[-1e308, 0], "to":[1e308, 0]}), json!({"handles":[h.clone()], "delta":[1e308, 1e308], "count": 2})]
    {
        let command = if params.get("count").is_some() { "copy" } else { "move" };
        assert!(s.execute(command, &params).is_err(), "{command} accepted numeric overflow: {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
    }
}

#[test]
fn explicitly_malformed_target_handles_never_silently_edit_selection() {
    let mut s = Session::new();
    let result = s.execute("line", &json!({"points": [[0, 0], [5, 0]]})).unwrap();
    let valid = result["handles"][0].as_str().unwrap().to_string();
    s.set_selection(vec![cadcraft_doc::Handle::parse_hex(&valid).unwrap()]);
    let original = s.state().unwrap().doc.clone();
    let selection = s.selection();
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();

    // A bad item must reject the WHOLE batch. A wrong type or a null
    // explicit handle must not accidentally operate on the pick-first selection.
    for params in [
        json!({"handles": [valid.clone(), "not-a-handle"], "delta": [2, 3]}),
        json!({"handles": {"bad": valid.clone()}, "delta": [2, 3]}),
        json!({"handles": [valid.clone(), -1], "delta": [2, 3]}),
        json!({"handles": [valid.clone(), null], "delta": [2, 3]}),
        json!({"handle": null, "delta": [2, 3]}),
        json!({"handle": {"bad": valid.clone()}, "delta": [2, 3]}),
    ] {
        assert!(s.execute("move", &params).is_err(), "move accepted malformed targets: {params}");
        assert!(std::sync::Arc::ptr_eq(&original, &s.state().unwrap().doc));
        assert_eq!(s.selection(), selection);
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_count);
    }

    // The normal valid explicit target still works, regardless of selection.
    s.set_selection(Vec::new());
    let changed = s.execute("move", &json!({"handle": valid, "delta": [2, 3]})).unwrap();
    assert_eq!(changed["moved"], 1);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Line(line) => {
            assert_eq!((line.a.x, line.a.y), (2.0, 3.0));
            assert_eq!((line.b.x, line.b.y), (7.0, 3.0));
        }
        other => panic!("expected translated line, got {other:?}"),
    }
}

#[test]
fn duplicated_explicit_handles_transform_each_entity_only_once() {
    let mut s = Session::new();
    let result = s.execute("line", &json!({"points": [[0, 0], [5, 0]]})).unwrap();
    let handle = result["handles"][0].as_str().unwrap();
    let moved = s.execute("move", &json!({"handles": [handle, handle], "delta": [2, 3]})).unwrap();
    assert_eq!(moved["moved"], 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Line(line) => {
            assert_eq!((line.a.x, line.a.y), (2.0, 3.0));
            assert_eq!((line.b.x, line.b.y), (7.0, 3.0));
        }
        other => panic!("expected a single translated line, got {other:?}"),
    }
}

#[test]
fn every_registered_command_error_preserves_drawing_and_undo() {
    // This cross-cutting safety test does not certify successful command behavior.
    // Invalid requests must be atomic even when they fail late in a multi-step edit.
    for spec in command_specs() {
        for request in [json!(null), json!({"invalid_probe": [1e308, -1e308]})] {
            let mut s = Session::new();
            let state = s.state().unwrap();
            let uid = state.uid;
            let document = state.doc.clone();
            let revision = state.revision;
            let undo_len = state.undo.len();
            let redo_len = state.redo.len();
            let selection = state.selection.clone();

            if s.execute(spec.id, &request).is_err() {
                let state = s
                    .docs
                    .iter()
                    .find(|doc| doc.uid == uid)
                    .unwrap_or_else(|| panic!("{} discarded the open document when rejecting invalid params", spec.id));
                assert!(std::sync::Arc::ptr_eq(&document, &state.doc), "{} changed the document despite returning an error", spec.id);
                assert_eq!(state.selection, selection, "{} changed selection on error", spec.id);
                assert_eq!(state.revision, revision, "{} advanced revision on error", spec.id);
                assert_eq!(state.undo.len(), undo_len, "{} changed undo on error", spec.id);
                assert_eq!(state.redo.len(), redo_len, "{} changed redo on error", spec.id);
            }
        }
    }
}

#[test]
fn degenerate_drawing_commands_fail_without_silent_geometry() {
    for (id, params) in [
        ("polygon", json!({"sides": 5, "center": [0, 0], "radius": 0})),
        ("polygon", json!({"sides": 5, "edge": [[1, 1], [1, 1]]})),
        ("polygon", json!({"sides": 5, "edge": [[0, 0], [1e308, 1e308]]})),
        ("arc", json!({"center": [0, 0], "radius": 0, "start": 0, "end": 90})),
        ("ellipse", json!({"center": [0, 0], "major": [1.7e308, 1.7e308], "ratio": 0.5})),
        ("ellipse.arc", json!({"center": [0, 0], "major": [1.7e308, 1.7e308], "ratio": 0.5, "start": 0, "end": 90})),
        ("ellipse.axis", json!({"p1": [-1e308, 0], "p2": [1e308, 0], "distance": 2})),
        ("point.multiple", json!({"points": []})),
        ("donut", json!({"center": [0, 0], "inside": 5, "outside": 3})),
        ("donut", json!({"center": [0, 0], "inside": -1, "outside": 3})),
        ("donut", json!({"center": [0, 0], "inside": 2, "outside": 2})),
        ("donut", json!({"center": [0, 0], "inside": 2})),
        ("donut", json!({"center": [1.7e308, 0], "inside": 1e308, "outside": 1.1e308})),
    ] {
        let mut s = Session::new();
        let before = s.state().unwrap().doc.clone();
        let revision = s.state().unwrap().revision;
        assert!(s.execute(id, &params).is_err(), "{id} silently accepted degenerate input: {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc), "{id} mutated the document on error");
        assert_eq!(s.state().unwrap().revision, revision, "{id} unexpectedly changed revision");
        assert!(s.doc().unwrap().model.is_empty(), "{id} created degenerate geometry");
    }
}

#[test]
fn hostile_params_never_panic() {
    let mut s = Session::new();
    for c in command_specs() {
        for p in [
            json!(null),
            json!([]),
            json!("x"),
            json!({"points": [[1e308, -1e308], ["a"]]}),
            json!({"handles": ["zz", 99999]}),
            json!({"radius": -1, "center": [0, 0]}),
        ] {
            let _ = s.execute(c.id, &p);
        }
    }
}

#[test]
fn every_interactive_command_starts_and_cancels() {
    for c in command_specs().iter().filter(|c| c.interactive.is_some()) {
        let mut s = Session::new();
        s.start(c.id).unwrap();
        let _ = s.prompt_text();
        let _ = s.preview(Vec2::new(1.0, 1.0));
        s.input(Input::Point(Vec2::new(1.0, 2.0))).unwrap();
        let _ = s.preview(Vec2::new(3.0, 1.0));
        s.cancel();
        assert!(s.running.is_none(), "{} still running", c.id);
    }
}

#[test]
fn sample_drawing_builds() {
    let d = sample::bracket();
    assert!(d.model.len() > 30);
    let l = cadcraft_render::build(&d, &cadcraft_doc::Space::Model, &cadcraft_render::Options::default());
    assert!(l.prims.len() > 100);
}

#[test]
fn osnap_toggle() {
    let mut s = Session::new();
    s.execute("osnap", &json!({"modes": ["end", "mid", "cen"]})).unwrap();
    assert_eq!(s.settings.osmode, 7);
    s.execute("osnap", &json!({"on": false})).unwrap();
    assert!(s.settings.osmode & snap::mode::OFF != 0);
}

#[test]
fn dimlinear_interactive_and_continue() {
    let mut s = Session::new();
    s.cmdline("dimlinear 0,0 10,0 5,2").unwrap();
    assert!(s.running.is_none());
    s.cmdline("dimcontinue 25,0").unwrap();
    s.cmdline("").unwrap();
    let dims: Vec<_> =
        s.doc().unwrap().model.iter().filter_map(|e| if let EntityKind::Dimension(d) = &e.kind { Some(d.clone()) } else { None }).collect();
    assert_eq!(dims.len(), 2);
    assert!(dims[1].p13.xy().near(Vec2::new(10.0, 0.0), 1e-9));
    assert!(dims[1].p14.xy().near(Vec2::new(25.0, 0.0), 1e-9));
    assert!(s.log.iter().any(|l| l.contains("Dimension text = 10.0000")));
}

#[test]
fn dim_auto_vertical_and_radius() {
    let mut s = Session::new();
    s.execute("dimlinear", &json!({"p1": [0, 0], "p2": [3, 8], "at": [6, 4]})).unwrap();
    let c = s.execute("circle", &json!({"center": [20, 0], "radius": 2})).unwrap();
    s.execute("dimradius", &json!({"handle": c["handle"], "at": [23, 1]})).unwrap();
    let d = s.doc().unwrap();
    let kinds: Vec<_> = d.model.iter().filter_map(|e| if let EntityKind::Dimension(d) = &e.kind { Some(d.kind) } else { None }).collect();
    assert!(matches!(kinds[0], cadcraft_doc::DimKind::Linear { rotation } if (rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-12));
    assert!(matches!(kinds[1], cadcraft_doc::DimKind::Radius));
}

#[test]
fn hatch_by_pick_point_with_island() {
    let mut s = Session::new();
    s.execute("rectang", &json!({"p1": [0, 0], "p2": [10, 10]})).unwrap();
    s.execute("circle", &json!({"center": [5, 5], "radius": 2})).unwrap();
    s.cmdline("hatch 1,1").unwrap();
    s.cmdline("").unwrap();
    let h = s.doc().unwrap().model.iter().find_map(|e| if let EntityKind::Hatch(h) = &e.kind { Some(h.clone()) } else { None }).unwrap();
    assert_eq!(h.loops.len(), 2, "outer boundary plus the circle island");
    // Hatches go to the back of the draw order.
    assert!(matches!(s.doc().unwrap().model.iter().next().unwrap().kind, EntityKind::Hatch(_)));
    // No boundary → error message, no hatch.
    let mut s2 = Session::new();
    s2.execute("line", &json!({"points": [[0, 0], [5, 0]]})).unwrap();
    assert!(s2.execute("hatch", &json!({"points": [[1, 1]]})).is_err());
}

#[test]
fn hatch_from_overlapping_lines() {
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[-1, 0], [11, 0]]})).unwrap();
    s.execute("line", &json!({"points": [[10, -1], [10, 6]]})).unwrap();
    s.execute("line", &json!({"points": [[11, 5], [-1, 5]]})).unwrap();
    s.execute("line", &json!({"points": [[0, 6], [0, -1]]})).unwrap();
    let r = s.execute("boundary", &json!({"points": [[3, 3]]})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 1);
    let area = s.execute("area", &json!({"handle": r["handles"][0]})).unwrap();
    assert!((area["area"].as_f64().unwrap() - 50.0).abs() < 1e-6);
}

#[test]
fn block_insert_with_attributes() {
    let mut s = Session::new();
    s.execute("circle", &json!({"center": [0, 0], "radius": 1})).unwrap();
    s.execute("attdef", &json!({"tag": "TAG", "prompt": "Tag?", "default": "A1", "at": [1.2, 0]})).unwrap();
    s.execute("selectall", &json!({})).unwrap();
    s.cmdline("block").unwrap();
    s.cmdline("Valve").unwrap();
    s.cmdline("0,0").unwrap(); // pickfirst selection used
    assert!(s.running.is_none());
    assert!(s.doc().unwrap().block("Valve").is_some());
    assert_eq!(s.doc().unwrap().model.len(), 1, "originals converted to one insert");
    s.cmdline("insert Valve 10,0 2 90 V-101").unwrap();
    let ins: Vec<_> =
        s.doc().unwrap().model.iter().filter_map(|e| if let EntityKind::Insert(i) = &e.kind { Some(i.clone()) } else { None }).collect();
    assert_eq!(ins.len(), 2);
    assert_eq!(ins[1].attribs[0].text.value, "V-101");
    assert!((ins[1].scale.x - 2.0).abs() < 1e-12);
    // Explode the scaled insert back into geometry.
    let h = s.doc().unwrap().model.last().unwrap().handle;
    s.execute("explode", &json!({"handles": [h.hex()]})).unwrap();
    assert!(s.doc().unwrap().model.iter().any(|e| matches!(&e.kind, EntityKind::Circle(c) if (c.radius - 2.0).abs() < 1e-9)));
    // PURGE keeps the used block.
    s.execute("purge", &json!({})).unwrap();
    assert!(s.doc().unwrap().block("Valve").is_some());
}

#[test]
fn mleader_and_qdim() {
    let mut s = Session::new();
    s.cmdline("mleader 0,0 3,2").unwrap();
    s.cmdline("NOTE A").unwrap();
    assert!(
        s.doc().unwrap().model.iter().any(|e| matches!(&e.kind, EntityKind::MLeader(m) if m.text.as_ref().is_some_and(|t| t.contents == "NOTE A")))
    );
    s.execute("line", &json!({"points": [[0, 0], [4, 0], [9, 0]]})).unwrap();
    let hs: Vec<String> = s.doc().unwrap().model.iter().filter(|e| matches!(e.kind, EntityKind::Line(_))).map(|e| e.handle.hex()).collect();
    let r = s.execute("qdim", &json!({"handles": hs, "at": [0, -2]})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 2);
}
