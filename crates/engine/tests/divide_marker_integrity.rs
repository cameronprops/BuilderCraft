//! DIVIDE must publish all requested finite marker positions or none.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn divide_uses_exact_station_count_and_positions() {
    let mut s = Session::new();
    let line = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let result = s.execute("divide", &json!({"handle":line,"segments":5})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 4);
    assert_eq!(s.doc().unwrap().model.len(), 5);

    let mut stations: Vec<_> = s.doc().unwrap().model.iter().filter_map(|entity| match &entity.kind {
        EntityKind::Point(p) => Some((p.p.x, p.p.y)),
        _ => None,
    }).collect();
    stations.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(stations, vec![(2.0, 0.0), (4.0, 0.0), (6.0, 0.0), (8.0, 0.0)]);
}

#[test]
fn divide_invalid_requests_preserve_document_revision_and_undo() {
    let mut s = Session::new();
    let line = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for params in [
        json!({"handle":line,"segments":1}),
        json!({"handle":line,"segments":32768}),
        json!({"handle":line,"segments":5,"block":"DOES_NOT_EXIST"}),
    ] {
        assert!(s.execute("divide", &params).is_err(), "divide accepted {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }
}
