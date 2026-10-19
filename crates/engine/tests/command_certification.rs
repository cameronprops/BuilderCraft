//! First full command-certification candidates.
//! These tests enforce the five gates for *headless, non-mutating* operations.
//! Passing local fixtures is not a cross-platform release certification.

use cadcraft_doc::EntityKind;
use cadcraft_engine::{Session, command_specs, find_command};
use serde_json::{Value, json};
use std::collections::HashSet;

struct Candidate {
    id: &'static str,
    operation: &'static str,
    node: &'static str,
    output_kind: &'static str,
}

const CANDIDATES: [Candidate; 3] = [
    Candidate { id: "worldwright.point.distance", operation: "kernel.point.distance", node: "orbweaver.point.distance", output_kind: "number" },
    Candidate { id: "worldwright.point.midpoint", operation: "kernel.point.midpoint", node: "orbweaver.point.midpoint", output_kind: "point" },
    Candidate {
        id: "worldwright.point.interpolate",
        operation: "kernel.point.interpolate",
        node: "orbweaver.point.interpolate",
        output_kind: "point",
    },
];

fn point(x: f64, y: f64, z: f64) -> Value {
    json!({"kind":"point","value":{"x":x,"y":y,"z":z}})
}

fn inputs(x: f64, parameter: f64, interpolate: bool) -> Value {
    let mut value = json!({
        "a": point(x, 0.0, 0.0),
        "b": point(x + 3.0, 4.0, 0.0)
    });
    if interpolate {
        value["t"] = json!({"kind":"number","value":parameter});
    }
    value
}

fn expected(kind: &str, x: f64) -> Value {
    match kind {
        "number" => json!({"kind":"number","value":5.0}),
        "midpoint" => json!({"kind":"point","value":{"x":x + 1.5,"y":2.0,"z":0.0}}),
        "interpolate" => json!({"kind":"point","value":{"x":x + 0.75,"y":1.0,"z":0.0}}),
        other => panic!("unknown candidate kind: {other}"),
    }
}

fn assert_document_unchanged(s: &Session, revision: u64, saved: &[u8], context: &str) {
    assert_eq!(s.state().unwrap().revision, revision, "revision changed: {context}");
    assert_eq!(cadcraft_io::write(s.doc().unwrap(), "certification.dftba").unwrap(), saved, "document changed: {context}");
    assert!(s.selection().is_empty(), "selection changed: {context}");
}

fn verify(candidate: &Candidate, kind: &str) {
    let mut s = Session::new();

    // Persistence/undo applicability: these are read-only numeric commands.
    // Seed a real document and verify they never perturb it or its history.
    let line = s.execute("line", &json!({"points":[[1,2],[4,6]]})).unwrap();
    assert!(line["handles"][0].is_string());
    let revision = s.state().unwrap().revision;
    let original = cadcraft_io::write(s.doc().unwrap(), "certification.dftba").unwrap();

    // Correctness and interface parity: CAD and both shared API aliases must
    // produce independently known results on a right-triangle fixture.
    for index in 0..64 {
        let x = f64::from(index);
        let ports = inputs(x, 0.25, kind == "interpolate");
        let direct = s.execute(candidate.id, &json!({"inputs":ports})).unwrap();
        let generic = s
            .execute(
                "worldwright.tool.run",
                &json!({
                    "operation":candidate.operation,"inputs":ports
                }),
            )
            .unwrap();
        let orb = s
            .execute(
                "worldwright.tool.run",
                &json!({
                    "operation":candidate.node,"inputs":ports
                }),
            )
            .unwrap();
        assert_eq!(direct["output"], generic["output"], "generic API mismatch: {}", candidate.id);
        assert_eq!(direct["output"], orb["output"], "OrbWeaver mismatch: {}", candidate.id);
        assert_eq!(direct["output"]["kind"], candidate.output_kind);
        assert_eq!(direct["output"], expected(kind, x), "incorrect result: {} fixture {index}", candidate.id);
    }
    assert_document_unchanged(&s, revision, &original, candidate.id);

    // Invalid/missing/extra typed ports and malformed coordinates are errors,
    // never a silent success or partially committed operation.
    let base = inputs(0.0, 0.25, kind == "interpolate");
    let mut bad_cases = Vec::new();
    let mut missing = base.clone();
    missing.as_object_mut().unwrap().remove("a");
    bad_cases.push(missing);
    let mut extra = base.clone();
    extra["unexpected"] = json!({"kind":"number","value":1.0});
    bad_cases.push(extra);
    let mut wrong_kind = base.clone();
    wrong_kind["a"] = json!({"kind":"number","value":2.0});
    bad_cases.push(wrong_kind);
    let mut bad_coordinate = base.clone();
    bad_coordinate["a"]["value"]["x"] = json!("invalid");
    bad_cases.push(bad_coordinate);
    for ports in bad_cases {
        assert!(s.execute(candidate.id, &json!({"inputs":ports})).is_err(), "accepted bad input: {}", candidate.id);
        assert_document_unchanged(&s, revision, &original, "failed input");
    }

    // Save/reopen must preserve the source document even after success/failure
    // queries, and those queries must not insert phantom undo entries.
    let restored = cadcraft_io::read(&original, "certification.dftba").unwrap();
    assert_eq!(restored.model.len(), 1);
    match &restored.model.iter().next().unwrap().kind {
        EntityKind::Line(line) => {
            assert_eq!((line.a.x, line.a.y), (1.0, 2.0));
            assert_eq!((line.b.x, line.b.y), (4.0, 6.0));
        }
        other => panic!("saved geometry changed: {other:?}"),
    }
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 0, "read-only query added an undo entry");
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1, "read-only query invalidated redo");
}

#[test]
fn certification_registry_matches_executable_candidates() {
    let manifest: Value = serde_json::from_str(include_str!("../../../docs/commands/certification-registry.json")).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    let entries = manifest["candidates"].as_array().unwrap();
    assert_eq!(entries.len(), CANDIDATES.len(), "every candidate must have an executable five-gate test");
    let live: HashSet<_> = command_specs().iter().map(|c| c.id).collect();
    let mut seen = HashSet::new();
    for candidate in &CANDIDATES {
        let record = entries.iter().find(|r| r["command"] == candidate.id).expect("candidate missing from ledger");
        assert!(seen.insert(candidate.id), "duplicate candidate {}", candidate.id);
        assert!(live.contains(candidate.id));
        assert!(find_command(candidate.id).is_some());
        assert_eq!(record["operation"], candidate.operation);
        assert_eq!(record["orbweaver_node"], candidate.node);
        assert_eq!(record["document_mutation"], false);
        assert_eq!(record["status"], "candidate_ci_pending");
        for gate in ["correctness", "failure_preservation", "persistence", "interface", "stability"] {
            assert!(record["gates"][gate].as_str().is_some_and(|s| !s.is_empty()), "missing {gate} on {}", candidate.id);
        }
    }
}

#[test]
fn certify_point_distance_candidate() {
    verify(&CANDIDATES[0], "number");
}

#[test]
fn certify_point_midpoint_candidate() {
    verify(&CANDIDATES[1], "midpoint");
}

#[test]
fn certify_point_interpolate_candidate() {
    verify(&CANDIDATES[2], "interpolate");
}
