//! Read-only inquiry commands must never report missing objects or overflow as valid measurements.
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn stale_list_target_and_invalid_measurements_are_explicit_errors_without_edits() {
    let mut s = Session::new();
    let result = s.execute("line", &json!({"points":[[0,0],[3,4]]})).unwrap();
    let existing = result["handles"][0].clone();
    let circle = s.execute("circle", &json!({"center":[0,0],"radius":3})).unwrap()["handle"].clone();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    for (cmd, params) in [
        ("list", json!({"handles":[existing, "FFFFFFFFFFFFFFFF"]})),
        ("dist", json!({"p1":[-1e308,0],"p2":[1e308,0]})),
        ("area", json!({"points":[[0,0],[1,1]]})),
        ("area", json!({"handle":circle,"points":[[0,0],"bad",[1,1]]})),
        ("area", json!({"points":[[0,0],[1e308,0],[0,1e308]]})),
    ] {
        assert!(s.execute(cmd, &params).is_err(), "{cmd} silently accepted {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }
}

#[test]
fn list_and_measurement_queries_report_actual_geometry() {
    let mut s = Session::new();
    let result = s.execute("line", &json!({"points":[[0,0],[3,4]]})).unwrap();
    let handle = result["handles"][0].clone();

    let details = s.execute("list", &json!({"handles":[handle]})).unwrap();
    assert!(details["message"].as_str().unwrap().contains("LINE"));
    assert!(details["message"].as_str().unwrap().contains("Length = 5"));
    let d = s.execute("dist", &json!({"p1":[0,0],"p2":[3,4]})).unwrap();
    assert_eq!(d["distance"], 5.0);
    assert_eq!(d["dx"], 3.0);
    assert_eq!(d["dy"], 4.0);

    let area = s.execute("area", &json!({"points":[[0,0],[4,0],[4,3],[0,3]]})).unwrap();
    assert_eq!(area["area"], 12.0);
    assert_eq!(area["perimeter"], 14.0);
}
