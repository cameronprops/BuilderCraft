//! Disposable, bounded OpenCascade display proxy. Exact BRep bytes stay in the
//! document; these triangles and edges are never used as geometric source.
use cadcraft_geom::{Vec2, Vec3, camera::OrthoFrame};
use serde_json::Value;

const MAX_VERTICES: usize = 40_000;
const MAX_TRIANGLES: usize = 15_000;
const MAX_WIRE_POINTS: usize = 50_000;
const MAX_CACHED_SOLIDS: usize = 16;
const MAX_SCENE_VERTICES: usize = 60_000;
const MAX_SCENE_TRIANGLES: usize = 30_000;
const MAX_SCENE_WIRE_POINTS: usize = 80_000;

#[derive(Clone, Debug)]
pub struct BrepPreview {
    pub document_uid: u64,
    pub source_revision: u64,
    pub object_id: u64,
    pub mesh: DisplayMesh,
}

#[derive(Clone, Debug)]
pub struct DisplayMesh {
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
    edge_chains: Vec<Vec<Vec3>>,
}

fn point(value: &Value) -> Result<Vec3, String> {
    let values = value.as_array().filter(|v| v.len() == 3).ok_or("invalid exact BRep display point")?;
    let mut xyz = [0.; 3];
    for (index, coordinate) in xyz.iter_mut().enumerate() {
        *coordinate = values[index].as_f64().filter(|v| v.is_finite() && v.abs() <= 1e12).ok_or("invalid exact BRep display coordinate")?;
    }
    Ok(Vec3::new(xyz[0], xyz[1], xyz[2]))
}

fn points(value: &Value, limit: usize) -> Result<Vec<Vec3>, String> {
    let array = value.as_array().filter(|v| !v.is_empty() && v.len() <= limit).ok_or("exact BRep display vertex budget exceeded or empty")?;
    array.iter().map(point).collect()
}

impl BrepPreview {
    pub fn from_worker(reply: &Value, document_uid: u64, source_revision: u64, object_id: u64) -> Result<Self, String> {
        let mesh = reply.get("mesh").ok_or("exact BRep worker omitted display mesh")?;
        if mesh["exact"] != false {
            return Err("BRep display must remain a derived, inexact proxy".into());
        }
        let vertices = points(&mesh["vertices"], MAX_VERTICES)?;
        let normals = points(&mesh["normals"], MAX_VERTICES)?;
        if vertices.len() != normals.len() || normals.iter().any(|n| !(0.5..=1.5).contains(&n.dot(*n))) {
            return Err("BRep display vertex normals are invalid".into());
        }
        let indices = mesh["indices"].as_array().ok_or("exact BRep display has no triangle indices")?;
        if indices.is_empty() || indices.len() % 3 != 0 || indices.len() / 3 > MAX_TRIANGLES {
            return Err("exact BRep display triangle budget exceeded or invalid".into());
        }
        let mut triangles = Vec::with_capacity(indices.len() / 3);
        for face in indices.chunks_exact(3) {
            let mut triangle = [0; 3];
            for (slot, value) in triangle.iter_mut().zip(face) {
                *slot = value
                    .as_u64()
                    .and_then(|n| u32::try_from(n).ok())
                    .filter(|n| (*n as usize) < vertices.len())
                    .ok_or("exact BRep display triangle index is out of range")?;
            }
            if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
                return Err("exact BRep display triangle repeats a vertex".into());
            }
            triangles.push(triangle);
        }
        let chains = mesh["edge_chains"].as_array().ok_or("exact BRep display wire chains missing")?;
        let mut edge_chains = Vec::new();
        let mut wire_points = 0usize;
        for chain in chains {
            let array = chain.as_array().ok_or("invalid BRep wire chain")?;
            wire_points = wire_points.checked_add(array.len()).ok_or("BRep wire point count overflow")?;
            if wire_points > MAX_WIRE_POINTS {
                return Err("BRep display wire point budget exceeded".into());
            }
            if array.len() >= 2 {
                edge_chains.push(array.iter().map(point).collect::<Result<Vec<_>, _>>()?);
            }
        }
        Ok(Self { document_uid, source_revision, object_id, mesh: DisplayMesh { vertices, normals, triangles, edge_chains } })
    }

    pub fn current(&self, uid: u64, revision: u64) -> bool {
        self.document_uid == uid && self.source_revision == revision
    }

    /// Whole-solid hit only. OCCT face IDs are transient and never promoted into
    /// stable editable subobject identifiers from a lossy display triangle.
    pub fn pick_depth(&self, frame: OrthoFrame, center: Vec3, scale: f64, cursor: Vec2) -> Option<f64> {
        if !cursor.is_finite() || !center.is_finite() || !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let toward_camera = frame.right().cross(frame.up());
        self.mesh
            .triangles
            .iter()
            .filter_map(|triangle| {
                crate::mesh_picking::hit_triangle_vertices(&self.mesh.vertices, *triangle, frame, center, toward_camera, scale, cursor)
            })
            .max_by(f64::total_cmp)
    }

    /// Backward-compatible single-solid entrypoint using the same batcher as
    /// multi-solid display. Never tessellate exact topology in the UI.
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, frame: OrthoFrame, center: Vec3, scale: f64, selected: bool) {
        paint_previews([(self, selected)], painter, rect, frame, center, scale);
    }
}

/// Cross-document reuse is forbidden and the scene budget applies globally,
/// not once per object. Reject over-budget additions without evicting an
/// unrelated visible solid or partially changing the preview cache.
pub fn insert_preview(previews: &mut Vec<BrepPreview>, preview: BrepPreview) -> Result<(), String> {
    let mut objects = 1usize;
    let mut vertices = preview.mesh.vertices.len();
    let mut triangles = preview.mesh.triangles.len();
    let mut wires = preview.mesh.edge_chains.iter().map(|chain| chain.len()).sum::<usize>();
    for old in previews.iter().filter(|p| p.current(preview.document_uid, preview.source_revision) && p.object_id != preview.object_id) {
        objects += 1;
        vertices += old.mesh.vertices.len();
        triangles += old.mesh.triangles.len();
        wires += old.mesh.edge_chains.iter().map(|chain| chain.len()).sum::<usize>();
    }
    if objects > MAX_CACHED_SOLIDS || vertices > MAX_SCENE_VERTICES || triangles > MAX_SCENE_TRIANGLES || wires > MAX_SCENE_WIRE_POINTS {
        return Err("exact BRep scene preview budget exceeded; clear some previews before loading another".into());
    }
    previews.retain(|p| p.current(preview.document_uid, preview.source_revision) && p.object_id != preview.object_id);
    previews.push(preview);
    Ok(())
}

/// Draw all visible exact-solid previews in a single indexed egui mesh,
/// sorting triangles far-to-near. Vertex data are uploaded only once per
/// unique OCCT vertex instead of once per triangle, eliminating the former
/// 3x vertex duplication. This is painter ordering, NOT a GPU Z buffer:
/// intersecting triangles and mixed geometry still need depth attachments.
pub fn paint_previews<'a>(
    previews: impl IntoIterator<Item = (&'a BrepPreview, bool)>,
    painter: &egui::Painter,
    rect: egui::Rect,
    frame: OrthoFrame,
    center: Vec3,
    scale: f64,
) {
    if !rect.is_positive() || !scale.is_finite() || scale <= 0. || !center.is_finite() {
        return;
    }
    let camera_axis = frame.right().cross(frame.up());
    let light = camera_axis * 0.8 + frame.up() * 0.35 + frame.right() * -0.2;
    let mut mesh = egui::Mesh::default();
    let mut triangles: Vec<(f64, usize, [u32; 3])> = Vec::new();
    let mut wires: Vec<(&BrepPreview, bool)> = Vec::new();
    for (preview, selected) in previews {
        let positions: Vec<_> = preview
            .mesh
            .vertices
            .iter()
            .map(|p| {
                let q = frame.project(*p, center);
                egui::pos2(rect.center().x + (q.x * scale) as f32, rect.center().y - (q.y * scale) as f32)
            })
            .collect();
        if positions.iter().any(|p| !p.is_finite()) {
            continue;
        }
        let base_index = mesh.vertices.len() as u32;
        let base = if selected { [255., 175., 90.] } else { [125., 190., 230.] };
        for (position, normal) in positions.into_iter().zip(&preview.mesh.normals) {
            let intensity = (0.35 + 0.6 * normal.dot(light).max(0.)).clamp(0., 1.);
            mesh.colored_vertex(
                position,
                egui::Color32::from_rgb((base[0] * intensity) as u8, (base[1] * intensity) as u8, (base[2] * intensity) as u8),
            );
        }
        for local in &preview.mesh.triangles {
            let depth = local.iter().map(|id| (preview.mesh.vertices[*id as usize] - center).dot(camera_axis)).sum::<f64>() / 3.;
            triangles.push((depth, triangles.len(), local.map(|id| base_index + id)));
        }
        wires.push((preview, selected));
    }
    triangles.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    for (_, _, ids) in triangles {
        mesh.add_triangle(ids[0], ids[1], ids[2]);
    }
    if !mesh.indices.is_empty() {
        painter.add(egui::Shape::mesh(mesh));
    }
    // X-ray edge overlay is explicit until the 3D GPU pipeline owns a depth
    // attachment. Wire points are never written back into the CAD document.
    for (preview, selected) in wires {
        let color = if selected { egui::Color32::from_rgb(255, 210, 125) } else { egui::Color32::from_rgb(65, 115, 150) };
        for chain in &preview.mesh.edge_chains {
            for pair in chain.windows(2) {
                let positions = [pair[0], pair[1]].map(|p| {
                    let q = frame.project(p, center);
                    egui::pos2(rect.center().x + (q.x * scale) as f32, rect.center().y - (q.y * scale) as f32)
                });
                if positions.iter().all(|p| p.is_finite()) {
                    painter.line_segment(positions, egui::Stroke::new(0.8, color));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn triangle() -> Value {
        json!({"mesh":{"exact":false,
            "vertices":[[0.,0.,2.],[10.,0.,2.],[0.,10.,2.]],
            "normals":[[0.,0.,1.],[0.,0.,1.],[0.,0.,1.]],
            "indices":[0,1,2],
            "edge_chains":[[[0.,0.,2.],[10.,0.,2.],[0.,10.,2.],[0.,0.,2.]]]}})
    }

    #[test]
    fn bounded_proxy_picks_visible_triangle_without_editable_topology() {
        let preview = BrepPreview::from_worker(&triangle(), 91, 8, 32).unwrap();
        let frame = OrthoFrame { yaw: 0., pitch: -std::f64::consts::FRAC_PI_2 };
        assert!(preview.current(91, 8));
        assert!(!preview.current(92, 8));
        assert!(!preview.current(91, 9));
        assert_eq!(preview.pick_depth(frame, Vec3::ZERO, 1., Vec2::new(2., 2.)), Some(2.));
        assert_eq!(preview.pick_depth(frame, Vec3::ZERO, 1., Vec2::new(9., 9.)), None);
        assert_eq!(preview.pick_depth(frame, Vec3::ZERO, 0., Vec2::new(2., 2.)), None);
    }

    #[test]
    fn rejects_malformed_workers_and_unbounded_proxies() {
        let mut value = triangle();
        value["mesh"]["indices"] = json!([0, 1, 9]);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
        value = triangle();
        value["mesh"]["normals"] = json!([[0., 0., 0.]]);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
        value = triangle();
        value["mesh"]["indices"] = json!([0, 0, 2]);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
        value = triangle();
        value["mesh"]["exact"] = json!(true);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
        value = triangle();
        value["mesh"]["vertices"] = Value::Array(vec![json!([0., 0., 0.]); MAX_VERTICES + 1]);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
    }

    #[test]
    fn painter_emits_single_indexed_mesh_for_shaded_preview() {
        let preview = BrepPreview::from_worker(&triangle(), 1, 0, 7).unwrap();
        let ctx = egui::Context::default();
        let mut result = ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300., 300.));
            preview.paint(ui.painter(), rect, OrthoFrame { yaw: 0., pitch: 0. }, Vec3::ZERO, 10., true);
        });
        assert!(result.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Mesh(_))));
        result.textures_delta.clear();
    }
    #[test]
    fn multi_solid_batch_reuses_vertices_and_preserves_triangle_depth() {
        let first = BrepPreview::from_worker(&triangle(), 9, 0, 10).unwrap();
        let mut second = BrepPreview::from_worker(&triangle(), 9, 0, 11).unwrap();
        for vertex in &mut second.mesh.vertices {
            vertex.z += 3.;
        }
        let mut previews = Vec::new();
        insert_preview(&mut previews, first).unwrap();
        insert_preview(&mut previews, second).unwrap();
        assert_eq!(previews.len(), 2);
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300., 300.));
            paint_previews(
                previews.iter().map(|p| (p, false)),
                ui.painter(),
                rect,
                OrthoFrame { yaw: 0., pitch: -std::f64::consts::FRAC_PI_2 },
                Vec3::ZERO,
                10.,
            );
        });
        let meshes: Vec<_> = output.shapes.iter().filter_map(|shape| if let egui::Shape::Mesh(m) = &shape.shape { Some(m) } else { None }).collect();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].vertices.len(), 6, "triangles must share projected vertices");
        assert_eq!(meshes[0].indices.len(), 6);
        assert_eq!(&meshes[0].indices[..3], &[0, 1, 2], "rear triangle must draw first");
        assert_eq!(&meshes[0].indices[3..], &[3, 4, 5], "front triangle draws last");
        output.textures_delta.clear();
    }

    #[test]
    fn wire_budget_counts_points_across_loaded_solids_not_chain_count() {
        let mut fixture = triangle();
        // Each preview fits its individual 50K-point cap. Both together
        // exceed the global 80K scene budget despite having just two chains.
        fixture["mesh"]["edge_chains"] = json!([vec![[0., 0., 2.]; 40_100]]);
        let first = BrepPreview::from_worker(&fixture, 1, 2, 1).unwrap();
        let second = BrepPreview::from_worker(&fixture, 1, 2, 2).unwrap();
        let mut previews = Vec::new();
        insert_preview(&mut previews, first).unwrap();
        assert!(insert_preview(&mut previews, second).is_err());
        assert_eq!(previews.len(), 1, "rejected preview must leave prior scene intact");
        assert_eq!(previews[0].object_id, 1);
    }

    #[test]
    fn cache_replacement_is_bounded_transactional_and_revision_safe() {
        let fixture = triangle();
        let mut previews = Vec::new();
        for id in 0..MAX_CACHED_SOLIDS as u64 {
            insert_preview(&mut previews, BrepPreview::from_worker(&fixture, 1, 0, id).unwrap()).unwrap();
        }
        assert_eq!(previews.len(), MAX_CACHED_SOLIDS);
        let last = previews.last().unwrap().object_id;
        assert!(insert_preview(&mut previews, BrepPreview::from_worker(&fixture, 1, 0, 99).unwrap()).is_err());
        assert_eq!(previews.len(), MAX_CACHED_SOLIDS);
        assert_eq!(previews.last().unwrap().object_id, last);
        insert_preview(&mut previews, BrepPreview::from_worker(&fixture, 1, 0, 0).unwrap()).unwrap();
        assert_eq!(previews.len(), MAX_CACHED_SOLIDS);
        insert_preview(&mut previews, BrepPreview::from_worker(&fixture, 2, 0, 0).unwrap()).unwrap();
        assert_eq!(previews.len(), 1);
        assert!(previews[0].current(2, 0));
        insert_preview(&mut previews, BrepPreview::from_worker(&fixture, 2, 1, 8).unwrap()).unwrap();
        assert_eq!(previews.len(), 1);
        assert_eq!(previews[0].object_id, 8);
    }
}
