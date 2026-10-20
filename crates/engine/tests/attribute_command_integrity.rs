//! Block attribute edits must be typed, complete, and undoable.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

fn tagged_reference() -> (Session, String) {
    let mut s = Session::new();
    let def = s.execute("attdef", &json!({"tag":"MATERIAL","at":[0,0],"height":0.5,"default":"WOOD","prompt":"Finish"})).unwrap();
    s.execute("block", &json!({"name":"SceneryTag","base":[0,0],"handles":[def["handle"]],"keep":"delete"})).unwrap();
    let inserted = s.execute("insert", &json!({"name":"SceneryTag","at":[5,6]})).unwrap();
    (s, inserted["handle"].as_str().unwrap().to_string())
}

#[test]
fn attdef_and_attribute_manager_preserve_definition_metadata() {
    let (mut s, _) = tagged_reference();
    let report = s.execute("battman", &json!({"name":"SceneryTag"})).unwrap();
    assert_eq!(report["attributes"].as_array().unwrap().len(), 1);
    assert_eq!(report["attributes"][0]["tag"], "MATERIAL");
    assert_eq!(report["attributes"][0]["default"], "WOOD");
    assert_eq!(report["attributes"][0]["prompt"], "Finish");
}

#[test]
fn attedit_updates_existing_attribute_case_insensitively_and_undo_restores_it() {
    let (mut s, h) = tagged_reference();
    let result = s.execute("attedit", &json!({"handle":h,"values":{"material":"METAL"}})).unwrap();
    assert_eq!(result["changed"], 1);
    let entity = s.doc().unwrap().model.iter().next().unwrap();
    match &entity.kind {
        EntityKind::Insert(insert) => {
            assert_eq!(insert.attribs.len(), 1);
            assert_eq!(insert.attribs[0].tag, "MATERIAL");
            assert_eq!(insert.attribs[0].text.value, "METAL");
        }
        other => panic!("expected tagged INSERT, got {other:?}"),
    }
    s.undo().unwrap();
    let entity = s.doc().unwrap().model.iter().next().unwrap();
    assert!(matches!(&entity.kind, EntityKind::Insert(insert) if insert.attribs[0].text.value == "WOOD"));
}

#[test]
fn invalid_attribute_edits_reject_entire_request_without_mutation() {
    let (mut s, h) = tagged_reference();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for values in [
        json!(null),
        json!(42),
        json!({}),
        json!({"UNKNOWN":"A"}),
        json!({"MATERIAL":42}),
        json!({"MATERIAL":"PLASTIC","UNKNOWN":"B"}),
        json!({"MATERIAL":"METAL","material":"WOOD"}),
    ] {
        assert!(s.execute("attedit", &json!({"handle":h,"values":values})).is_err(), "accepted invalid edit: {values}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }
    s.execute("layer.new", &json!({"name":"ProtectedTag"})).unwrap();
    s.execute("laymch", &json!({"handles":[h],"layer":"ProtectedTag"})).unwrap();
    s.execute("layer.set", &json!({"name":"ProtectedTag","locked":true})).unwrap();
    let protected = s.state().unwrap().doc.clone();
    assert!(s.execute("attedit", &json!({"handle":h,"values":{"MATERIAL":"NEW"}})).is_err());
    assert!(std::sync::Arc::ptr_eq(&protected, &s.state().unwrap().doc));
}

#[test]
fn attdef_nonpositive_and_non_numeric_height_is_rejected_atomically() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    for height in [json!(0), json!(-1), json!("invalid"), json!(null)] {
        assert!(s.execute("attdef", &json!({"tag":"TEST","at":[0,0],"height":height})).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
    }
}
