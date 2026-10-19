//! Reject impossible exact-solid primitives before launching a native OCCT worker.
use cadcraft_engine::Session;
use serde_json::json;
use std::sync::Arc;

fn assert_rejected_without_mutation(s: &mut Session, command: &str, input: serde_json::Value) {
    let before = s.state().unwrap().doc.clone();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();
    assert!(s.execute(command, &input).is_err(), "{command} accepted {input}");
    assert!(Arc::ptr_eq(&before, &s.state().unwrap().doc));
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo);
    assert_eq!(s.doc().unwrap().exact_breps.len(), 0);
}

#[test]
fn invalid_box_extents_are_rejected_with_axis_specific_diagnostics() {
    let mut s = Session::new();
    for (minimum, maximum, label) in [
        ([0, 0, 0], [0, 4, 5], "x"),
        ([0, 0, 0], [4, 0, 5], "y"),
        ([0, 0, 0], [4, 5, 0], "z"),
        ([4, 0, 0], [3, 4, 5], "x"),
        ([0, 4, 0], [4, 3, 5], "y"),
        ([0, 0, 4], [4, 5, 3], "z"),
    ] {
        let input = json!({"name":"Ride BRep","min":minimum,"max":maximum});
        let err = s.execute("brep.box", &input).unwrap_err();
        assert!(err.to_string().contains(&format!("box {label} extent must be positive")));
        assert_rejected_without_mutation(&mut s, "brep.box", input);
    }
    for input in [
        json!({"name":"Ride BRep","min":[0,0],"max":[4,5,6]}),
        json!({"name":"Ride BRep","min":[0,0,0],"max":[4,5,null]}),
        json!({"name":"Ride BRep","min":[0,0,0],"max":[4,5,6],"tolerance":0}),
    ] {
        assert_rejected_without_mutation(&mut s, "brep.box", input);
    }
}

#[test]
fn zero_height_and_malformed_cylinder_parameters_are_atomic_failures() {
    let mut s = Session::new();
    let zero = json!({"name":"Column","radius":3.0,"height":[0,0,0]});
    let err = s.execute("brep.cylinder", &zero).unwrap_err();
    assert!(err.to_string().contains("height vector must be nonzero"));
    for input in [
        zero,
        json!({"name":"Column","radius":3.0,"height":[0,0]}),
        json!({"name":"Column","radius":3.0,"height":[0,null,10]}),
        json!({"name":"Column","radius":-1.0,"height":[0,0,10]}),
        json!({"name":"Column","radius":3.0,"height":[0,0,10],"tolerance":-0.1}),
    ] {
        assert_rejected_without_mutation(&mut s, "brep.cylinder", input);
    }
}
