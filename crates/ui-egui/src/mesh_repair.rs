//! WorldWright Mesh Repair: independent metrology-style task shell.
//! Space toggles Selection <-> Navigate as in classic PolyWorks, but all
//! geometry operations delegate to shared document/mesh Rust commands.
use crate::CadApp;
use buildercraft_kernel::{FillPlaneReport, PolygonHoleFillMode};
use cadcraft_geom::Vec3;
use egui::{Color32, Key, Modifiers};
use serde_json::{Value, json};

pub const PANEL: Color32 = Color32::from_rgb(37, 44, 50);
pub const PANEL_DEEP: Color32 = Color32::from_rgb(29, 36, 42);
pub const VIEWPORT: Color32 = Color32::from_rgb(20, 29, 35);
pub const ACCENT: Color32 = Color32::from_rgb(100, 205, 188);
pub const MESH: Color32 = Color32::from_rgb(131, 150, 161);
pub const PICK: Color32 = Color32::from_rgb(255, 179, 77);
pub const PATCH: Color32 = Color32::from_rgb(100, 225, 185);

#[derive(Clone, Debug, PartialEq)]
pub struct HolePatchPreview {
    pub object_id: u64,
    pub uid: u64,
    pub revision: u64,
    pub loop_index: u32,
    pub boundary_vertices: Vec<u32>,
    pub triangles: Vec<[u32; 3]>,
    pub source_vertex_count: u32,
    pub new_vertices: Vec<Vec3>,
    pub mode: PolygonHoleFillMode,
    pub cap_plane: Option<FillPlaneReport>,
}

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
    pub fill_mode: PolygonHoleFillMode,
    pub picking_hole: bool,
    pub patch: Option<HolePatchPreview>,
    pub inspection: Option<Inspection>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            active: false,
            selecting: false,
            edge: [0, 1],
            fraction: 0.5,
            hole_index: 0,
            fill_mode: PolygonHoleFillMode::PlanarOnly,
            picking_hole: false,
            patch: None,
            inspection: None,
        }
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
    app.ui.mesh_repair.picking_hole = false;
    app.ui.mesh_repair.patch = None;
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
        app.ui.mesh_repair.picking_hole = false;
        app.ui.mesh_repair.patch = None;
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
    let open = boundaries["report"]["unresolved_edges"].as_array().map_or(0, Vec::len);
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

/// Distance-based screen-space pick on the kernel's oriented boundary loops.
/// This is a UI projection query, not a second topology implementation.
pub fn closest_hole_loop(
    mesh: &buildercraft_kernel::PolygonMesh,
    loops: &[Vec<u32>],
    frame: cadcraft_geom::camera::OrthoFrame,
    center: cadcraft_geom::Vec3,
    scale: f64,
    cursor: cadcraft_geom::Vec2,
    radius: f64,
) -> Option<u32> {
    if !cursor.is_finite() || !center.is_finite() || !scale.is_finite() || scale <= 0. || !radius.is_finite() || radius <= 0. {
        return None;
    }
    let mut closest = radius * radius;
    let mut selected = None;
    for (loop_id, ids) in loops.iter().enumerate() {
        if ids.len() < 3 || ids.len() > 256 {
            continue;
        }
        for i in 0..ids.len() {
            let (Some(&a), Some(&b)) = (mesh.vertices.get(ids[i] as usize), mesh.vertices.get(ids[(i + 1) % ids.len()] as usize)) else { continue };
            let a = frame.project(a, center) * scale;
            let b = frame.project(b, center) * scale;
            let ab = b - a;
            if !a.is_finite() || !b.is_finite() {
                continue;
            }
            let length = ab.dot(ab);
            if length <= 1e-15 {
                continue;
            }
            let t = ((cursor - a).dot(ab) / length).clamp(0., 1.);
            let delta = cursor - (a + ab * t);
            let d = delta.dot(delta);
            if d < closest {
                closest = d;
                selected = u32::try_from(loop_id).ok();
            }
        }
    }
    selected
}

pub fn patch_is_current(app: &CadApp, patch: &HolePatchPreview) -> bool {
    app.session.state().is_ok_and(|s| s.uid == patch.uid && s.revision == patch.revision)
        && app.session.doc().is_ok_and(|doc| doc.mesh3d.iter().any(|m| m.id == patch.object_id))
}

/// Preview uses the exact same kernel operation as commit. Only patch
/// indices are transferred to UI state, never an entire replacement mesh.
/// Resolve transient vertices from the exact validated preview geometry.
pub fn preview_vertex(mesh: &buildercraft_kernel::PolygonMesh, patch: &HolePatchPreview, index: u32) -> Option<Vec3> {
    let len = mesh.vertices.len();
    if patch.source_vertex_count as usize != len {
        return None;
    }
    if (index as usize) < len { mesh.vertices.get(index as usize).copied() } else { patch.new_vertices.get(index as usize - len).copied() }
}

pub fn preview_hole(app: &mut CadApp, id: u64, loop_index: u32) -> Result<(), String> {
    let st = app.session.state().map_err(|e| e.to_string())?;
    let uid = st.uid;
    let revision = st.revision;
    let mode = app.ui.mesh_repair.fill_mode.clone();
    let response = app.run(
        "mesh3d.hole_preview",
        json!({
            "id":id,"loop_index":loop_index,"selected_revision":revision,"mode":mode
        }),
    )?;
    let raw = response["new_triangles"].as_array().ok_or("Missing patch triangles")?;
    if raw.len() > 1024 {
        return Err("Hole patch is too large".into());
    }
    let triangles = raw.iter().map(|v| serde_json::from_value::<[u32; 3]>(v.clone()).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
    let boundary_vertices = response["boundary_vertices"]
        .as_array()
        .ok_or("Missing boundary vertices")?
        .iter()
        .map(|v| v.as_u64().and_then(|n| u32::try_from(n).ok()).ok_or("Invalid boundary vertex"))
        .collect::<Result<Vec<_>, _>>()?;
    let source_vertex_count = response["source_vertex_count"].as_u64().and_then(|n| u32::try_from(n).ok()).ok_or("Invalid source vertex count")?;
    let new_vertices = response["new_vertices"]
        .as_array()
        .ok_or("Missing new patch vertices")?
        .iter()
        .map(|v| serde_json::from_value::<Vec3>(v.clone()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if new_vertices.len() > 256 {
        return Err("Too many planar cap vertices".into());
    }
    let cap_plane: Option<FillPlaneReport> = serde_json::from_value(response["cap_plane"].clone()).map_err(|e| e.to_string())?;
    let now = app.session.state().map_err(|e| e.to_string())?;
    if now.uid != uid || now.revision != revision || response["source_revision"].as_u64() != Some(revision) {
        return Err("Hole preview became stale".into());
    }
    app.ui.mesh_repair.hole_index = loop_index;
    app.ui.mesh_repair.patch = Some(HolePatchPreview {
        object_id: id,
        uid,
        revision,
        loop_index,
        boundary_vertices,
        triangles,
        source_vertex_count,
        new_vertices,
        mode,
        cap_plane,
    });
    app.ui.mesh_repair.picking_hole = false;
    app.set_status(format!("Validated hole {} preview, ready to commit", loop_index));
    Ok(())
}

/// Consume the click while boundary picking is armed, so a missed edge
/// cannot select an unrelated CAD face.
pub fn pick_hole_at(app: &mut CadApp, rect: egui::Rect, pointer: egui::Pos2) {
    let Some(id) = selected_mesh_id(app) else {
        app.set_status("Select a measured mesh first");
        return;
    };
    let result = (|| -> Result<Option<u32>, String> {
        let report = app.run("mesh3d.boundaries", json!({"id":id}))?;
        let lists = report["report"]["closed_loops"].as_array().ok_or("Missing boundary loop list")?;
        let loops: Vec<Vec<u32>> = lists
            .iter()
            .map(|v| {
                let vertices = v["vertices"].as_array().ok_or("Invalid loop")?;
                vertices.iter().map(|v| v.as_u64().and_then(|i| u32::try_from(i).ok()).ok_or("Invalid vertex")).collect()
            })
            .collect::<Result<_, _>>()?;
        let doc = app.session.doc().map_err(|e| e.to_string())?;
        let object = doc.mesh3d.iter().find(|o| o.id == id).ok_or("Missing selected mesh")?;
        if !object.visible || doc.layer(&object.layer).is_some_and(|l| !l.visible() || l.locked) {
            return Err("Cannot pick hidden or locked mesh".into());
        }
        if object.mesh.faces.len() > app.machine_profile.tuning.viewport_faces {
            return Err("Mesh exceeds visible face budget, enter loop index manually".into());
        }
        let camera = cadcraft_geom::camera::OrthoFrame { yaw: app.ui.orbit_yaw, pitch: app.ui.orbit_pitch };
        let cursor = cadcraft_geom::Vec2::new(f64::from(pointer.x - rect.center().x), f64::from(rect.center().y - pointer.y));
        Ok(closest_hole_loop(&object.mesh, &loops, camera, app.ui.center3d, app.ui.scale3d, cursor, 10.))
    })();
    match result {
        Ok(Some(loop_id)) => {
            if let Err(error) = preview_hole(app, id, loop_id) {
                app.set_status(error);
            }
        }
        Ok(None) => app.set_status("Click within 10 pixels of a visible hole boundary"),
        Err(error) => app.set_status(error),
    }
}

fn run_edit(app: &mut CadApp, id: u64, action: Value) -> Result<(), String> {
    app.run("mesh3d.edit", json!({"id":id,"edit":action}))?;
    app.ui.mesh_repair.inspection = None;
    app.ui.mesh_repair.patch = None;
    app.ui.mesh_repair.picking_hole = false;
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
                        app.ui.mesh_repair.patch = None;
                        app.ui.mesh_repair.picking_hole = false;
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
            ui.strong("INTERACTIVE HOLE FILL");
            ui.small("Select boundary, choose surface, preview and commit.");
            let old_mode = app.ui.mesh_repair.fill_mode.clone();
            egui::ComboBox::from_label("Fill surface")
                .selected_text(match &app.ui.mesh_repair.fill_mode {
                    PolygonHoleFillMode::PlanarOnly => "Planar boundary only",
                    PolygonHoleFillMode::Faceted => "Nonplanar faceted fill",
                    PolygonHoleFillMode::BestFitPlanar => "Flat cap: average best-fit",
                    PolygonHoleFillMode::BoundaryNormalPlanar => "Flat cap: automatic direction",
                    PolygonHoleFillMode::DirectionPlanar { .. } => "Flat cap: custom normal",
                })
                .show_ui(ui, |ui| {
                    for (label, mode) in [
                        ("Planar boundary only", PolygonHoleFillMode::PlanarOnly),
                        ("Follow nonplanar boundary", PolygonHoleFillMode::Faceted),
                        ("Average best-fit plane", PolygonHoleFillMode::BestFitPlanar),
                        ("Automatic boundary direction", PolygonHoleFillMode::BoundaryNormalPlanar),
                        ("Custom plane direction", PolygonHoleFillMode::DirectionPlanar { direction: Vec3::Z }),
                    ] {
                        ui.selectable_value(&mut app.ui.mesh_repair.fill_mode, mode, label);
                    }
                });
            if let PolygonHoleFillMode::DirectionPlanar { direction } = &mut app.ui.mesh_repair.fill_mode {
                ui.horizontal(|ui| {
                    ui.label("Normal");
                    ui.add(egui::DragValue::new(&mut direction.x).speed(0.05));
                    ui.add(egui::DragValue::new(&mut direction.y).speed(0.05));
                    ui.add(egui::DragValue::new(&mut direction.z).speed(0.05));
                });
            }
            if old_mode != app.ui.mesh_repair.fill_mode {
                app.ui.mesh_repair.patch = None;
            }
            if ui.selectable_label(app.ui.mesh_repair.picking_hole, "Pick hole boundary in viewport").clicked() {
                app.ui.mesh_repair.picking_hole = !app.ui.mesh_repair.picking_hole;
                app.ui.mesh_repair.selecting = true;
            }
            ui.horizontal(|ui| {
                ui.label("Loop");
                ui.add(egui::DragValue::new(&mut app.ui.mesh_repair.hole_index).speed(1.));
                if ui.button("Preview").clicked() {
                    let loop_index = app.ui.mesh_repair.hole_index;
                    if let Err(err) = preview_hole(app, id, loop_index) {
                        app.set_status(err);
                    }
                }
            });
            if let Some(patch) = app.ui.mesh_repair.patch.as_ref().filter(|p| p.object_id == id && patch_is_current(app, p)) {
                ui.colored_label(PATCH, format!("{} triangles, {} cap vertices", patch.triangles.len(), patch.new_vertices.len()));
                if let Some(plane) = &patch.cap_plane {
                    ui.small(format!("Plane deviation: RMS {:.5}, max {:.5} document units", plane.rms_distance, plane.max_distance));
                }
                let revision = patch.revision;
                let loop_index = patch.loop_index;
                let mode = patch.mode.clone();
                if ui.button("Commit fill").clicked() {
                    if let Err(err) = run_edit(
                        app,
                        id,
                        json!({
                            "kind":"fill_hole","selected_revision":revision,"loop_index":loop_index,"mode":mode
                        }),
                    ) {
                        app.set_status(err);
                    }
                }
            } else {
                ui.weak("Preview a simple hole first. Folded caps are rejected.");
            }
            if ui.button("Cancel hole preview").clicked() {
                app.ui.mesh_repair.patch = None;
                app.ui.mesh_repair.picking_hole = false;
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
            ui.label("ESC  Cancel hole tool / clear face");
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
        if app.ui.mesh_repair.picking_hole {
            ui.colored_label(PATCH, "Click a hole boundary edge");
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

    #[test]
    fn click_boundary_returns_nearest_inner_loop_or_none() {
        let mesh = PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(4., 0., 0.),
                Vec3::new(4., 4., 0.),
                Vec3::new(0., 4., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(3., 1., 0.),
                Vec3::new(3., 3., 0.),
                Vec3::new(1., 3., 0.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        };
        let loops = buildercraft_kernel::polygon_mesh_boundary_loops(&mesh).unwrap().closed_loops.into_iter().map(|l| l.vertices).collect::<Vec<_>>();
        let camera = cadcraft_geom::camera::OrthoFrame { yaw: 0., pitch: -std::f64::consts::FRAC_PI_2 };
        let hit = closest_hole_loop(&mesh, &loops, camera, Vec3::ZERO, 100., cadcraft_geom::Vec2::new(200., 101.), 10.);
        assert!(hit.is_some());
        assert!(loops[hit.unwrap() as usize].iter().all(|v| *v >= 4));
        assert!(closest_hole_loop(&mesh, &loops, camera, Vec3::ZERO, 100., cadcraft_geom::Vec2::new(220., 220.), 10.).is_none());
    }

    #[test]
    fn hole_preview_is_nonmutating_and_undo_clears_patch() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let mesh = PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(4., 0., 0.),
                Vec3::new(4., 4., 0.),
                Vec3::new(0., 4., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(3., 1., 0.),
                Vec3::new(3., 3., 0.),
                Vec3::new(1., 3., 0.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        };
        let id = app.run("mesh3d.create", json!({"name":"Ring","mesh":mesh})).unwrap()["id"].as_u64().unwrap();
        let boundary = app.run("mesh3d.boundaries", json!({"id":id})).unwrap();
        let index = boundary["report"]["closed_loops"]
            .as_array()
            .unwrap()
            .iter()
            .position(|loop_data| loop_data["vertices"].as_array().unwrap().iter().all(|v| v.as_u64().unwrap() >= 4))
            .unwrap() as u32;
        let revision = app.session.state().unwrap().revision;
        let old = app.session.doc().unwrap().mesh3d[0].mesh.clone();
        preview_hole(&mut app, id, index).unwrap();
        let patch = app.ui.mesh_repair.patch.clone().unwrap();
        assert_eq!(patch.triangles.len(), 2);
        assert!(patch_is_current(&app, &patch));
        assert_eq!(app.session.state().unwrap().revision, revision);
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh, old);
        run_edit(&mut app, id, json!({"kind":"fill_planar_hole","selected_revision":revision,"loop_index":index})).unwrap();
        assert!(app.ui.mesh_repair.patch.is_none());
        assert!(!patch_is_current(&app, &patch));
        app.run("undo", json!({})).unwrap();
        assert_eq!(app.session.doc().unwrap().mesh3d[0].mesh.as_ref(), old.as_ref());
    }

    #[test]
    fn planar_cap_preview_uses_generated_transient_vertices() {
        let mut app = CadApp::new(cadcraft_engine::Session::new(), crate::Services::default());
        let mesh = PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(4., 0., 0.),
                Vec3::new(4., 4., 0.),
                Vec3::new(0., 4., 0.),
                Vec3::new(1., 1., 0.07),
                Vec3::new(3., 1., 0.),
                Vec3::new(3., 3., -0.08),
                Vec3::new(1., 3., 0.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        };
        let id = app.run("mesh3d.create", json!({"name":"Warped boundary","mesh":mesh})).unwrap()["id"].as_u64().unwrap();
        let loops = app.run("mesh3d.boundaries", json!({"id":id})).unwrap();
        let index = loops["report"]["closed_loops"]
            .as_array()
            .unwrap()
            .iter()
            .position(|l| l["vertices"].as_array().unwrap().iter().all(|v| v.as_u64().unwrap() >= 4))
            .unwrap() as u32;
        let revision = app.session.state().unwrap().revision;
        app.ui.mesh_repair.fill_mode = PolygonHoleFillMode::BestFitPlanar;
        preview_hole(&mut app, id, index).unwrap();
        let patch = app.ui.mesh_repair.patch.as_ref().unwrap();
        assert_eq!(patch.triangles.len(), 10);
        assert_eq!(patch.new_vertices.len(), 4);
        assert!(patch.cap_plane.is_some());
        assert_eq!(app.session.state().unwrap().revision, revision);
        let original = &app.session.doc().unwrap().mesh3d[0].mesh;
        assert!(preview_vertex(original, patch, patch.source_vertex_count).is_some());
        assert_eq!(preview_vertex(original, patch, u32::MAX), None);
    }
}
