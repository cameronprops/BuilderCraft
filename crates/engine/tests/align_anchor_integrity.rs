//! ALIGN must not silently discard malformed second point pairs or scaling.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn invalid_alignment_pairs_are_atomic_and_do_not_fall_back_to_translation() {
    let mut s = Session::new();
    let handle = s.execute("line", &json!({"points":[[0,0],[2,0]]})).unwrap()["handles"][0].as_str().unwrap().to_string();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    for params in [
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"s2":[2,0]}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"d2":[5,9]}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"s2":"wrong","d2":[5,9]}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"s2":[2,0],"d2":null}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"s2":[0,0],"d2":[5,9]}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"s2":[2,0],"d2":[5,5]}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"scale":true}),
        json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"scale":"true"}),
    ] {
        let error = s.execute("align", &params).expect_err("invalid ALIGN input must not silently degrade");
        assert!(error.to_string().contains("align"), "unexpected ALIGN error: {error}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }

    let result = s.execute("align", &json!({"handles":[handle],"s1":[0,0],"d1":[5,5],"s2":[2,0],"d2":[5,9],"scale":true})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 1);
    let entity = s.doc().unwrap().model.iter().next().unwrap();
    match &entity.kind {
        EntityKind::Line(line) => {
            assert!((line.a.x - 5.0).abs() < 1e-9);
            assert!((line.a.y - 5.0).abs() < 1e-9);
            assert!((line.b.x - 5.0).abs() < 1e-9);
            assert!((line.b.y - 9.0).abs() < 1e-9);
        }
        other => panic!("expected line after ALIGN, got {other:?}"),
    }
}
