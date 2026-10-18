//! 3D curve creation must validate every vertex and never persist overflow.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn invalid_3d_polyline_point_does_not_disappear_silently() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    for params in [
        json!({"points": [[0, 0, 0], [1, 2, 3], {"invalid": true}, [4, 5, 6]]}),
        json!({"points": [[0, 0, 0], [1, 2, "invalid"], [4, 5, 6]]}),
        json!({"points": [[0, 0, 0], [1, 2, 3, 4], [4, 5, 6]]}),
        json!({"points": [[0, 0, 0], [1, 2, 3], [1, 2, 3]]}),
    ] {
        assert!(s.execute("3dpoly", &params).is_err(), "accepted invalid 3D polyline {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
    }
    s.execute("3dpoly", &json!({"points": [[0, 0], [1, 2, 3], [4, 5, 6]]})).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Polyline3d(pl) => {
            assert_eq!(pl.points.len(), 3);
            assert_eq!(pl.points[0].z, 0.0);
            assert_eq!(pl.points[1].z, 3.0);
        }
        other => panic!("expected 3D polyline, got {other:?}"),
    }
}

#[test]
fn overflowed_helix_rejects_without_publishing_geometry() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    assert!(s.execute("helix", &json!({
        "center": [1.7e308, 0], "baseRadius": 1.7e308,
        "topRadius": 1.7e308, "height": 3, "turns": 1
    })).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    s.execute("helix", &json!({
        "center": [0, 0], "baseRadius": 2,
        "topRadius": 2, "height": 7, "turns": 1
    })).unwrap();
    match &s.doc().unwrap().model.iter().next().unwrap().kind {
        EntityKind::Polyline3d(pl) => {
            assert!(pl.points.len() >= 36);
            assert_eq!(pl.points.first().unwrap().z, 0.0);
            assert!((pl.points.last().unwrap().z - 7.0).abs() < 1e-9);
            assert!(pl.points.iter().all(|v| v.is_finite()));
        }
        other => panic!("expected helix polyline, got {other:?}"),
    }
}
