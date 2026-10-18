//! DIVIDE must generate the expected number of markers or reject the command.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn divide_five_equal_sections_generates_four_exact_interior_markers() {
    let mut s = Session::new();
    let h = s.execute("line", &json!({"points":[[0,0],[10,0]]})).unwrap()["handles"][0].clone();
    let result = s.execute("divide", &json!({"handle":h, "segments":5})).unwrap();
    assert_eq!(result["handles"].as_array().unwrap().len(), 4);
    assert_eq!(s.doc().unwrap().model.len(), 5);
    let mut positions: Vec<_> = s.doc().unwrap().model.iter().filter_map(|e| match &e.kind {
        EntityKind::Point(p) => Some(p.p.x),
        _ => None,
    }).collect();
    positions.sort_by(f64::total_cmp);
    assert_eq!(positions, vec![2.0, 4.0, 6.0, 8.0]);
}

#[test]
fn oversized_closed_curve_division_rejects_without_changing_document() {
    let mut s = Session::new();
    let h = s.execute("circle", &json!({"center":[0,0],"radius":10})).unwrap()["handle"].clone();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    let result = s.execute("divide", &json!({"handle":h,"segments":20001}));
    let error = result.expect_err("DIVIDE should reject more than 20,000 markers");
    assert!(error.to_string().contains("20000"), "expected concrete budget in error: {error}");
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_len);
    assert_eq!(s.doc().unwrap().model.len(), 1);
}
