//! World-axis handles. Preview matrices never mutate the document.
use crate::CadApp;
use cadcraft_geom::{Mat4, Vec3};
use serde_json::json;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Mode {
    #[default]
    Move,
    Rotate,
    Scale,
}
#[derive(Clone, Debug, Default)]
pub struct Gizmo {
    mode: Mode,
    drag: Option<Drag>,
}
#[derive(Clone, Debug)]
struct Drag {
    ids: Vec<u64>,
    uid: u64,
    revision: u64,
    camera: [f64; 6],
    axis: Vec3,
    origin: Vec3,
    screen_axis: egui::Vec2,
    start: egui::Pos2,
    center: egui::Pos2,
    operation: buildercraft_kernel::Transform,
    copy: bool,
    rect: egui::Rect,
}
fn array(v: Vec3) -> [f64; 3] {
    [v.x, v.y, v.z]
}
fn camera(app: &CadApp) -> [f64; 6] {
    [app.ui.orbit_yaw, app.ui.orbit_pitch, app.ui.scale3d, app.ui.center3d.x, app.ui.center3d.y, app.ui.center3d.z]
}
pub fn controls(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Gizmo");
        for (label, mode) in [("Move", Mode::Move), ("Rotate", Mode::Rotate), ("Scale", Mode::Scale)] {
            if ui.selectable_value(&mut app.ui.gizmo.mode, mode, label).changed() {
                app.ui.gizmo.drag = None;
            }
        }
        ui.small("Click objects to select. Drag colored handles; Esc cancels.");
    });
}
pub fn preview(app: &CadApp, id: u64) -> Option<Mat4> {
    let drag = app.ui.gizmo.drag.as_ref()?;
    drag.ids.contains(&id).then(|| drag.operation.matrix().ok()).flatten()
}
pub fn cancel(app: &mut CadApp) {
    app.ui.gizmo.drag = None;
}
pub fn interact(app: &mut CadApp, ui: &egui::Ui, rect: egui::Rect, response: &egui::Response) -> bool {
    if app.ui.command_search.open {
        return true;
    }
    if app.session.selection().len() > 1024 {
        app.ui.gizmo.drag = None;
        return false;
    }
    let ids: Vec<_> = app.session.selection().iter().map(|h| h.0).collect();
    let Ok(state) = app.session.state() else {
        app.ui.gizmo.drag = None;
        return false;
    };
    let (uid, revision) = (state.uid, state.revision);
    let cam = camera(app);
    if !cam.iter().all(|v| v.is_finite()) || cam[2] <= 0. {
        app.ui.gizmo.drag = None;
        return false;
    }
    let was_dragging = app.ui.gizmo.drag.is_some();
    if ui.input(|i| i.key_pressed(egui::Key::Escape))
        || app.ui.gizmo.drag.as_ref().is_some_and(|d| d.ids != ids || d.uid != uid || d.revision != revision || d.camera != cam || d.rect != rect)
    {
        app.ui.gizmo.drag = None;
        return was_dragging;
    }
    // Bounded control-hull center, deliberately not an exact geometric centroid.
    let Ok(doc) = app.session.doc() else {
        return false;
    };
    let mut lo = Vec3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
    let mut hi = Vec3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    let mut count = 0usize;
    let mut objects = 0usize;
    for object in &doc.geometry3d {
        if !ids.contains(&object.id) {
            continue;
        }
        if !object.visible || doc.layer(&object.layer).is_some_and(|l| !l.visible() || l.locked) {
            app.ui.gizmo.drag = None;
            return was_dragging;
        }
        objects += 1;
        let rows: Vec<&[Vec3]> = match &object.shape {
            cadcraft_doc::organization::Shape::Curve(c) => vec![&c.control],
            cadcraft_doc::organization::Shape::Surface(s) => s.rows.iter().map(|r| r.control.as_slice()).collect(),
        };
        for p in rows.into_iter().flatten() {
            count += 1;
            if count > 100_000 || !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
                app.ui.gizmo.drag = None;
                return was_dragging;
            }
            lo = Vec3::new(lo.x.min(p.x), lo.y.min(p.y), lo.z.min(p.z));
            hi = Vec3::new(hi.x.max(p.x), hi.y.max(p.y), hi.z.max(p.z));
        }
    }
    if count == 0 || objects != ids.len() {
        app.ui.gizmo.drag = None;
        return was_dragging;
    }
    let origin = lo * 0.5 + hi * 0.5;
    let frame = cadcraft_geom::camera::OrthoFrame { yaw: cam[0], pitch: cam[1] };
    let project = |p| {
        let q = frame.project(p, app.ui.center3d);
        rect.center() + egui::vec2((q.x * cam[2]) as f32, (-q.y * cam[2]) as f32)
    };
    let center = project(origin);
    let painter = ui.painter_at(rect);
    let mode = app.ui.gizmo.mode;
    let mut hit = None;
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let pick = if response.drag_started() { ui.input(|i| i.pointer.press_origin()) } else { pointer };
    for (axis, color) in
        [(Vec3::new(1., 0., 0.), egui::Color32::RED), (Vec3::new(0., 1., 0.), egui::Color32::GREEN), (Vec3::Z, egui::Color32::LIGHT_BLUE)]
    {
        let direction = project(origin + axis * (70. / cam[2])) - center;
        if mode != Mode::Rotate && direction.length_sq() < 100. {
            continue;
        }
        if mode == Mode::Rotate {
            let u = if axis == Vec3::new(1., 0., 0.) { Vec3::new(0., 1., 0.) } else { Vec3::new(1., 0., 0.) };
            let v = axis.cross(u);
            let a = frame.project(u, Vec3::ZERO);
            let b = frame.project(v, Vec3::ZERO);
            if (a.x * b.y - a.y * b.x).abs() < 1e-6 {
                continue;
            }
            for i in 0..64 {
                let point = |n: f64| {
                    let t = n * std::f64::consts::TAU / 64.;
                    project(origin + (u * t.cos() + v * t.sin()) * (55. / cam[2]))
                };
                let (a, b) = (point(f64::from(i)), point(f64::from(i + 1)));
                painter.line_segment([a, b], egui::Stroke::new(2., color));
                if pick.is_some_and(|p| distance(p, a, b) < 6.) {
                    hit = Some((axis, direction));
                }
            }
        } else {
            let end = center + direction;
            painter.line_segment([center, end], egui::Stroke::new(3., color));
            painter.circle_filled(end, 6., color);
            if pick.is_some_and(|p| distance(p, center + direction * 0.25, end) < 8.) {
                hit = Some((axis, direction));
            }
        }
    }
    if response.drag_started()
        && let Some((axis, screen_axis)) = hit
        && let Some(start) = ui.input(|i| i.pointer.press_origin())
    {
        app.ui.gizmo.drag = Some(Drag {
            ids,
            uid,
            revision,
            camera: cam,
            axis,
            origin,
            screen_axis,
            start,
            center,
            operation: buildercraft_kernel::Transform::Move { delta: [0.; 3] },
            copy: app.ui.transform_copy,
            rect,
        });
    }
    if let Some(d) = &mut app.ui.gizmo.drag {
        if let Some(p) = pointer {
            d.operation = operation(mode, d, p);
        }
        if response.drag_stopped()
            && let Some(d) = app.ui.gizmo.drag.take()
        {
            let _ = app.run("geometry3d.transform", json!({"ids":d.ids,"operation":d.operation,"copy":d.copy}));
        }
        ui.ctx().request_repaint();
        return true;
    }
    was_dragging || (hit.is_some() && response.clicked())
}
fn distance(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let v = b - a;
    let t = if v.length_sq() > 0. { (p - a).dot(v) / v.length_sq() } else { 0. };
    p.distance(a + v * t.clamp(0., 1.))
}
fn operation(mode: Mode, d: &Drag, p: egui::Pos2) -> buildercraft_kernel::Transform {
    let units = f64::from((p - d.start).dot(d.screen_axis) / d.screen_axis.length_sq()) * 70. / d.camera[2];
    match mode {
        Mode::Move => buildercraft_kernel::Transform::Move { delta: array(d.axis * units) },
        Mode::Scale => {
            buildercraft_kernel::Transform::Scale { origin: array(d.origin), factor: (1. + units * d.camera[2] / 70.).clamp(0.001, 1000.) }
        }
        Mode::Rotate => {
            // Invert the projected rotation plane instead of assuming circular screen rings.
            let frame = cadcraft_geom::camera::OrthoFrame { yaw: d.camera[0], pitch: d.camera[1] };
            let u = if d.axis == Vec3::new(1., 0., 0.) { Vec3::new(0., 1., 0.) } else { Vec3::new(1., 0., 0.) };
            let v = d.axis.cross(u);
            let a = frame.project(u, Vec3::ZERO);
            let b = frame.project(v, Vec3::ZERO);
            let det = a.x * b.y - a.y * b.x;
            let angle = |p: egui::Pos2| {
                let x = f64::from(p.x - d.center.x);
                let y = -f64::from(p.y - d.center.y);
                ((a.x * y - a.y * x) / det).atan2((x * b.y - y * b.x) / det)
            };
            let degrees = if det.abs() > 1e-6 { (angle(p) - angle(d.start)).to_degrees() } else { 0. };
            buildercraft_kernel::Transform::Rotate { origin: array(d.origin), axis: array(d.axis), angle_degrees: degrees }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn drag() -> Drag {
        Drag {
            ids: vec![1],
            uid: 1,
            revision: 1,
            camera: [0., 0., 10., 0., 0., 0.],
            axis: Vec3::new(1., 0., 0.),
            origin: Vec3::ZERO,
            screen_axis: egui::vec2(70., 0.),
            start: egui::pos2(0., 0.),
            center: egui::pos2(0., 0.),
            operation: buildercraft_kernel::Transform::Move { delta: [0.; 3] },
            copy: false,
            rect: egui::Rect::NOTHING,
        }
    }
    #[test]
    fn projected_axis_distance_and_positive_scale() {
        let d = drag();
        let m = operation(Mode::Move, &d, egui::pos2(35., 99.)).matrix().unwrap();
        assert_eq!(m.apply(Vec3::ZERO), Vec3::new(3.5, 0., 0.));
        let m = operation(Mode::Scale, &d, egui::pos2(-700., 0.)).matrix().unwrap();
        assert!((m.apply(Vec3::new(1., 0., 0.)).x - 0.001).abs() < 1e-12);
    }
    #[test]
    fn rotation_uses_projected_plane_and_preserves_radius() {
        let mut d = drag();
        d.axis = Vec3::new(0., 1., 0.);
        d.start = egui::pos2(55., 0.);
        let m = operation(Mode::Rotate, &d, egui::pos2(0., 55.)).matrix().unwrap();
        let p = m.apply(Vec3::new(1., 0., 0.));
        assert!(p.x.abs() < 1e-12 && (p.z + 1.).abs() < 1e-12);
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    fn fixture() -> CadApp {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let r=app.run("nurbs.curve3d",json!({"name":"Gizmo fixture","curve":{"degree":1,"control":[{"x":-1.,"y":0.,"z":0.},{"x":1.,"y":0.,"z":0.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}})).unwrap();
        app.session.set_selection(vec![cadcraft_doc::Handle(r["id"].as_u64().unwrap())]);
        app.ui.orbit_yaw = 0.;
        app.ui.orbit_pitch = 0.;
        app.ui.scale3d = 10.;
        app
    }
    fn frame(app: &mut CadApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input =
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 400.))), events, ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(400., 400.), egui::Sense::drag());
            interact(app, ui, rect, &response);
        });
        if std::env::var_os("WORLDWRIGHT_GIZMO_CAPTURE").is_some() {
            for delta in output.textures_delta.set.values().flatten() {
                let egui::ImageData::Color(texture) = &delta.image;
                image::RgbaImage::from_raw(
                    texture.size[0] as u32,
                    texture.size[1] as u32,
                    texture.pixels.iter().flat_map(|p| p.to_array()).collect(),
                )
                .unwrap()
                .save("/tmp/worldwright-gizmo-atlas.png")
                .unwrap();
            }
            let meshes = ctx.tessellate(output.shapes, 1.).into_iter().filter_map(|p| {
                let egui::epaint::Primitive::Mesh(m)=p.primitive else {return None;};
                Some(json!({"indices":m.indices,"vertices":m.vertices.iter().map(|v|json!([v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.to_array()])).collect::<Vec<_>>()}))
            }).collect::<Vec<_>>();
            std::fs::write(format!("/tmp/worldwright-gizmo-{:?}.json", app.ui.gizmo.mode), serde_json::to_vec(&meshes).unwrap()).unwrap();
        }
        output.textures_delta.clear();
    }
    fn pointer(p: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
        ]
    }
    #[test]
    fn drag_preview_commit_and_undo() {
        for mode in [Mode::Move, Mode::Rotate, Mode::Scale] {
            let mut app = fixture();
            app.ui.gizmo.mode = mode;
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![]);
            frame(&mut app, &ctx, vec![]);
            let before = app.session.doc().unwrap().geometry3d.clone();
            let start = if mode == Mode::Rotate { egui::pos2(255., 200.) } else { egui::pos2(260., 200.) };
            let end = if mode == Mode::Rotate { egui::pos2(200., 255.) } else { egui::pos2(295., 200.) };
            frame(&mut app, &ctx, pointer(start, true));
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
            assert!(app.ui.gizmo.drag.is_some(), "{mode:?}");
            assert_eq!(app.session.doc().unwrap().geometry3d, before);
            frame(&mut app, &ctx, pointer(end, false));
            assert!(app.ui.gizmo.drag.is_none());
            assert_ne!(app.session.doc().unwrap().geometry3d, before, "{mode:?}");
            app.run("undo", json!({})).unwrap();
            assert_eq!(app.session.doc().unwrap().geometry3d, before);
        }
    }
    #[test]
    fn camera_revision_and_escape_discard_preview() {
        for cause in 0..3 {
            let mut app = fixture();
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![]);
            frame(&mut app, &ctx, vec![]);
            frame(&mut app, &ctx, pointer(egui::pos2(260., 200.), true));
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(egui::pos2(295., 200.))]);
            assert!(app.ui.gizmo.drag.is_some());
            let before = app.session.doc().unwrap().geometry3d.clone();
            let events = match cause {
                0 => {
                    app.ui.scale3d = 20.;
                    vec![]
                }
                1 => {
                    app.session.state_mut().unwrap().revision += 1;
                    vec![]
                }
                _ => {
                    vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }]
                }
            };
            frame(&mut app, &ctx, events);
            assert!(app.ui.gizmo.drag.is_none());
            assert_eq!(app.session.doc().unwrap().geometry3d, before);
        }
    }
}
