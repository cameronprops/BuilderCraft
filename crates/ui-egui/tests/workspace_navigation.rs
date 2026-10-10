//! Alpha UI navigation regression: a workspace is a view over one document,
//! never a copy, a conversion operation or an undoable document edit.

use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::{Value, json};

fn snapshot(app: &CadApp) -> (Value, u64) {
    (
        serde_json::to_value(app.session.doc().unwrap()).unwrap(),
        app.session.state().unwrap().revision,
    )
}

#[test]
fn switching_2d_3d_back_preserves_document_revision_selection_and_history() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.run("line", json!({"points": [[0.0, 0.0], [7.0, 3.0]]})).unwrap();
    let initial = snapshot(&app);
    let selection = app.session.selection();
    assert!(app.ui.view3d);

    let first = app.run("ui.workspace.2d", json!({})).unwrap();
    assert_eq!(first["workspace"], "2d");
    assert!(!app.ui.view3d);
    assert_eq!(app.ui.toolset_tab, "Drafting");
    assert_eq!(snapshot(&app), initial);
    assert_eq!(app.session.selection(), selection);

    let second = app.run("ui.workspace.3d", json!({})).unwrap();
    assert_eq!(second["workspace"], "3d");
    assert!(app.ui.view3d);
    assert_eq!(snapshot(&app), initial);

    let back = app.run("ui.workspace.previous", json!({})).unwrap();
    assert_eq!(back["workspace"], "2d");
    assert!(!app.ui.view3d);
    assert_eq!(snapshot(&app), initial);

    app.run("undo", json!({})).unwrap();
    assert_ne!(snapshot(&app).0, initial.0);
    app.run("redo", json!({})).unwrap();
    assert_eq!(snapshot(&app).0, initial.0);
}

#[test]
fn workspace_commands_are_available_without_geometry_or_an_existing_document() {
    let mut app = CadApp::new(Session::empty(), Services::default());
    assert!(app.run("ui.workspace.previous", json!({})).is_err());
    assert_eq!(app.run("ui.workspace.2d", json!({})).unwrap()["workspace"], "2d");
    assert_eq!(app.run("ui.workspace.previous", json!({})).unwrap()["workspace"], "3d");
    assert!(app.session.docs.is_empty());
    assert!(app.ui.show_command_line);
}

#[test]
fn ui_preferences_roundtrip_remembers_workspace_return_path() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.run("ui.workspace.2d", json!({})).unwrap();
    let data = serde_json::to_string(&app.ui).unwrap();
    let restored: cadcraft_ui_egui::UiState = serde_json::from_str(&data).unwrap();
    assert!(!restored.view3d);
    assert_eq!(restored.previous_workspace_3d, Some(true));
    let old: cadcraft_ui_egui::UiState = serde_json::from_str("{}").unwrap();
    assert!(old.view3d);
    assert_eq!(old.previous_workspace_3d, None);
}
