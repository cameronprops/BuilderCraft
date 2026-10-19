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
