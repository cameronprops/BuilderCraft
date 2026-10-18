//! Array/copy replication requests must reject excessive output before editing.
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn large_replication_requests_are_rejected_before_any_document_write() {
    let mut session = Session::new();
    let result = session.execute("line", &json!({"points":[[0,0],[1,0],[2,0],[3,0]]})).unwrap();
    let handles = result["handles"].as_array().unwrap().clone();
    let before = session.state().unwrap().doc.clone();
    let revision = session.state().unwrap().revision;
    let undo_len = session.state().unwrap().undo.len();

    for (command, params) in [
        ("arrayrect", json!({"handles":[handles[0].clone()], "rows":1000, "cols":1000, "rowSpacing":1, "colSpacing":1})),
        ("arraypolar", json!({"handles":handles, "center":[0,0], "count":10000})),
        ("copy", json!({"handles":result["handles"], "delta":[1,0], "count":10000})),
    ] {
        let failure = session.execute(command, &params).expect_err("excessive replication must be rejected");
        assert!(failure.to_string().contains("20"), "{command} returned an unrelated failure: {failure}");
        assert!(std::sync::Arc::ptr_eq(&before, &session.state().unwrap().doc), "{command} changed the drawing");
        assert_eq!(session.state().unwrap().revision, revision, "{command} changed revision");
        assert_eq!(session.state().unwrap().undo.len(), undo_len, "{command} changed undo");
        assert_eq!(session.doc().unwrap().model.len(), 3);
    }
}
