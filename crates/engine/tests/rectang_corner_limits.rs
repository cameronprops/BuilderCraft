//! RECTANG must not silently discard impossible or malformed corner options.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn oversized_fillet_and_chamfer_report_maximums_without_edits() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();

    let too_large_fillet = s.execute("rectang", &json!({"p1":[0,0],"p2":[10,8],"fillet":5})).unwrap_err();
    assert!(too_large_fillet.to_string().contains("maximum feasible radius 4.000000"), "unexpected fillet limit: {too_large_fillet}");
    let too_large_chamfer = s.execute("rectang", &json!({"p1":[0,0],"p2":[10,8],"chamfer":[6,2]})).unwrap_err();
    assert!(too_large_chamfer.to_string().contains("maximum d1=5.000000, d2=4.000000"), "unexpected chamfer limit: {too_large_chamfer}");
    for options in [
        json!({"fillet":-1}),
        json!({"fillet":null}),
        json!({"fillet":"invalid"}),
        json!({"fillet":2,"chamfer":[1,1]}),
        json!({"chamfer":[2,0]}),
        json!({"chamfer":[0,3]}),
        json!({"chamfer":[2,5]}),
        json!({"chamfer":[2]}),
        json!({"chamfer":"invalid"}),
        json!({"chamfer":[null,2]}),
        json!({"chamfer":[2,"invalid"]}),
    ] {
        let mut params = json!({"p1":[0,0],"p2":[10,8]});
        params.as_object_mut().unwrap().extend(options.as_object().unwrap().clone());
        assert!(s.execute("rectang", &params).is_err(), "accepted invalid rectangle corners: {params}");
    }
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo);
    assert_eq!(s.doc().unwrap().model.len(), 0);
}

#[test]
fn valid_fillet_and_chamfer_preserve_requested_corner_geometry() {
    let mut s = Session::new();
    let rounded = s.execute("rectang", &json!({"p1":[0,0],"p2":[10,8],"fillet":2})).unwrap();
    let rounded_handle = cadcraft_doc::Handle::parse_hex(rounded["handle"].as_str().unwrap()).unwrap();
    match &s.doc().unwrap().entity(rounded_handle).unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert!(poly.closed);
            assert_eq!(poly.vertices.len(), 8);
            assert_eq!(poly.vertices.iter().filter(|v| v.bulge.abs() > 0.001).count(), 4);
        }
        other => panic!("expected rounded rectangle, got {other:?}"),
    }
    let cut = s.execute("rectang", &json!({"p1":[20,0],"p2":[30,8],"chamfer":[2,1]})).unwrap();
    let cut_handle = cadcraft_doc::Handle::parse_hex(cut["handle"].as_str().unwrap()).unwrap();
    match &s.doc().unwrap().entity(cut_handle).unwrap().kind {
        EntityKind::LwPolyline(poly) => {
            assert!(poly.closed);
            assert_eq!(poly.vertices.len(), 8);
            assert!(poly.vertices.iter().all(|v| v.bulge == 0.0));
            assert!(poly.vertices.iter().any(|v| (v.p.x - 22.0).abs() < 1e-9 && v.p.y.abs() < 1e-9));
        }
        other => panic!("expected chamfered rectangle, got {other:?}"),
    }
    assert_eq!(s.doc().unwrap().model.len(), 2);
}
