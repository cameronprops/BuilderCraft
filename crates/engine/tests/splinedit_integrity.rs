//! SPLINEDIT must preserve protected geometry and honor explicit input constraints.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

fn make_spline(s: &mut Session) -> String {
    s.execute("spline.cv", &json!({"control":[[0,0],[1,2],[2,1],[3,0]]})).unwrap()["handle"].as_str().unwrap().to_string()
}

#[test]
fn splinedit_invalid_refit_and_precision_are_transactional() {
    let mut s = Session::new();
    let h = make_spline(&mut s);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();
    for params in [
        json!({"handle":h,"option":"polyline","precision":0}),
        json!({"handle":h,"option":"polyline","precision":100}),
        json!({"handle":h,"option":"polyline","precision":2.5}),
        json!({"handle":h,"option":"polyline","precision":null}),
        json!({"handle":h,"option":"refit","fit":[[0,0],"invalid",[2,0]]}),
        json!({"handle":h,"option":"refit","fit":{"not":"an array"}}),
    ] {
        assert!(s.execute("splinedit", &params).is_err(), "accepted invalid spline edit: {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_count);
    }
}

#[test]
fn splinedit_rejects_locked_source_without_mutating_it() {
    let mut s = Session::new();
    let h = make_spline(&mut s);
    s.execute("layer.new", &json!({"name":"Protected"})).unwrap();
    s.execute("laymch", &json!({"handles":[h],"layer":"Protected"})).unwrap();
    s.execute("layer.set", &json!({"name":"Protected","locked":true})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();
    for option in ["reverse", "close", "purge", "polyline"] {
        assert!(s.execute("splinedit", &json!({"handle":h,"option":option})).is_err(), "edited a locked spline");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_count);
    }
}

#[test]
fn splinedit_polyline_and_refit_produce_valid_geometry() {
    let mut s = Session::new();
    let h = make_spline(&mut s);
    let result = s.execute("splinedit", &json!({"handle":h,"option":"refit","fit":[[0,0],[1,3],[3,0]]})).unwrap();
    assert_eq!(result["handle"], h);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Spline(sp) => {
            assert_eq!(sp.fit.len(), 3);
            assert!(sp.is_valid());
        }
        other => panic!("expected refit spline, got {other:?}"),
    }

    let control_count = match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Spline(sp) => sp.control.len(),
        _ => unreachable!(),
    };
    let result = s.execute("splinedit", &json!({"handle":h,"option":"polyline","precision":4})).unwrap();
    assert_eq!(result["handle"], h);
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::LwPolyline(pl) => {
            assert_eq!(pl.vertices.len(), 4 * control_count.max(2) + 1);
            assert!(pl.vertices.iter().all(|v| v.p.is_finite()));
        }
        other => panic!("expected converted polyline, got {other:?}"),
    }
}
