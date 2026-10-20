//! Nested block copying must validate every explicitly supplied point.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

fn block_reference(s: &mut Session) -> String {
    let line = s.execute("line", &json!({"points":[[0,0],[3,4]]})).unwrap();
    s.execute("block", &json!({
        "name":"NestedTest", "base":[0,0], "handles":line["handles"], "keep":"delete"
    }))
    .unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0);
    s.execute("insert", &json!({"name":"NestedTest","at":[5,6]}))
        .unwrap()["handle"].as_str().unwrap().to_string()
}

#[test]
fn ncopy_preserves_nested_world_coordinates_and_original_reference() {
    let mut s = Session::new();
    let source = block_reference(&mut s);
    let result = s.execute("ncopy", &json!({"handle":source,"delta":[10,-2]})).unwrap();
    let copied = result["handles"].as_array().unwrap();
    assert_eq!(copied.len(), 1);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let entity = s.doc().unwrap().model.iter().find(|e| e.handle.hex() == copied[0].as_str().unwrap()).unwrap();
    match &entity.kind {
        EntityKind::Line(line) => {
            assert_eq!((line.a.x, line.a.y), (15.0, 4.0));
            assert_eq!((line.b.x, line.b.y), (18.0, 8.0));
        }
        other => panic!("expected independent copied line, got {other:?}"),
    }
    assert!(s.doc().unwrap().model.iter().any(|e| e.handle.hex() == source && matches!(&e.kind, EntityKind::Insert(_))));
}

#[test]
fn ncopy_rejects_malformed_or_conflicting_coordinates_atomically() {
    let mut s = Session::new();
    let source = block_reference(&mut s);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    for params in [
        json!({"handle":source,"delta":"invalid"}),
        json!({"handle":source,"delta":null}),
        json!({"handle":source,"from":[0,0]}),
        json!({"handle":source,"to":[10,0]}),
        json!({"handle":source,"delta":[2,0],"from":[0,0],"to":[2,0]}),
        json!({"handle":source,"pick":"invalid","delta":[2,0]}),
        json!({"handle":source,"pick":null}),
        json!({"handle":source,"from":[-1e308,0],"to":[1e308,0]}),
    ] {
        assert!(s.execute("ncopy", &params).is_err(), "NCOPY silently accepted {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
        assert_eq!(s.doc().unwrap().model.len(), 1);
    }
    let valid = s.execute("ncopy", &json!({"handle":source,"from":[0,0],"to":[2,0]})).unwrap();
    assert_eq!(valid["handles"].as_array().unwrap().len(), 1);
    assert_eq!(s.doc().unwrap().model.len(), 2);
}
