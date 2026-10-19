//! Explicit CAD spline degrees must not be silently clamped.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn invalid_explicit_spline_degree_is_atomic_for_both_command_forms() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();
    for cmd in ["spline", "spline.cv"] {
        for degree in [json!(0), json!(4), json!(11), json!(-1), json!(2.5), json!("3"), json!(null)] {
            let params = json!({"control":[[0,0],[1,2],[2,1],[3,0]],"degree":degree});
            let err = s.execute(cmd, &params).expect_err("invalid explicit spline degree should not silently clamp");
            assert!(err.to_string().contains("degree"), "unexpected error for {cmd}: {err}");
            assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
            assert_eq!(s.state().unwrap().revision, revision);
            assert_eq!(s.state().unwrap().undo.len(), undo_len);
        }
    }
}

#[test]
fn valid_spline_degree_and_short_control_defaults_are_preserved() {
    let mut s = Session::new();
    for cmd in ["spline", "spline.cv"] {
        let result = s.execute(cmd, &json!({"control":[[0,0],[1,2],[2,1],[3,0]],"degree":2})).unwrap();
        let handle = result["handle"].as_str().expect("command must return a handle");
        let entity = s.doc().unwrap().model.iter().find(|e| e.handle.hex() == handle).expect("created entity must exist");
        match &entity.kind {
            EntityKind::Spline(sp) => {
                assert_eq!(sp.degree, 2);
                assert_eq!(sp.control.len(), 4);
                assert!(sp.is_valid());
            }
            other => panic!("expected a valid spline, got {other:?}"),
        }
    }
    for cmd in ["spline", "spline.cv"] {
        let result = s.execute(cmd, &json!({"control":[[0,0],[5,0]]})).unwrap();
        let handle = result["handle"].as_str().unwrap();
        let entity = s.doc().unwrap().model.iter().find(|e| e.handle.hex() == handle).unwrap();
        assert!(matches!(&entity.kind, EntityKind::Spline(sp) if sp.degree == 1 && sp.is_valid()));
    }
}
