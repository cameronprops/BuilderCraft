//! Bounded five-gate candidates for read-only arc-length polyline commands.
use cadcraft_engine::{Session, command_specs, find_command};
use serde_json::{Value, json};
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Operation {
    Length,
    DivideCount,
    DivideDistance,
}

impl Operation {
    fn suffix(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::DivideCount => "divide_count",
            Self::DivideDistance => "divide_distance",
        }
    }

    fn inputs(self, offset: f64) -> Value {
        let mut inputs = json!({"points": path(offset)});
        match self {
            Self::Length => {}
            Self::DivideCount => inputs["count"] = json!({"kind":"count","value":2}),
            Self::DivideDistance => inputs["spacing"] = json!({"kind":"number","value":2.0}),
        }
        inputs
    }

    fn expected(self, offset: f64) -> Value {
        match self {
            Self::Length => json!(7.0),
            Self::DivideCount => json!([point(offset, 0.0), point(offset + 3.0, 0.5), point(offset + 3.0, 4.0)]),
            Self::DivideDistance => {
                json!([point(offset, 0.0), point(offset + 2.0, 0.0), point(offset + 3.0, 1.0), point(offset + 3.0, 3.0), point(offset + 3.0, 4.0)])
            }
        }
    }

    fn output_kind(self) -> &'static str {
        match self {
            Self::Length => "number",
            Self::DivideCount | Self::DivideDistance => "polyline",
        }
    }
}

const OPERATIONS: [Operation; 3] = [Operation::Length, Operation::DivideCount, Operation::DivideDistance];

fn point(x: f64, y: f64) -> Value {
    json!({"x":x,"y":y,"z":0.0})
}

fn path(offset: f64) -> Value {
    json!({"kind":"polyline","value":[
        point(offset, 0.0), point(offset + 3.0, 0.0), point(offset + 3.0, 4.0)
    ]})
}

fn assert_near(actual: &Value, expected: &Value, kind: &str) {
    if kind == "number" {
        let a = actual.as_f64().unwrap();
        let b = expected.as_f64().unwrap();
        assert!((a - b).abs() < 1e-10, "wrong polyline length {a} != {b}");
        return;
    }
    let points = actual.as_array().expect("polyline output array");
    let correct = expected.as_array().expect("expected point array");
    assert_eq!(points.len(), correct.len(), "wrong number of stations");
    for (a, b) in points.iter().zip(correct) {
        for axis in ["x", "y", "z"] {
            let av = a[axis].as_f64().unwrap();
            let bv = b[axis].as_f64().unwrap();
            assert!((av - bv).abs() < 1e-10, "station coordinate {axis}: {av} != {bv}");
        }
    }
}

fn assert_read_only(s: &Session, bytes: &[u8], doc: &Arc<cadcraft_doc::Drawing>, revision: u64, undo: usize, redo: usize) {
    assert!(Arc::ptr_eq(doc, &s.state().unwrap().doc), "command replaced the document");
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo);
    assert_eq!(s.state().unwrap().redo.len(), redo);
    assert!(s.selection().is_empty());
    assert_eq!(cadcraft_io::write(s.doc().unwrap(), "polyline.dftba").unwrap(), bytes);
}

fn verify(op: Operation) {
    let suffix = op.suffix();
    let cad = format!("worldwright.polyline.{suffix}");
    let kernel = format!("kernel.polyline.{suffix}");
    let node = format!("orbweaver.polyline.{suffix}");
    let mut s = Session::new();
    s.execute("line", &json!({"points":[[1.0,2.0],[4.0,6.0]]})).unwrap();
    let before = s.state().unwrap().doc.clone();
    let bytes = cadcraft_io::write(s.doc().unwrap(), "polyline.dftba").unwrap();
    let revision = s.state().unwrap().revision;
    let undo = s.state().unwrap().undo.len();
    let redo = s.state().unwrap().redo.len();

    // Translation leaves the analytic L-shaped path's arc length invariant,
    // but moves all division stations. Check numerical outputs and three APIs.
    for index in -32..=31 {
        let offset = f64::from(index) / 4.0;
        let ports = op.inputs(offset);
        let direct = s.execute(&cad, &json!({"inputs":ports})).unwrap();
        let shared = s.execute("worldwright.tool.run", &json!({"operation":kernel,"inputs":ports})).unwrap();
        let graph = s.execute("worldwright.tool.run", &json!({"operation":node,"inputs":ports})).unwrap();
        assert_eq!(direct["output"], shared["output"], "{cad} differs from kernel at {index}");
        assert_eq!(direct["output"], graph["output"], "{cad} differs from OrbWeaver at {index}");
        assert_eq!(direct["output"]["kind"], op.output_kind());
        assert_near(&direct["output"]["value"], &op.expected(offset), op.output_kind());
    }
    assert_read_only(&s, &bytes, &before, revision, undo, redo);

    // Independent invalid-port and boundary failures cannot alter drawing,
    // history, selection, or persisted bytes.
    let base = op.inputs(0.0);
    let mut invalid = Vec::new();
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("points");
    invalid.push(missing);
    let mut wrong = base.clone();
    wrong["points"]["kind"] = json!("point");
    invalid.push(wrong);
    let mut malformed = base.clone();
    malformed["points"]["value"][1]["x"] = json!("bad coordinate");
    invalid.push(malformed);
    let mut extra = base.clone();
    extra["unexpected"] = json!({"kind":"number","value":1.0});
    invalid.push(extra);
    let mut short = base.clone();
    short["points"]["value"] = json!([point(0.0, 0.0)]);
    invalid.push(short);
    match op {
        Operation::Length => {}
        Operation::DivideCount => {
            let mut zero = base.clone();
            zero["count"]["value"] = json!(0);
            invalid.push(zero);
            let mut too_large = base.clone();
            too_large["count"]["value"] = json!(100000);
            invalid.push(too_large);
            let mut coincident = base.clone();
            coincident["points"] = json!({"kind":"polyline","value":[point(0.0,0.0),point(0.0,0.0)]});
            invalid.push(coincident);
        }
        Operation::DivideDistance => {
            for spacing in [0.0, -1.0, 0.000001] {
                let mut bad = base.clone();
                bad["spacing"]["value"] = json!(spacing);
                invalid.push(bad);
            }
            let mut coincident = base.clone();
            coincident["points"] = json!({"kind":"polyline","value":[point(0.0,0.0),point(0.0,0.0)]});
            invalid.push(coincident);
        }
    }
    for ports in invalid {
        assert!(s.execute(&cad, &json!({"inputs":ports})).is_err(), "invalid {cad} ports accepted: {ports}");
        assert_read_only(&s, &bytes, &before, revision, undo, redo);
    }
    assert_eq!(cadcraft_io::read(&bytes, "polyline.dftba").unwrap().model.len(), 1);
    s.execute("undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().model.is_empty(), "read-only command added an undo record");
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1, "read-only command invalidated redo");
}

#[test]
fn polyline_ledger_candidates_are_unique_and_registered() {
    let manifest: Value = serde_json::from_str(include_str!("../../../docs/commands/certification-registry.json")).unwrap();
    let entries = manifest["candidates"].as_array().unwrap();
    let live: std::collections::HashSet<_> = command_specs().iter().map(|x| x.id).collect();
    assert_eq!(manifest["certified_count"], 3, "point certifications must not be silently promoted");
    for op in OPERATIONS {
        let command = format!("worldwright.polyline.{}", op.suffix());
        let entry = entries.iter().find(|e| e["command"] == command).expect("missing polyline ledger candidate");
        assert_eq!(entries.iter().filter(|e| e["command"] == command).count(), 1);
        assert_eq!(entry["status"], "candidate_ci_pending");
        assert_eq!(entry["test_file"], "crates/engine/tests/polyline_command_certification.rs");
        assert_eq!(entry["operation"], format!("kernel.polyline.{}", op.suffix()));
        assert_eq!(entry["orbweaver_node"], format!("orbweaver.polyline.{}", op.suffix()));
        assert!(live.contains(command.as_str()));
        assert!(find_command(&command).is_some());
        for gate in ["correctness", "failure_preservation", "persistence", "interface", "stability"] {
            assert!(entry["gates"][gate].as_str().is_some_and(|x| !x.is_empty()));
        }
    }
}

#[test]
fn certify_polyline_length_candidate() {
    verify(Operation::Length);
}

#[test]
fn certify_polyline_divide_count_candidate() {
    verify(Operation::DivideCount);
}

#[test]
fn certify_polyline_divide_distance_candidate() {
    verify(Operation::DivideDistance);
}
