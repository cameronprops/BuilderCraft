//! MEASURE must publish the entire finite marker sequence or none.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn measure_line_places_all_markers_at_requested_spacing() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let r = s.execute("measure", &json!({"handle":h,"length":2})).unwrap();
    assert_eq!(r["handles"].as_array().unwrap().len(), 4);
    let mut stations: Vec<_> = s
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Point(p) => Some((p.p.x, p.p.y)),
            _ => None,
        })
        .collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(stations, vec![(2.0, 0.0), (4.0, 0.0), (6.0, 0.0), (8.0, 0.0)]);
    assert_eq!(s.doc().unwrap().model.len(), 5);
}

#[test]
fn measure_bad_spacing_preserves_document_and_undo() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for params in [
        json!({"handle":h,"length":0}),
        json!({"handle":h,"length":-2}),
        json!({"handle":h,"length":1e-9}),
        json!({"handle":h,"length":null}),
        json!({"handle":h,"length":2,"block":"DOES_NOT_EXIST"}),
    ] {
        assert!(s.execute("measure", &params).is_err(), "measure accepted {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }
}