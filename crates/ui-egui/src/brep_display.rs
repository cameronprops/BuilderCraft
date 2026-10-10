//! Disposable, bounded OpenCascade display proxy. Exact BRep bytes stay in the
//! document; these triangles and edges are never used as geometric source.
use cadcraft_geom::{Vec2, Vec3, camera::OrthoFrame};
use serde_json::Value;

const MAX_VERTICES: usize = 40_000;
const MAX_TRIANGLES: usize = 15_000;
const MAX_WIRE_POINTS: usize = 50_000;

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
        *coordinate = values[index].as_f64().filter(|v| v.is_finite() && v.abs() <= 1e12)
            .ok_or("invalid exact BRep display coordinate")?;
    }
    Ok(Vec3::new(xyz[0], xyz[1], xyz[2]))
}

fn points(value: &Value, limit: usize) -> Result<Vec<Vec3>, String> {
    let array = value.as_array().filter(|v| !v.is_empty() && v.len() <= limit)
        .ok_or("exact BRep display vertex budget exceeded or empty")?;
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
                *slot = value.as_u64().and_then(|n| u32::try_from(n).ok())
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
        self.mesh.triangles.iter().filter_map(|triangle| {
            crate::mesh_picking::hit_triangle_vertices(
                &self.mesh.vertices, *triangle, frame, center, toward_camera, scale, cursor
            )
        }).max_by(f64::total_cmp)
    }

    /// Painter's algorithm for a single preview solid, with one egui Mesh
    /// allocation per frame. No FFI, new renderer, tessellation or GPU readback.
    /// Intersections with *other* drawing types still require a common depth
    /// buffer later; this does not claim occlusion-correct mixed-scene shading.
    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, frame: OrthoFrame, center: Vec3, scale: f64, selected: bool) {
        if !rect.is_positive() || !scale.is_finite() || scale <= 0.0 {
            return;
        }
        let camera_axis = frame.right().cross(frame.up());
        let light = camera_axis * 0.8 + frame.up() * 0.35 + frame.right() * -0.2;
        let positions: Vec<_> = self.mesh.vertices.iter().map(|p| {
            let q = frame.project(*p, center);
            egui::pos2(rect.center().x + (q.x * scale) as f32, rect.center().y - (q.y * scale) as f32)
        }).collect();
        let base = if selected { [255., 175., 90.] } else { [125., 190., 230.] };
        let colors: Vec<_> = self.mesh.normals.iter().map(|n| {
            let intensity = (0.35 + 0.6 * n.dot(light).max(0.0)).clamp(0., 1.);
            egui::Color32::from_rgb(
                (base[0] * intensity) as u8, (base[1] * intensity) as u8, (base[2] * intensity) as u8
            )
        }).collect();
        let mut order: Vec<(f64, usize)> = self.mesh.triangles.iter().enumerate().map(|(i, ids)| {
            let depth = ids.iter().map(|id| (self.mesh.vertices[*id as usize] - center).dot(camera_axis)).sum::<f64>() / 3.0;
            (depth, i)
        }).collect();
        // Farthest first; deterministic ties avoid flickering during orbit.
        order.sort_unstable_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut mesh = egui::Mesh::default();
        for (_, index) in order {
            let triangle = self.mesh.triangles[index];
            if triangle.iter().any(|id| !positions[*id as usize].is_finite()) {
                continue;
            }
            let start = mesh.vertices.len() as u32;
            for id in triangle {
                mesh.colored_vertex(positions[id as usize], colors[id as usize]);
            }
            mesh.add_triangle(start, start + 1, start + 2);
        }
        if !mesh.indices.is_empty() {
            painter.add(egui::Shape::mesh(mesh));
        }
        // A lightweight X-ray wire overlay is deliberate. Hidden-line removal
        // will use the common depth buffer, not BRep topology mutation.
        let wire_color = if selected {
            egui::Color32::from_rgb(255, 210, 125)
        } else {
            egui::Color32::from_rgb(65, 115, 150)
        };
        for chain in &self.mesh.edge_chains {
            for pair in chain.windows(2) {
                let points = [pair[0], pair[1]].map(|p| {
                    let q = frame.project(p, center);
                    egui::pos2(rect.center().x + (q.x * scale) as f32, rect.center().y - (q.y * scale) as f32)
                });
                if points.iter().all(|p| p.is_finite()) {
                    painter.line_segment(points, egui::Stroke::new(0.8, wire_color));
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
        value["mesh"]["indices"] = json!([0,1,9]);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
        value = triangle();
        value["mesh"]["normals"] = json!([[0.,0.,0.]]);
        assert!(BrepPreview::from_worker(&value, 1, 1, 1).is_err());
        value = triangle();
        value["mesh"]["indices"] = json!([0,0,2]);
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
}
