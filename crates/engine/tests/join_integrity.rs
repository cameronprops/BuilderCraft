//! Failed JOIN must not discard unrelated or incompatible curves.
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn disconnected_curve_is_not_deleted_when_partial_join_would_succeed() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[1,0],[2,0]]})).unwrap();
    let detached = s.execute("line", &json!({"points":[[10,0],[11,0]]})).unwrap();
    let handles = json!([
        a["handles"][0].clone(),
        a["handles"][1].clone(),
        detached["handles"][0].clone()
    ]);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    let err = s.execute("join", &json!({"handles":handles})).expect_err("disconnected JOIN must fail");
    assert!(err.to_string().contains("disconnected"), "unexpected error: {err}");
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);
    assert_eq!(s.doc().unwrap().model.len(), 3);
}

#[test]
fn mixed_unsupported_curve_and_missing_target_do_not_disappear() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[1,0],[2,0]]})).unwrap();
    let c = s.execute("circle", &json!({"center":[0,0],"radius":2})).unwrap();
    let before = s.state().unwrap().doc.clone();
    for extra in [c["handle"].clone(), json!("FFFFFFFFFFFFFFFF")] {
        let params = json!({"handles":[a["handles"][0].clone(), a["handles"][1].clone(), extra]});
        assert!(s.execute("join", &params).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.doc().unwrap().model.len(), 3);
    }
}
