use cadcraft_engine::Session;
use serde_json::{Value, json};
fn query() -> Value {
    json!({"pixel":[200.,200.],"viewport":[400.,400.],"center":[0.,0.,0.],"yaw":0.,"pitch":-std::f64::consts::FRAC_PI_2,"scale":10.,"midpoints":false,
        "plane":{"origin":[0.,0.,5.],"x_axis":[1.,0.,0.],"y_axis":[0.,1.,0.]}})
}
#[test]
fn endpoints_override_plane_depth_and_respect_visibility() {
    let mut s = Session::new();
    let id = s.execute("nurbs.controlcurve3d", &json!({"points":[[0.,0.,20.],[10.,0.,20.]]})).unwrap()["id"].as_u64().unwrap();
    let revision = s.state().unwrap().revision;
    let mut q = query();
    let hit = s.execute("geometry3d.snap", &q).unwrap();
    assert_eq!(hit["source_id"], id);
    assert_eq!(hit["point"], json!([0., 0., 20.]));
    q["endpoints"] = json!(false);
    let point = s.execute("geometry3d.snap", &q).unwrap();
    assert!((point["point"][2].as_f64().unwrap() - 5.).abs() < 1e-10);
    assert_eq!(s.state().unwrap().revision, revision);
    q["endpoints"] = json!(true);
    s.execute("geometry3d.set", &json!({"id":id,"visible":false})).unwrap();
    assert!(s.execute("geometry3d.snap", &q).unwrap()["source_id"].is_null());
    s.execute("geometry3d.set", &json!({"id":id,"visible":true})).unwrap();
    s.doc_mut().unwrap().layer_mut("0").unwrap().locked = true;
    assert!(s.execute("geometry3d.snap", &q).unwrap()["source_id"].is_null());
    q["pitch"] = json!(0.);
    assert!(s.execute("geometry3d.snap", &q).is_err());
}
#[test]
fn curve_creation_is_one_transaction_with_stale_and_invalid_preflight() {
    let mut s = Session::new();
    let revision = s.state().unwrap().revision;
    let uid = s.state().unwrap().uid;
    let params = json!({"points":[[0.,0.,0.],[1.,2.,3.],[3.,4.,5.]],"expected_uid":uid,"expected_revision":revision});
    s.execute("nurbs.controlcurve3d", &params).unwrap();
    let shapes = s.doc().unwrap().geometry3d.clone();
    let restored = cadcraft_io::read(&cadcraft_io::write(s.doc().unwrap(), "draft.bcraft").unwrap(), "draft.bcraft").unwrap();
    assert_eq!(restored.geometry3d, shapes);
    assert!(s.execute("nurbs.controlcurve3d", &params).is_err());
    assert_eq!(s.doc().unwrap().geometry3d, shapes);
    s.undo().unwrap();
    assert!(s.doc().unwrap().geometry3d.is_empty());
    s.redo().unwrap();
    assert_eq!(s.doc().unwrap().geometry3d, shapes);
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    for bad_params in
        [json!({"points":[[0.,0.,0.]]}), json!({"points":[[0.,0.,0.],[1.,1.,1.]],"degree":5}), json!({"points":[[0.,0.,0.],[1e13,0.,0.]]})]
    {
        assert!(s.execute("nurbs.controlcurve3d", &bad_params).is_err());
        assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, revision);
    }
    let id = shapes[0].id;
    s.doc_mut().unwrap().handseed = id;
    let before = s.state().unwrap().doc.clone();
    assert!(s.execute("nurbs.controlcurve3d", &json!({"points":[[0.,0.,0.],[1.,1.,1.]]})).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
}

#[test]
fn drafted_curve_cannot_reuse_existing_mesh_identity() {
    let mut s = Session::new();
    let id = s.execute("mesh3d.create", &json!({"name":"Mesh", "mesh":{"vertices":[{"x":0.,"y":0.,"z":0.},{"x":1.,"y":0.,"z":0.},{"x":0.,"y":1.,"z":0.}],"faces":[{"triangle":[0,1,2]}]}})).unwrap()["id"].as_u64().unwrap();
    s.doc_mut().unwrap().handseed = id;
    let before = s.state().unwrap().doc.clone();
    assert!(s.execute("nurbs.controlcurve3d", &json!({"points":[[0.,0.,0.],[1.,1.,1.]]})).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &s.state().unwrap().doc));
}
