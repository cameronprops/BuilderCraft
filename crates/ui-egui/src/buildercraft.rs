//! BuilderCraft workspace: document-backed browser and docked command input.
use crate::CadApp;
use cadcraft_doc::organization::{ModelNode, NodeKind};
use serde_json::json;

pub fn workspace_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::top("buildercraft_workspace").exact_size(28.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong("BuilderCraft");
            if ui.selectable_value(&mut app.ui.view3d, true, "3D").clicked() {
                app.ui.toolset_tab = "Modeling".into();
            }
            if ui.selectable_value(&mut app.ui.view3d, false, "2D / Drafting").clicked() {
                app.ui.toolset_tab = "Drafting".into();
            }
            ui.selectable_value(&mut app.ui.buildercraft_workspace, true, "Modeling workspace");
            ui.selectable_value(&mut app.ui.buildercraft_workspace, false, "CADCraft drafting workspace");
        });
    });
}
pub fn command_panel(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::top("buildercraft_commands").exact_size(94.0).show(ui, |ui| {
        let lines: Vec<_> = app.session.log.iter().rev().take(2).rev().cloned().collect();
        for line in lines {
            ui.monospace(line);
        }
        let prompt = app.session.current_prompt().map(|p| p.message).unwrap_or_else(|| "Command".into());
        let mut submit = false;
        ui.horizontal(|ui| {
            ui.label(prompt);
            let response = ui.add(egui::TextEdit::singleline(&mut app.cmd.buffer).desired_width(f32::INFINITY));
            submit = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        });
        if submit {
            crate::cmdline::submit(app);
        }
    });
}
pub fn model_browser(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.heading("Model Browser");
    ui.text_edit_singleline(&mut app.ui.model_name);
    ui.horizontal(|ui| {
        for (label, kind) in [("Assembly", "assembly"), ("Component", "component"), ("Body", "body")] {
            if ui.button(label).clicked() {
                let _ = app.run("model.create", json!({"name": app.ui.model_name, "kind":kind}));
            }
        }
    });
    let nodes = app.session.doc().map(|d| d.organization.nodes.clone()).unwrap_or_default();
    if nodes.is_empty() {
        ui.label("Select geometry, then create a named body.");
    }
    for node in nodes.iter().filter(|n| n.parent.is_none()) {
        show_node(app, ui, &nodes, node, 0);
    }
    let objects = app.session.doc().map(|d| d.geometry3d.clone()).unwrap_or_default();
    for object in objects {
        if ui.selectable_label(app.session.selection().contains(&cadcraft_doc::Handle(object.id)), format!("3D body: {}", object.name)).clicked() {
            app.session.set_selection(vec![cadcraft_doc::Handle(object.id)]);
        }
        let mut name = object.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != object.name {
            let _ = app.run("geometry3d.set", json!({"id":object.id,"name":name}));
        }
        egui::CollapsingHeader::new("Control points").id_salt(("control", object.id)).show(ui, |ui| {
            let rows = match &object.shape {
                cadcraft_doc::organization::Shape::Curve(c) => vec![c.control.clone()],
                cadcraft_doc::organization::Shape::Surface(s) => s.rows.iter().map(|r| r.control.clone()).collect(),
            };
            for (row, points) in rows.iter().enumerate() {
                for (index, p) in points.iter().enumerate() {
                    let mut xyz = [p.x, p.y, p.z];
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label(format!("{row}:{index}"));
                        for x in &mut xyz {
                            changed |= ui.add(egui::DragValue::new(x).speed(0.1)).changed();
                        }
                    });
                    if changed {
                        let _ = app.run("geometry3d.controlpoint", json!({"id":object.id,"row":row,"index":index,"point":xyz}));
                    }
                }
            }
        });
        let mut visible = object.visible;
        if ui.checkbox(&mut visible, "Visible").changed() {
            let _ = app.run("geometry3d.set", json!({"id":object.id,"visible":visible}));
        }
    }
    transform_panel(app, ui);
    ui.separator();
}
fn show_node(app: &mut CadApp, ui: &mut egui::Ui, nodes: &[ModelNode], node: &ModelNode, depth: usize) {
    if depth >= 64 {
        ui.label("Tree depth limit reached");
        return;
    }
    let selected = app.session.selection();
    let active = !node.entities.is_empty() && node.entities.iter().any(|h| selected.contains(h));
    let label = format!("{:?}: {}", node.kind, node.name);
    egui::CollapsingHeader::new(label).id_salt(node.id).default_open(true).show(ui, |ui| {
        if ui.selectable_label(active, "Select contents").clicked() {
            let _ = app.run("model.select", json!({"id":node.id}));
        }
        let mut name = node.name.clone();
        let response = ui.text_edit_singleline(&mut name);
        if response.lost_focus() && name != node.name {
            let _ = app.run("model.rename", json!({"id":node.id,"name":name}));
        }
        ui.horizontal(|ui| {
            for (label, visible) in [("Show", true), ("Hide", false)] {
                if ui.button(label).clicked() {
                    let _ = app.run("model.visible", json!({"id":node.id,"visible":visible}));
                }
            }
            if node.kind != NodeKind::Body && ui.button("Add body from selection").clicked() {
                let _ = app.run("model.create", json!({"name":app.ui.model_name,"kind":"body","parent":node.id}));
            }
        });
        for child in nodes.iter().filter(|n| n.parent == Some(node.id)) {
            show_node(app, ui, nodes, child, depth + 1);
        }
    });
}

pub fn viewport3d(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Orthographic 3D").on_hover_text("Drag to orbit; Shift-drag to pan; scroll to zoom");
        if ui.button("New editable curve").clicked() {
            let _ = new_curve(app);
        }
        if ui.button("New control surface").clicked() {
            let _ = new_surface(app);
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (label, id) in [
            ("Top", "ui.buildercraft.top"),
            ("Front", "ui.buildercraft.front"),
            ("Right", "ui.buildercraft.right"),
            ("Isometric", "ui.buildercraft.iso"),
            ("Fit", "ui.buildercraft.fit"),
        ] {
            if ui.button(label).clicked() {
                let _ = app.run(id, json!({}));
            }
        }
    });
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());
    let rect = rect.intersect(ui.clip_rect());
    app.session.viewport_px = (f64::from(rect.width()), f64::from(rect.height()));
    if response.dragged() {
        let d = ui.input(|i| i.pointer.delta());
        if ui.input(|i| i.modifiers.shift) {
            let frame = cadcraft_geom::camera::OrthoFrame { yaw: app.ui.orbit_yaw, pitch: app.ui.orbit_pitch };
            if let Some(center) = frame.pan(app.ui.center3d, cadcraft_geom::Vec2::new(f64::from(d.x), f64::from(d.y)), app.ui.scale3d) {
                app.ui.center3d = center;
            }
        } else {
            app.ui.orbit_yaw = (app.ui.orbit_yaw + f64::from(d.x) * 0.01).rem_euclid(std::f64::consts::TAU);
            app.ui.orbit_pitch = (app.ui.orbit_pitch + f64::from(d.y) * 0.01).clamp(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2);
        }
    }
    if response.hovered() {
        let d = ui.input(|i| i.smooth_scroll_delta.y);
        app.ui.scale3d = (app.ui.scale3d * (f64::from(d) * 0.002).exp()).clamp(1e-9, 1e9);
    }
    let yaw = app.ui.orbit_yaw;
    let pitch = app.ui.orbit_pitch;
    let scale = app.ui.scale3d;
    let project = |p: cadcraft_geom::Vec3| {
        let q = cadcraft_geom::camera::OrthoFrame { yaw, pitch }.project(p, app.ui.center3d);
        egui::pos2(rect.center().x + (q.x * scale) as f32, rect.center().y - (q.y * scale) as f32)
    };
    let painter = ui.painter_at(rect);
    let line = |a, b, color| {
        painter.line_segment([project(a), project(b)], egui::Stroke::new(1., color));
    };
    for i in -20..=20 {
        let n = f64::from(i);
        line(cadcraft_geom::Vec3::new(n, -20., 0.), cadcraft_geom::Vec3::new(n, 20., 0.), egui::Color32::from_gray(55));
        line(cadcraft_geom::Vec3::new(-20., n, 0.), cadcraft_geom::Vec3::new(20., n, 0.), egui::Color32::from_gray(55));
    }
    line(cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(15., 0., 0.), egui::Color32::RED);
    line(cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(0., 15., 0.), egui::Color32::GREEN);
    line(cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(0., 0., 15.), egui::Color32::BLUE);
    if let Ok(d) = app.session.doc() {
        for object in &d.geometry3d {
            if !object.visible || d.layer(&object.layer).is_some_and(|l| !l.visible()) {
                continue;
            }
            match &object.shape {
                cadcraft_doc::organization::Shape::Curve(c) => {
                    for i in 0..96 {
                        if let Some((a, b)) = c.evaluate(f64::from(i) / 96.).zip(c.evaluate(f64::from(i + 1) / 96.)) {
                            line(a, b, egui::Color32::from_rgb(255, 210, 90));
                        }
                    }
                    for p in &c.control {
                        painter.circle_filled(project(*p), 3., egui::Color32::WHITE);
                    }
                }
                cadcraft_doc::organization::Shape::Surface(s) => {
                    for i in 0..=12 {
                        for j in 0..24 {
                            let a = f64::from(i) / 12.;
                            let b = f64::from(j) / 24.;
                            let c = f64::from(j + 1) / 24.;
                            for (p, q) in [s.evaluate(a, b).zip(s.evaluate(a, c)), s.evaluate(b, a).zip(s.evaluate(c, a))].into_iter().flatten() {
                                line(p, q, egui::Color32::from_rgb(90, 200, 240));
                            }
                        }
                    }
                }
            }
        }
    }
    crate::cmdline::keyboard(app, ui.ctx());
}

pub fn new_curve(app: &mut CadApp) -> Result<serde_json::Value, String> {
    app.ui.view3d = true;
    app.run("nurbs.curve3d",json!({"name":"3D curve","curve":{"degree":3,"control":[{"x":-10.,"y":0.,"z":0.},{"x":-5.,"y":8.,"z":12.},{"x":5.,"y":-8.,"z":12.},{"x":10.,"y":0.,"z":0.}],"weights":[1.,1.,1.,1.],"knots":[0.,0.,0.,0.,1.,1.,1.,1.]}}))
}
pub fn new_surface(app: &mut CadApp) -> Result<serde_json::Value, String> {
    app.ui.view3d = true;
    let row = |y: f64, z: f64| json!({"degree":2,"control":[{"x":-10.,"y":y,"z":0.},{"x":0.,"y":y,"z":z},{"x":10.,"y":y,"z":0.}],"weights":[1.,1.,1.],"knots":[0.,0.,0.,1.,1.,1.]});
    app.run(
        "nurbs.surface",
        json!({"name":"Control surface","surface":{"rows":[row(-10.,0.),row(0.,18.),row(10.,0.)],"degree_v":2,"knots_v":[0.,0.,0.,1.,1.,1.]}}),
    )
}

/// UI camera commands do not change drawing geometry or document undo history.
pub fn camera_command(app: &mut CadApp, id: &str) -> Result<serde_json::Value, String> {
    let angles = match id {
        "ui.buildercraft.top" => Some((0., -std::f64::consts::FRAC_PI_2)),
        "ui.buildercraft.front" => Some((0., 0.)),
        "ui.buildercraft.right" => Some((-std::f64::consts::FRAC_PI_2, 0.)),
        "ui.buildercraft.iso" => Some((-std::f64::consts::FRAC_PI_4, -(1.0_f64 / 3.0_f64.sqrt()).asin())),
        _ => None,
    };
    if let Some((yaw, pitch)) = angles {
        app.ui.orbit_yaw = yaw;
        app.ui.orbit_pitch = pitch;
    } else if id == "ui.buildercraft.fit" {
        let d = app.session.doc().map_err(|e| e.to_string())?;
        let points = d.geometry3d.iter().filter(|o| o.visible && d.layer(&o.layer).is_none_or(|l| l.visible())).flat_map(|o| {
            let rows: Box<dyn Iterator<Item = &cadcraft_geom::nurbs3d::Curve> + '_> = match &o.shape {
                cadcraft_doc::organization::Shape::Curve(c) => Box::new(std::iter::once(c.as_ref())),
                cadcraft_doc::organization::Shape::Surface(s) => Box::new(s.rows.iter()),
            };
            rows.flat_map(|c| c.control.iter().copied())
        });
        let frame = cadcraft_geom::camera::OrthoFrame { yaw: app.ui.orbit_yaw, pitch: app.ui.orbit_pitch };
        let (center, scale) = frame
            .fit(points, app.session.viewport_px.0, app.session.viewport_px.1)
            .ok_or_else(|| "No finite visible control hull to fit, or viewport/control budget exceeded".to_string())?;
        app.ui.center3d = center;
        app.ui.scale3d = scale;
    } else {
        return Err("Unknown camera command".into());
    }
    app.ui.view3d = true;
    Ok(json!({"projection":"orthographic","center":[app.ui.center3d.x,app.ui.center3d.y,app.ui.center3d.z],"scale":app.ui.scale3d}))
}

fn transform_panel(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Transform selected 3D objects").default_open(true).show(ui, |ui| {
        let ids = app.session.selection().iter().map(|h| h.0).collect::<Vec<_>>();
        ui.label(format!("Selected objects: {}", ids.len()));
        let fields = |ui: &mut egui::Ui, label: &str, values: &mut [f64; 3]| {
            ui.label(label);
            ui.horizontal_wrapped(|ui| {
                for (axis, value) in ["X", "Y", "Z"].into_iter().zip(values) {
                    ui.add(egui::DragValue::new(value).speed(0.1).prefix(format!("{axis}: ")).max_decimals(4));
                }
            });
        };
        ui.checkbox(&mut app.ui.transform_copy, "Copy geometry (new IDs, unassigned copies)");
        fields(ui, "Move delta", &mut app.ui.transform_delta);
        let mut operation = None;
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Move selected")).clicked() {
            operation = Some(json!({"kind":"move","delta":app.ui.transform_delta}));
        }
        fields(ui, "Origin / plane point", &mut app.ui.transform_origin);
        fields(ui, "Rotation axis / mirror normal", &mut app.ui.transform_axis);
        ui.add(egui::DragValue::new(&mut app.ui.transform_angle).prefix("Angle °: ").speed(1.));
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Rotate selected")).clicked() {
            operation =
                Some(json!({"kind":"rotate","origin":app.ui.transform_origin,"axis":app.ui.transform_axis,"angle_degrees":app.ui.transform_angle}));
        }
        ui.add(egui::DragValue::new(&mut app.ui.transform_factor).prefix("Scale: ").speed(0.01));
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Scale selected")).clicked() {
            operation = Some(json!({"kind":"scale","origin":app.ui.transform_origin,"factor":app.ui.transform_factor}));
        }
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Mirror selected")).clicked() {
            operation = Some(json!({"kind":"mirror","origin":app.ui.transform_origin,"normal":app.ui.transform_axis}));
        }
        if let Some(operation) = operation {
            let _ = app.run("geometry3d.transform", json!({"ids":ids,"operation":operation,"copy":app.ui.transform_copy}));
        }
        ui.small("Exact curves/control surfaces only. World coordinates; numeric controls, no gumball yet.");
    });
}
