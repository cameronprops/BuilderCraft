//! Prompt-engine certification for the native CAD command line.
//! This drives real command aliases, selections, prompts, previews, and undo.
//! It is not a claim that desktop mouse/keyboard rendering has been tested.

use cadcraft_doc::{EntityKind, Handle};
use cadcraft_engine::{Input, Session, geom::Vec2};
use serde_json::json;

fn line_coordinates(s: &Session, id: Handle) -> (f64, f64, f64, f64) {
    let entity = s.doc().unwrap().entity(id).unwrap();
    match &entity.kind {
        EntityKind::Line(line) => (line.a.x, line.a.y, line.b.x, line.b.y),
        other => panic!("expected line, got {other:?}"),
    }
}

fn assert_near(actual: (f64, f64, f64, f64), wanted: (f64, f64, f64, f64)) {
    for (a, b) in [(actual.0, wanted.0), (actual.1, wanted.1), (actual.2, wanted.2), (actual.3, wanted.3)] {
        assert!((a - b).abs() < 1e-9, "observed {actual:?}, expected {wanted:?}");
    }
}

fn input_point(s: &mut Session, x: f64, y: f64) {
    s.input(Input::Point(Vec2::new(x, y))).unwrap();
}

fn input_text(s: &mut Session, text: &str) {
    s.input(Input::Text(text.to_string())).unwrap();
}

fn created_line(s: &mut Session, x: f64) -> Handle {
    let result = s.execute("line", &json!({"points": [[x, 0], [x + 3.0, 4.0]]})).unwrap();
    Handle::parse_hex(result["handles"][0].as_str().unwrap()).unwrap()
}

fn roundtrip_preserves(s: &Session, handles: &[(Handle, (f64, f64, f64, f64))]) {
    let bytes = cadcraft_io::write(s.doc().unwrap(), "interactive.dftba").unwrap();
    let reopened = cadcraft_io::read(&bytes, "interactive.dftba").unwrap();
    for &(handle, wanted) in handles {
        let item = reopened.entity(handle).unwrap();
        match &item.kind {
            EntityKind::Line(line) => assert_near((line.a.x, line.a.y, line.b.x, line.b.y), wanted),
            other => panic!("expected reopened line, got {other:?}"),
        }
    }
}

#[test]
fn line_alias_preview_prompts_commit_and_undo() {
    for offset in 0..12 {
        let mut s = Session::new();
        let x = f64::from(offset);
        s.start("L").unwrap();
        assert!(s.prompt_text().to_ascii_lowercase().contains("first point"));
        input_point(&mut s, x, 0.0);
        let preview = s.preview(Vec2::new(x + 3.0, 4.0));
        assert_eq!(preview.len(), 1);
        match &preview[0].kind {
            EntityKind::Line(line) => {
                assert_near((line.a.x, line.a.y, line.b.x, line.b.y), (x, 0.0, x + 3.0, 4.0));
            }
            other => panic!("expected line preview, got {other:?}"),
        }
        assert_eq!(s.doc().unwrap().model.len(), 0, "preview mutated document");
        input_point(&mut s, x + 3.0, 4.0);
        s.input(Input::Enter).unwrap();
        assert!(s.running.is_none());
        assert_eq!(s.doc().unwrap().model.len(), 1);
        let h = s.doc().unwrap().model.iter().next().unwrap().handle;
        assert_near(line_coordinates(&s, h), (x, 0.0, x + 3.0, 4.0));
        roundtrip_preserves(&s, &[(h, (x, 0.0, x + 3.0, 4.0))]);
        s.execute("undo", &json!({})).unwrap();
        assert!(s.doc().unwrap().model.is_empty());
        s.execute("redo", &json!({})).unwrap();
        assert_near(line_coordinates(&s, h), (x, 0.0, x + 3.0, 4.0));
    }
}

#[test]
fn line_cancel_preserves_legacy_partial_segment_as_one_undo() {
    let mut s = Session::new();
    s.start("line").unwrap();
    input_point(&mut s, 0.0, 0.0);
    s.cancel();
    assert!(s.running.is_none());
    assert!(s.doc().unwrap().model.is_empty());
    assert!(s.state().unwrap().undo.is_empty());

    s.start("line").unwrap();
    input_point(&mut s, 0.0, 0.0);
    input_point(&mut s, 3.0, 4.0);
    s.cancel();
    assert!(s.running.is_none());
    assert_eq!(s.doc().unwrap().model.len(), 1);
    s.execute("undo", &json!({})).unwrap();
    assert!(s.doc().unwrap().model.is_empty());
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1);
}

#[test]
fn move_alias_pickfirst_and_prompted_displacement() {
    for offset in 0..12 {
        let mut s = Session::new();
        let x = f64::from(offset);
        let handle = created_line(&mut s, x);
        s.set_selection(vec![handle]);
        s.start("m").unwrap();
        assert!(s.prompt_text().contains("base point"));
        input_point(&mut s, 0.0, 0.0);
        assert!(s.prompt_text().contains("second point"));
        input_point(&mut s, 3.0, 4.0);
        assert!(s.running.is_none());
        assert_near(line_coordinates(&s, handle), (x + 3.0, 4.0, x + 6.0, 8.0));
        roundtrip_preserves(&s, &[(handle, (x + 3.0, 4.0, x + 6.0, 8.0))]);
        s.execute("undo", &json!({})).unwrap();
        assert_near(line_coordinates(&s, handle), (x, 0.0, x + 3.0, 4.0));
        s.execute("redo", &json!({})).unwrap();
        assert_near(line_coordinates(&s, handle), (x + 3.0, 4.0, x + 6.0, 8.0));
    }
}

#[test]
fn move_without_pickfirst_selects_handles_and_invalidates_nothing_on_cancel() {
    let mut s = Session::new();
    let handle = created_line(&mut s, 0.0);
    s.settings.pickfirst = false;
    let revision = s.state().unwrap().revision;
    let undo_count = s.state().unwrap().undo.len();
    s.start("move").unwrap();
    assert!(s.prompt_text().contains("Select objects"));
    s.input(Input::Pick(vec![handle])).unwrap();
    s.input(Input::Enter).unwrap();
    assert!(s.prompt_text().contains("base point"));
    s.cancel();
    assert_eq!(s.state().unwrap().revision, revision);
    assert_eq!(s.state().unwrap().undo.len(), undo_count);
    assert_eq!(line_coordinates(&s, handle), (0.0, 0.0, 3.0, 4.0));
    s.start("move").unwrap();
    s.input(Input::Pick(vec![handle])).unwrap();
    s.input(Input::Enter).unwrap();
    input_point(&mut s, 0.0, 0.0);
    input_point(&mut s, 2.0, 1.0);
    assert_eq!(line_coordinates(&s, handle), (2.0, 1.0, 5.0, 5.0));
}

#[test]
fn copy_prompt_mode_preserves_source_and_creates_new_handle() {
    let mut s = Session::new();
    let source = created_line(&mut s, 0.0);
    s.set_selection(vec![source]);
    s.start("co").unwrap();
    input_point(&mut s, 0.0, 0.0);
    input_point(&mut s, 0.0, 2.0);
    assert!(s.running.is_some(), "interactive COPY should permit another placement");
    s.input(Input::Enter).unwrap();
    assert!(s.running.is_none());
    assert_eq!(s.doc().unwrap().model.len(), 2);
    let copy = s.doc().unwrap().model.iter().find(|item| item.handle != source).unwrap().handle;
    assert_eq!(line_coordinates(&s, source), (0.0, 0.0, 3.0, 4.0));
    assert_eq!(line_coordinates(&s, copy), (0.0, 2.0, 3.0, 6.0));
    roundtrip_preserves(&s, &[(source, (0.0, 0.0, 3.0, 4.0)), (copy, (0.0, 2.0, 3.0, 6.0))]);
    s.execute("undo", &json!({})).unwrap();
    assert_eq!(s.doc().unwrap().model.len(), 1);
    s.execute("redo", &json!({})).unwrap();
    assert_eq!(line_coordinates(&s, copy), (0.0, 2.0, 3.0, 6.0));
}

#[test]
fn rotate_and_scale_prompted_numbers_reject_invalid_and_commit() {
    for (command, prompt_value, wanted) in [("ro", "90", (0.0, 0.0, -4.0, 3.0)), ("sc", "2", (0.0, 0.0, 6.0, 8.0))] {
        let mut s = Session::new();
        let handle = created_line(&mut s, 0.0);
        s.set_selection(vec![handle]);
        s.start(command).unwrap();
        input_point(&mut s, 0.0, 0.0);
        let revision = s.state().unwrap().revision;
        input_text(&mut s, "not-a-number");
        assert!(s.running.is_some(), "invalid numeric input should re-prompt");
        assert_eq!(s.state().unwrap().revision, revision);
        assert_eq!(line_coordinates(&s, handle), (0.0, 0.0, 3.0, 4.0));
        input_text(&mut s, prompt_value);
        assert!(s.running.is_none());
        assert_near(line_coordinates(&s, handle), wanted);
        roundtrip_preserves(&s, &[(handle, wanted)]);
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(line_coordinates(&s, handle), (0.0, 0.0, 3.0, 4.0));
        s.execute("redo", &json!({})).unwrap();
        assert_near(line_coordinates(&s, handle), wanted);
    }
}

#[test]
fn mirror_prompt_confirmations_copy_or_erase_source() {
    for (answer, expected_count) in [("No", 2), ("Yes", 1)] {
        let mut s = Session::new();
        let source = created_line(&mut s, 0.0);
        s.set_selection(vec![source]);
        s.start("mi").unwrap();
        input_point(&mut s, 0.0, 0.0);
        input_point(&mut s, 0.0, 1.0);
        assert!(s.prompt_text().contains("Erase source objects"));
        input_text(&mut s, answer);
        assert!(s.running.is_none());
        assert_eq!(s.doc().unwrap().model.len(), expected_count);
        if answer == "No" {
            assert_eq!(line_coordinates(&s, source), (0.0, 0.0, 3.0, 4.0));
            let reflected = s.doc().unwrap().model.iter().find(|item| item.handle != source).unwrap().handle;
            assert_near(line_coordinates(&s, reflected), (0.0, 0.0, -3.0, 4.0));
            roundtrip_preserves(&s, &[(source, (0.0, 0.0, 3.0, 4.0)), (reflected, (0.0, 0.0, -3.0, 4.0))]);
        } else {
            assert_near(line_coordinates(&s, source), (0.0, 0.0, -3.0, 4.0));
            roundtrip_preserves(&s, &[(source, (0.0, 0.0, -3.0, 4.0))]);
        }
        s.execute("undo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().model.len(), 1);
        assert_eq!(line_coordinates(&s, source), (0.0, 0.0, 3.0, 4.0));
        s.execute("redo", &json!({})).unwrap();
        assert_eq!(s.doc().unwrap().model.len(), expected_count);
    }
}

#[test]
fn interactive_engine_evidence_is_registered_for_all_six_commands() {
    let registry: serde_json::Value = serde_json::from_str(include_str!("../../../docs/commands/certification-registry.json")).unwrap();
    let candidates = registry["candidates"].as_array().unwrap();
    let expected = [
        ("line", "line_alias_preview_prompts_commit_and_undo"),
        ("move", "move_alias_pickfirst_and_prompted_displacement"),
        ("copy", "copy_prompt_mode_preserves_source_and_creates_new_handle"),
        ("rotate", "rotate_and_scale_prompted_numbers_reject_invalid_and_commit"),
        ("scale", "rotate_and_scale_prompted_numbers_reject_invalid_and_commit"),
        ("mirror", "mirror_prompt_confirmations_copy_or_erase_source"),
    ];
    for (id, test) in expected {
        let entry = candidates.iter().find(|item| item["command"] == id).unwrap();
        assert_eq!(entry["interactive_engine_test"], test, "{id} lacks prompt evidence");
        assert_eq!(entry["interactive_engine_test_file"], "crates/engine/tests/interactive_command_certification.rs");
        assert_eq!(entry["status"], "candidate_ci_pending");
        assert_eq!(entry["scope"], "headless_api_interactive_path_pending");
        assert!(entry["required_ci"].as_array().unwrap().iter().any(|x| x == "interactive_ui_followup"));
    }
}
