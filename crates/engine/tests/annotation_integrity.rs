//! TEXT and MTEXT must reject invalid annotation geometry without mutating drawings.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn text_rejects_bad_explicit_height_and_rotation_atomically() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();

    for params in [
        json!({"at":[2,3],"text":"Label","height":-1}),
        json!({"at":[2,3],"text":"Label","height":0}),
        json!({"at":[2,3],"text":"Label","height":"wrong"}),
        json!({"at":[2,3],"text":"Label","height":null}),
        json!({"at":[2,3],"text":"Label","rotation":"wrong"}),
        json!({"at":[2,3],"text":"Label","rotation":null}),
    ] {
        assert!(s.execute("text", &params).is_err(), "accepted malformed text {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }
    let result = s.execute("text", &json!({"at":[2,3],"text":"Label","height":2,"rotation":90})).unwrap();
    assert!(result["handle"].is_string());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Text(t) => {
            assert_eq!(t.value, "Label");
            assert_eq!((t.insert.x, t.insert.y), (2.0, 3.0));
            assert_eq!(t.height, 2.0);
            assert!((t.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        }
        other => panic!("expected text entity, got {other:?}"),
    }
}

#[test]
fn mtext_rejects_silent_clamping_or_defaulting_without_edits() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();

    for (key, value) in [
        ("height", json!(-1)),
        ("height", json!(null)),
        ("width", json!(-2)),
        ("width", json!("invalid")),
        ("attach", json!(0)),
        ("attach", json!(10)),
        ("attach", json!(1.5)),
        ("attach", json!("invalid")),
        ("rotation", json!(null)),
        ("lineSpacing", json!(0)),
        ("lineSpacing", json!(-1)),
    ] {
        let mut params = json!({"at":[4,5],"text":"Fabrication"});
        params[key] = value;
        assert!(s.execute("mtext", &params).is_err(), "accepted malformed mtext {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }
    let result = s.execute("mtext", &json!({"at":[4,5],"text":"Scenic\nNotes","height":2,"width":10,"attach":9,"lineSpacing":1.25})).unwrap();
    assert!(result["handle"].is_string());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::MText(t) => {
            assert_eq!(t.contents, "Scenic\\PNotes");
            assert_eq!((t.insert.x, t.insert.y), (4.0, 5.0));
            assert_eq!(t.height, 2.0);
            assert_eq!(t.width, 10.0);
            assert_eq!(t.attach, 9);
            assert_eq!(t.line_spacing, 1.25);
        }
        other => panic!("expected mtext entity, got {other:?}"),
    }
}
