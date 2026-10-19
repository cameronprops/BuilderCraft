//! Finite segment constraints must be available without an edit.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

fn setup(s: &mut Session) -> (String, String) {
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,0],[0,8]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    (a, b)
}

#[test]
fn fillet_max_radius_is_reported_and_failed_edits_are_atomic() {
    let mut s = Session::new();
    let (a, b) = setup(&mut s);
    let query = json!({"h1":a,"p1":[8,0],"h2":b,"p2":[0,7],"limitsOnly":true});
    let limits = s.execute("fillet", &query).unwrap();
    assert!((limits["maxRadius"].as_f64().unwrap() - 8.0).abs() < 1e-9);
    assert_eq!(limits["availableFirst"], 10.0);
    assert_eq!(limits["availableSecond"], 8.0);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();
    for radius in [json!(9), json!(-1), json!("invalid")] {
        let result = s.execute("fillet", &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"radius":radius}));
        assert!(result.is_err(), "fillet accepted {radius}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }
    let err = s.execute("fillet", &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"radius":9})).unwrap_err();
    assert!(err.to_string().contains("8.000000"), "missing maximum: {err}");
    let result = s.execute("fillet", &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"radius":2})).unwrap();
    assert!(result["arc"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 3);
    assert!(s.doc().unwrap().model.iter().any(|e| matches!(&e.kind, EntityKind::Arc(arc) if (arc.radius - 2.0).abs() < 1e-9)));
}

#[test]
fn chamfer_reports_individual_max_distances() {
    let mut s = Session::new();
    let (a, b) = setup(&mut s);
    let limits = s.execute("chamfer", &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"limitsOnly":true})).unwrap();
    assert_eq!(limits["maxD1"], 10.0);
    assert_eq!(limits["maxD2"], 8.0);
    let before = s.state().unwrap().doc.clone();
    for (d1, d2) in [(11.0, 2.0), (2.0, 9.0), (-1.0, 2.0)] {
        assert!(s.execute("chamfer", &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"d1":d1,"d2":d2})).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    }
    let result = s.execute("chamfer", &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"d1":2,"d2":3})).unwrap();
    assert!(result["line"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

#[test]
fn locked_or_identical_targets_never_mutate() {
    let mut s = Session::new();
    let (a, b) = setup(&mut s);
    s.execute("layer.new", &json!({"name":"LockedSource"})).unwrap();
    s.execute("laymch", &json!({"handles":[b],"layer":"LockedSource"})).unwrap();
    s.execute("layer.set", &json!({"name":"LockedSource","locked":true})).unwrap();
    let before = s.state().unwrap().doc.clone();
    for cmd in ["fillet", "chamfer"] {
        assert!(s.execute(cmd, &json!({"h1":a,"h2":b,"p1":[8,0],"p2":[0,7],"radius":1,"d1":1,"d2":1})).is_err());
        assert!(s.execute(cmd, &json!({"h1":a,"h2":a,"p1":[8,0],"p2":[8,0],"radius":1,"d1":1,"d2":1})).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    }
}
