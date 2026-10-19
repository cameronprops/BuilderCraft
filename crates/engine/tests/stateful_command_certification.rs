//! State-changing CAD command certification candidates.
//! Headless command/API behavior only: interactive viewport paths remain release gates.

use cadcraft_doc::{Drawing, EntityKind, Handle};
use cadcraft_engine::{Session, command_specs, find_command};
use serde_json::{Value, json};
use std::collections::HashSet;

const COMMANDS: [(&str, &str, &str); 3] =
    [("line", "l", "certify_line_stateful"), ("move", "m", "certify_move_stateful"), ("copy", "co", "certify_copy_stateful")];

fn points(doc: &Drawing, id: &str) -> (f64, f64, f64, f64) {
    let handle = Handle::parse_hex(id).unwrap();
    match &doc.entity(handle).unwrap().kind {
        EntityKind::Line(line) => (line.a.x, line.a.y, line.b.x, line.b.y),
        other => panic!("expected line at {id}, got {other:?}"),
    }
}

fn drawing(s: &Session) -> Vec<u8> {
    cadcraft_io::write(s.doc().unwrap(), "certification.dftba").unwrap()
}

fn read(bytes: &[u8]) -> Drawing {
    cadcraft_io::read(bytes, "certification.dftba").unwrap()
}

struct Snapshot {
    bytes: Vec<u8>,
    revision: u64,
    undo: usize,
    redo: usize,
    selection: Vec<Handle>,
}

fn snapshot(s: &Session) -> Snapshot {
    let state = s.state().unwrap();
    Snapshot { bytes: drawing(s), revision: state.revision, undo: state.undo.len(), redo: state.redo.len(), selection: s.selection() }
}

fn unchanged(s: &Session, before: &Snapshot) {
    let state = s.state().unwrap();
    assert_eq!(drawing(s), before.bytes, "failed command mutated drawing");
    assert_eq!(state.revision, before.revision, "failed command advanced revision");
    assert_eq!(state.undo.len(), before.undo, "failed command created undo");
    assert_eq!(state.redo.len(), before.redo, "failed command erased redo");
    assert_eq!(s.selection(), before.selection, "failed command changed selection");
}

fn rejects(s: &mut Session, command: &str, bad: &Value) {
    let before = snapshot(s);
    assert!(s.execute(command, bad).is_err(), "{command} accepted malformed request: {bad}");
    unchanged(s, &before);
}

fn create_line(s: &mut Session, ax: f64) -> String {
    let output = s.execute("line", &json!({"points": [[ax, 0.0], [ax + 3.0, 4.0]]})).unwrap();
    output["handles"][0].as_str().unwrap().to_owned()
}

fn check_registration(id: &str, alias: &str) {
    let spec = find_command(id).expect("candidate missing from live command registry");
    assert!(spec.undoable, "{id} should be undoable");
    assert!(spec.interactive.is_some(), "{id} missing interactive command path");
    assert!(spec.aliases.contains(&alias), "{id} missing expected command-line alias");
    assert!(!spec.params.is_empty(), "{id} missing API parameter contract");
}

#[test]
fn stateful_candidates_match_certification_ledger() {
    let registry: Value = serde_json::from_str(include_str!("../../../docs/commands/certification-registry.json")).unwrap();
    let candidates = registry["candidates"].as_array().unwrap();
    let stateful: Vec<_> = candidates.iter().filter(|item| item["document_mutation"] == true).collect();
    assert_eq!(stateful.len(), COMMANDS.len());
    let mut unique = HashSet::new();
    let registered: HashSet<_> = command_specs().iter().map(|spec| spec.id).collect();
    for (id, alias, test) in COMMANDS {
        check_registration(id, alias);
        assert!(registered.contains(id));
        assert!(unique.insert(id));
        let entry = stateful.iter().find(|entry| entry["command"] == id).unwrap();
        assert_eq!(entry["status"], "candidate_ci_pending");
        assert_eq!(entry["test"], test);
        assert_eq!(entry["scope"], "headless_api_interactive_path_pending");
        for gate in ["correctness", "failure_preservation", "persistence", "interface", "stability"] {
            assert!(entry["gates"][gate].as_str().is_some_and(|text| !text.is_empty()), "{id} missing {gate}");
        }
    }
}

#[test]
fn certify_line_stateful() {
    check_registration("line", "l");
    for offset in 0..32 {
        let mut s = Session::new();
        let x = f64::from(offset);
        let h = create_line(&mut s, x);
        assert_eq!(s.doc().unwrap().model.len(), 1);
        assert_eq!(points(s.doc().unwrap(), &h), (x, 0.0, x + 3.0, 4.0));
        let saved = drawing(&s);
        assert_eq!(points(&read(&saved), &h), (x, 0.0, x + 3.0, 4.0));

        rejects(&mut s, "line", &json!({"points": [[1.0, 1.0]]}));
        rejects(&mut s, "line", &json!({"points": [[1.0, 1.0], [1.0, 1.0]]}));
        rejects(&mut s, "line", &json!({"points": [[0.0, 0.0], ["bad", 2.0]]}));

        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().model.len(), 0);
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(points(s.doc().unwrap(), &h), (x, 0.0, x + 3.0, 4.0));
    }
}

#[test]
fn certify_move_stateful() {
    check_registration("move", "m");
    for offset in 0..32 {
        let mut s = Session::new();
        let x = f64::from(offset);
        let h = create_line(&mut s, x);
        let original = drawing(&s);
        let result = s.execute("move", &json!({"handles": [h], "delta": [3.0, 4.0]})).unwrap();
        assert_eq!(result["moved"], 1);
        assert_eq!(s.doc().unwrap().model.len(), 1);
        assert_eq!(points(s.doc().unwrap(), &h), (x + 3.0, 4.0, x + 6.0, 8.0));
        let moved = drawing(&s);
        assert_eq!(points(&read(&moved), &h), (x + 3.0, 4.0, x + 6.0, 8.0));

        rejects(&mut s, "move", &json!({"handles": [h], "delta": ["bad", 2.0]}));
        rejects(&mut s, "move", &json!({"handles": [h], "from": [0.0, 0.0]}));

        s.execute("undo", &json!({})).unwrap();
        assert_eq!(points(s.doc().unwrap(), &h), (x, 0.0, x + 3.0, 4.0));
        assert_eq!(drawing(&s), original);
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(points(s.doc().unwrap(), &h), (x + 3.0, 4.0, x + 6.0, 8.0));
    }
}

#[test]
fn certify_copy_stateful() {
    check_registration("copy", "co");
    for offset in 0..24 {
        let mut s = Session::new();
        let x = f64::from(offset);
        let source = create_line(&mut s, x);
        let source_saved = drawing(&s);
        let result = s.execute("copy", &json!({"handles": [source], "delta": [0.0, 2.0], "count": 2})).unwrap();
        let copies = result["handles"].as_array().unwrap();
        assert_eq!(copies.len(), 2);
        let a = copies[0].as_str().unwrap();
        let b = copies[1].as_str().unwrap();
        assert_ne!(a, b);
        assert_ne!(a, source);
        assert_ne!(b, source);
        assert_eq!(s.doc().unwrap().model.len(), 3);
        assert_eq!(points(s.doc().unwrap(), &source), (x, 0.0, x + 3.0, 4.0));
        assert_eq!(points(s.doc().unwrap(), a), (x, 2.0, x + 3.0, 6.0));
        assert_eq!(points(s.doc().unwrap(), b), (x, 4.0, x + 3.0, 8.0));
        let saved = drawing(&s);
        let reopened = read(&saved);
        assert_eq!(reopened.model.len(), 3);
        assert_eq!(points(&reopened, a), (x, 2.0, x + 3.0, 6.0));
        assert_eq!(points(&reopened, b), (x, 4.0, x + 3.0, 8.0));

        rejects(&mut s, "copy", &json!({"handles": [source], "delta": ["bad", 0.0]}));
        rejects(&mut s, "copy", &json!({"handles": [source], "from": [0.0, 0.0]}));

        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().model.len(), 1);
        assert_eq!(drawing(&s), source_saved);
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().model.len(), 3);
        assert_eq!(points(s.doc().unwrap(), b), (x, 4.0, x + 3.0, 8.0));
    }

    // Reject an oversized operation before allocating or inserting copies.
    let mut s = Session::new();
    let source = s.execute("line", &json!({"points": [[0, 0], [1, 0], [2, 0], [3, 0]]})).unwrap();
    let handles = source["handles"].clone();
    assert_eq!(s.doc().unwrap().model.len(), 3);
    rejects(&mut s, "copy", &json!({"handles": handles, "delta": [1.0, 0.0], "count": 10000}));
    assert_eq!(s.doc().unwrap().model.len(), 3);
}
