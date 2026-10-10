//! Alpha workstation acceptance: a single egui shell must retain the same
//! document and global command session while switching 2D/3D workspaces.

use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::json;

fn render_frame(app: &mut CadApp) {
    let ctx = egui::Context::default();
    let mut result = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280., 800.))),
            ..Default::default()
        },
        |ui| {
            app.logic(ui.ctx());
            app.ui(ui);
        },
    );
    result.textures_delta.clear();
}

#[test]
fn workspace_changes_are_presentational_and_document_identity_is_stable() {
    let mut app = CadApp::new(Session::new(), Services::default());
    let source_uid = app.session.state().unwrap().uid;
    let source_revision = app.session.state().unwrap().revision;
    let source_doc = app.session.state().unwrap().doc.clone();

    // Modes deliberately share the same CAD command interpreter and document.
    for (three_dimensional, dedicated_workspace) in
        [(false, true), (true, true), (false, false), (true, false)]
    {
        app.ui.view3d = three_dimensional;
        app.ui.buildercraft_workspace = dedicated_workspace;
        app.ui.toolset_tab = if three_dimensional { "Modeling" } else { "Drafting" }.into();
        render_frame(&mut app);
        assert_eq!(app.session.state().unwrap().uid, source_uid);
        assert_eq!(app.session.state().unwrap().revision, source_revision);
        assert!(std::sync::Arc::ptr_eq(&source_doc, &app.session.state().unwrap().doc));
        assert!(app.ui.show_command_line, "The global command entry must remain available");
    }
}

#[test]
fn common_2d_and_3d_commands_survive_switching_and_undo() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.ui.view3d = false;
    app.ui.buildercraft_workspace = false;
    app.run("line", json!({"points":[[0.,0.],[10.,0.]]})).unwrap();
    app.run("circle", json!({"center":[2.,3.],"radius":1.5})).unwrap();
    assert_eq!(app.session.doc().unwrap().entity_count(), 2);

    app.ui.view3d = true;
    app.ui.buildercraft_workspace = true;
    let created = app.run(
        "nurbs.curve3d",
        json!({"name":"Alpha curve","curve":{"degree":1,"control":[
            {"x":0.,"y":0.,"z":0.},{"x":4.,"y":2.,"z":3.}
        ],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}}),
    );
    assert!(created.is_ok(), "{created:?}");
    assert_eq!(app.session.doc().unwrap().geometry3d.len(), 1);

    app.run("undo", json!({})).unwrap();
    assert!(app.session.doc().unwrap().geometry3d.is_empty());
    app.run("redo", json!({})).unwrap();
    assert_eq!(app.session.doc().unwrap().geometry3d.len(), 1);
    assert_eq!(app.session.doc().unwrap().entity_count(), 2);
    app.ui.view3d = false;
    render_frame(&mut app);
    assert_eq!(app.session.doc().unwrap().entity_count(), 2);
}
