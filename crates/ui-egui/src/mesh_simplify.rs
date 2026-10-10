//! Explicit, revision-bound QEM preview and bake controls for native polygon meshes.
//!
//! The engine owns all geometric decisions. UI caches a disposable triangle
//! wire overlay only, and never modifies source geometry outside the undoable
//! CAD command. Cached preview is not a selectable model object.
use crate::CadApp;
use buildercraft_kernel::{TriangleMesh, validate_triangle_mesh};
use cadcraft_geom::Vec3;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MAX_PREVIEW_FACES: usize = 20_000;
const MAX_PREVIEW_EDGES: usize = 40_000;

#[derive(Clone, Debug, PartialEq)]
struct Options {
    target_faces: usize,
    max_quadric_error: f64,
    max_normal_change_degrees: f64,
    preserve_boundary: bool,
    preserve_creases: bool,
    crease_degrees: f64,
    replace: bool,
}

impl Options {
    fn default_for(triangles: usize) -> Self {
        Self {
            target_faces: triangles / 2,
            max_quadric_error: 1e12,
            max_normal_change_degrees: 80.0,
            preserve_boundary: true,
            preserve_creases: true,
            crease_degrees: 65.0,
            replace: false,
        }
    }

    fn request(&self, id: u64, revision: u64, mode: Option<&str>) -> Value {
        let mut value = json!({
            "id":id, "selected_revision":revision,
            "target_faces":self.target_faces,
            "max_quadric_error":self.max_quadric_error,
            "max_normal_change_degrees":self.max_normal_change_degrees,
            "preserve_boundary":self.preserve_boundary,
        });
        if self.preserve_creases {
            value["preserve_creases_above_degrees"] = json!(self.crease_degrees);
        }
        if let Some(mode) = mode {
            value["mode"] = json!(mode);
        }
        value
    }

    fn valid(&self, faces: usize) -> bool {
        self.target_faces < faces
            && self.max_quadric_error.is_finite()
            && self.max_quadric_error >= 0.0
            && self.max_normal_change_degrees.is_finite()
            && (0.0..89.0).contains(&self.max_normal_change_degrees)
            && (!self.preserve_creases || (self.crease_degrees.is_finite() && (0.0..=180.0).contains(&self.crease_degrees)))
    }
}

#[derive(Clone, Debug)]
struct Controls {
    options: Options,
    info: Option<String>,
}
impl Controls {
    fn new(triangles: usize) -> Self {
        Self { options: Options::default_for(triangles), info: None }
    }
}

/// Disposable viewport cache: only the native document is saved as .dftba.
#[derive(Clone, Debug)]
pub struct Preview {
    pub document_uid: u64,
    pub source_revision: u64,
    pub object_id: u64,
    pub mesh: TriangleMesh,
    pub wire_edges: Vec<[u32; 2]>,
    pub removed_faces: usize,
    pub target_reached: bool,
}

impl Preview {
    fn decode(reply: &Value, document_uid: u64, source_revision: u64, object_id: u64) -> Result<Self, String> {
        if reply["id"].as_u64() != Some(object_id) || reply["source_revision"].as_u64() != Some(source_revision) {
            return Err("simplification preview no longer matches selected document".into());
        }
        let vertices: Vec<Vec3> = serde_json::from_value(reply["vertices"].clone()).map_err(|e| format!("preview vertices: {e}"))?;
        let triangles: Vec<[u32; 3]> = serde_json::from_value(reply["triangles"].clone()).map_err(|e| format!("preview triangles: {e}"))?;
        if triangles.len() > MAX_PREVIEW_FACES || vertices.len() > 100_000 {
            return Err("simplification preview exceeds interactive memory budget".into());
        }
        let mesh = TriangleMesh { vertices, triangles };
        validate_triangle_mesh(&mesh).map_err(|e| format!("invalid simplification preview: {e}"))?;
        let mut edges = BTreeSet::new();
        for face in &mesh.triangles {
            for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                edges.insert([a.min(b), a.max(b)]);
                if edges.len() > MAX_PREVIEW_EDGES {
                    return Err("simplification preview wire budget exceeded".into());
                }
            }
        }
        let removed_faces = reply["removed_faces"].as_u64().ok_or("preview missing removed face count")?;
        let removed_faces = usize::try_from(removed_faces).map_err(|_| "preview removed face count exceeds machine bounds")?;
        let target_reached = reply["target_reached"].as_bool().ok_or("preview missing target flag")?;
        Ok(Self { document_uid, source_revision, object_id, mesh, wire_edges: edges.into_iter().collect(), removed_faces, target_reached })
    }

    pub fn matches(&self, uid: u64, revision: u64, object_id: u64) -> bool {
        self.document_uid == uid && self.source_revision == revision && self.object_id == object_id
    }
}

/// Each document/object keeps transient slider settings in egui memory.
/// Preview/bake requests are intentionally button-triggered: no QEM solver on
/// every frame and no hidden CPU/GPU workload from dragging a slider.
/// Returns true after a successful document mutation, so callers can clear
/// face-pick state bound to the old revision.
pub fn panel(app: &mut CadApp, ui: &mut egui::Ui, object_id: u64, source_triangles: usize) -> bool {
    let Ok(state) = app.session.state() else {
        ui.weak("No active document");
        return false;
    };
    let (document_uid, revision) = (state.uid, state.revision);
    let key = egui::Id::new(("worldwright-qem-controls", document_uid, object_id));
    let mut controls = ui.ctx().data_mut(|data| data.get_temp::<Controls>(key)).unwrap_or_else(|| Controls::new(source_triangles));
    let previous = controls.options.clone();
    ui.separator();
    ui.strong("Simplify Mesh");
    ui.small(format!("{source_triangles} source triangles. QEM respects boundaries, creases and normal limits."));
    ui.add(egui::Slider::new(&mut controls.options.target_faces, 0..=source_triangles).text("Target triangles").integer());
    ui.checkbox(&mut controls.options.preserve_boundary, "Preserve border vertices");
    ui.checkbox(&mut controls.options.preserve_creases, "Preserve sharp edges");
    if controls.options.preserve_creases {
        ui.add(egui::Slider::new(&mut controls.options.crease_degrees, 0.0..=180.0).text("Crease angle").suffix("°"));
    }
    ui.add(egui::Slider::new(&mut controls.options.max_normal_change_degrees, 0.0..=88.0).text("Max normal change").suffix("°"));
    ui.horizontal(|ui| {
        ui.label("Max quadric error");
        ui.add(egui::DragValue::new(&mut controls.options.max_quadric_error).speed(0.01).range(0.0..=1e15));
    });
    ui.checkbox(&mut controls.options.replace, "Replace original mesh (destructive; undoable)");
    if controls.options != previous {
        controls.info = None;
        if app.ui.mesh_simplify_preview.as_ref().is_some_and(|p| p.object_id == object_id) {
            app.ui.mesh_simplify_preview = None;
        }
    }
    let valid = controls.options.valid(source_triangles) && source_triangles <= MAX_PREVIEW_FACES;
    if source_triangles > MAX_PREVIEW_FACES {
        ui.small("Preview/bake interactive limit: 20,000 source triangles. Headless decimation supports larger bounded inputs.");
    }
    let fresh = app.ui.mesh_simplify_preview.as_ref().is_some_and(|p| p.matches(document_uid, revision, object_id));
    let removable = fresh && app.ui.mesh_simplify_preview.as_ref().is_some_and(|p| p.removed_faces > 0);
    let mut applied = false;
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(valid, egui::Button::new("Preview reduced mesh")).clicked() {
            match app.run("mesh3d.preview_decimate", controls.options.request(object_id, revision, None)) {
                Ok(reply) => {
                    let snapshot = app.session.state().ok().map(|st| (st.uid, st.revision));
                    if snapshot != Some((document_uid, revision)) {
                        controls.info = Some("Document changed during preview; rerun preview.".into());
                        app.ui.mesh_simplify_preview = None;
                    } else {
                        match Preview::decode(&reply, document_uid, revision, object_id) {
                            Ok(preview) => {
                                controls.info = Some(format!("Preview: {} triangles removed. Target {}.", preview.removed_faces, if preview.target_reached { "reached" } else { "limited by protected geometry" }));
                                app.ui.mesh_simplify_preview = Some(preview);
                            }
                            Err(error) => {
                                controls.info = Some(error);
                                app.ui.mesh_simplify_preview = None;
                            }
                        }
                    }
                }
                Err(error) => {
                    controls.info = Some(error);
                    app.ui.mesh_simplify_preview = None;
                }
            }
        }
        let action = if controls.options.replace { "Apply replacement" } else { "Bake as new mesh" };
        if ui.add_enabled(valid && removable, egui::Button::new(action)).clicked() {
            let mode = if controls.options.replace { "replace" } else { "copy" };
            match app.run("mesh3d.simplify", controls.options.request(object_id, revision, Some(mode))) {
                Ok(reply) => {
                    controls.info = Some(format!("Created {}-triangle mesh (object {}). Use Undo to revert.", reply["result_faces"], reply["id"]));
                    app.ui.mesh_simplify_preview = None;
                    applied = true;
                }
                Err(error) => controls.info = Some(error),
            }
        }
        if ui.add_enabled(fresh, egui::Button::new("Clear preview")).clicked() {
            app.ui.mesh_simplify_preview = None;
            controls.info = None;
        }
    });
    if fresh {
        ui.small("Amber wire is a read-only, non-selectable preview; the source mesh remains pickable until Apply.");
    }
    if let Some(info) = &controls.info {
        ui.label(info);
    }
    ui.ctx().data_mut(|data| data.insert_temp(key, controls));
    applied
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_options_keep_original_and_reject_noop() {
        let c = Options::default_for(8);
        assert_eq!(c.target_faces, 4);
        assert!(!c.replace);
        assert!(c.valid(8));
        let mut no_op = c.clone();
        no_op.target_faces = 8;
        assert!(!no_op.valid(8));
        let mut invalid = c;
        invalid.max_quadric_error = f64::NAN;
        assert!(!invalid.valid(8));
    }

    #[test]
    fn optional_crease_option_does_not_serialize_null_as_numeric() {
        let mut settings = Options::default_for(8);
        settings.preserve_creases = false;
        let request = settings.request(7, 3, Some("copy"));
        assert!(request.get("preserve_creases_above_degrees").is_none());
        assert_eq!(request["mode"], "copy");
        assert_eq!(request["selected_revision"], 3);
    }

    #[test]
    fn preview_requires_matching_revision_and_valid_triangles() {
        let response = json!({
            "id":9, "source_revision":10, "removed_faces":2,
            "target_reached":true,
            "vertices":[{"x":0.0,"y":0.0,"z":0.0},{"x":1.0,"y":0.0,"z":0.0},{"x":0.0,"y":1.0,"z":0.0}],
            "triangles":[[0,1,2]]
        });
        let p = Preview::decode(&response, 4, 10, 9);
        assert!(p.as_ref().is_ok_and(|v| v.matches(4, 10, 9) && v.wire_edges.len() == 3));
        assert!(Preview::decode(&response, 4, 11, 9).is_err());
        let mut bad = response;
        bad["triangles"] = json!([[0,1,4]]);
        assert!(Preview::decode(&bad, 4, 10, 9).is_err());
    }
}
