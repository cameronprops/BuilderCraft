//! TRIM/EXTEND must validate complete explicit cutter selections before editing a drawing.
use cadcraft_doc::{EntityKind, Handle};
use cadcraft_engine::Session;
use serde_json::json;

fn make_line(s: &mut Session, a: [f64; 2], b: [f64; 2]) -> String {
    s.execute("line", &json!({"points":[a,b]})).unwrap()["handles"][0].as_str().unwrap().to_owned()
}

#[test]
fn malformed_or_missing_cutting_edges_fail_without_any_edits() {
    let mut s = Session::new();
    let target = make_line(&mut s, [0.0, 0.0], [10.0, 0.0]);
    let cutter = make_line(&mut s, [5.0, -2.0], [5.0, 2.0]);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    for edges in [
        json!([cutter, "FFFFFFFFFFFFFFFF"]),
        json!([cutter, null]),
        json!([cutter, cutter]),
        json!([target]),
        json!([]),
        json!(null),
        json!("not-an-array"),
        json!({"unexpected":"object"}),
    ] {
        for cmd in ["trim", "extend"] {
            let result = s.execute(cmd, &json!({"handle":target,"pick":[8.0,0.0],"edges":edges}));
            assert!(result.is_err(), "{cmd} accepted invalid cutters {edges}");
            assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
            assert_eq!(s.state().unwrap().revision, revision);
            assert_eq!(s.state().unwrap().undo.len(), undo_len);
            assert_eq!(s.doc().unwrap().model.len(), 2);
        }
    }
}

#[test]
fn locked_trim_or_extend_target_is_untouched() {
    let mut s = Session::new();
    let target = make_line(&mut s, [0.0, 0.0], [10.0, 0.0]);
    let cutter = make_line(&mut s, [5.0, -2.0], [5.0, 2.0]);
    s.execute("layer.new", &json!({"name":"ProtectedGeometry"})).unwrap();
    s.execute("laymch", &json!({"handles":[target],"layer":"ProtectedGeometry"})).unwrap();
    s.execute("layer.set", &json!({"name":"ProtectedGeometry","locked":true})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    for cmd in ["trim", "extend"] {
        let err = s.execute(cmd, &json!({"handle":target,"pick":[8.0,0.0],"edges":[cutter]})).unwrap_err();
        assert!(err.to_string().contains("locked layer"), "{cmd}: {err}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }
}

#[test]
fn trim_removes_only_the_picked_piece_at_a_valid_cutting_edge() {
    let mut s = Session::new();
    let target = make_line(&mut s, [0.0, 0.0], [10.0, 0.0]);
    let cutter = make_line(&mut s, [5.0, -2.0], [5.0, 2.0]);
    let result = s.execute("trim", &json!({"handle":target,"pick":[8.0,0.0],"edges":[cutter]})).unwrap();
    let handles = result["handles"].as_array().unwrap();
    assert_eq!(handles.len(), 1);
    assert!(s.doc().unwrap().entity(Handle::parse_hex(&target).unwrap()).is_none());
    assert_eq!(s.doc().unwrap().model.len(), 2);

    let kept = Handle::parse_hex(handles[0].as_str().unwrap()).unwrap();
    match &s.doc().unwrap().entity(kept).unwrap().kind {
        EntityKind::Line(l) => {
            assert_eq!((l.a.x, l.a.y), (0.0, 0.0));
            assert!((l.b.x - 5.0).abs() < 1e-9 && l.b.y.abs() < 1e-9);
        }
        other => panic!("expected kept line, got {other:?}"),
    }
}

#[test]
fn extend_reaches_boundary_without_modifying_locked_reference_geometry() {
    let mut s = Session::new();
    let target = make_line(&mut s, [0.0, 0.0], [2.0, 0.0]);
    let cutter = make_line(&mut s, [5.0, -2.0], [5.0, 2.0]);
    s.execute("layer.new", &json!({"name":"Reference"})).unwrap();
    s.execute("laymch", &json!({"handles":[cutter],"layer":"Reference"})).unwrap();
    s.execute("layer.set", &json!({"name":"Reference","locked":true})).unwrap();

    s.execute("extend", &json!({"handle":target,"pick":[2.0,0.0],"edges":[cutter]})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let entity = s.doc().unwrap().entity(Handle::parse_hex(&target).unwrap()).unwrap();
    match &entity.kind {
        EntityKind::Line(l) => {
            assert_eq!((l.a.x, l.a.y), (0.0, 0.0));
            assert!((l.b.x - 5.0).abs() < 1e-9 && l.b.y.abs() < 1e-9);
        }
        other => panic!("expected extended line, got {other:?}"),
    }
    let reference = s.doc().unwrap().entity(Handle::parse_hex(&cutter).unwrap()).unwrap();
    match &reference.kind {
        EntityKind::Line(l) => assert_eq!((l.a.x, l.a.y, l.b.x, l.b.y), (5.0, -2.0, 5.0, 2.0)),
        other => panic!("expected unchanged boundary line, got {other:?}"),
    }
}
