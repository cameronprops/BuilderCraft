//! Five-gate headless certification candidates for shared 3D vector math.
//! Tests exercise the same Rust operation through CAD, generic API and OrbWeaver.
//! These are candidates, not a claim of full desktop command certification.

use cadcraft_engine::{Session, command_specs, find_command};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Copy)]
enum Operation {
    Length,
    Normalize,
    Dot,
    Cross,
}

impl Operation {
    fn label(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Normalize => "normalize",
            Self::Dot => "dot",
            Self::Cross => "cross",
        }
    }

    fn output_kind(self) -> &'static str {
        match self {
            Self::Length | Self::Dot => "number",
            Self::Normalize | Self::Cross => "vector",
        }
    }

    fn inputs(self, n: f64) -> Value {
        match self {
            Self::Length | Self::Normalize => json!({
                "v": vector(3.0 * n, 4.0 * n, 0.0)
            }),
            Self::Dot => json!({
                "a": vector(n, 2.0 * n, 3.0 * n),
                "b": vector(4.0, -5.0, 6.0)
            }),
            Self::Cross => json!({
                "a": vector(n, 2.0 * n, 3.0 * n),
                "b": vector(4.0, -5.0, 6.0)
            }),
        }
    }

    fn expected(self, n: f64) -> Value {
        match self {
            Self::Length => json!(5.0 * n.abs()),
            Self::Normalize => json!({"x": 0.6 * n.signum(), "y": 0.8 * n.signum(), "z": 0.0}),
            Self::Dot => json!(12.0 * n),
            Self::Cross => json!({"x": 27.0 * n, "y": 6.0 * n, "z": -13.0 * n}),
        }
    }

    fn required_port(self) -> &'static str {
        match self {
            Self::Length | Self::Normalize => "v",
            Self::Dot | Self::Cross => "a",
        }
    }
}

const OPERATIONS: [Operation; 4] = [Operation::Length, Operation::Normalize, Operation::Dot, Operation::Cross];

fn vector(x: f64, y: f64, z: f64) -> Value {
    json!({"kind": "vector", "value": {"x": x, "y": y, "z": z}})
}

fn assert_near(actual: &Value, wanted: &Value, kind: &str) {
    match kind {
        "number" => {
            let measured = actual.as_f64().expect("numeric output missing");
            let expected = wanted.as_f64().expect("numeric fixture missing");
            assert!((measured - expected).abs() <= 1e-12 * expected.abs().max(1.0), "{measured} != {expected}");
        }
        "vector" => {
            for component in ["x", "y", "z"] {
                let measured = actual[component].as_f64().expect("vector component missing");
                let expected = wanted[component].as_f64().expect("expected vector component missing");
                assert!((measured - expected).abs() <= 1e-12 * expected.abs().max(1.0), "{component}: {measured} != {expected}");
            }
        }
        other => panic!("unsupported vector output kind: {other}"),
    }
}

fn assert_unchanged(session: &Session, bytes: &[u8], revision: u64, undo: usize, redo: usize) {
    let state = session.state().unwrap();
    assert_eq!(cadcraft_io::write(session.doc().unwrap(), "vectors.dftba").unwrap(), bytes);
    assert_eq!(state.revision, revision);
    assert_eq!(state.undo.len(), undo);
    assert_eq!(state.redo.len(), redo);
    assert!(session.selection().is_empty());
}

fn verify(op: Operation) {
    let name = op.label();
    let direct = format!("worldwright.vector.{name}");
    let shared = format!("kernel.vector.{name}");
    let node = format!("orbweaver.vector.{name}");

    let mut session = Session::new();
    session.execute("line", &json!({"points": [[1.0, 2.0], [4.0, 6.0]]})).unwrap();
    let bytes = cadcraft_io::write(session.doc().unwrap(), "vectors.dftba").unwrap();
    let state = session.state().unwrap();
    let revision = state.revision;
    let undo = state.undo.len();
    let redo = state.redo.len();

    // Each signed, non-axis-aligned fixture has an independent analytic answer. Compare numeric
    // values as well as all three interfaces; don't merely check is_ok().
    for index in -64..=64 {
        if index == 0 {
            continue;
        }
        let n = f64::from(index);
        let ports = op.inputs(n);
        let cad = session.execute(&direct, &json!({"inputs":ports})).unwrap();
        let api = session.execute("worldwright.tool.run", &json!({"operation":shared,"inputs":ports})).unwrap();
        let graph = session.execute("worldwright.tool.run", &json!({"operation":node,"inputs":ports})).unwrap();
        assert_eq!(cad["output"], api["output"], "CAD versus shared API: {direct} at {index}");
        assert_eq!(cad["output"], graph["output"], "CAD versus OrbWeaver: {direct} at {index}");
        assert_eq!(cad["output"]["kind"], op.output_kind());
        assert_near(&cad["output"]["value"], &op.expected(n), op.output_kind());
    }
    assert_unchanged(&session, &bytes, revision, undo, redo);

    // Typed-port validation, missing inputs, malformed coordinates and
    // unexpected ports must not mutate a CAD document or its undo history.
    let base = op.inputs(1.0);
    let mut invalid = Vec::new();
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove(op.required_port());
    invalid.push(missing);
    let mut wrong_type = base.clone();
    wrong_type[op.required_port()]["kind"] = json!("point");
    invalid.push(wrong_type);
    let mut malformed = base.clone();
    malformed[op.required_port()]["value"]["x"] = json!("not-a-number");
    invalid.push(malformed);
    let mut extra = base.clone();
    extra["extraneous"] = vector(1.0, 0.0, 0.0);
    invalid.push(extra);
    for ports in invalid {
        assert!(session.execute(&direct, &json!({"inputs":ports})).is_err(), "{direct} accepted bad input");
        assert_unchanged(&session, &bytes, revision, undo, redo);
    }

    if matches!(op, Operation::Normalize) {
        let ports = json!({"v":vector(0.0, 0.0, 0.0)});
        assert!(session.execute(&direct, &json!({"inputs":ports})).is_err(), "zero vector normalized");
        assert_unchanged(&session, &bytes, revision, undo, redo);
    }

    // Native save/reopen and undo/redo of the source geometry remain valid.
    let restored = cadcraft_io::read(&bytes, "vectors.dftba").unwrap();
    assert_eq!(restored.model.len(), 1);
    session.execute("undo", &json!({})).unwrap();
    assert!(session.doc().unwrap().model.is_empty(), "read-only tool created an undo record");
    session.execute("redo", &json!({})).unwrap();
    assert_eq!(session.doc().unwrap().model.len(), 1);
}

#[test]
fn vector_certification_ledger_matches_four_registered_operations() {
    let manifest: Value = serde_json::from_str(include_str!("../../../docs/commands/certification-registry.json")).unwrap();
    let entries = manifest["candidates"].as_array().unwrap();
    let live: HashSet<_> = command_specs().iter().map(|spec| spec.id).collect();
    let certified = entries.iter().filter(|entry| entry["status"] == "certified_headless").count();
    assert_eq!(manifest["certified_count"], certified);
    assert_eq!(certified, 3, "previously certified point tools must remain certified");
    for op in OPERATIONS {
        let label = op.label();
        let command = format!("worldwright.vector.{label}");
        let entry = entries.iter().find(|entry| entry["command"] == command).expect("vector candidate missing");
        assert_eq!(entry["status"], "candidate_ci_pending");
        assert_eq!(entry["operation"], format!("kernel.vector.{label}"));
        assert_eq!(entry["orbweaver_node"], format!("orbweaver.vector.{label}"));
        assert_eq!(entry["test_file"], "crates/engine/tests/vector_command_certification.rs");
        assert_eq!(entry["document_mutation"], false);
        assert!(live.contains(command.as_str()));
        assert!(find_command(&command).is_some());
        for gate in ["correctness", "failure_preservation", "persistence", "interface", "stability"] {
            assert!(entry["gates"][gate].as_str().is_some_and(|value| !value.is_empty()));
        }
    }
}

#[test]
fn certify_vector_length_candidate() {
    verify(Operation::Length);
}

#[test]
fn certify_vector_normalize_candidate() {
    verify(Operation::Normalize);
}

#[test]
fn certify_vector_dot_candidate() {
    verify(Operation::Dot);
}

#[test]
fn certify_vector_cross_candidate() {
    verify(Operation::Cross);
}
