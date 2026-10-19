//! REVCLOUD object conversion must not consume a locked source or silently default bad input.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn revcloud_invalid_arc_length_and_locked_source_leave_drawing_unchanged() {
    let mut s = Session::new();
    let source = s.execute("circle", &json!({"center":[0,0],"radius":5})).unwrap()["handle"].as_str().unwrap().to_string();
    s.execute("layer.new", &json!({"name":"Protected"})).unwrap();
    s.execute("laymch", &json!({"handles":[source],"layer":"Protected"})).unwrap();
    s.execute("layer.set", &json!({"name":"Protected","locked":true})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();

    for params in [
        json!({"handle":source,"arcLength":3}),
        json!({"handle":source,"arcLength":-1}),
        json!({"handle":source,"arcLength":"invalid"}),
        json!({"handle":source,"arcLength":null}),
    ] {
        assert!(s.execute("revcloud", &params).is_err(), "accepted invalid or protected source: {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_count);
    }

    s.execute("layer.set", &json!({"name":"Protected","locked":false})).unwrap();
    let result = s.execute("revcloud", &json!({"handle":source,"arcLength":3})).unwrap();
    assert!(result["handle"].as_str().is_some());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    let entity = s.doc().unwrap().model.iter().next().unwrap();
    assert_eq!(entity.common.layer, "Protected");
    match &entity.kind {
        EntityKind::LwPolyline(pl) => {
            assert!(pl.closed);
            assert!(pl.vertices.len() >= 3);
            assert!(pl.vertices.iter().all(|v| v.p.is_finite()));
        }
        other => panic!("expected revision cloud polyline, got {other:?}"),
    }
}
