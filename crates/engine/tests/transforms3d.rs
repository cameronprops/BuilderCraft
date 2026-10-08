use cadcraft_engine::Session;
use serde_json::json;
fn object(s: &mut Session, x: f64) -> u64 {
    s.execute("nurbs.curve3d",&json!({"name":"Curve","curve":{"degree":1,"control":[{"x":x,"y":0.,"z":0.},{"x":x+1.,"y":0.,"z":0.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}})).unwrap()["id"].as_u64().unwrap()
}
#[test]
fn batch_transform_preserves_ids_and_undo_restores_source() {
    let mut s = Session::new();
    let a = object(&mut s, 0.);
    let b = object(&mut s, 10.);
    let original = s.state().unwrap().doc.clone();
    s.execute("geometry3d.transform", &json!({"ids":[a,b],"operation":{"kind":"move","delta":[1.,2.,3.]}})).unwrap();
    assert_eq!(s.doc().unwrap().geometry3d[0].id, a);
    let cadcraft_doc::organization::Shape::Curve(c) = &s.doc().unwrap().geometry3d[0].shape else { panic!() };
    assert_eq!(c.control[0], cadcraft_geom::Vec3::new(1., 2., 3.));
    s.undo().unwrap();
    assert!(std::sync::Arc::ptr_eq(&original, &s.state().unwrap().doc));
    s.redo().unwrap();
}
#[test]
fn failed_batches_leave_document_revision_and_identity_seed_unchanged() {
    let mut s = Session::new();
    let a = object(&mut s, 0.);
    let b = object(&mut s, 1e12 - 1.);
    for params in [
        json!({"ids":[a,b],"operation":{"kind":"move","delta":[2.,0.,0.]}}),
        json!({"ids":[a,999999],"operation":{"kind":"move","delta":[1.,0.,0.]}}),
        json!({"ids":[a,a],"operation":{"kind":"move","delta":[1.,0.,0.]}}),
        json!({"ids":[a],"operation":{"kind":"rotate","origin":[0.,0.,0.],"axis":[0.,0.,0.],"angle_degrees":90.}}),
        json!({"ids":[a],"operation":{"kind":"scale","origin":[0.,0.,0.],"factor":2.},"rigid":true}),
    ] {
        let before = s.state().unwrap().doc.clone();
        let revision = s.state().unwrap().revision;
        let seed = before.handseed;
        assert!(s.execute("geometry3d.transform", &params).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(s.doc().unwrap().handseed, seed);
    }
}
#[test]
fn copy_has_new_identity_and_does_not_change_original() {
    let mut s = Session::new();
    let a = object(&mut s, 0.);
    let original = s.doc().unwrap().geometry3d[0].clone();
    let out = s
        .execute("geometry3d.transform", &json!({"ids":[a],"operation":{"kind":"mirror","origin":[0.,0.,0.],"normal":[1.,0.,0.]},"copy":true}))
        .unwrap();
    assert_ne!(out["objects"][0]["id"], a);
    assert_eq!(s.doc().unwrap().geometry3d[0], original);
    assert_eq!(s.doc().unwrap().geometry3d.len(), 2);
    s.undo().unwrap();
    assert_eq!(s.doc().unwrap().geometry3d.len(), 1);
}
#[test]
fn copy_rejects_colliding_or_exhausted_identity_seed_before_mutation() {
    let mut s = Session::new();
    let id = object(&mut s, 0.);
    for seed in [id, u64::MAX] {
        s.doc_mut().unwrap().handseed = seed;
        let before = s.state().unwrap().doc.clone();
        let revision = s.state().unwrap().revision;
        assert!(s.execute("geometry3d.transform", &json!({"ids":[id],"operation":{"kind":"move","delta":[1.,0.,0.]},"copy":true})).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
    }
}

#[test]
fn directional_scale_copy_undo_and_invalid_axis_are_atomic() {
    let mut s = Session::new();
    let id = object(&mut s, 2.);
    let before = s.state().unwrap().doc.clone();
    for operation in [
        json!({"kind":"scale1d","origin":[1.,0.,0.],"axis":[1.,0.,0.],"factor":3.}),
        json!({"kind":"scale2d","origin":[1.,0.,0.],"normal":[0.,0.,1.],"factor":3.}),
        json!({"kind":"scale_nu","origin":[1.,0.,0.],"factors":[3.,2.,4.]}),
        json!({"kind":"scale_by_plane","origin":[1.,0.,0.],"x_axis":[1.,0.,0.],"y_axis":[0.,1.,0.],"factors":[3.,2.]}),
    ] {
        s.execute("geometry3d.transform", &json!({"ids":[id],"operation":operation,"copy":true})).unwrap();
        let cadcraft_doc::organization::Shape::Curve(c) = &s.doc().unwrap().geometry3d[1].shape else { panic!() };
        assert_eq!(c.control[0], cadcraft_geom::Vec3::new(4., 0., 0.));
        let bytes = cadcraft_io::write(s.doc().unwrap(), "scaled.bcraft").unwrap();
        let reopened = cadcraft_io::read(&bytes, "scaled.bcraft").unwrap();
        assert_eq!(reopened.geometry3d, s.doc().unwrap().geometry3d);
        s.undo().unwrap();
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
    }
    assert!(
        s.execute("geometry3d.transform", &json!({"ids":[id],"operation":{"kind":"scale1d","origin":[0.,0.,0.],"axis":[0.,0.,0.],"factor":2.}}))
            .is_err()
    );
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
}

#[test]
fn nonuniform_invalid_options_and_plane_preserve_document_revision() {
    let mut s = Session::new();
    let id = object(&mut s, 2.);
    for operation in [
        json!({"kind":"scale_by_plane","origin":[0.,0.,0.],"x_axis":[1.,0.,0.],"y_axis":[1.,1.,0.],"factors":[2.,3.]}),
        json!({"kind":"scale_nu","origin":[0.,0.,0.],"factors":[2.,-1.,3.]}),
        json!({"kind":"scale_nu","origin":[0.,0.,0.],"factors":[2.,1.,3.],"cplane":true}),
    ] {
        let before = s.state().unwrap().doc.clone();
        let revision = s.state().unwrap().revision;
        assert!(s.execute("geometry3d.transform", &json!({"ids":[id],"operation":operation,"copy":true})).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
    }
}
