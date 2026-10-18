//! UI command dispatch must provide visible feedback when a command fails.
use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::json;

#[test]
fn failed_modeling_command_reports_status_and_preserves_document() {
    let mut app = CadApp::new(Session::new(), Services::default());
    let before = app.session.state().unwrap().doc.clone();
    let revision = app.session.state().unwrap().revision;

    assert!(app.run("line", json!({"points": [[0, 0]]})).is_err());
    assert!(std::sync::Arc::ptr_eq(&before, &app.session.state().unwrap().doc));
    assert_eq!(app.session.state().unwrap().revision, revision);

    let status = app.status.as_ref().expect("failed command must appear in the visible status bar");
    assert!(status.0.contains("line"), "status did not identify the failed command: {}", status.0);
    assert!(app.session.log.last().is_some_and(|line| line.contains("line")));

    // Error feedback does not prevent the next correctly submitted operation.
    assert!(app.run("line", json!({"points": [[0, 0], [2, 0]]})).is_ok());
    assert_eq!(app.session.doc().unwrap().model.len(), 1);
}

#[test]
fn interactive_start_failure_reports_visible_status() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.start("this_command_is_not_registered");
    let status = app.status.as_ref().expect("failed interactive command must appear in the visible status bar");
    assert!(status.0.contains("this_command_is_not_registered"));
    assert!(app.session.log.last().is_some_and(|line| line.contains("this_command_is_not_registered")));
}
