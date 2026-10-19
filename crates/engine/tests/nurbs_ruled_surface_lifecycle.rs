//! Native rational ruled surfaces keep editable source rails, exact geometry and undo.
use cadcraft_doc::organization::Shape;
use cadcraft_engine::Session;
use cadcraft_geom::{Vec3, nurbs3d::{Curve, Surface}};
use serde_json::json;
use std::sync::Arc;

fn rail(z: f64) -> Curve {
    Curve {
        degree: 2,
        control: vec![Vec3::new(1.0, 0.0, z), Vec3::new(1.0, 1.0, z), Vec3::new(0.0, 1.0, z)],
        weights: vec![1.0, 0.5_f64.sqrt(), 1.0],
        knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
    }
}

fn curve_object(s: &mut Session, name: &str, curve: Curve) -> u64 {
    s.execute("nurbs.curve3d", &json!({"name":name,"curve":curve})).unwrap()["id"].as_u64().unwrap()
}

fn read_surface(s: &Session, id: u64) -> Arc<Surface> {
    let object = s.doc().unwrap().geometry3d.iter().find(|object| object.id == id).unwrap();
    match &object.shape {
        Shape::Surface(surface) => surface.clone(),
        other => panic!("expected editable rational NURBS surface, got {other:?}"),
    }
}

#[test]
fn ruled_surface_follows_exact_rational_rails_and_survives_dftba_undo_redo() {
    let mut s = Session::new();
    let bottom = curve_object(&mut s, "Lower Rail", rail(0.0));
    let top = curve_object(&mut s, "Upper Rail", rail(10.0));
    let result = s.execute(
        "nurbs.surface.ruled",
        &json!({"name":"Ruled Gallery Wall","a_id":bottom,"b_id":top}),
    ).unwrap();
    let id = result["id"].as_u64().unwrap();
    let surface = read_surface(&s, id);
    assert!(surface.valid());
    assert_eq!(surface.degree_v, 1);
    assert_eq!(surface.rows, vec![rail(0.0), rail(10.0)]);
    assert_eq!(s.doc().unwrap().geometry3d.len(), 3);
    for &u in &[0.0, 0.125, 0.5, 0.875, 1.0] {
        for &v in &[0.0, 0.3, 0.5, 1.0] {
            let p = surface.evaluate(u, v).unwrap();
            assert!((p.x * p.x + p.y * p.y - 1.0).abs() < 1e-12);
            assert!((p.z - 10.0 * v).abs() < 1e-12);
        }
        assert!((surface.evaluate(u, 0.0).unwrap() - rail(0.0).evaluate(u).unwrap()).len() < 1e-12);
        assert!((surface.evaluate(u, 1.0).unwrap() - rail(10.0).evaluate(u).unwrap()).len() < 1e-12);
    }

    let bytes = cadcraft_io::write(s.doc().unwrap(), "gallery.dftba").unwrap();
    let reopened = cadcraft_io::read(&bytes, "gallery.dftba").unwrap();
    assert_eq!(reopened.geometry3d, s.doc().unwrap().geometry3d);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().geometry3d.len(), 2);
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().geometry3d.len(), 3);
    assert_eq!(read_surface(&s, id), surface);
}

#[test]
fn direct_surface_construction_and_controlpoint_edit_remain_undoable() {
    let mut s = Session::new();
    let source = Surface {
        rows: vec![rail(0.0), rail(10.0)],
        degree_v: 1,
        knots_v: vec![0.0, 0.0, 1.0, 1.0],
    };
    let id = s.execute("nurbs.surface", &json!({"name":"Custom Wall","surface":source})).unwrap()["id"].as_u64().unwrap();
    assert_eq!(*read_surface(&s, id), source);
    let original_mid = read_surface(&s, id).evaluate(0.5, 1.0).unwrap();
    let output = s.execute(
        "geometry3d.controlpoint",
        &json!({"id":id,"row":1,"index":1,"point":[1.0,1.0,12.0]}),
    ).unwrap();
    assert_eq!(output["id"], id);
    let edited = read_surface(&s, id);
    assert!(edited.valid());
    assert_eq!(edited.rows[1].control[1], Vec3::new(1.0, 1.0, 12.0));
    assert_ne!(edited.evaluate(0.5, 1.0).unwrap(), original_mid);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(*read_surface(&s, id), source);
}

#[test]
fn ruled_surface_rejects_incompatible_rails_and_stale_revision_atomically() {
    let mut s = Session::new();
    let a = curve_object(&mut s, "Rail A", rail(0.0));
    let b = curve_object(&mut s, "Rail B", rail(10.0));
    let incompatible = curve_object(
        &mut s,
        "Incompatible",
        Curve {
            degree: 1,
            control: vec![Vec3::ZERO, Vec3::new(2.0, 0.0, 3.0)],
            weights: vec![1.0, 1.0],
            knots: vec![0.0, 0.0, 1.0, 1.0],
        },
    );
    let revision = s.state().unwrap().revision;
    let before = s.state().unwrap().doc.clone();
    let undo = s.state().unwrap().undo.len();
    let requests = [
        ("nurbs.surface.ruled", json!({"name":"Bad","a_id":a,"b_id":a})),
        ("nurbs.surface.ruled", json!({"name":"Bad","a_id":a,"b_id":9999999})),
        ("nurbs.surface.ruled", json!({"name":"Bad","a_id":a,"b_id":incompatible})),
        ("nurbs.surface.ruled", json!({"name":"Bad","a_id":a,"b_id":b,"selected_revision":revision+1})),
        ("nurbs.surface", json!({"name":"Bad","surface":{"rows":[rail(0.0),rail(10.0)],"degree_v":2,"knots_v":[0.0,0.0,1.0,1.0]}})),
    ];
    for (command, request) in requests {
        assert!(s.execute(command, &request).is_err(), "{command} accepted invalid parameters {request}");
        assert!(Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }
    assert_eq!(s.doc().unwrap().geometry3d.len(), 3);
}
