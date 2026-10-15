//! Exact solid controls use the existing transactional OCCT commands.
use crate::CadApp;
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct SolidPanel {
    document: Option<u64>,
    minimum: [f64; 3],
    maximum: [f64; 3],
    radius: f64,
    height: f64,
    left: Option<u64>,
    right: Option<u64>,
    translation: [f64; 3],
    origin: [f64; 3],
    axis: [f64; 3],
    degrees: f64,
    scale: f64,
    error: Option<String>,
}

impl Default for SolidPanel {
    fn default() -> Self {
        Self {
            document: None,
            minimum: [0.; 3],
            maximum: [10.; 3],
            radius: 5.,
            height: 10.,
            left: None,
            right: None,
            translation: [0.; 3],
            origin: [0.; 3],
            axis: [0., 0., 1.],
            degrees: 90.,
            scale: 1.,
            error: None,
        }
    }
}

impl SolidPanel {
    fn reconcile(&mut self, document: Option<u64>, ids: &[u64]) {
        if self.document != document {
            self.document = document;
            self.left = None;
            self.right = None;
            self.error = None;
        }
        self.left = self.left.filter(|id| ids.contains(id));
        self.right = self.right.filter(|id| ids.contains(id));
    }

    fn boolean_enabled(&self) -> bool {
        self.left.is_some() && self.right.is_some() && self.left != self.right
    }
}

fn xyz(ui: &mut egui::Ui, label: &str, coordinates: &mut [f64; 3]) {
    ui.label(label);
    ui.horizontal(|ui| {
        for coordinate in coordinates {
            ui.add(egui::DragValue::new(coordinate).speed(0.1));
        }
    });
}

fn execute(app: &mut CadApp, command: &str, parameters: Value) -> Result<(), String> {
    let result = app.run(command, parameters)?;
    if let Some(id) = result.get("ids").and_then(Value::as_array).and_then(|ids| ids.first()).and_then(Value::as_u64) {
        app.session.set_selection(vec![cadcraft_doc::Handle(id)]);
        // Preview failure must not misrepresent a successfully committed solid.
        if let Err(error) = crate::buildercraft::load_exact_brep_preview(app, id) {
            app.ui.brep_preview = None;
            return Err(format!("Solid updated; shaded preview unavailable: {error}"));
        }
    }
    Ok(())
}

pub(crate) fn panel(app: &mut CadApp, ui: &mut egui::Ui) {
    let document = app.session.state().ok().map(|state| state.uid);
    let objects = app.session.doc().map(|doc| doc.exact_breps.iter().map(|o| (o.id, o.name.clone())).collect::<Vec<_>>()).unwrap_or_default();
    app.ui.solid_panel.reconcile(document, &objects.iter().map(|(id, _)| *id).collect::<Vec<_>>());
    let selected =
        app.session.selection().first().map(|h| h.0).filter(|id| app.session.selection().len() == 1 && objects.iter().any(|(other, _)| other == id));
    let mut action = None;
    ui.collapsing("Exact solids", |ui| {
        ui.small("Dimensions in document units. Native OCCT worker required.");
        let state = &mut app.ui.solid_panel;
        xyz(ui, "Box minimum", &mut state.minimum);
        xyz(ui, "Box maximum", &mut state.maximum);
        if ui.button("Create box").clicked() {
            action = Some(("brep.box", json!({"name":"Box","min":state.minimum,"max":state.maximum})));
        }
        ui.horizontal(|ui| {
            ui.label("Radius");
            ui.add(egui::DragValue::new(&mut state.radius).speed(0.1));
        });
        if ui.button("Create sphere").clicked() {
            action = Some(("brep.sphere", json!({"name":"Sphere","radius":state.radius})));
        }
        ui.horizontal(|ui| {
            ui.label("Cylinder height");
            ui.add(egui::DragValue::new(&mut state.height).speed(0.1));
        });
        if ui.button("Create cylinder").clicked() {
            action = Some(("brep.cylinder", json!({"name":"Cylinder","radius":state.radius,"height":[0.,0.,state.height]})));
        }
        for (label, selected) in [("Left operand", &mut state.left), ("Right operand", &mut state.right)] {
            egui::ComboBox::from_id_salt(label)
                .selected_text(selected.and_then(|id| objects.iter().find(|(other, _)| *other == id)).map(|(_, name)| name.as_str()).unwrap_or(label))
                .show_ui(ui, |ui| {
                    ui.selectable_value(selected, None, "Choose solid");
                    for (id, name) in &objects {
                        ui.selectable_value(selected, Some(*id), format!("{name} ({id})"));
                    }
                });
        }
        ui.small("Difference removes Right from Left. Operands are preserved.");
        for (label, operation) in [("Union", "union"), ("Difference", "difference"), ("Intersection", "intersection")] {
            if ui.add_enabled(state.boolean_enabled(), egui::Button::new(label)).clicked() {
                action = Some(("brep.boolean", json!({"name":label,"left_id":state.left,"right_id":state.right,"operation":operation})));
            }
        }
        ui.separator();
        ui.label("Transform selected exact solid");
        ui.small("Select one solid. Each action preserves its ID and supports Undo.");
        xyz(ui, "Move offset", &mut state.translation);
        xyz(ui, "Rotation / scale origin", &mut state.origin);
        xyz(ui, "Rotation axis", &mut state.axis);
        ui.horizontal(|ui| {
            ui.label("Degrees");
            ui.add(egui::DragValue::new(&mut state.degrees).speed(1.));
            ui.label("Uniform scale");
            ui.add(egui::DragValue::new(&mut state.scale).speed(0.01));
        });
        for label in ["Move solid", "Rotate solid", "Scale solid"] {
            if ui.add_enabled(selected.is_some(), egui::Button::new(label)).clicked() {
                action = Some((
                    "brep.transform",
                    json!({"id":selected,
                    "translation":if label == "Move solid" { state.translation } else { [0.; 3] },
                    "origin":state.origin,
                    "axis":if label == "Rotate solid" { state.axis } else { [0., 0., 1.] },
                    "radians":if label == "Rotate solid" { state.degrees.to_radians() } else { 0. },
                    "scale":if label == "Scale solid" { state.scale } else { 1. }}),
                ));
            }
        }
        if let Some(error) = &state.error {
            ui.label(error);
        }
    });
    if let Some((command, parameters)) = action {
        app.ui.solid_panel.error = execute(app, command, parameters).err();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(app: &mut CadApp, ctx: &egui::Context, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(480., 1300.))), events, ..Default::default() },
            |ui| panel(app, ui),
        );
        if std::env::var_os("WORLDWRIGHT_SOLID_CAPTURE").is_some() {
            for delta in output.textures_delta.set.values().flatten() {
                let egui::ImageData::Color(texture) = &delta.image;
                let patch = image::RgbaImage::from_raw(
                    texture.size[0] as u32,
                    texture.size[1] as u32,
                    texture.pixels.iter().flat_map(|p| p.to_array()).collect(),
                )
                .unwrap();
                let atlas = if let Some([x, y]) = delta.pos {
                    let mut atlas = image::open("/tmp/worldwright-solid-atlas.png").unwrap().to_rgba8();
                    image::imageops::replace(&mut atlas, &patch, x as i64, y as i64);
                    atlas
                } else {
                    patch
                };
                atlas.save("/tmp/worldwright-solid-atlas.png").unwrap();
            }
            let meshes: Vec<_> = ctx.tessellate(output.shapes.clone(), 1.).into_iter().filter_map(|primitive| {
                let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else { return None; };
                Some(json!({"clip":[primitive.clip_rect.min.x,primitive.clip_rect.min.y,primitive.clip_rect.max.x,primitive.clip_rect.max.y],
                    "indices":mesh.indices,"vertices":mesh.vertices.iter().map(|v| json!([v.pos.x,v.pos.y,v.uv.x,v.uv.y,v.color.to_array()])).collect::<Vec<_>>()}))
            }).collect();
            std::fs::write("/tmp/worldwright-solid-meshes.json", serde_json::to_vec(&meshes).unwrap()).unwrap();
        }
        output.textures_delta.clear();
        output
    }

    fn label_position(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
        fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
            match shape {
                egui::Shape::Text(text) if text.galley.text() == label => Some(text.pos + text.galley.size() * 0.5),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
                _ => None,
            }
        }
        output.shapes.iter().find_map(|shape| find(&shape.shape, label)).unwrap()
    }

    fn click(app: &mut CadApp, ctx: &egui::Context, position: egui::Pos2) {
        for pressed in [true, false] {
            frame(
                app,
                ctx,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton { pos: position, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() },
                ],
            );
        }
    }

    #[test]
    fn pointer_invalid_input_reports_error_without_creating_solid() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let ctx = egui::Context::default();
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.style_mut_of(theme, |style| style.animation_time = 0.);
        }
        let output = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, label_position(&output, "Exact solids"));
        app.ui.solid_panel.radius = -1.;
        let before = format!("{:?}", app.session.state().unwrap());
        frame(&mut app, &ctx, vec![]);
        let output = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, label_position(&output, "Create sphere"));
        assert!(app.ui.solid_panel.error.as_ref().is_some_and(|error| error.contains("positive")));
        assert_eq!(before, format!("{:?}", app.session.state().unwrap()));
        assert!(app.session.doc().unwrap().exact_breps.is_empty());
    }

    #[test]
    fn invalid_primitive_preserves_document_and_selection() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let before = format!("{:?}", app.session.state().unwrap());
        assert!(execute(&mut app, "brep.sphere", json!({"name":"Invalid","radius":-1.})).is_err());
        assert_eq!(before, format!("{:?}", app.session.state().unwrap()));
        assert!(app.session.doc().unwrap().exact_breps.is_empty());
        assert!(app.session.selection().is_empty());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn native_solid_panel_workflow_when_worker_is_installed() {
        if std::env::var_os("WORLDWRIGHT_BREP_WORKER").is_none() {
            return;
        }
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let ctx = egui::Context::default();
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.style_mut_of(theme, |style| style.animation_time = 0.);
        }
        let output = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, label_position(&output, "Exact solids"));
        for (minimum, maximum) in [([0.; 3], [2.; 3]), ([1.; 3], [3.; 3])] {
            app.ui.solid_panel.minimum = minimum;
            app.ui.solid_panel.maximum = maximum;
            frame(&mut app, &ctx, vec![]);
            let output = frame(&mut app, &ctx, vec![]);
            click(&mut app, &ctx, label_position(&output, "Create box"));
            assert!(app.ui.solid_panel.error.is_none(), "{:?}", app.ui.solid_panel.error);
        }
        let ids: Vec<_> = app.session.doc().unwrap().exact_breps.iter().map(|o| o.id).collect();
        app.ui.solid_panel.left = Some(ids[0]);
        app.ui.solid_panel.right = Some(ids[1]);
        for (label, volume) in [("Union", 15.), ("Difference", 7.), ("Intersection", 1.)] {
            let output = frame(&mut app, &ctx, vec![]);
            click(&mut app, &ctx, label_position(&output, label));
            assert!(app.ui.solid_panel.error.is_none(), "{:?}", app.ui.solid_panel.error);
            let object = app.session.doc().unwrap().exact_breps.last().unwrap();
            assert!((object.volume - volume).abs() < 1e-8);
            assert_eq!(app.session.selection(), &[cadcraft_doc::Handle(object.id)]);
            assert_eq!(app.ui.brep_preview.as_ref().unwrap().object_id, object.id);
            app.run("undo", json!({})).unwrap();
            assert_eq!(app.session.doc().unwrap().exact_breps.len(), 2);
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn native_transform_buttons_when_worker_is_installed() {
        if std::env::var_os("WORLDWRIGHT_BREP_WORKER").is_none() {
            return;
        }
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        execute(&mut app, "brep.box", json!({"name":"Transform me","min":[0.,0.,0.],"max":[2.,3.,4.]})).unwrap();
        let source = app.session.doc().unwrap().exact_breps[0].clone();
        let ctx = egui::Context::default();
        for theme in [egui::Theme::Dark, egui::Theme::Light] {
            ctx.style_mut_of(theme, |style| style.animation_time = 0.);
        }
        let output = frame(&mut app, &ctx, vec![]);
        click(&mut app, &ctx, label_position(&output, "Exact solids"));
        app.ui.solid_panel.translation = [10., 0., 0.];
        app.ui.solid_panel.scale = 2.;
        for (label, volume) in [("Move solid", 24.), ("Rotate solid", 24.), ("Scale solid", 192.)] {
            frame(&mut app, &ctx, vec![]);
            let output = frame(&mut app, &ctx, vec![]);
            click(&mut app, &ctx, label_position(&output, label));
            assert!(app.ui.solid_panel.error.is_none(), "{:?}", app.ui.solid_panel.error);
            let objects = &app.session.doc().unwrap().exact_breps;
            assert_eq!(objects.len(), 1);
            assert_eq!(objects[0].id, source.id);
            assert_ne!(objects[0].brep, source.brep);
            assert!((objects[0].volume - volume).abs() < 1e-7);
            assert_eq!(app.session.selection(), &[cadcraft_doc::Handle(source.id)]);
            assert_eq!(app.ui.brep_preview.as_ref().unwrap().object_id, source.id);
            app.run("undo", json!({})).unwrap();
            assert_eq!(app.session.doc().unwrap().exact_breps[0], source);
        }
    }

    #[test]
    fn operands_are_document_scoped_and_removed_on_undo() {
        let mut state = SolidPanel::default();
        state.reconcile(Some(1), &[2, 3]);
        state.left = Some(2);
        state.right = Some(3);
        assert!(state.boolean_enabled());
        state.reconcile(Some(1), &[2]);
        assert_eq!(state.right, None);
        assert!(!state.boolean_enabled());
        state.reconcile(Some(4), &[2, 3]);
        assert_eq!(state.left, None);
        state.left = Some(2);
        state.right = Some(2);
        assert!(!state.boolean_enabled());
    }
}
