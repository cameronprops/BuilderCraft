//! CAD and OrbWeaver parity for geometric G2 exact NURBS blend.
use cadcraft_engine::Session;
use cadcraft_geom::{Vec3, nurbs3d::Curve};
use serde_json::{Value, json};

fn rails() -> (Curve, Curve) {
    let a = Curve::from_control(vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)], 2).unwrap();
    let b = Curve::from_control(vec![Vec3::new(4.0, 2.0, 0.0), Vec3::new(5.0, 3.0, 0.0), Vec3::new(6.0, 3.0, 0.0)], 2).unwrap();
    (a, b)
}

fn typed(a: &Curve, b: &Curve, reverse_a: usize, reverse_b: usize) -> Value {
    json!({
        "a": {"kind": "curve", "value": a},
        "b": {"kind": "curve", "value": b},
        "continuity": {"kind": "count", "value": 2},
        "strength_a": {"kind": "number", "value": 0.3},
        "strength_b": {"kind": "number", "value": 0.4},
        "reverse_a": {"kind": "count", "value": reverse_a},
        "reverse_b": {"kind": "count", "value": reverse_b}
    })
}

fn close(a: Vec3, b: Vec3) {
    assert!((a - b).len() < 1e-8, "expected {b:?}, got {a:?}");
}

#[test]
fn g2_matches_curvature_through_cad_kernel_and_orbweaver() {
    let (a, b) = rails();
    let mut session = Session::new();
    let revision = session.state().unwrap().revision;
    for (reverse_a, reverse_b) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let inputs = typed(&a, &b, reverse_a, reverse_b);
        let direct = session.execute("worldwright.curve.blend", &json!({"inputs": inputs})).unwrap();
        let generic = session.execute("worldwright.tool.run", &json!({"operation": "kernel.curve.blend", "inputs": inputs})).unwrap();
        let graph = session.execute("worldwright.tool.run", &json!({"operation": "orbweaver.curve.blend", "inputs": inputs})).unwrap();
        assert_eq!(direct["output"], generic["output"]);
        assert_eq!(direct["output"], graph["output"]);
        assert_eq!(direct["output"]["kind"], "curve");
        let curve: Curve = serde_json::from_value(direct["output"]["value"].clone()).unwrap();
        assert_eq!(curve.degree, 5);
        let ta = if reverse_a == 0 { 1.0 } else { 0.0 };
        let tb = if reverse_b == 0 { 0.0 } else { 1.0 };
        close(curve.evaluate(0.0).unwrap(), a.evaluate(ta).unwrap());
        close(curve.evaluate(1.0).unwrap(), b.evaluate(tb).unwrap());
        let tangent_a = a.tangent(ta).unwrap() * if reverse_a == 0 { 1.0 } else { -1.0 };
        let tangent_b = b.tangent(tb).unwrap() * if reverse_b == 0 { 1.0 } else { -1.0 };
        assert!(curve.tangent(0.0).unwrap().dot(tangent_a) > 1.0 - 1e-9);
        assert!(curve.tangent(1.0).unwrap().dot(tangent_b) > 1.0 - 1e-9);
        close(curve.curvature_vector(0.0).unwrap(), a.curvature_vector(ta).unwrap());
        close(curve.curvature_vector(1.0).unwrap(), b.curvature_vector(tb).unwrap());
    }
    assert_eq!(session.state().unwrap().revision, revision);
    assert!(session.doc().unwrap().model.is_empty());
}

#[test]
fn g2_rejects_unsupported_modes_and_stationary_source_without_mutation() {
    let (a, b) = rails();
    let mut session = Session::new();
    let before = cadcraft_io::write(session.doc().unwrap(), "g2.dftba").unwrap();
    let mut inputs = typed(&a, &b, 0, 0);
    inputs["continuity"]["value"] = json!(3);
    assert!(session.execute("worldwright.curve.blend", &json!({"inputs":inputs})).is_err());
    inputs["continuity"]["value"] = json!(2);
    inputs["strength_a"]["value"] = json!(0.0);
    assert!(session.execute("worldwright.curve.blend", &json!({"inputs":inputs})).is_err());
    inputs["strength_a"]["value"] = json!(0.3);
    inputs["reverse_b"]["value"] = json!(2);
    assert!(session.execute("worldwright.curve.blend", &json!({"inputs":inputs})).is_err());
    inputs["reverse_b"]["value"] = json!(0);
    let stationary = Curve::from_control(vec![Vec3::ZERO, Vec3::ZERO, Vec3::ZERO], 2).unwrap();
    inputs["a"]["value"] = serde_json::to_value(stationary).unwrap();
    assert!(session.execute("worldwright.curve.blend", &json!({"inputs":inputs})).is_err());
    assert!(session.doc().unwrap().model.is_empty());
    assert_eq!(cadcraft_io::write(session.doc().unwrap(), "g2.dftba").unwrap(), before);
    assert_eq!(session.state().unwrap().revision, 0);
}
