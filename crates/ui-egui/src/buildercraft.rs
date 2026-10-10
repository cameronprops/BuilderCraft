//! BuilderCraft workspace: document-backed browser and docked command input.
use crate::CadApp;
use cadcraft_doc::organization::{ModelNode, NodeKind};
use serde_json::json;

pub use crate::brep_display::BrepPreview;

fn load_exact_brep_preview(app: &mut CadApp, id: u64) -> Result<(), String> {
    let state = app.session.state().map_err(|e| e.to_string())?;
    let (document_uid, revision) = (state.uid, state.revision);
    let reply = app.run("brep.preview", json!({"id":id,"linear_deflection":0.15,"angular_deflection":0.4}))?;
    let preview = BrepPreview::from_worker(&reply, document_uid, revision, id)?;
    if !app.session.state().is_ok_and(|st| preview.current(st.uid, st.revision)) {
        return Err("BRep source changed while generating shaded preview".into());
    }
    app.ui.brep_preview = Some(preview);
    Ok(())
}
pub fn workspace_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::top("buildercraft_workspace").exact_size(28.0).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong("Worldwright");
            if ui.selectable_value(&mut app.ui.view3d, true, "3D").clicked() {
                app.ui.toolset_tab = "Modeling".into();
            }
            if ui.selectable_value(&mut app.ui.view3d, false, "2D / Drafting").clicked() {
                app.ui.toolset_tab = "Drafting".into();
            }
            ui.menu_button("Workspace", |ui| {
                for (label, id) in [
                    ("Modeling", "ui.workspace.modeling"),
                    ("Drafting", "ui.workspace.drafting"),
                    ("Focus", "ui.workspace.focus"),
                    ("Save custom layout", "ui.workspace.save"),
                    ("Restore custom layout", "ui.workspace.restore"),
                ] {
                    if ui.button(label).clicked() {
                        app.start(id);
                        ui.close();
                    }
                }
            });
            if ui.button("Search commands  Ctrl/Cmd+K").clicked() {
                app.start("ui.command.search");
            }
            ui.weak("ALPHA").on_hover_text("Exact solid booleans and the seven OrbWeaver demo graphs still require acceptance validation.");
        });
    });
}
pub fn command_panel(app: &mut CadApp, ui: &mut egui::Ui) {
    let panel = egui::Panel::bottom("buildercraft_commands")
        .default_size(app.ui.layout.command_height)
        .size_range(60.0..=180.0)
        .resizable(true)
        .show(ui, |ui| {
            // Retain the allocated height even when the command log is empty.
            // Otherwise content sizing immediately shrinks a restored layout.
            ui.set_min_height(ui.available_height());
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
    app.ui.layout.command_height = panel.response.rect.height();
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
    let meshes = app.session.doc().map(|d| d.mesh3d.clone()).unwrap_or_default();
    for object in meshes {
        let selected = app.session.selection().contains(&cadcraft_doc::Handle(object.id));
        if ui.selectable_label(selected, format!("Polygon mesh: {}", object.name)).clicked() {
            app.session.set_selection(vec![cadcraft_doc::Handle(object.id)]);
            clear_picked_mesh_face(app);
        }
        let mut name = object.name.clone();
        if ui.text_edit_singleline(&mut name).lost_focus() && name != object.name {
            let _ = app.run("mesh3d.set", json!({"id":object.id,"name":name}));
        }
        let mut visible = object.visible;
        if ui.checkbox(&mut visible, "Visible").changed() {
            let _ = app.run("mesh3d.set", json!({"id":object.id,"visible":visible}));
        }
        egui::CollapsingHeader::new("Polygon mesh repair").id_salt(("mesh", object.id)).show(ui, |ui| {
            ui.label(format!("{} vertices, {} native faces", object.mesh.vertices.len(), object.mesh.faces.len()));
            ui.small("Click a visible polygon face in the 3D viewport, or enter its index.");
            let live_revision = app.session.state().ok().map(|s| s.revision);
            if app.ui.mesh_face_object_id == Some(object.id) && app.ui.mesh_face_revision.is_some() {
                if picked_face_is_current(app, object.id) {
                    ui.label(format!("Viewport pick: face {}", app.ui.mesh_face_index));
                } else {
                    ui.label("Viewport pick is stale; click the mesh again.");
                }
            }
            ui.horizontal(|ui| {
                ui.label("Face:");
                if ui.add(egui::DragValue::new(&mut app.ui.mesh_face_index).range(0..=object.mesh.faces.len().saturating_sub(1) as u32)).changed() {
                    clear_picked_mesh_face(app);
                }
                let stale_pick = app.ui.mesh_face_object_id == Some(object.id) && !picked_face_is_current(app, object.id);
                if ui.add_enabled(!object.mesh.faces.is_empty() && !stale_pick, egui::Button::new("Delete face")).clicked()
                    && let Some(now) = live_revision
                {
                    let revision = if picked_face_is_current(app, object.id) { app.ui.mesh_face_revision.unwrap_or(now) } else { now };
                    let _ = delete_mesh_face(app, object.id, revision, app.ui.mesh_face_index);
                }
            });
            if let Ok(report) = buildercraft_kernel::polygon_mesh_boundary_loops(&object.mesh) {
                if !report.unresolved_edges.is_empty() {
                    ui.label("Some boundary edges are ambiguous; repair these before hole filling.");
                }
                for (index, loop_data) in report.closed_loops.iter().enumerate().take(16) {
                    if ui.button(format!("Try planar patch on loop {} ({} vertices)", index, loop_data.vertices.len())).clicked()
                        && let Ok(state) = app.session.state()
                    {
                        let revision = state.revision;
                        let _ = app.run(
                            "mesh3d.edit",
                            json!({
                                "id":object.id,
                                "edit":{
                                    "kind":"fill_planar_hole",
                                    "selected_revision":revision,
                                    "loop_index":index
                                }
                            }),
                        );
                    }
                }
            }
        });
    }
    // Exact trimmed solids remain separate from their disposable viewport proxies.
    // Until the shaded renderer has a stable revision-aware topology pick map,
    // expose safe document inspection without implying geometric subobject edits.
    let breps = app.session.doc().map(|d| d.exact_breps.clone()).unwrap_or_default();
    if !breps.is_empty() {
        ui.separator();
        ui.heading("Exact BRep solids");
        for object in breps {
            let selected = app.session.selection().contains(&cadcraft_doc::Handle(object.id));
            if ui.selectable_label(selected, format!("Solid: {}", object.name)).clicked() {
                app.session.set_selection(vec![cadcraft_doc::Handle(object.id)]);
            }
            ui.label(format!("{:.6} volume units³ | {} faces | {} edges", object.volume, object.faces, object.edges));
            let mut title = object.name.clone();
            if ui.text_edit_singleline(&mut title).lost_focus() && title != object.name {
                let _ = app.run("brep.set", json!({"id": object.id, "name": title}));
            }
            let mut visible = object.visible;
            if ui.checkbox(&mut visible, "Visible").changed() {
                let _ = app.run("brep.set", json!({"id": object.id, "visible": visible}));
            }
            if ui.button("Validate native topology").clicked() {
                let _ = app.run("brep.inspect", json!({"id": object.id}));
            }
            if ui.button("Load shaded BRep preview").clicked()
                && let Err(error) = load_exact_brep_preview(app, object.id)
            {
                app.session.echo(error);
            }
        }
        ui.small("Shaded display and whole-solid picks are disposable OCCT proxies. Wire edges use X-ray overlay; mixed-scene depth and stable face picking remain future gates.");
    }
    transform_panel(app, ui);
    crate::feature_history::panel(app, ui);
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

fn clear_picked_mesh_face(app: &mut CadApp) {
    app.ui.mesh_face_object_id = None;
    app.ui.mesh_face_document_uid = None;
    app.ui.mesh_face_revision = None;
}

fn picked_face_is_current(app: &CadApp, object_id: u64) -> bool {
    app.ui.mesh_face_object_id == Some(object_id)
        && app
            .session
            .state()
            .is_ok_and(|state| app.ui.mesh_face_document_uid == Some(state.uid) && app.ui.mesh_face_revision == Some(state.revision))
}

fn delete_mesh_face(app: &mut CadApp, object_id: u64, selected_revision: u64, face_index: u32) -> Result<serde_json::Value, String> {
    let result = app.run(
        "mesh3d.edit",
        json!({
            "id": object_id,
            "edit": {
                "kind": "delete_faces",
                "selected_revision": selected_revision,
                "selected_faces": [face_index]
            }
        }),
    )?;
    clear_picked_mesh_face(app);
    Ok(result)
}

/// Select a native mesh face or fall back to the exact-NURBS wire picker.
/// Selection is transient; a mesh-face edit remains bound to its source revision.
fn select_3d_at(app: &mut CadApp, rect: egui::Rect, pointer: egui::Pos2, toggle: bool) {
    let offset = cadcraft_geom::Vec2::new(f64::from(pointer.x - rect.center().x), f64::from(rect.center().y - pointer.y));
    let camera = cadcraft_geom::camera::OrthoFrame { yaw: app.ui.orbit_yaw, pitch: app.ui.orbit_pitch };
    let picked = app.session.doc().ok().and_then(|d| {
        crate::mesh_picking::pick_visible_mesh_face(
            d.mesh3d
                .iter()
                .filter(|o| o.visible && d.layer(&o.layer).is_none_or(|layer| layer.visible() && !layer.locked))
                .map(|o| (o.id, o.mesh.as_ref())),
            camera,
            app.ui.center3d,
            app.ui.scale3d,
            offset,
        )
    });
    // Compare the cached BRep proxy against native mesh hits using the same
    // orthographic barycentric depth calculation. Mesh wins an exact tie.
    let brep_hit = app.ui.brep_preview.as_ref().and_then(|cache| {
        let state = app.session.state().ok()?;
        if !cache.current(state.uid, state.revision) {
            return None;
        }
        let document = app.session.doc().ok()?;
        let solid = document.exact_breps.iter().find(|o| o.id == cache.object_id)?;
        if !solid.visible || document.layer(&solid.layer).is_some_and(|l| !l.visible() || l.locked) {
            return None;
        }
        cache.pick_depth(camera, app.ui.center3d, app.ui.scale3d, offset).map(|depth| (solid.id, depth))
    });
    if let Some((id, depth)) = brep_hit
        && picked.is_none_or(|mesh| depth > mesh.depth + 1e-9)
    {
        let handle = cadcraft_doc::Handle(id);
        let mut selection = if toggle { app.session.selection() } else { Vec::new() };
        if toggle && selection.contains(&handle) {
            selection.retain(|current| *current != handle);
        } else {
            selection.push(handle);
        }
        app.session.set_selection(selection);
        clear_picked_mesh_face(app);
        return;
    }
    if let Some(hit) = picked {
        let handle = cadcraft_doc::Handle(hit.object_id);
        let mut selection = if toggle { app.session.selection() } else { Vec::new() };
        if toggle && selection.contains(&handle) {
            selection.retain(|selected| *selected != handle);
            app.session.set_selection(selection);
            clear_picked_mesh_face(app);
            return;
        }
        selection.push(handle);
        app.session.set_selection(selection);
        app.ui.mesh_face_object_id = Some(hit.object_id);
        app.ui.mesh_face_index = hit.face_index;
        if let Ok(state) = app.session.state() {
            app.ui.mesh_face_document_uid = Some(state.uid);
            app.ui.mesh_face_revision = Some(state.revision);
        }
    } else {
        clear_picked_mesh_face(app);
        pick_at(app, rect, pointer, toggle);
    }
}

pub fn viewport3d(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Orthographic 3D").on_hover_text("Click to select; Shift-click to toggle; drag to orbit; Shift-drag to pan; scroll to zoom");
        if ui.button("Draw control curve").clicked() {
            let _ = app.run("ui.buildercraft.drawcurve", json!({}));
        }
        if ui.button("New control surface").clicked() {
            let _ = new_surface(app);
        }
        if ui.button("New editable mesh").clicked() {
            let _ = new_mesh_sample(app);
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
        if let (Some(object_id), Some(picked_revision)) = (app.ui.mesh_face_object_id, app.ui.mesh_face_revision) {
            let fresh = picked_face_is_current(app, object_id);
            ui.label(if fresh { format!("Mesh {object_id} · face {}", app.ui.mesh_face_index) } else { "Mesh face selection is stale".into() });
            if ui.add_enabled(fresh, egui::Button::new("Delete picked face")).clicked() {
                let _ = delete_mesh_face(app, object_id, picked_revision, app.ui.mesh_face_index);
            }
        }
    });
    crate::point_input::controls(app, ui);
    if !crate::point_input::active(app) {
        crate::gizmo::controls(app, ui);
    }
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    let rect = rect.intersect(ui.clip_rect());
    app.session.viewport_px = (f64::from(rect.width()), f64::from(rect.height()));
    let point_input = crate::point_input::interact(app, ui, rect, &response);
    let gizmo_drag = !point_input && crate::gizmo::interact(app, ui, rect, &response);
    if response.dragged() && !gizmo_drag {
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
    if response.hovered() && !gizmo_drag {
        let d = ui.input(|i| i.smooth_scroll_delta.y);
        app.ui.scale3d = (app.ui.scale3d * (f64::from(d) * 0.002).exp()).clamp(1e-9, 1e9);
    }
    // Mesh and NURBS picking share click / Shift-click semantics.
    if response.clicked()
        && !point_input
        && !gizmo_drag
        && let Some(pointer) = response.interact_pointer_pos().filter(|p| rect.contains(*p))
    {
        select_3d_at(app, rect, pointer, ui.input(|i| i.modifiers.shift));
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
    if let Some(plane) = crate::point_input::plane(app) {
        let grid = crate::theme::Tokens::get().grid_major;
        let (u, v) = (plane.x_axis, plane.y_axis);
        for i in -20..=20 {
            let n = f64::from(i);
            line(plane.origin + u * n - v * 20., plane.origin + u * n + v * 20., grid);
            line(plane.origin + v * n - u * 20., plane.origin + v * n + u * 20., grid);
        }
    }
    line(cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(15., 0., 0.), egui::Color32::RED);
    line(cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(0., 15., 0.), egui::Color32::GREEN);
    line(cadcraft_geom::Vec3::ZERO, cadcraft_geom::Vec3::new(0., 0., 15.), egui::Color32::BLUE);
    let mut work = 50_000_000;
    let mut preview_limited = false;
    let selected_ids: std::collections::HashSet<_> = app.session.selection().into_iter().collect();
    if let Ok(d) = app.session.doc() {
        preview_limited = d.geometry3d.len() > 4096;
        for object in d.geometry3d.iter().take(4096) {
            if !object.visible || d.layer(&object.layer).is_some_and(|l| !l.visible()) {
                continue;
            }
            let matrix = crate::gizmo::preview(app, object.id).unwrap_or(cadcraft_geom::Mat4::IDENTITY);
            let project = |p| project(matrix.apply(p));
            let line = |a, b, color| {
                painter.line_segment([project(a), project(b)], egui::Stroke::new(1., color));
            };
            let selected = selected_ids.contains(&cadcraft_doc::Handle(object.id));
            let color = if selected {
                egui::Color32::from_rgb(255, 130, 40)
            } else {
                match &object.shape {
                    cadcraft_doc::organization::Shape::Curve(_) => egui::Color32::from_rgb(255, 210, 90),
                    cadcraft_doc::organization::Shape::Surface(_) => egui::Color32::from_rgb(90, 200, 240),
                }
            };
            if buildercraft_kernel::visit_preview_wires(&object.shape, &mut work, |a, b| line(a, b, color)).is_err() {
                preview_limited = true;
                break;
            }
            if let cadcraft_doc::organization::Shape::Curve(c) = &object.shape {
                for p in c.control.iter().take(4096) {
                    painter.circle_filled(project(*p), 3., egui::Color32::WHITE);
                }
            }
        }
        // The face budget applies to the whole visible mesh scene, matching
        // mesh_picking exactly. No hidden face should remain interactive.
        let mut mesh_faces_remaining = crate::mesh_picking::MAX_VIEWPORT_FACES;
        for object in &d.mesh3d {
            if !object.visible || d.layer(&object.layer).is_some_and(|l| !l.visible()) {
                continue;
            }
            let visible_faces = mesh_faces_remaining.min(object.mesh.faces.len());
            mesh_faces_remaining -= visible_faces;
            if visible_faces < object.mesh.faces.len() {
                preview_limited = true;
            }
            let picked = app.session.selection().contains(&cadcraft_doc::Handle(object.id));
            let color = if picked { egui::Color32::from_rgb(255, 200, 75) } else { egui::Color32::from_rgb(110, 230, 180) };
            let selected_face_is_current = picked_face_is_current(app, object.id);
            for (face_index, face) in object.mesh.faces.iter().take(visible_faces).enumerate() {
                let highlighted = selected_face_is_current && app.ui.mesh_face_index as usize == face_index;
                let color = if highlighted { egui::Color32::from_rgb(255, 245, 80) } else { color };
                let stroke = egui::Stroke::new(if highlighted { 3.0 } else { 1.0 }, color);
                let corners = face.indices();
                for side in 0..corners.len() {
                    let a = object.mesh.vertices[corners[side] as usize];
                    let b = object.mesh.vertices[corners[(side + 1) % corners.len()] as usize];
                    painter.line_segment([project(a), project(b)], stroke);
                }
            }
        }
    }
    // OCCT triangulation is a derived, bounded, revision-scoped display proxy.
    // One painter Mesh batches shaded triangles; edges are a cheap X-ray overlay.
    if let Some(cache) = &app.ui.brep_preview
        && app.session.state().is_ok_and(|st| cache.current(st.uid, st.revision))
        && let Ok(d) = app.session.doc()
        && let Some(object) = d.exact_breps.iter().find(|o| o.id == cache.object_id)
        && object.visible
        && d.layer(&object.layer).is_none_or(|l| l.visible())
    {
        let frame = cadcraft_geom::camera::OrthoFrame { yaw, pitch };
        cache.paint(&painter, rect, frame, app.ui.center3d, scale, selected_ids.contains(&cadcraft_doc::Handle(object.id)));
    }
    if preview_limited {
        painter.text(
            rect.left_bottom() + egui::vec2(8., -8.),
            egui::Align2::LEFT_BOTTOM,
            "Preview limit reached; scene partly displayed",
            egui::FontId::proportional(14.),
            egui::Color32::YELLOW,
        );
    }
    crate::point_input::draw(app, ui, rect, project);
    if !point_input {
        crate::cmdline::keyboard(app, ui.ctx());
    }
}

/// Picking is a shared engine query; failed or over-budget queries preserve selection.
#[cfg(test)]
fn select_response(app: &mut CadApp, ui: &egui::Ui, rect: egui::Rect, response: &egui::Response, gizmo_drag: bool) {
    if response.clicked()
        && !gizmo_drag
        && let Some(pixel) = response.interact_pointer_pos()
    {
        pick_at(app, rect, pixel, ui.input(|i| i.modifiers.shift));
    }
}
fn pick_at(app: &mut CadApp, rect: egui::Rect, pixel: egui::Pos2, toggle: bool) {
    let result = app.run(
        "geometry3d.pick",
        json!({
            "pixel":[pixel.x-rect.min.x,pixel.y-rect.min.y],"viewport":[rect.width(),rect.height()],
            "center":[app.ui.center3d.x,app.ui.center3d.y,app.ui.center3d.z],
            "yaw":app.ui.orbit_yaw,"pitch":app.ui.orbit_pitch,"scale":app.ui.scale3d
        }),
    );
    if let Ok(hit) = result {
        let ids = hit["id"].as_u64().into_iter().collect::<Vec<_>>();
        let _ = app.run("geometry3d.select", json!({"ids":ids,"mode":if toggle {"toggle"} else {"replace"}}));
    }
}

#[cfg(test)]
mod picking_tests {
    use super::*;
    fn frame(app: &mut CadApp, ctx: &egui::Context, mut events: Vec<egui::Event>, shift: bool) {
        events.insert(0, egui::Event::ModifiersChanged(egui::Modifiers { shift, ..Default::default() }));
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 400.))), events, ..Default::default() },
            |ui| {
                let (rect, response) = ui.allocate_exact_size(egui::vec2(400., 400.), egui::Sense::click_and_drag());
                let consumed = crate::gizmo::interact(app, ui, rect, &response);
                select_response(app, ui, rect, &response, consumed);
            },
        );
        output.textures_delta.clear();
    }
    fn pointer(p: egui::Pos2, down: bool, shift: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed: down,
                modifiers: egui::Modifiers { shift, ..Default::default() },
            },
        ]
    }
    #[test]
    fn click_toggle_empty_and_gizmo_drag_share_the_viewport() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let id=app.run("nurbs.curve3d",json!({"name":"Click fixture","curve":{"degree":1,"control":[{"x":-10.,"y":0.,"z":0.},{"x":10.,"y":0.,"z":0.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}})).unwrap()["id"].as_u64().unwrap();
        app.ui.orbit_yaw = 0.;
        app.ui.orbit_pitch = 0.;
        app.ui.scale3d = 10.;
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], false);
        frame(&mut app, &ctx, vec![], false);
        let click = egui::pos2(150., 200.);
        frame(&mut app, &ctx, pointer(click, true, false), false);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(egui::pos2(130., 200.))], false);
        frame(&mut app, &ctx, pointer(egui::pos2(130., 200.), false, false), false);
        assert!(app.session.selection().is_empty(), "camera drags must not click-select");
        frame(&mut app, &ctx, pointer(click, true, false), false);
        frame(&mut app, &ctx, pointer(click, false, false), false);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(id)]);
        frame(&mut app, &ctx, pointer(click, true, true), true);
        frame(&mut app, &ctx, pointer(click, false, true), true);
        assert!(app.session.selection().is_empty());
        frame(&mut app, &ctx, pointer(click, true, false), false);
        frame(&mut app, &ctx, pointer(click, false, false), false);
        let before = app.session.doc().unwrap().geometry3d.clone();
        frame(&mut app, &ctx, pointer(egui::pos2(260., 200.), true, false), false);
        frame(&mut app, &ctx, pointer(egui::pos2(260., 200.), false, false), false);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(id)]);
        assert_eq!(app.session.doc().unwrap().geometry3d, before);
        frame(&mut app, &ctx, pointer(egui::pos2(260., 200.), true, false), false);
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(egui::pos2(295., 200.))], false);
        assert_eq!(app.session.doc().unwrap().geometry3d, before);
        frame(&mut app, &ctx, pointer(egui::pos2(295., 200.), false, false), false);
        assert_ne!(app.session.doc().unwrap().geometry3d, before);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(id)]);
        app.run("undo", json!({})).unwrap();
        assert_eq!(app.session.doc().unwrap().geometry3d, before);
        let empty = egui::pos2(20., 20.);
        frame(&mut app, &ctx, pointer(empty, true, false), false);
        frame(&mut app, &ctx, pointer(empty, false, false), false);
        assert!(app.session.selection().is_empty());
        if std::env::var_os("WORLDWRIGHT_SELECTION_CAPTURE").is_some() {
            app.run("geometry3d.select", json!({"ids":[id]})).unwrap();
            new_surface(&mut app).unwrap();
            app.ui.orbit_yaw = 0.65;
            app.ui.orbit_pitch = 0.45;
            app.ui.scale3d = 12.;
            let capture = egui::Context::default();
            for _ in 0..2 {
                let mut output = capture.run_ui(
                    egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.))), ..Default::default() },
                    |ui| viewport3d(&mut app, ui),
                );
                for delta in output.textures_delta.set.values().flatten() {
                    let egui::ImageData::Color(texture) = &delta.image;
                    image::RgbaImage::from_raw(
                        texture.size[0] as u32,
                        texture.size[1] as u32,
                        texture.pixels.iter().flat_map(|p| p.to_array()).collect(),
                    )
                    .unwrap()
                    .save("/tmp/worldwright-selection-atlas.png")
                    .unwrap();
                }
                let meshes=capture.tessellate(output.shapes,1.).into_iter().filter_map(|p|{
                    let egui::epaint::Primitive::Mesh(m)=p.primitive else{return None;};
                    Some(json!({"clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],"indices":m.indices,"vertices":m.vertices.iter().map(|v|json!([v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.to_array()])).collect::<Vec<_>>()}))
                }).collect::<Vec<_>>();
                std::fs::write("/tmp/worldwright-selection-meshes.json", serde_json::to_vec(&meshes).unwrap()).unwrap();
                output.textures_delta.clear();
            }
        }
    }
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

/// A small polygon ring that can be filled, undone and saved as .dftba.
pub fn new_mesh_sample(app: &mut CadApp) -> Result<serde_json::Value, String> {
    app.ui.view3d = true;
    let result = app.run(
        "mesh3d.create",
        json!({
            "name":"Editable mesh ring",
            "mesh":{
                "vertices":[
                    {"x":-8.,"y":-8.,"z":0.},{"x":8.,"y":-8.,"z":0.},
                    {"x":8.,"y":8.,"z":0.},{"x":-8.,"y":8.,"z":0.},
                    {"x":-3.,"y":-3.,"z":0.},{"x":3.,"y":-3.,"z":0.},
                    {"x":3.,"y":3.,"z":0.},{"x":-3.,"y":3.,"z":0.}
                ],
                "faces":[
                    {"quad":[0,1,5,4]},{"quad":[1,2,6,5]},
                    {"quad":[2,3,7,6]},{"quad":[3,0,4,7]}
                ]
            }
        }),
    )?;
    if let Some(id) = result["id"].as_u64() {
        app.session.set_selection(vec![cadcraft_doc::Handle(id)]);
    }
    Ok(result)
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
        let points = points.chain(
            d.mesh3d.iter().filter(|o| o.visible && d.layer(&o.layer).is_none_or(|l| l.visible())).flat_map(|o| o.mesh.vertices.iter().copied()),
        );
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
        fields(ui, "Axis (1D/rotate) / normal (2D/mirror)", &mut app.ui.transform_axis);
        ui.add(egui::DragValue::new(&mut app.ui.transform_angle).prefix("Angle °: ").speed(1.));
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Rotate selected")).clicked() {
            operation =
                Some(json!({"kind":"rotate","origin":app.ui.transform_origin,"axis":app.ui.transform_axis,"angle_degrees":app.ui.transform_angle}));
        }
        ui.add(egui::DragValue::new(&mut app.ui.transform_factor).prefix("Scale: ").speed(0.01));
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Scale selected")).clicked() {
            operation = Some(json!({"kind":"scale","origin":app.ui.transform_origin,"factor":app.ui.transform_factor}));
        }
        ui.label("Space objects without changing their size");
        ui.horizontal_wrapped(|ui| {
            for (label, mode) in [
                ("Space 1D", json!({"kind":"one_d","axis":app.ui.transform_axis})),
                ("Space 2D", json!({"kind":"two_d","normal":app.ui.transform_axis})),
                ("Space 3D", json!({"kind":"three_d"})),
            ] {
                if ui.add_enabled(!ids.is_empty(), egui::Button::new(label)).clicked() {
                    operation=Some(json!({"kind":"scale_positions","origin":app.ui.transform_origin,"factor":app.ui.transform_factor,"mode":mode,"tolerance":0.000001}));
                }
            }
        });
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Mirror selected")).clicked() {
            operation = Some(json!({"kind":"mirror","origin":app.ui.transform_origin,"normal":app.ui.transform_axis}));
        }
        fields(ui, "Shear direction (perpendicular to normal above)", &mut app.ui.shear_direction);
        ui.add(egui::DragValue::new(&mut app.ui.shear_angle).prefix("Shear angle °: ").speed(1.));
        if ui.add_enabled(!ids.is_empty(), egui::Button::new("Shear selected")).clicked() {
            operation = Some(json!({"kind":"shear","origin":app.ui.transform_origin,"direction":app.ui.shear_direction,"normal":app.ui.transform_axis,"angle_degrees":app.ui.shear_angle}));
        }
        egui::CollapsingHeader::new("Orient by three points").default_open(true).show(ui, |ui| {
            for (i, value) in app.ui.orient_source.iter_mut().enumerate() {
                fields(ui, &format!("Source point {}", i + 1), value);
            }
            for (i, value) in app.ui.orient_target.iter_mut().enumerate() {
                fields(ui, &format!("Target point {}", i + 1), value);
            }
            ui.checkbox(&mut app.ui.orient_scale, "Scale using first two points");
            if ui.add_enabled(!ids.is_empty(), egui::Button::new("Orient3Pt selected")).clicked() {
                operation = Some(json!({"kind":"orient3pt","source":app.ui.orient_source,"target":app.ui.orient_target,"scale":app.ui.orient_scale}));
            }
        });
        if let Some(operation) = operation {
            let _ = app.run("geometry3d.transform", json!({"ids":ids,"operation":operation,"copy":app.ui.transform_copy}));
        }
        ui.small("Exact curves/control surfaces only. World coordinates; numeric controls, no gumball yet.");
    });
}

#[cfg(test)]
mod spacing_ui_tests {
    use super::*;
    #[test]
    fn spacing_buttons_dispatch_real_geometry_commands() {
        fn text_center(shape: &egui::epaint::Shape, label: &str) -> Option<egui::Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(t.pos + t.galley.size() * 0.5),
                egui::epaint::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_center(s, label)),
                _ => None,
            }
        }
        for label in ["Space 1D", "Space 2D", "Space 3D", "Shear selected", "Orient3Pt selected"] {
            let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
            let result=app.run("nurbs.curve3d",json!({"name":"Spacing fixture","curve":{"degree":1,"control":[{"x":2.,"y":4.,"z":6.},{"x":4.,"y":6.,"z":8.}],"weights":[1.,1.],"knots":[0.,0.,1.,1.]}})).unwrap();
            let id = result["id"].as_u64().unwrap();
            app.session.set_selection(vec![cadcraft_doc::Handle(id)]);
            app.ui.transform_factor = 2.;
            app.ui.transform_origin = [0.; 3];
            app.ui.transform_axis = if label == "Shear selected" { [0., 0., 1.] } else { [1., 0., 0.] };
            app.ui.orient_target = [[0., 0., 0.], [0., 1., 0.], [-1., 0., 0.]];
            let ctx = egui::Context::default();
            let input =
                egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(450., 1000.))), ..Default::default() };
            let mut frame = ctx.run_ui(input.clone(), |ui| transform_panel(&mut app, ui));
            let capture = std::env::var_os("WORLDWRIGHT_UI_CAPTURE").is_some() && label == "Orient3Pt selected";
            if capture {
                for delta in frame.textures_delta.set.values().flatten() {
                    let egui::ImageData::Color(texture) = &delta.image;
                    let pixels = texture.pixels.iter().flat_map(|p| p.to_array()).collect::<Vec<_>>();
                    image::RgbaImage::from_raw(texture.size[0] as u32, texture.size[1] as u32, pixels)
                        .unwrap()
                        .save("/tmp/worldwright-panel-atlas.png")
                        .unwrap();
                }
            }
            frame.textures_delta.clear();
            let mut frame = ctx.run_ui(input.clone(), |ui| transform_panel(&mut app, ui));
            if capture {
                let meshes = ctx.tessellate(frame.shapes.clone(), 1.).into_iter().filter_map(|p| {
                    let egui::epaint::Primitive::Mesh(m) = p.primitive else { return None };
                    Some(json!({"clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],"indices":m.indices,"vertices":m.vertices.iter().map(|v| json!([v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.to_array()])).collect::<Vec<_>>()}))
                }).collect::<Vec<_>>();
                std::fs::write("/tmp/worldwright-panel-meshes.json", serde_json::to_vec(&meshes).unwrap()).unwrap();
            }
            let pos = frame.shapes.iter().find_map(|s| text_center(&s.shape, label)).unwrap();
            frame.textures_delta.clear();
            for pressed in [true, false] {
                let mut event = input.clone();
                event.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::default() },
                ];
                let mut frame = ctx.run_ui(event, |ui| transform_panel(&mut app, ui));
                frame.textures_delta.clear();
            }
            let cadcraft_doc::organization::Shape::Curve(c) = &app.session.doc().unwrap().geometry3d[0].shape else { panic!() };
            let expected = match label {
                "Space 1D" => [5., 4., 6.],
                "Space 2D" => [2., 9., 13.],
                "Shear selected" => [8., 4., 6.],
                "Orient3Pt selected" => [-4., 2., 6.],
                _ => [5., 9., 13.],
            };
            for (actual, wanted) in [c.control[0].x, c.control[0].y, c.control[0].z].into_iter().zip(expected) {
                assert!((actual - wanted).abs() < 1e-10, "{label}: {actual} != {wanted}");
            }
        }
    }
}

#[cfg(test)]
mod mesh_ui_tests {
    use super::*;

    #[test]
    fn viewport_drafting_consumes_mesh_selection_clicks() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        new_mesh_sample(&mut app).unwrap();
        app.ui.orbit_yaw = 0.;
        app.ui.orbit_pitch = -std::f64::consts::FRAC_PI_2;
        app.ui.scale3d = 25.;
        app.ui.center3d = cadcraft_geom::Vec3::ZERO;
        app.session.set_selection(Vec::new());
        let revision = app.session.state().unwrap().revision;
        crate::point_input::begin(&mut app);
        let ctx = egui::Context::default();
        let base = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 650.))), ..Default::default() };
        for _ in 0..2 {
            ctx.run_ui(base.clone(), |ui| viewport3d(&mut app, ui)).textures_delta.clear();
        }
        for pos in [egui::pos2(400., 490.), egui::pos2(450., 490.)] {
            for pressed in [true, false] {
                let mut input = base.clone();
                input.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
                ];
                ctx.run_ui(input, |ui| viewport3d(&mut app, ui)).textures_delta.clear();
            }
        }
        assert!(app.session.selection().is_empty());
        assert!(app.ui.mesh_face_object_id.is_none());
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert!(crate::point_input::active(&app));
        let mut input = base;
        input.events =
            vec![egui::Event::Key { key: egui::Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }];
        ctx.run_ui(input, |ui| viewport3d(&mut app, ui)).textures_delta.clear();
        assert_eq!(app.session.doc().unwrap().geometry3d.len(), 1);
        assert_eq!(app.session.doc().unwrap().mesh3d.len(), 1);
    }

    #[test]
    fn mesh_shift_toggle_and_locked_layers_match_exact_selection() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let first = new_mesh_sample(&mut app).unwrap()["id"].as_u64().unwrap();
        let second = new_mesh_sample(&mut app).unwrap()["id"].as_u64().unwrap();
        let revision = app.session.state().unwrap().revision;
        app.ui.center3d = cadcraft_geom::Vec3::ZERO;
        app.ui.orbit_yaw = 0.;
        app.ui.orbit_pitch = -std::f64::consts::FRAC_PI_2;
        app.ui.scale3d = 10.;
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 400.));
        // Two coplanar native meshes overlap. Stable ID order chooses the first.
        let point = egui::pos2(150., 250.);
        select_3d_at(&mut app, viewport, point, true);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(second), cadcraft_doc::Handle(first)]);
        assert_eq!(app.ui.mesh_face_object_id, Some(first));
        select_3d_at(&mut app, viewport, point, true);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(second)]);
        assert_eq!(app.ui.mesh_face_object_id, None);
        select_3d_at(&mut app, viewport, point, false);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(first)]);
        assert_eq!(app.session.state().unwrap().revision, revision, "selection must not edit geometry");

        let layer_name = app.session.doc().unwrap().mesh3d[0].layer.clone();
        app.session.doc_mut().unwrap().layer_mut(&layer_name).unwrap().locked = true;
        app.session.set_selection(Vec::new());
        select_3d_at(&mut app, viewport, point, false);
        assert!(app.session.selection().is_empty(), "viewport must not select locked mesh layers");
        assert!(app.ui.mesh_face_object_id.is_none());
    }

    #[test]
    fn sample_mesh_can_be_created_previewed_repaired_and_undone() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let created = new_mesh_sample(&mut app).unwrap();
        let id = created["id"].as_u64().unwrap();
        assert!(app.session.selection().contains(&cadcraft_doc::Handle(id)));
        let original = app.session.doc().unwrap().mesh3d[0].mesh.clone();
        assert_eq!(original.faces.len(), 4);

        // The application viewport can draw the model in headless egui.
        let context = egui::Context::default();
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640., 480.))), ..Default::default() };
        let mut frame = context.run_ui(input, |ui| viewport3d(&mut app, ui));
        assert!(!frame.shapes.is_empty());
        // Headless egui tests do not have a renderer consuming texture deltas.
        frame.textures_delta.clear();

        let bounds = app.run("mesh3d.boundaries", json!({"id":id})).unwrap();
        let loops = bounds["report"]["closed_loops"].as_array().unwrap();
        let inner = loops
            .iter()
            .position(|entry| entry["vertices"].as_array().is_some_and(|v| v.iter().all(|i| i.as_u64().is_some_and(|i| i >= 4))))
            .unwrap();
        let revision = bounds["source_revision"].as_u64().unwrap();
        app.run(
            "mesh3d.edit",
            json!({"id":id,"edit":{
                "kind":"fill_planar_hole",
                "selected_revision":revision,
                "loop_index":inner
            }}),
        )
        .unwrap();
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.faces.len(), 6);
        app.session.undo().unwrap();
        let restored = &app.session.doc().unwrap().mesh3d[0].mesh;
        assert!(std::sync::Arc::ptr_eq(restored, &original));
    }
    #[test]
    fn picked_face_deletion_is_undoable_and_rejects_stale_revision() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let id = new_mesh_sample(&mut app).unwrap()["id"].as_u64().unwrap();
        let original = app.session.doc().unwrap().mesh3d[0].mesh.clone();
        let revision = app.session.state().unwrap().revision;
        let face = crate::mesh_picking::pick_visible_mesh_face(
            [(id, original.as_ref())],
            cadcraft_geom::camera::OrthoFrame { yaw: 0., pitch: -std::f64::consts::FRAC_PI_2 },
            cadcraft_geom::Vec3::ZERO,
            10.,
            cadcraft_geom::Vec2::new(0., -50.),
        )
        .unwrap();
        assert_eq!(face.face_index, 0);

        app.ui.mesh_face_object_id = Some(id);
        app.ui.mesh_face_document_uid = Some(app.session.state().unwrap().uid);
        app.ui.mesh_face_revision = Some(revision);
        app.ui.mesh_face_index = face.face_index;
        delete_mesh_face(&mut app, id, revision, face.face_index).unwrap();
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.faces.len(), 3);
        assert_eq!(app.ui.mesh_face_object_id, None);
        assert_eq!(app.ui.mesh_face_document_uid, None);
        assert_eq!(app.ui.mesh_face_revision, None);

        let after = app.session.state().unwrap().revision;
        assert!(delete_mesh_face(&mut app, id, revision, 0).is_err());
        assert_eq!(app.session.state().unwrap().revision, after);
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.faces.len(), 3);

        app.session.undo().unwrap();
        assert!(std::sync::Arc::ptr_eq(&app.session.doc().unwrap().mesh3d[0].mesh, &original));
    }

    #[test]
    fn viewport_pointer_click_picks_face_without_editing_document() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let id = new_mesh_sample(&mut app).unwrap()["id"].as_u64().unwrap();
        app.ui.orbit_yaw = 0.;
        app.ui.orbit_pitch = -std::f64::consts::FRAC_PI_2;
        app.ui.scale3d = 25.;
        app.ui.center3d = cadcraft_geom::Vec3::ZERO;
        app.session.set_selection(Vec::new());
        let revision = app.session.state().unwrap().revision;

        let ctx = egui::Context::default();
        let base = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 650.))), ..Default::default() };
        let mut initial = ctx.run_ui(base.clone(), |ui| viewport3d(&mut app, ui));
        assert!(!initial.shapes.is_empty());
        initial.textures_delta.clear();

        // The sample mesh's bottom strip covers y=-8..-3 in the top view.
        // Two toolbar rows leave the mesh strip around screen y=490.
        let pos = egui::pos2(400., 490.);
        for pressed in [true, false] {
            let mut frame = base.clone();
            frame.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::default() },
            ];
            let mut output = ctx.run_ui(frame, |ui| viewport3d(&mut app, ui));
            output.textures_delta.clear();
        }
        assert_eq!(app.ui.mesh_face_object_id, Some(id));
        assert_eq!(app.ui.mesh_face_index, 0);
        assert_eq!(app.ui.mesh_face_document_uid, Some(app.session.state().unwrap().uid));
        assert_eq!(app.ui.mesh_face_revision, Some(revision));
        assert!(app.session.selection().contains(&cadcraft_doc::Handle(id)));
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.faces.len(), 4);
    }

    #[test]
    fn face_pick_cannot_cross_to_another_document_at_the_same_revision() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let id = new_mesh_sample(&mut app).unwrap()["id"].as_u64().unwrap();
        let first_uid = app.session.state().unwrap().uid;
        app.ui.mesh_face_object_id = Some(id);
        app.ui.mesh_face_document_uid = Some(first_uid);
        app.ui.mesh_face_revision = Some(app.session.state().unwrap().revision);
        app.ui.mesh_face_index = 0;
        assert!(picked_face_is_current(&app, id));

        // A second document may reuse the same mesh object IDs and revision.
        // That must NOT make a saved UI pick valid for this different document.
        let duplicate = app.session.doc().unwrap().clone();
        app.session.open_drawing(duplicate, "Copy", None);
        assert_ne!(app.session.state().unwrap().uid, first_uid);
        app.ui.mesh_face_revision = Some(app.session.state().unwrap().revision);
        assert!(!picked_face_is_current(&app, id));
    }
}

#[cfg(test)]
mod brep_selection_tests {
    use super::*;
    use serde_json::json;

    fn solid_preview(app: &mut CadApp, id: u64) {
        use cadcraft_doc::organization::ExactBrepObject;
        use std::sync::Arc;

        app.session.doc_mut().unwrap().exact_breps.push(ExactBrepObject {
            id,
            name: "Fixture".into(),
            layer: "0".into(),
            visible: true,
            brep: Arc::new("fixture-only, never written".into()),
            volume: 1.,
            faces: 1,
            edges: 3,
        });
        let state = app.session.state().unwrap();
        let proxy = json!({"mesh":{"exact":false,
            "vertices":[[0.,0.,2.],[10.,0.,2.],[0.,10.,2.]],
            "normals":[[0.,0.,1.],[0.,0.,1.],[0.,0.,1.]],
            "indices":[0,1,2],"edge_chains":[]}});
        app.ui.brep_preview = Some(BrepPreview::from_worker(&proxy, state.uid, state.revision, id).unwrap());
    }

    #[test]
    fn shaded_solid_pick_is_reversible_and_does_not_change_document() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        solid_preview(&mut app, 28);
        app.ui.orbit_yaw = 0.;
        app.ui.orbit_pitch = -std::f64::consts::FRAC_PI_2;
        app.ui.scale3d = 10.;
        app.ui.center3d = cadcraft_geom::Vec3::ZERO;
        let state = app.session.state().unwrap();
        let uid = state.uid;
        let revision = state.revision;
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 400.));
        let cursor = egui::pos2(220., 180.);
        select_3d_at(&mut app, viewport, cursor, false);
        assert_eq!(app.session.selection(), vec![cadcraft_doc::Handle(28)]);
        select_3d_at(&mut app, viewport, cursor, true);
        assert!(app.session.selection().is_empty());
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert_eq!(app.session.state().unwrap().uid, uid);
        assert_eq!(app.session.doc().unwrap().exact_breps.len(), 1);

        // Visibility and stale revision must prevent picking a ghost solid.
        app.session.doc_mut().unwrap().exact_breps[0].visible = false;
        select_3d_at(&mut app, viewport, cursor, false);
        assert!(app.session.selection().is_empty());
        app.session.doc_mut().unwrap().exact_breps[0].visible = true;
        app.ui.brep_preview.as_mut().unwrap().source_revision += 1;
        select_3d_at(&mut app, viewport, cursor, false);
        assert!(app.session.selection().is_empty());
    }
}
