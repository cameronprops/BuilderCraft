//! Exact curve split/trim: CAD and Orb Weaver share one NURBS operation.
use cadcraft_engine::Session;
use cadcraft_geom::{Vec3, nurbs3d::Curve};
use serde_json::{Value, json};

fn source() -> Curve {
    Curve::from_control(
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 2.0, 0.0),
            Vec3::new(3.0, 2.0, 1.0),
            Vec3::new(4.0, 0.0, 1.0),
        ],
        3,
    )
    .unwrap()
}

fn execute_three(session: &mut Session, cad: &str, kernel: &str, graph: &str, inputs: &Value) -> Value {
    let direct = session.execute(cad, &json!({"inputs":inputs})).unwrap();
    let generic = session.execute("worldwright.tool.run", &json!({"operation":kernel,"inputs":inputs})).unwrap();
    let node = session.execute("worldwright.tool.run", &json!({"operation":graph,"inputs":inputs})).unwrap();
    assert_eq!(direct["output"], generic["output"]);
    assert_eq!(direct["output"], node["output"]);
    direct["output"].clone()
}

fn near(a: Vec3, b: Vec3) {
    assert!((a - b).len() < 1e-8, "expected {b:?}, got {a:?}");
}

#[test]
fn split_preserves_exact_source_curve_on_both_subdomains() {
    let curve = source();
    let original = curve.clone();
    let mut session = Session::new();
    let revision = session.state().unwrap().revision;
    for parameter in [0.2, 0.5, 0.8] {
        let inputs = json!({
            "curve":{"kind":"curve","value":curve},
            "t":{"kind":"number","value":parameter}
        });
        let output = execute_three(&mut session, "worldwright.nurbs.split", "kernel.nurbs.split", "orbweaver.nurbs.split", &inputs);
        assert_eq!(output["kind"], "tree");
        let branches = output["value"]["branches"].as_array().unwrap();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0]["path"], json!([0]));
        assert_eq!(branches[1]["path"], json!([1]));
        let left: Curve = serde_json::from_value(branches[0]["items"][0]["value"].clone()).unwrap();
        let right: Curve = serde_json::from_value(branches[1]["items"][0]["value"].clone()).unwrap();
        assert!(left.valid() && right.valid());
        for i in 0..=10 {
            let u = f64::from(i) / 10.0;
            near(left.evaluate(u).unwrap(), curve.evaluate(u * parameter).unwrap());
            near(right.evaluate(u).unwrap(), curve.evaluate(parameter + u * (1.0 - parameter)).unwrap());
        }
    }
    assert_eq!(curve, original);
    assert_eq!(session.state().unwrap().revision, revision);
    assert!(session.doc().unwrap().model.is_empty());
}

#[test]
fn trim_preserves_exact_interior_curve_and_rejects_invalid_ranges() {
    let curve = source();
    let mut session = Session::new();
    let revision = session.state().unwrap().revision;
    for (start, end) in [(0.1, 0.9), (0.25, 0.75), (0.0, 0.5)] {
        let inputs = json!({
            "curve":{"kind":"curve","value":curve},
            "start":{"kind":"number","value":start},
            "end":{"kind":"number","value":end}
        });
        let output = execute_three(&mut session, "worldwright.nurbs.trim", "kernel.nurbs.trim", "orbweaver.nurbs.trim", &inputs);
        assert_eq!(output["kind"], "curve");
        let trimmed: Curve = serde_json::from_value(output["value"].clone()).unwrap();
        assert!(trimmed.valid());
        for i in 0..=10 {
            let u = f64::from(i) / 10.0;
            near(trimmed.evaluate(u).unwrap(), curve.evaluate(start + u * (end - start)).unwrap());
        }
    }
    for (start, end) in [(0.8, 0.2), (-0.1, 0.5), (0.4, 1.1), (0.5, 0.5)] {
        let bad = json!({
            "curve":{"kind":"curve","value":curve},
            "start":{"kind":"number","value":start},
            "end":{"kind":"number","value":end}
        });
        assert!(session.execute("worldwright.nurbs.trim", &json!({"inputs":bad})).is_err());
    }
    assert_eq!(session.state().unwrap().revision, revision);
    assert!(session.doc().unwrap().model.is_empty());
}
