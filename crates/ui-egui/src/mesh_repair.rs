//! WorldWright Mesh Repair: independent metrology-style task shell.
//! Space toggles Selection <-> Navigate as in classic PolyWorks, but all
//! geometry operations delegate to shared document/mesh Rust commands.
use crate::CadApp;
use egui::{Color32, Key, Modifiers};
use serde_json::{Value, json};

pub const PANEL: Color32 = Color32::from_rgb(37, 44, 50);
pub const PANEL_DEEP: Color32 = Color32::from_rgb(29, 36, 42);
pub const VIEWPORT: Color32 = Color32::from_rgb(20, 29, 35);
pub const ACCENT: Color32 = Color32::from_rgb(100, 205, 188);
pub const MESH: Color32 = Color32::from_rgb(131, 150, 161);
pub const PICK: Color32 = Color32::from_rgb(255, 179, 77);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inspection {
    pub object_id: u64,
    pub document_uid: u64,
    pub revision: u64,
    pub bad_vertices: usize,
    pub boundary_loops: usize,
    pub open_boundaries: usize,
    pub invalid_edges: usize,
}

/// UI-only state. Mesh geometry and undo remain owned by the CAD document.
#[derive(Clone, Debug)]
pub struct State {
    pub active: bool,
    pub selecting: bool,
    pub edge: [u32; 2],
    pub fraction: f64,
    pub hole_index: u32,
    pub inspection: Option<Inspection>,
}
impl Default for State {
    fn default() -> Self {
        Self { active: false, selecting: false, edge: [0, 1], fraction: 0.5, hole_index: 0, inspection: None }
    }
}

pub fn set_active(app: &mut CadApp, active: bool) {
    app.ui.mesh_repair.active = active;
    if active {
        app.ui.mesh_repair.selecting = false;
        app.ui.view3d = true;
        app.ui.point_input = Default::default();
        app.ui.gizmo = Default::default();
    }
    app.ui.mesh_repair.inspection = None;
}

/// PolyWorks-style spacebar switch. The key is only consumed when the
/// repair workspace owns keyboard focus; CAD drafting keeps its original
/// Space behavior. Text-entry fields and command search always take priority.
pub fn shortcuts(app: &mut CadApp, ctx: &egui::Context) {
    if !app.ui.mesh_repair.active || app.ui.command_search.open || ctx.egui_wants_keyboard_input() {
        return;
    }
    let toggle = ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Space));
    if toggle {
        app.ui.mesh_repair.selecting = !app.ui.mesh_repair.selecting;
        app.set_status(if app.ui.mesh_repair.selecting { "Mesh Repair · Selection" } else { "Mesh Repair · Navigate" });
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        app.ui.mesh_face_object_id = None;
        app.ui.mesh_face_document_uid = None;
        app.ui.mesh_face_revision = None;
        app.ui.mesh_repair.selecting = false;
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::F)) {
        let _ = app.run("ui.buildercraft.fit", Value::Null);
    }
    if ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::I))
        && let Some(id) = selected_mesh_id(app)
    {
        let _ = inspect(app, id);
    }
}

pub fn selected_mesh_id(app: &CadApp) -> Option<u64> {
    let selection = app.session.selection();
    let doc = app.session.doc().ok()?;
    doc.mesh3d.iter().find(|mesh| selection.contains(&cadcraft_doc::Handle(mesh.id))).map(|mesh| mesh.id)
}

fn current(app: &CadApp, record: &Inspection) -> bool {
    app.session.state().is_ok_and(|s| s.uid == record.document_uid && s.revision == record.revision)
        && app.session.doc().is_ok_and(|d| d.mesh3d.iter().any(|m| m.id == record.object_id))
}

/// One explicit inspection request, bounded by the engine's interactive
/// mesh limits. Subsequent UI frames use the cached revision-bound summary.
pub fn inspect(app: &mut CadApp, id: u64) -> Result<(), String> {
    let before = app.session.state().map_err(|e| e.to_string())?;
    let uid = before.uid;
    let revision = before.revision;
    let topology = app.run("mesh3d.topology", json!({"id":id}))?;
    let boundaries = app.run("mesh3d.boundaries", json!({"id":id}))?;
    let selected = topology["selected_vertices"].as_array().ok_or("Topology response missing vertices")?.len();
    let loops = boundaries["report"]["closed_loops"].as_array().ok_or("Boundary response missing loops")?.len();
    let open = boundaries["report"]["open_chains"].as_array().map_or(0, Vec::len);
    let invalid_edges = topology["edge_diagnostics"]["non_manifold_edges"].as_array().map_or(0, Vec::len)
        + topology["edge_diagnostics"]["inconsistent_winding_edges"].as_array().map_or(0, Vec::len);
    let now = app.session.state().map_err(|e| e.to_string())?;
    if uid != now.uid
        || revision != now.revision
        || topology["source_revision"].as_u64() != Some(revision)
        || boundaries["source_revision"].as_u64() != Some(revision)
    {
        return Err("Document changed while computing mesh inspection".into());
    }
    // Reuse the mesh defect overlay from the same headless operation.
    let marked = crate::buildercraft::inspect_mesh(app, id)?;
    debug_assert_eq!(marked, selected);
    app.ui.mesh_repair.inspection = Some(Inspection {
        object_id: id,
        document_uid: uid,
        revision,
        bad_vertices: selected,
        boundary_loops: loops,
        open_boundaries: open,
        invalid_edges,
    });
    Ok(())
}

fn run_edit(app: &mut CadApp, id: u64, action: Value) -> Result<(), String> {
    app.run("mesh3d.edit", json!({"id":id,"edit":action}))?;
    app.ui.mesh_repair.inspection = None;
    app.ui.mesh_defects = None;
    app.ui.mesh_face_object_id = None;
    app.ui.mesh_face_document_uid = None;
    app.ui.mesh_face_revision = None;
    Ok(())
}

/// Floating at the top of the app instead of the CAD command toolbar.
pub fn bar(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::top("ww_mesh_repair_bar").frame(egui::Frame::NONE.fill(PANEL_DEEP).inner_margin(egui::Margin::symmetric(12, 8))).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.colored_label(ACCENT, egui::RichText::new("WORLDWRIGHT").strong());
            ui.separator();
            ui.strong("MESH REPAIR");
            ui.weak("METROLOGY WORKSPACE");
            ui.separator();
            crate::hardware_profile::menu(app, ui);
            ui.separator();
            if ui.button("Open").clicked() {
                app.start("ui.open");
            }
            if ui.button("Save").clicked() {
                let _ = app.run("qsave", Value::Null);
            }
            if ui.button("Undo").clicked() {
                let _ = app.run("undo", json!({}));
            }
            if ui.button("Redo").clicked() {
                let _ = app.run("redo", json!({}));
            }
            ui.separator();
            if ui.button("Return to CAD").clicked() {
                app.start("ui.workspace.modeling");
            }
        });
    });
}

pub fn workspace_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::top("ww_mesh_repair_workspace").frame(egui::Frame::NONE.fill(PANEL).inner_margin(egui::Margin::symmetric(12, 5))).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.colored_label(ACCENT, "SCAN / SURFACE");
            ui.separator();
            ui.label("1  Project");
            ui.label("  →  2  Select");
            ui.label("  →  3  Repair");
            ui.label("  →  4  Verify");
            ui.separator();
            ui.label(if app.ui.mesh_repair.selecting { "SPACE · Selection active" } else { "SPACE · Navigation active" });
        });
    });
}

pub fn project_panel(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::left("ww_mesh_repair_tree")
        .default_size(230.)
        .resizable(true)
        .frame(egui::Frame::NONE.fill(PANEL).inner_margin(egui::Margin::same(10)))
        .show(ui, |ui| {
            ui.colored_label(ACCENT, "PROJECT EXPLORER");
            ui.separator();
            ui.strong("Measured meshes");
            let meshes = app.session.doc().map(|d| d.mesh3d.clone()).unwrap_or_default();
            if meshes.is_empty() {
                ui.weak("No mesh objects. Open a model or add an editable mesh.");
            }
            for mesh in meshes {
                let chosen = app.session.selection().contains(&cadcraft_doc::Handle(mesh.id));
                ui.horizontal(|ui| {
                    if ui.selectable_label(chosen, &mesh.name).clicked() {
                        app.session.set_selection(vec![cadcraft_doc::Handle(mesh.id)]);
                        app.ui.mesh_repair.inspection = None;
                        app.ui.mesh_face_object_id = None;
                        app.ui.mesh_face_revision = None;
                        app.ui.mesh_face_document_uid = None;
                    }
                    ui.weak(format!("{} f", mesh.mesh.faces.len()));
                });
                if chosen {
                    ui.small(format!("{} vertices  ·  {} polygons", mesh.mesh.vertices.len(), mesh.mesh.faces.len()));
                }
            }
            ui.separator();
            ui.colored_label(ACCENT, "REFERENCE / NOMINAL");
            let models = app.session.doc().map(|d| d.geometry3d.iter().map(|g| g.name.clone()).take(32).collect::<Vec<_>>()).unwrap_or_default();
            for name in models {
                ui.weak(name);
            }
            ui.separator();
            if ui.button("Add sample mesh").clicked() {
                let _ = crate::buildercraft::new_mesh_sample(app);
            }
        });
}

pub fn tool_panel(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::right("ww_mesh_repair_tools")
        .default_size(285.)
        .resizable(true)
        .frame(egui::Frame::NONE.fill(PANEL).inner_margin(egui::Margin::same(10)))
        .show(ui, |ui| {
            ui.colored_label(ACCENT, "SURFACE TOOLS");
            ui.separator();
            let selected = selected_mesh_id(app);
            let Some(id) = selected else {
                ui.label("Select a measured mesh from the Project Explorer.");
                ui.weak("SPACE switches between selection and scene navigation.");
                return;
            };
            if ui.button("Inspect mesh  [I]").clicked() {
                if let Err(err) = inspect(app, id) {
                    app.set_status(err);
                }
            }
            if let Some(record) = app.ui.mesh_repair.inspection.as_ref().filter(|r| r.object_id == id && current(app, r)) {
                ui.separator();
                ui.strong("QUALITY CHECK");
                ui.label(format!("Non-manifold vertices: {}", record.bad_vertices));
                ui.label(format!("Boundary loops: {}", record.boundary_loops));
                ui.label(format!("Open boundary chains: {}", record.open_boundaries));
                ui.label(format!("Invalid edge incidences: {}", record.invalid_edges));
            } else {
                ui.weak("Run inspection for a revision-bound quality summary.");
            }
            ui.separator();
            ui.strong("POLYGON REPAIR");
            let state = app.session.state().ok();
            let current_revision = state.map(|s| s.revision);
            if let (Some(object), Some(revision)) = (app.ui.mesh_face_object_id, app.ui.mesh_face_revision)
                && object == id
                && Some(revision) == current_revision
            {
                ui.label(format!("Picked face: {}", app.ui.mesh_face_index));
                if ui.button("Delete picked face").clicked() {
                    let operation = json!({"kind":"delete_faces","selected_revision":revision,"selected_faces":[app.ui.mesh_face_index]});
                    if let Err(err) = run_edit(app, id, operation) {
                        app.set_status(err);
                    }
                }
            }
            ui.horizontal(|ui| {
                ui.label("Boundary loop");
                ui.add(egui::DragValue::new(&mut app.ui.mesh_repair.hole_index).speed(1.));
            });
            if ui.button("Fill planar hole").clicked()
                && let Some(revision) = current_revision
            {
                let operation = json!({"kind":"fill_planar_hole","selected_revision":revision,"loop_index":app.ui.mesh_repair.hole_index});
                if let Err(err) = run_edit(app, id, operation) {
                    app.set_status(err);
                }
            }
            ui.separator();
            ui.strong("QUAD STRIP CUT");
            ui.small("Enter two edge vertex indices until edge picking is implemented.");
            ui.horizontal(|ui| {
                ui.label("Vertices");
                ui.add(egui::DragValue::new(&mut app.ui.mesh_repair.edge[0]).speed(1.));
                ui.add(egui::DragValue::new(&mut app.ui.mesh_repair.edge[1]).speed(1.));
            });
            ui.horizontal(|ui| {
                ui.label("Fraction");
                ui.add(egui::DragValue::new(&mut app.ui.mesh_repair.fraction).range(0.000001..=0.999999).speed(0.01));
            });
            if ui.button("Subdivide quad strip").clicked()
                && let Some(revision) = current_revision
            {
                let operation = json!({
                    "kind":"split_quad_strip","selected_revision":revision,
                    "edge_vertices":app.ui.mesh_repair.edge,"fraction":app.ui.mesh_repair.fraction
                });
                if let Err(err) = run_edit(app, id, operation) {
                    app.set_status(err);
                }
            }
            ui.separator();
            ui.weak("Deviation maps, alignment and datum systems will use shared metrology kernels.");
            ui.label("SPACE  Select / Navigate");
            ui.label("F  Fit model");
            ui.label("I  Inspect mesh");
            ui.label("ESC  Clear face pick");
        });
}

pub fn viewport_toolbar(app: &mut CadApp, ui: &mut egui::Ui) {
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(ACCENT, egui::RichText::new("SURFACE VIEW").strong());
        ui.separator();
        let selecting = app.ui.mesh_repair.selecting;
        if ui.selectable_label(!selecting, "Navigate  [SPACE]").clicked() {
            app.ui.mesh_repair.selecting = false;
        }
        if ui.selectable_label(selecting, "Select  [SPACE]").clicked() {
            app.ui.mesh_repair.selecting = true;
        }
        ui.separator();
        if ui.button("Fit  [F]").clicked() {
            let _ = app.run("ui.buildercraft.fit", Value::Null);
        }
        if ui.button("Top").clicked() {
            let _ = app.run("ui.buildercraft.top", Value::Null);
        }
        if ui.button("Iso").clicked() {
            let _ = app.run("ui.buildercraft.iso", Value::Null);
        }
        if ui.button("Inspect  [I]").clicked()
            && let Some(id) = selected_mesh_id(app)
        {
            let _ = inspect(app, id);
        }
    });
}

pub fn status_bar(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::Panel::bottom("ww_mesh_repair_status").frame(egui::Frame::NONE.fill(PANEL_DEEP).inner_margin(egui::Margin::symmetric(12, 5))).show(
        ui,
        |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(ACCENT, if app.ui.mesh_repair.selecting { "SELECTION" } else { "NAVIGATION" });
                ui.separator();
                ui.weak("Space: toggle  ·  Drag: orbit  ·  Shift-drag: pan  ·  Wheel: zoom");
                if let Some((message, _)) = &app.status {
                    ui.separator();
                    ui.label(message);
                }
            });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildercraft_kernel::{PolygonFace, PolygonMesh};
    use cadcraft_geom::Vec3;

    fn fixture() -> (CadApp, u64) {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let mesh = PolygonMesh {
            vertices: vec![Vec3::new(0., 0., 0.), Vec3::new(2., 0., 0.), Vec3::new(0., 2., 0.), Vec3::new(-2., 0., 0.), Vec3::new(0., -2., 0.)],
            faces: vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 3, 4])],
        };
        let id = app.run("mesh3d.create", json!({"name":"Pinched scan", "mesh":mesh})).unwrap()["id"].as_u64().unwrap();
        app.session.set_selection(vec![cadcraft_doc::Handle(id)]);
        (app, id)
    }

    fn space(ctx: &egui::Context, app: &mut CadApp) {
        let events = vec![egui::Event::Key { key: Key::Space, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE }];
        let mut full = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |_| shortcuts(app, ctx));
        full.textures_delta.clear();
    }

    #[test]
    fn spacebar_toggles_only_inside_mesh_workbench() {
        let (mut app, _) = fixture();
        let ctx = egui::Context::default();
        space(&ctx, &mut app);
        assert!(!app.ui.mesh_repair.selecting, "CAD space must remain untouched");
        set_active(&mut app, true);
        space(&ctx, &mut app);
        assert!(app.ui.mesh_repair.selecting);
        space(&ctx, &mut app);
        assert!(!app.ui.mesh_repair.selecting);
        set_active(&mut app, false);
        space(&ctx, &mut app);
        assert!(!app.ui.mesh_repair.selecting);
    }

    #[test]
    fn inspection_is_nondestructive_and_revision_bound() {
        let (mut app, id) = fixture();
        set_active(&mut app, true);
        let revision = app.session.state().unwrap().revision;
        let before = app.session.doc().unwrap().mesh3d.clone();
        assert!(inspect(&mut app, id).is_ok());
        let found = app.ui.mesh_repair.inspection.as_ref().unwrap();
        assert_eq!(found.bad_vertices, 1);
        assert!(current(&app, found));
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert_eq!(app.session.doc().unwrap().mesh3d, before);
        app.run("mesh3d.set", json!({"id":id,"name":"New scan name"})).unwrap();
        assert!(!current(&app, app.ui.mesh_repair.inspection.as_ref().unwrap()));
    }

    #[test]
    fn repair_uses_native_document_undo() {
        let (mut app, id) = fixture();
        let original = app.session.doc().unwrap().mesh3d[0].mesh.clone();
        let revision = app.session.state().unwrap().revision;
        run_edit(&mut app, id, json!({"kind":"delete_faces","selected_revision":revision,"selected_faces":[1]})).unwrap();
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.faces.len(), 1);
        app.run("undo", json!({})).unwrap();
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.as_ref(), original.as_ref());
    }
}
