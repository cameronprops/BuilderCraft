//! Explicit DRAWORDER batches must fail atomically when a handle is not editable.
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn stale_or_locked_draworder_target_preserves_order_and_undo() {
    let mut s = Session::new();
    let a = s.execute("line", &json!({"points":[[0,0],[1,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let b = s.execute("line", &json!({"points":[[0,2],[1,2]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    s.execute("layer.new", &json!({"name":"Locked"})).unwrap();
    s.execute("laymch", &json!({"handles":[b], "layer":"Locked"})).unwrap();
    s.execute("layer.set", &json!({"name":"Locked","locked":true})).unwrap();

    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for cmd in ["draworder.front", "draworder.back"] {
        for target in [json!("FFFFFFFFFFFFFFFF"), json!(b)] {
            let params = json!({"handles":[a, target]});
            assert!(s.execute(cmd, &params).is_err(), "{cmd} accepted a stale or locked target");
            assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
            assert_eq!(s.state().unwrap().revision, revision);
            assert_eq!(s.state().unwrap().undo.len(), undo_len);
        }
    }

    s.execute("draworder.front", &json!({"handles":[a]})).unwrap();
    let order: Vec<_> = s.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    assert_eq!(order, vec![b.clone(), a.clone()]);
    s.execute("draworder.back", &json!({"handles":[a.clone()]})).unwrap();
    let order: Vec<_> = s.doc().unwrap().model.iter().map(|e| e.handle.hex()).collect();
    assert_eq!(order, vec![a,b]);
}
