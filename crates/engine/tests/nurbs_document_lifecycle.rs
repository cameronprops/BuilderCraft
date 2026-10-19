//! Independent document-level NURBS split/trim acceptance with save, undo and rollback.
use cadcraft_doc::organization::Shape;
use cadcraft_engine::Session;
use cadcraft_geom::{Vec3, nurbs3d::Curve};
use serde_json::json;
use std::sync::Arc;

fn original_curve() -> Curve {
    Curve {
        degree: 2,
        control: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 4.0, 0.0), Vec3::new(4.0, 0.0, 0.0)],
        weights: vec![1.0, 0.5, 1.0],
        knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
    }
}

fn create_curve(s: &mut Session) -> u64 {
    let created = s.execute("nurbs.curve3d", &json!({"name":"Fair Centerline","curve":original_curve()})).unwrap();
    created["id"].as_u64().unwrap()
}

fn curve(s: &Session, id: u64) -> Arc<Curve> {
    let object = s.doc().unwrap().geometry3d.iter().find(|o| o.id == id).unwrap();
    match &object.shape {
        Shape::Curve(c) => c.clone(),
        other => panic!("expected document-owned curve, got {other:?}"),
    }
}

#[test]
fn split_preserves_rational_shape_ids_and_source_across_undo_redo_and_dftba() {
    let mut s = Session::new();
    let original_id = create_curve(&mut s);
    let original = curve(&s, original_id);
    let split = s.execute("nurbs.split_object", &json!({"id":original_id,"t":0.35})).unwrap();
    let ids = split["ids"].as_array().unwrap();
    assert_eq!(ids.len(), 2);
    let left = ids[0].as_u64().unwrap();
    let right = ids[1].as_u64().unwrap();
    assert_ne!(left, right);
    assert_ne!(left, original_id);
    assert_ne!(right, original_id);
    assert_eq!(s.doc().unwrap().geometry3d.len(), 3);
    assert_eq!(curve(&s, original_id), original);
    assert_eq!(split["representation"], "rational_nurbs");
    assert_eq!(split["source_preserved"], true);
    for i in 0..=20 {
        let t = i as f64 / 20.0;
        let (piece, local) = if t <= 0.35 { (left, t / 0.35) } else { (right, (t - 0.35) / 0.65) };
        let expected = original.evaluate(t).unwrap();
        let actual = curve(&s, piece).evaluate(local).unwrap();
        assert!((actual - expected).len() < 1e-8, "split rational curve changed at {t}");
    }

    let archive = cadcraft_io::write(s.doc().unwrap(), "sample.dftba").unwrap();
    let restored = cadcraft_io::read(&archive, "sample.dftba").unwrap();
    assert_eq!(restored.geometry3d, s.doc().unwrap().geometry3d);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().geometry3d.len(), 1);
    assert_eq!(curve(&s, original_id), original);
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().geometry3d.len(), 3);
    assert_eq!(s.doc().unwrap().geometry3d, restored.geometry3d);
}

#[test]
fn trim_copy_then_in_place_retains_stable_identity_and_undo_restores_original() {
    let mut s = Session::new();
    let id = create_curve(&mut s);
    let original = curve(&s, id);
    let copied = s.execute("nurbs.trim_object", &json!({"id":id,"start":0.2,"end":0.8,"copy":true})).unwrap();
    let copy_id = copied["id"].as_u64().unwrap();
    assert_ne!(copy_id, id);
    assert_eq!(s.doc().unwrap().geometry3d.len(), 2);
    assert_eq!(curve(&s, id), original);
    for i in 0..=10 {
        let t = i as f64 / 10.0;
        let expected = original.evaluate(0.2 + t * 0.6).unwrap();
        assert!((curve(&s, copy_id).evaluate(t).unwrap() - expected).len() < 1e-8);
    }
    let source_name = s.doc().unwrap().geometry3d.iter().find(|o| o.id == id).unwrap().name.clone();
    let edited = s.execute("nurbs.trim_object", &json!({"id":id,"start":0.1,"end":0.9,"copy":false})).unwrap();
    assert_eq!(edited["id"], id);
    assert_ne!(curve(&s, id), original);
    assert_eq!(s.doc().unwrap().geometry3d.iter().find(|o| o.id == id).unwrap().name, source_name);
    let archive = cadcraft_io::write(s.doc().unwrap(), "trim.dftba").unwrap();
    let restored = cadcraft_io::read(&archive, "trim.dftba").unwrap();
    assert_eq!(restored.geometry3d, s.doc().unwrap().geometry3d);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(curve(&s, id), original);
    assert_eq!(s.doc().unwrap().geometry3d.len(), 2);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().geometry3d.len(), 1);
    assert_eq!(curve(&s, id), original);
}

#[test]
fn invalid_nurbs_edit_requests_preserve_document_identity_revision_and_undo() {
    let mut s = Session::new();
    let id = create_curve(&mut s);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();
    let invalid = [
        ("nurbs.split_object", json!({"id":id,"t":0.0})),
        ("nurbs.split_object", json!({"id":id,"t":1.0})),
        ("nurbs.split_object", json!({"id":id,"t":null})),
        ("nurbs.split_object", json!({"id":id,"t":"bad"})),
        ("nurbs.split_object", json!({"id":id,"t":0.3,"selected_revision":revision+1})),
        ("nurbs.split_object", json!({"id":0,"t":0.3})),
        ("nurbs.trim_object", json!({"id":id,"start":0.8,"end":0.2})),
        ("nurbs.trim_object", json!({"id":id,"start":0.0,"end":1.0,"copy":"wrong"})),
        ("nurbs.trim_object", json!({"id":id,"start":0.2,"end":0.8,"selected_revision":revision+1})),
        ("nurbs.trim_object", json!({"id":0,"start":0.2,"end":0.8})),
    ];
    for (command, params) in invalid {
        assert!(s.execute(command, &params).is_err(), "{command} accepted {params}");
        assert!(Arc::ptr_eq(&before, &s.state().unwrap().doc), "invalid request copied/mutated the drawing");
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }
    assert_eq!(s.doc().unwrap().geometry3d.len(), 1);
}
