//! Transient control-point drafting; the document changes only on Finish.
use crate::CadApp;
use cadcraft_geom::{Vec3, snap3d::ConstructionPlane};
use serde_json::json;

#[derive(Clone, Debug, Default)]
pub struct PointTool {
    draft: Option<Draft>,
    hover: Option<Vec3>,
    snapped: bool,
    message: String,
}
#[derive(Clone, Debug)]
struct Draft {
    uid: u64,
    revision: u64,
    plane: usize,
    origin: [f64; 3],
    points: Vec<Vec3>,
}
pub fn active(app: &CadApp) -> bool {
    app.ui.point_input.draft.is_some()
}
pub fn plane(app: &CadApp) -> Option<ConstructionPlane> {
    let (x_axis, y_axis) = match app.ui.plane3d {
        0 => (Vec3::new(1., 0., 0.), Vec3::new(0., 1., 0.)),
        1 => (Vec3::new(1., 0., 0.), Vec3::new(0., 0., 1.)),
        2 => (Vec3::new(0., 1., 0.), Vec3::new(0., 0., 1.)),
        _ => return None,
    };
    let [x, y, z] = app.ui.plane_origin;
    ConstructionPlane::from_axes(Vec3::new(x, y, z), x_axis, y_axis)
}
pub fn begin(app: &mut CadApp) {
    crate::gizmo::cancel(app);
    if let Ok(s) = app.session.state() {
        app.ui.point_input = PointTool {
            draft: Some(Draft { uid: s.uid, revision: s.revision, plane: app.ui.plane3d, origin: app.ui.plane_origin, points: Vec::new() }),
            ..Default::default()
        };
    }
}
fn finish(app: &mut CadApp, tool: &mut PointTool) {
    let Some(d) = tool.draft.as_ref() else {
        return;
    };
    if d.plane != app.ui.plane3d || d.origin != app.ui.plane_origin {
        *tool = PointTool::default();
        app.session.echo("Point input cancelled because the construction plane changed");
        return;
    }
    if d.points.len() < 2 {
        tool.message = "Choose at least two control points".into();
        return;
    }
    let result = app.run(
        "nurbs.controlcurve3d",
        json!({"name":"Control curve","points":d.points.iter().map(|p|[p.x,p.y,p.z]).collect::<Vec<_>>(),
        "degree":app.ui.curve_degree.min(d.points.len()-1),"expected_uid":d.uid,"expected_revision":d.revision}),
    );
    match result {
        Ok(_) => *tool = PointTool::default(),
        Err(e) => tool.message = e,
    }
}
pub fn controls(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label("CPlane");
        for (index, label) in [(0, "XY"), (1, "XZ"), (2, "YZ")] {
            ui.selectable_value(&mut app.ui.plane3d, index, label);
        }
        ui.label("Origin");
        for value in &mut app.ui.plane_origin {
            ui.add(egui::DragValue::new(value).speed(0.1));
        }
        ui.checkbox(&mut app.ui.endpoint_snap, "End/corner snap");
        ui.label("Degree");
        ui.add(egui::DragValue::new(&mut app.ui.curve_degree).range(1..=5));
    });
    if active(app) {
        let mut tool = std::mem::take(&mut app.ui.point_input);
        ui.horizontal(|ui| {
            ui.label(format!("{} control points; Enter finishes, Esc cancels", tool.draft.as_ref().map_or(0, |d| d.points.len())));
            if ui.button("Finish curve").clicked() {
                finish(app, &mut tool);
            }
            if ui.button("Cancel curve").clicked() {
                tool = PointTool::default();
            }
        });
        app.ui.point_input = tool;
    }
}
pub fn interact(app: &mut CadApp, ui: &egui::Ui, rect: egui::Rect, response: &egui::Response) -> bool {
    if !active(app) {
        return false;
    }
    let mut tool = std::mem::take(&mut app.ui.point_input);
    let current = app.session.state().ok().map(|s| (s.uid, s.revision));
    let stale =
        tool.draft.as_ref().is_some_and(|d| current != Some((d.uid, d.revision)) || d.plane != app.ui.plane3d || d.origin != app.ui.plane_origin);
    if stale || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if stale {
            app.session.echo("Point input cancelled because the document or construction plane changed");
        }
        return true;
    }
    if response.secondary_clicked() || (!ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
        finish(app, &mut tool);
    } else if response.hovered()
        && let Some(pixel) = ui.input(|i| i.pointer.hover_pos())
        && let Some(plane) = plane(app)
    {
        let result=app.session.execute("geometry3d.snap",&json!({"pixel":[pixel.x-rect.min.x,pixel.y-rect.min.y],"viewport":[rect.width(),rect.height()],
            "center":[app.ui.center3d.x,app.ui.center3d.y,app.ui.center3d.z],"yaw":app.ui.orbit_yaw,"pitch":app.ui.orbit_pitch,"scale":app.ui.scale3d,
            "plane":{"origin":[plane.origin.x,plane.origin.y,plane.origin.z],"x_axis":[plane.x_axis.x,plane.x_axis.y,plane.x_axis.z],"y_axis":[plane.y_axis.x,plane.y_axis.y,plane.y_axis.z]},"endpoints":app.ui.endpoint_snap,"midpoints":false}));
        match result {
            Ok(hit) => {
                tool.hover = hit.get("point").and_then(|v| serde_json::from_value::<[f64; 3]>(v.clone()).ok()).map(|[x, y, z]| Vec3::new(x, y, z));
                tool.snapped = !hit["source_id"].is_null();
                tool.message.clear();
                if response.clicked_by(egui::PointerButton::Primary)
                    && let Some(point) = tool.hover
                    && let Some(d) = tool.draft.as_mut()
                {
                    if d.points.len() >= 256 {
                        tool.message = "Control-point limit reached; finish this curve".into();
                    } else if d.points.last().is_none_or(|p| (*p - point).len() > 1e-9) {
                        d.points.push(point);
                    }
                }
            }
            Err(e) => {
                tool.hover = None;
                tool.message = e.to_string();
            }
        }
    } else {
        tool.hover = None;
    }
    app.ui.point_input = tool;
    true
}
pub fn draw(app: &CadApp, ui: &egui::Ui, rect: egui::Rect, project: impl Fn(Vec3) -> egui::Pos2) {
    let tool = &app.ui.point_input;
    let Some(d) = &tool.draft else {
        return;
    };
    let painter = ui.painter_at(rect);
    let colors = crate::theme::Tokens::get();
    let mut points = d.points.clone();
    if let Some(p) = tool.hover
        && points.last().is_none_or(|last| (*last - p).len() > 1e-9)
    {
        points.push(p);
    }
    for pair in points.windows(2) {
        if let [a, b] = pair {
            painter.line_segment([project(*a), project(*b)], egui::Stroke::new(1., colors.text_dim));
        }
    }
    for point in &d.points {
        painter.circle_filled(project(*point), 3., colors.text);
    }
    if points.len() >= 2 && app.ui.curve_degree > 0 {
        let degree = app.ui.curve_degree.min(points.len() - 1);
        if let Some(c) = cadcraft_geom::nurbs3d::Curve::from_control(points, degree) {
            let _ = buildercraft_kernel::visit_preview_wires(&buildercraft_kernel::ExactShape::Curve(c.into()), &mut 50_000_000, |a, b| {
                painter.line_segment([project(a), project(b)], egui::Stroke::new(2., colors.snap));
            });
        }
    }
    if let Some(point) = tool.hover {
        let position = project(point);
        painter.circle_stroke(position, 6., egui::Stroke::new(2., colors.hover));
        painter.text(
            position + egui::vec2(10., -10.),
            egui::Align2::LEFT_BOTTOM,
            if tool.snapped { "End/corner" } else { "CPlane" },
            egui::FontId::proportional(14.),
            colors.hover,
        );
    }
    if !tool.message.is_empty() {
        painter.text(rect.left_top() + egui::vec2(8., 8.), egui::Align2::LEFT_TOP, &tool.message, egui::FontId::proportional(14.), colors.grip_hot);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(app: &mut CadApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 400.))), events, ..Default::default() },
            |ui| {
                let (rect, response) = ui.allocate_exact_size(egui::vec2(400., 400.), egui::Sense::click_and_drag());
                interact(app, ui, rect, &response);
            },
        );
        output.textures_delta.clear();
    }
    fn click(app: &mut CadApp, ctx: &egui::Context, p: egui::Pos2) {
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
                ],
            );
        }
    }
    fn key(app: &mut CadApp, ctx: &egui::Context, key: egui::Key) {
        frame(app, ctx, vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]);
        frame(app, ctx, vec![egui::Event::Key { key, physical_key: None, pressed: false, repeat: false, modifiers: Default::default() }]);
    }
    #[test]
    fn point_input_click_finish_undo_escape_and_stale_document() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        app.ui.orbit_pitch = -std::f64::consts::FRAC_PI_2;
        app.ui.orbit_yaw = 0.;
        app.ui.scale3d = 10.;
        let ctx = egui::Context::default();
        begin(&mut app);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, egui::pos2(150., 200.));
        click(&mut app, &ctx, egui::pos2(220., 140.));
        assert_eq!(app.ui.point_input.draft.as_ref().unwrap().points.len(), 2);
        assert!(app.session.doc().unwrap().geometry3d.is_empty());
        if std::env::var_os("WORLDWRIGHT_POINT_CAPTURE").is_some() {
            app.ui.point_input.draft.as_mut().unwrap().points.push(Vec3::new(8., -4., 0.));
            let capture = egui::Context::default();
            for _ in 0..2 {
                let mut output = capture.run_ui(
                    egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.))), ..Default::default() },
                    |ui| crate::buildercraft::viewport3d(&mut app, ui),
                );
                for delta in output.textures_delta.set.values().flatten() {
                    let egui::ImageData::Color(texture) = &delta.image;
                    image::RgbaImage::from_raw(
                        texture.size[0] as u32,
                        texture.size[1] as u32,
                        texture.pixels.iter().flat_map(|p| p.to_array()).collect(),
                    )
                    .unwrap()
                    .save("/tmp/worldwright-point-atlas.png")
                    .unwrap();
                }
                let meshes = capture
                    .tessellate(output.shapes, 1.)
                    .into_iter()
                    .filter_map(|p| {
                        let egui::epaint::Primitive::Mesh(m) = p.primitive else {
                            return None;
                        };
                        Some(json!({"clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],"indices":m.indices,
                        "vertices":m.vertices.iter().map(|v|json!([v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.to_array()])).collect::<Vec<_>>()}))
                    })
                    .collect::<Vec<_>>();
                std::fs::write("/tmp/worldwright-point-meshes.json", serde_json::to_vec(&meshes).unwrap()).unwrap();
                output.textures_delta.clear();
            }
        }
        key(&mut app, &ctx, egui::Key::Enter);
        assert!(!active(&app));
        assert_eq!(app.session.doc().unwrap().geometry3d.len(), 1);
        app.run("undo", json!({})).unwrap();
        assert!(app.session.doc().unwrap().geometry3d.is_empty());
        begin(&mut app);
        click(&mut app, &ctx, egui::pos2(150., 200.));
        key(&mut app, &ctx, egui::Key::Escape);
        assert!(!active(&app));
        assert!(app.session.doc().unwrap().geometry3d.is_empty());
        begin(&mut app);
        app.run("nurbs.controlcurve3d", json!({"points":[[0.,0.,20.],[10.,0.,20.]]})).unwrap();
        frame(&mut app, &ctx, vec![]);
        assert!(!active(&app));
        begin(&mut app);
        click(&mut app, &ctx, egui::pos2(200., 200.));
        assert_eq!(app.ui.point_input.draft.as_ref().unwrap().points[0].z, 20.);
        app.ui.plane3d = 1;
        frame(&mut app, &ctx, vec![]);
        assert!(!active(&app));
    }
}
