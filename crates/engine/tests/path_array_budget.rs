//! Large path arrays must be bounded by total output entity count.
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn path_array_large_selection_fails_atomically_but_small_array_succeeds() {
    let mut s = Session::new();
    let points: Vec<_> = (0..=50).map(|i| json!([i, 0])).collect();
    let source = s.execute("line", &json!({"points": points})).unwrap();
    let path = s.execute("line", &json!({"points":[[0,5],[100,5]]})).unwrap()["handles"][0].clone();
    let source_handles = source["handles"].clone();

    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();
    let oversized = json!({"handles": source_handles, "path": path, "count": 1000});
    let failure = s.execute("arraypath", &oversized).expect_err("oversized path array should fail");
    assert!(failure.to_string().contains("20000"), "unexpected error: {failure}");
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_count);
    assert_eq!(s.doc().unwrap().model.len(), 51);

    let small = s.execute("arraypath", &json!({"handles":[source["handles"][0].clone()], "path":path, "count":4})).unwrap();
    assert_eq!(small["handles"].as_array().unwrap().len(), 3);
    assert_eq!(s.doc().unwrap().model.len(), 54);
}
