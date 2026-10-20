//! Shared Curve Rebuild acceptance: real NURBS, numeric shape, bounded failure.
//! This tests the existing control-fitting operation, not Rhino interpolation parity.
use cadcraft_engine::Session;
use cadcraft_geom::{Vec3, nurbs3d::Curve};
use serde_json::{Value, json};

fn inputs(controls: usize, degree: usize) -> Value {
    json!({
        "points":{"kind":"polyline","value":[
            {"x":0.0,"y":0.0,"z":0.0},
            {"x":3.0,"y":0.0,"z":0.0},
            {"x":6.0,"y":0.0,"z":0.0}
        ]},
        "controls":{"kind":"count","value":controls},
        "degree":{"kind":"count","value":degree}
    })
}

#[test]
fn rebuild_matches_straight_rail_and_shared_orbweaver_operation() {
    let mut session = Session::new();
    let initial_revision = session.state().unwrap().revision;
    for (controls, degree) in [(2,1),(4,2),(6,3),(8,5)] {
        let ports = inputs(controls, degree);
        let direct = session.execute("worldwright.curve.rebuild", &json!({"inputs":ports})).unwrap();
        let kernel = session.execute("worldwright.tool.run", &json!({
            "operation":"kernel.curve.rebuild","inputs":ports
        })).unwrap();
        let graph = session.execute("worldwright.tool.run", &json!({
            "operation":"orbweaver.curve.rebuild","inputs":ports
        })).unwrap();
        assert_eq!(direct["output"],kernel["output"]);
        assert_eq!(direct["output"],graph["output"]);
        assert_eq!(direct["output"]["kind"],"curve");
        let curve: Curve = serde_json::from_value(direct["output"]["value"].clone()).unwrap();
        assert!(curve.valid());
        assert_eq!(curve.degree,degree);
        assert_eq!(curve.control.len(),controls);
        for (i,p) in curve.control.iter().enumerate() {
            let expected_x=6.0*(i as f64)/((controls-1) as f64);
            assert!((p.x-expected_x).abs()<1e-9);
            assert!(p.y.abs()<1e-9 && p.z.abs()<1e-9);
        }
        for i in 0..=20 {
            let p=curve.evaluate((i as f64)/20.0).unwrap();
            assert!(p.y.abs()<1e-9 && p.z.abs()<1e-9);
            assert!((-1e-9..=6.0+1e-9).contains(&p.x));
        }
    }
    assert_eq!(session.state().unwrap().revision,initial_revision);
    assert!(session.doc().unwrap().model.is_empty());
}

#[test]
fn rebuild_rejects_invalid_control_degree_and_duplicate_stations_atomically() {
    let mut session=Session::new();
    let initial_revision=session.state().unwrap().revision;
    for (controls,degree) in [(1,1),(3,3),(8,6),(4097,3),(2,0)] {
        assert!(session.execute("worldwright.curve.rebuild",&json!({"inputs":inputs(controls,degree)})).is_err());
    }
    let mut duplicate=inputs(4,2);
    duplicate["points"]["value"][1]=json!({"x":0.0,"y":0.0,"z":0.0});
    assert!(session.execute("worldwright.curve.rebuild",&json!({"inputs":duplicate})).is_err());
    assert_eq!(session.state().unwrap().revision,initial_revision);
    assert!(session.doc().unwrap().model.is_empty());
}
