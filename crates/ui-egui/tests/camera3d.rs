use cadcraft_engine::Session;
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::json;
#[test]
fn camera_commands_frame_visible_geometry_without_editing_document() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.run("nurbs.curve3d",json!({"name":"Far from origin","curve":{"degree":1,"control":[{"x":10000.,"y":20000.,"z":30000.},{"x":10200.,"y":20400.,"z":30100.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}})).unwrap();
    let before = app.session.state().unwrap().doc.clone();
    let revision = app.session.state().unwrap().revision;
    for id in ["ui.buildercraft.top", "ui.buildercraft.front", "ui.buildercraft.right", "ui.buildercraft.iso"] {
        app.run(id, json!({})).unwrap();
        app.run("ui.buildercraft.fit", json!({})).unwrap();
        assert!(app.ui.center3d.x > 9000. && app.ui.scale3d > 0.);
    }
    assert_eq!(app.session.state().unwrap().revision, revision);
    assert!(std::sync::Arc::ptr_eq(&before, &app.session.state().unwrap().doc));
    let curve = app.session.doc().unwrap().geometry3d[0].id;
    app.run("geometry3d.set", json!({"id":curve,"visible":false})).unwrap();
    let center = app.ui.center3d;
    let scale = app.ui.scale3d;
    assert!(app.run("ui.buildercraft.fit", json!({})).is_err());
    assert_eq!(app.ui.center3d, center);
    assert_eq!(app.ui.scale3d, scale);
}

#[test]
fn navigation_controls_preserve_the_visible_canvas_width() {
    let mut app = CadApp::new(Session::new(), Services::default());
    app.run("ui.buildercraft.curve", json!({})).unwrap();
    let ctx = egui::Context::default();
    let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1000., 700.))), ..Default::default() };
    let mut frame = ctx.run_ui(input, |ui| {
        app.logic(ui.ctx());
        app.ui(ui)
    });
    frame.textures_delta.clear();
    // With the default browser and toolset panes, camera sizing must stay in the
    // visible center pane rather than follow an overflowing toolbar row.
    assert!(app.session.viewport_px.0 > 0. && app.session.viewport_px.0 < 600.);
    assert!(app.session.viewport_px.1 > 0. && app.session.viewport_px.1 < 700.);
    app.run("ui.buildercraft.fit", json!({})).unwrap();
}
