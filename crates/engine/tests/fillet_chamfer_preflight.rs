//! FILLET and CHAMFER must validate every selected target before changing geometry.
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn invalid_fillet_and_chamfer_requests_leave_drawing_untouched() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,0],[0,10]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    s.execute("layer.new", &json!({"name":"Protected"})).unwrap();
    s.execute("laymch", &json!({"handles":[b.clone()],"layer":"Protected"})).unwrap();
    s.execute("layer.set", &json!({"name":"Protected","locked":true})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();

    for (cmd, params) in [
        ("fillet", json!({"h1":a, "h2":b, "radius":2})),
        ("chamfer", json!({"h1":a, "h2":b, "d1":2, "d2":2})),
        ("fillet", json!({"h1":a, "h2":a, "radius":1})),
        ("chamfer", json!({"h1":a, "h2":"FFFFFFFFFFFFFFFF", "d1":1})),
        ("fillet", json!({"h1":a, "h2":b, "radius":-1})),
        ("fillet", json!({"h1":a, "h2":b, "radius":"invalid"})),
        ("chamfer", json!({"h1":a, "h2":b, "d1":-1})),
        ("chamfer", json!({"h1":a, "h2":b, "d2":"invalid"})),
    ] {
        assert!(s.execute(cmd, &params).is_err(), "{cmd} accepted {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }

    s.execute("layer.set", &json!({"name":"Protected","locked":false})).unwrap();
    let result = s.execute("chamfer", &json!({"h1":a, "h2":b, "p1":[9,0], "p2":[0,9], "d1":2,"d2":2})).unwrap();
    assert!(result["line"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

#[test]
fn fillet_reports_maximum_feasible_radius_and_preserves_geometry() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,0],[0,10]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let pick_a = json!([9,0]);
    let pick_b = json!([0,9]);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    let failed = s.execute("fillet", &json!({"h1":a,"h2":b,"p1":pick_a,"p2":pick_b,"radius":11})).unwrap_err();
    assert!(failed.to_string().contains("maximum feasible radius 10.000000"), "unexpected radius diagnostic: {failed}");
    let failed_chamfer = s.execute("chamfer", &json!({"h1":a,"h2":b,"p1":pick_a,"p2":pick_b,"d1":11,"d2":2})).unwrap_err();
    assert!(failed_chamfer.to_string().contains("maximum d1=10.000000"), "unexpected chamfer diagnostic: {failed_chamfer}");
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);

    let r = s.execute("fillet", &json!({"h1":a,"h2":b,"p1":pick_a,"p2":pick_b,"radius":2})).unwrap();
    assert!(r["arc"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 3);
    let arcs: Vec<_> = s.doc().unwrap().model.iter().filter_map(|e| match &e.kind {
        cadcraft_doc::EntityKind::Arc(arc) => Some(arc.radius),
        _ => None,
    }).collect();
    assert_eq!(arcs, vec![2.0]);
}
