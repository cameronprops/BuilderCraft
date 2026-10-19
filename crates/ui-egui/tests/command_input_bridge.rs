//! End-to-end native UI-control input bridges for fundamental CAD editing.
//! This exercises CadApp and the same UI control dispatcher used by the
//! desktop app. GPU presentation remains a separate visual certification gate.

use cadcraft_doc::{EntityKind, Handle};
use cadcraft_engine::Session;
use cadcraft_ui_egui::control::{ControlRequest, Outcome, handle};
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::{Value, json};

fn send(app: &mut CadApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (request, _reply) = ControlRequest::new(method, params);
    match handle(app, ctx, &request) {
        Outcome::Done(response) => {
            assert_eq!(response["ok"], true, "{method} failed: {response}");
            response["result"].clone()
        }
        Outcome::Screenshot { .. } => panic!("unexpected screenshot outcome for {method}"),
    }
}

fn point(app: &mut CadApp, ctx: &egui::Context, x: f64, y: f64) {
    let result = send(app, ctx, "ui.pointer", json!({"x": x, "y": y, "space": "world", "button": "left", "action": "click"}));
    assert_eq!(result["world"], json!([x, y]));
}

fn endpoints(app: &CadApp, handle: Handle) -> (f64, f64, f64, f64) {
    let entity = app.session.doc().unwrap().entity(handle).unwrap();
    match &entity.kind {
        EntityKind::Line(line) => (line.a.x, line.a.y, line.b.x, line.b.y),
        other => panic!("expected CAD line, found {other:?}"),
    }
}

fn only_handle(app: &CadApp) -> Handle {
    let model = &app.session.doc().unwrap().model;
    assert_eq!(model.len(), 1);
    model.iter().next().unwrap().handle
}

#[test]
fn cadapp_command_line_and_world_pointer_create_undoable_line() {
    let ctx = egui::Context::default();
    for x in [0.0, 5.0, 12.0] {
        let mut app = CadApp::new(Session::new(), Services::default());
        let started = send(&mut app, &ctx, "cmdline.input", json!({"text": "l"}));
        assert_eq!(started["running"], "line");
        assert!(started["prompt"].as_str().unwrap().contains("first point"));
        point(&mut app, &ctx, x, 0.0);
        assert!(app.session.preview(cadcraft_geom::Vec2::new(x + 3.0, 4.0)).len() == 1);
        assert_eq!(app.session.doc().unwrap().model.len(), 0, "rubber-band preview created an entity");
        point(&mut app, &ctx, x + 3.0, 4.0);
        let closed = send(&mut app, &ctx, "cmdline.key", json!({"key": "enter"}));
        assert_eq!(closed["running"], Value::Null);
        let handle = only_handle(&app);
        assert_eq!(endpoints(&app, handle), (x, 0.0, x + 3.0, 4.0));
        send(&mut app, &ctx, "engine.execute", json!({"command": "undo"}));
        assert!(app.session.doc().unwrap().model.is_empty());
        send(&mut app, &ctx, "engine.execute", json!({"command": "redo"}));
        assert_eq!(endpoints(&app, handle), (x, 0.0, x + 3.0, 4.0));
    }
}

#[test]
fn cadapp_pointer_picking_then_move_respects_selection_history_and_cancel() {
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    let result = send(&mut app, &ctx, "engine.execute", json!({
        "command": "line", "params": {"points": [[0.0, 0.0], [3.0, 4.0]]}
    }));
    let handle = Handle::parse_hex(result["handles"][0].as_str().unwrap()).unwrap();

    // Real idle pointer hit-testing, not a hard-coded selected handle.
    point(&mut app, &ctx, 1.5, 2.0);
    assert!(app.session.selection().contains(&handle), "line not picked through UI pointer");
    let revision = app.session.state().unwrap().revision;
    let undo_depth = app.session.state().unwrap().undo.len();
    let started = send(&mut app, &ctx, "cmdline.input", json!({"text":"m"}));
    assert_eq!(started["running"], "move");
    assert!(started["prompt"].as_str().unwrap().contains("base point"));
    send(&mut app, &ctx, "cmdline.key", json!({"key":"escape"}));
    assert_eq!(endpoints(&app, handle), (0.0, 0.0, 3.0, 4.0));
    assert_eq!(app.session.state().unwrap().revision, revision);
    assert_eq!(app.session.state().unwrap().undo.len(), undo_depth);

    // Selection again through the actual viewport control bridge.
    point(&mut app, &ctx, 1.5, 2.0);
    let started = send(&mut app, &ctx, "cmdline.input", json!({"text":"m"}));
    assert_eq!(started["running"], "move");
    point(&mut app, &ctx, 0.0, 0.0);
    point(&mut app, &ctx, 2.0, 1.0);
    assert_eq!(endpoints(&app, handle), (2.0, 1.0, 5.0, 5.0));
    send(&mut app, &ctx, "engine.execute", json!({"command":"undo"}));
    assert_eq!(endpoints(&app, handle), (0.0, 0.0, 3.0, 4.0));
    send(&mut app, &ctx, "engine.execute", json!({"command":"redo"}));
    assert_eq!(endpoints(&app, handle), (2.0, 1.0, 5.0, 5.0));
}

#[test]
fn cadapp_pointer_copy_retains_source_and_one_undo_group() {
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    let result = send(&mut app, &ctx, "engine.execute", json!({
        "command":"line","params":{"points":[[0.0,0.0],[3.0,4.0]]}
    }));
    let source = Handle::parse_hex(result["handles"][0].as_str().unwrap()).unwrap();
    point(&mut app, &ctx, 1.5, 2.0);
    let started = send(&mut app, &ctx, "cmdline.input", json!({"text":"co"}));
    assert_eq!(started["running"], "copy");
    point(&mut app, &ctx, 0.0, 0.0);
    point(&mut app, &ctx, 0.0, 2.0);
    assert_eq!(app.session.doc().unwrap().model.len(), 2);
    assert_eq!(endpoints(&app, source), (0.0, 0.0, 3.0, 4.0));
    send(&mut app, &ctx, "cmdline.key", json!({"key":"enter"}));
    let copy = app.session.doc().unwrap().model.iter().find(|item| item.handle != source).unwrap().handle;
    assert_eq!(endpoints(&app, copy), (0.0, 2.0, 3.0, 6.0));
    send(&mut app, &ctx, "engine.execute", json!({"command":"undo"}));
    assert_eq!(app.session.doc().unwrap().model.len(), 1);
    send(&mut app, &ctx, "engine.execute", json!({"command":"redo"}));
    assert_eq!(endpoints(&app, copy), (0.0, 2.0, 3.0, 6.0));
}
