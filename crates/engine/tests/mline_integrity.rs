//! MLINE should create both rails or none, with a validated positive scale.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn invalid_mline_scale_rejects_without_mutating_document_or_undo() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for value in [json!(0), json!(-2), json!("not a number"), json!(null)] {
        let r = s.execute("mline", &json!({"points":[[0,0],[10,0]], "scale":value, "justification":"zero"}));
        assert!(r.is_err(), "MLINE accepted invalid scale");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }
}

#[test]
fn valid_mline_creates_two_parallel_offset_rails() {
    let mut s = Session::new();
    let r = s.execute("mline", &json!({"points":[[0,0],[10,0]], "scale":2, "justification":"zero"})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 2);
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let mut ys = Vec::new();
    for e in s.doc().unwrap().model.iter() {
        match &e.kind {
            EntityKind::LwPolyline(pl) => {
                assert_eq!(pl.vertices.len(), 2);
                assert!((pl.vertices[1].p.x - pl.vertices[0].p.x - 10.0).abs() < 1e-9);
                assert!((pl.vertices[1].p.y - pl.vertices[0].p.y).abs() < 1e-9);
                ys.push(pl.vertices[0].p.y);
            }
            other => panic!("MLINE produced unexpected rail: {other:?}"),
        }
    }
    ys.sort_by(f64::total_cmp);
    assert!((ys[1] - ys[0] - 2.0).abs() < 1e-9, "MLINE rail spacing wrong: {ys:?}");
}
