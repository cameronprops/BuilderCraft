//! Model organization must bridge native 3D geometry, selection, visibility and undo.
use cadcraft_engine::Session;
use serde_json::json;
use std::sync::Arc;

fn source_curve(s: &mut Session) -> u64 {
    s.execute(
        "nurbs.curve3d",
        &json!({
            "name":"Vehicle Guide",
            "curve":{
                "degree":1,
                "control":[{"x":0.0,"y":0.0,"z":0.0},{"x":10.0,"y":0.0,"z":0.0}],
                "weights":[1.0,1.0],
                "knots":[0.0,0.0,1.0,1.0]
            }
        }),
    )
    .unwrap()["id"]
        .as_u64()
        .unwrap()
}

fn make_hierarchy(s: &mut Session, geometry_id: u64) -> (u64, u64, u64) {
    let assembly = s.execute("model.create", &json!({"name":"Ride System","kind":"assembly"})).unwrap()["id"].as_u64().unwrap();
    let component = s.execute("model.create", &json!({"name":"Vehicle","kind":"component","parent":assembly})).unwrap()["id"].as_u64().unwrap();
    let body = s
        .execute("model.create", &json!({"name":"Guide Body","kind":"body","parent":component,"handles":[format!("{geometry_id:x}")]}))
        .unwrap()["id"]
        .as_u64()
        .unwrap();
    (assembly, component, body)
}

#[test]
fn model_hierarchy_references_a_native_curve_and_round_trips() {
    let mut s = Session::new();
    let curve_id = source_curve(&mut s);
    let (assembly, component, body) = make_hierarchy(&mut s, curve_id);
    let list = s.execute("model.list", &json!({})).unwrap();
    assert_eq!(list["nodes"].as_array().unwrap().len(), 3);
    let nodes = &s.doc().unwrap().organization.nodes;
    assert_eq!(nodes.iter().find(|n| n.id == component).unwrap().parent, Some(assembly));
    let body_node = nodes.iter().find(|n| n.id == body).unwrap();
    assert_eq!(body_node.parent, Some(component));
    assert_eq!(body_node.entities, vec![cadcraft_doc::Handle(curve_id)]);

    let bytes = cadcraft_io::write(s.doc().unwrap(), "ride.dftba").unwrap();
    let restored = cadcraft_io::read(&bytes, "ride.dftba").unwrap();
    assert_eq!(restored.organization.nodes, s.doc().unwrap().organization.nodes);
    assert_eq!(restored.geometry3d, s.doc().unwrap().geometry3d);
    let chosen = s.execute("model.select", &json!({"id":assembly})).unwrap();
    assert_eq!(chosen["count"], 1);
    assert_eq!(s.selection(), vec![cadcraft_doc::Handle(curve_id)]);
}

#[test]
fn assembly_visibility_and_rename_apply_to_owned_geometry_and_undo() {
    let mut s = Session::new();
    let curve_id = source_curve(&mut s);
    let (assembly, _, _) = make_hierarchy(&mut s, curve_id);
    assert!(s.doc().unwrap().geometry3d.iter().find(|g| g.id == curve_id).unwrap().visible);
    let visible = s.execute("model.visible", &json!({"id":assembly,"visible":false})).unwrap();
    assert_eq!(visible["count"], 1);
    assert!(!s.doc().unwrap().geometry3d.iter().find(|g| g.id == curve_id).unwrap().visible);
    s.execute("undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().geometry3d.iter().find(|g| g.id == curve_id).unwrap().visible);

    let renamed = s.execute("model.rename", &json!({"id":assembly,"name":"Ride System Rev B"})).unwrap();
    assert_eq!(renamed["id"], assembly);
    assert_eq!(s.doc().unwrap().organization.nodes.iter().find(|n| n.id == assembly).unwrap().name, "Ride System Rev B");
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().organization.nodes.iter().find(|n| n.id == assembly).unwrap().name, "Ride System");
}

#[test]
fn invalid_hierarchy_edits_are_atomic_and_cannot_double_own_geometry() {
    let mut s = Session::new();
    let curve_id = source_curve(&mut s);
    let (assembly, component, body) = make_hierarchy(&mut s, curve_id);
    let before = s.state().unwrap().doc.clone();
    let rev = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();
    let invalid = [
        ("model.create", json!({"name":"Orphan","kind":"body","parent":999999999})),
        ("model.create", json!({"name":"Illegal Child","kind":"component","parent":body})),
        ("model.create", json!({"name":"Illegal Assembly","kind":"assembly","parent":component})),
        ("model.create", json!({"name":"Second Owner","kind":"body","parent":component,"handles":[format!("{curve_id:x}")]})),
        ("model.rename", json!({"id":assembly,"name":""})),
        ("model.visible", json!({"id":assembly,"visible":"not boolean"})),
        ("model.select", json!({"id":999999999})),
    ];
    for (command, input) in invalid {
        assert!(s.execute(command, &input).is_err(), "accepted {command} with {input}");
        assert!(Arc::ptr_eq(&before, &s.state().unwrap().doc));
        assert_eq!(s.state().unwrap().revision, rev);
        assert_eq!(s.state().unwrap().undo.len(), undo);
    }
}
