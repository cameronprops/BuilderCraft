//! PEDIT JOIN must not silently discard an invalid explicit target.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn malformed_pedit_join_handles_fail_without_converting_or_mutating_base() {
    let mut s = Session::new();
    let base = s.execute("pline", &json!({"vertices":[[0,0],[2,0]]})).unwrap()["handle"].as_str().unwrap().to_string();
    let other = s.execute("line", &json!({"points":[[2,0],[4,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for handles2 in [
        json!([other.clone(), "malformed handle"]),
        json!([other.clone(), null]),
        json!({"not": "an array"}),
    ] {
        let err = s.execute("pedit", &json!({"handle":base, "option":"join", "handles2":handles2})).expect_err("invalid handle must reject entire join request");
        assert!(err.to_string().contains("handle"), "unexpected error: {err}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
        assert_eq!(s.doc().unwrap().model.len(), 2);
    }
    let result = s.execute("pedit", &json!({"handle":base, "option":"join", "handles2":[other]})).unwrap();
    assert_eq!(result["joined"], 1);
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(p) => {
            assert_eq!(p.vertices.len(), 3);
            assert_eq!(p.vertices.last().unwrap().p.x, 4.0);
        }
        other => panic!("expected joined polyline, got {other:?}"),
    }
}
