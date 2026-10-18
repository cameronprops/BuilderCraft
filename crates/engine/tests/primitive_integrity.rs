//! Validated primitive construction must not silently discard bad inputs.
use cadcraft_doc::EntityKind;
use cadcraft_engine::Session;
use serde_json::json;

#[test]
fn malformed_polyline_vertex_rejects_entire_batch() {
    let mut s = Session::new();
    s.execute("line", &json!({"points": [[0, 0], [1, 0]]})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo_len = s.state().unwrap().undo.len();

    for params in [
        json!({"vertices": [[0, 0], "bad", [2, 0]]}),
        json!({"vertices": [[0, 0], {"p": [1, 1], "bulge": "bad"}, [2, 0]]}),
        json!({"vertices": [[0, 0], {"p": [1, 1], "startWidth": -1e308, "endWidth": "bad"}, [2, 0]]}),
        json!({"vertices": [[0, 0], [0, 0], [2, 0]]}),
        json!({"vertices": [[0, 0], [1, 1]], "width": -1.0}),
        json!({"vertices": [[0, 0], [1, 1]], "width": "invalid"}),
    ] {
        assert!(s.execute("pline", &params).is_err(), "accepted malformed polyline {params}");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo_len);
    }

    s.execute("pline", &json!({"vertices": [[0, 0], {"p": [1, 1], "bulge": 0.25}, [2, 0]], "width": 0.5})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let entity = s.doc().unwrap().model.iter().last().unwrap();
    match &entity.kind {
        EntityKind::LwPolyline(p) => {
            assert_eq!(p.vertices.len(), 3);
            assert_eq!(p.vertices[1].bulge, 0.25);
            assert_eq!(p.const_width, 0.5);
        }
        other => panic!("expected polyline, got {other:?}"),
    }
}

#[test]
fn zero_length_line_and_negative_circle_dimensions_are_not_persisted() {
    let mut s = Session::new();
    let before = s.state().unwrap().doc.clone();
    for (id, params) in [
        ("line", json!({"points": [[0, 0], [1, 1], [1, 1]]})),
        ("circle", json!({"center": [0, 0], "radius": -3})),
        ("circle", json!({"center": [0, 0], "diameter": -6})),
    ] {
        assert!(s.execute(id, &params).is_err(), "{id} accepted degenerate geometry");
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    }
    assert!(s.doc().unwrap().model.is_empty());

    s.execute("line", &json!({"points": [[0, 0], [4, 0]]})).unwrap();
    s.execute("circle", &json!({"center": [0, 0], "radius": 3})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 2);
}
