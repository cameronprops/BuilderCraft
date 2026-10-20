//! CHSPACE must never silently skip invalid entities or lose destination data.
use cadcraft_doc::{EntityKind, Space};
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn chspace_rejects_mixed_locked_stale_and_wrong_space_batches_atomically() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,2],[10,2]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    s.execute("layout.new", &json!({"name":"AuditSheet"})).unwrap();
    s.execute("layer.new", &json!({"name":"Protected"})).unwrap();
    s.execute("laymch", &json!({"handles":[b.clone()],"layer":"Protected"})).unwrap();
    s.execute("layer.set", &json!({"name":"Protected","locked":true})).unwrap();

    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for params in [
        json!({"handles":[a.clone(),b.clone()],"to":"AuditSheet"}),
        json!({"handles":[a.clone(),"FFFFFFFFFFFFFFFF"],"to":"AuditSheet"}),
    ] {
        assert!(s.execute("chspace", &params).is_err(), "invalid CHSPACE accepted {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }

    s.execute("layer.set", &json!({"name":"Protected","locked":false})).unwrap();
    let moved = s.execute("chspace", &json!({"handles":[b.clone()],"to":"AuditSheet"})).unwrap();
    assert_eq!(moved["moved"], 1);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    assert!(s.execute("chspace", &json!({"handles":[a.clone(),b.clone()],"to":"AuditSheet"})).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);

    let success = s.execute("chspace", &json!({"handles":[a.clone()],"to":"AuditSheet"})).unwrap();
    assert_eq!(success["moved"], 1);
    assert_eq!(s.doc().unwrap().space_of(cadcraft_doc::Handle::parse_hex(&a).unwrap()), Some(Space::Paper("AuditSheet".to_string())));
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

#[test]
fn chspace_rejects_viewport_entities_without_moving_them() {
    let mut s = Session::new();
    s.execute("layout.new", &json!({"name":"AuditSheet"})).unwrap();
    let space = Space::Paper("AuditSheet".to_string());
    let viewport = s
        .doc()
        .unwrap()
        .space(&space)
        .unwrap()
        .iter()
        .find(|entity| matches!(&entity.kind, EntityKind::Viewport(_)))
        .expect("new layout must have a viewport")
        .handle
        .hex();
    s.state_mut().unwrap().space = space.clone();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    let error = s.execute("chspace", &json!({"handles":[viewport.clone()]})).unwrap_err();
    assert!(error.to_string().contains("viewport"), "unexpected error: {error}");
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);
    assert_eq!(s.doc().unwrap().space_of(cadcraft_doc::Handle::parse_hex(&viewport).unwrap()), Some(space));
}
