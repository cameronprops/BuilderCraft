//! Deterministic, double-sided polygon face picking for Worldwright's orthographic
//! wireframe viewport. This is a face picker, not a full depth-buffer occlusion
//! service (NURBS curves/surfaces are currently only displayed as wireframes).
use buildercraft_kernel::{PolygonFace, PolygonMesh};
use cadcraft_geom::{Vec2, Vec3, camera::OrthoFrame};

/// Keep selection and drawing limits identical. Larger meshes need a spatial
/// index/LOD service rather than invisible interactive faces.
pub const MAX_VIEWPORT_FACES: usize = 15_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickedMeshFace {
    pub object_id: u64,
    pub face_index: u32,
    /// Signed distance along the camera-facing axis; larger is nearer.
    pub depth: f64,
}

fn cross2(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

/// Orthographic ray hit with a projected triangle. Barycentric depth is
/// evaluated at the actual cursor, not the triangle's average vertex depth.
fn hit_triangle(
    mesh: &PolygonMesh,
    triangle: [u32; 3],
    camera: OrthoFrame,
    center: Vec3,
    toward_camera: Vec3,
    scale: f64,
    cursor: Vec2,
) -> Option<f64> {
    let a = *mesh.vertices.get(triangle[0] as usize)?;
    let b = *mesh.vertices.get(triangle[1] as usize)?;
    let c = *mesh.vertices.get(triangle[2] as usize)?;
    let projected = [a, b, c].map(|point| camera.project(point, center) * scale);
    if projected.iter().any(|p| !p.is_finite()) {
        return None;
    }
    let [p0, p1, p2] = projected;
    let area = cross2(p1 - p0, p2 - p0);
    // Edge-on or tiny projected faces are not selectable by area.
    if !area.is_finite() || area.abs() <= 1e-10 {
        return None;
    }
    let weights = [cross2(p1 - cursor, p2 - cursor) / area, cross2(p2 - cursor, p0 - cursor) / area, cross2(p0 - cursor, p1 - cursor) / area];
    if weights.iter().any(|w| !w.is_finite() || *w < -1e-9 || *w > 1.0 + 1e-9) {
        return None;
    }
    let depth = [a, b, c].into_iter().zip(weights).map(|(point, weight)| (point - center).dot(toward_camera) * weight).sum::<f64>();
    depth.is_finite().then_some(depth)
}

fn preferred(candidate: PickedMeshFace, current: Option<PickedMeshFace>) -> bool {
    match current {
        None => true,
        Some(other) => {
            // Depth dominates object order. Exact ties use IDs for reproducibility.
            candidate.depth > other.depth + 1e-9
                || ((candidate.depth - other.depth).abs() <= 1e-9
                    && (candidate.object_id, candidate.face_index) < (other.object_id, other.face_index))
        }
    }
}

/// Pick the nearest polygon face under a viewport-centered pixel position.
/// Cursor X points right and Y points UP, exactly like OrthoFrame::project.
/// Inputs include only visible, unlocked-for-picking objects from the caller.
/// Quads are projected using their same [0,2] diagonal as kernel triangulation,
/// but the returned face index always refers to the authoritative native quad.
pub fn pick_visible_mesh_face<'a>(
    objects: impl IntoIterator<Item = (u64, &'a PolygonMesh)>,
    camera: OrthoFrame,
    center: Vec3,
    scale: f64,
    cursor: Vec2,
) -> Option<PickedMeshFace> {
    if !cursor.is_finite() || !center.is_finite() || !camera.yaw.is_finite() || !camera.pitch.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let toward_camera = camera.right().cross(camera.up());
    let mut best = None;
    // The cap is shared across all visible objects, not separately per mesh.
    // Picking and viewport drawing traverse the same bounded prefix.
    let mut remaining_faces = MAX_VIEWPORT_FACES;
    for (object_id, mesh) in objects {
        let visible_faces = remaining_faces.min(mesh.faces.len());
        remaining_faces -= visible_faces;
        for (index, face) in mesh.faces.iter().take(visible_faces).enumerate() {
            let triangles = match face {
                PolygonFace::Triangle(indices) => [Some(*indices), None],
                PolygonFace::Quad([a, b, c, d]) => [Some([*a, *b, *c]), Some([*a, *c, *d])],
            };
            for triangle in triangles.into_iter().flatten() {
                if let Some(depth) = hit_triangle(mesh, triangle, camera, center, toward_camera, scale, cursor) {
                    let candidate = PickedMeshFace { object_id, face_index: index as u32, depth };
                    if preferred(candidate, best) {
                        best = Some(candidate);
                    }
                }
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top() -> OrthoFrame {
        OrthoFrame { yaw: 0.0, pitch: -std::f64::consts::FRAC_PI_2 }
    }
    fn square(z: f64) -> PolygonMesh {
        PolygonMesh {
            vertices: vec![Vec3::new(0., 0., z), Vec3::new(2., 0., z), Vec3::new(2., 2., z), Vec3::new(0., 2., z)],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
        }
    }
    fn pick<'a>(objects: impl IntoIterator<Item = (u64, &'a PolygonMesh)>, cursor: Vec2) -> Option<PickedMeshFace> {
        pick_visible_mesh_face(objects, top(), Vec3::ZERO, 100., cursor)
    }

    #[test]
    fn clicks_inside_a_quad_keep_native_face_index() {
        let mesh = square(0.);
        let result = pick([(9, &mesh)], Vec2::new(75., 125.));
        assert_eq!(result.map(|hit| (hit.object_id, hit.face_index)), Some((9, 0)));
        assert!(pick([(9, &mesh)], Vec2::new(250., 125.)).is_none());
    }

    #[test]
    fn picking_returns_frontmost_depth_not_first_drawn() {
        let rear = square(-5.);
        let front = square(2.);
        let cursor = Vec2::new(100., 100.);
        let a = pick([(1, &rear), (2, &front)], cursor);
        let b = pick([(2, &front), (1, &rear)], cursor);
        assert_eq!(a, b);
        assert_eq!(a.map(|hit| hit.object_id), Some(2));
        assert!(a.is_some_and(|hit| (hit.depth - 2.).abs() < 1e-8));
    }

    #[test]
    fn picks_triangle_faces_and_rejects_outside_silhouette() {
        let mut mesh = square(0.);
        mesh.faces = vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 2, 3])];
        assert_eq!(pick([(5, &mesh)], Vec2::new(150., 50.)).map(|hit| hit.face_index), Some(0));
        assert_eq!(pick([(5, &mesh)], Vec2::new(50., 150.)).map(|hit| hit.face_index), Some(1));
        assert_eq!(pick([(5, &mesh)], Vec2::new(-5., 50.)).map(|hit| hit.face_index), None);
    }

    #[test]
    fn top_view_edges_are_not_pickable_in_edge_on_front_view() {
        let mesh = square(0.);
        let front = OrthoFrame { yaw: 0., pitch: 0. };
        assert!(pick_visible_mesh_face([(4, &mesh)], front, Vec3::ZERO, 100., Vec2::new(100., 0.)).is_none());
    }

    #[test]
    fn rejects_bad_viewport_parameters() {
        let mesh = square(0.);
        assert!(pick_visible_mesh_face([(4, &mesh)], top(), Vec3::ZERO, 0., Vec2::new(100., 100.)).is_none());
        assert!(pick_visible_mesh_face([(4, &mesh)], top(), Vec3::ZERO, 100., Vec2::new(f64::NAN, 50.)).is_none());
    }

    #[test]
    fn stable_tie_break_for_coplanar_objects() {
        let mesh = square(0.);
        let a = pick([(8, &mesh), (2, &mesh)], Vec2::new(80., 70.));
        let b = pick([(2, &mesh), (8, &mesh)], Vec2::new(80., 70.));
        assert_eq!(a, b);
        assert_eq!(a.map(|hit| hit.object_id), Some(2));
    }

    #[test]
    fn off_center_camera_and_scale_are_respected() {
        let mesh = square(0.);
        let hit = pick_visible_mesh_face([(7, &mesh)], top(), Vec3::new(1., 1., 0.), 30., Vec2::new(0., 0.));
        assert_eq!(hit.map(|hit| hit.object_id), Some(7));
    }

    #[test]
    fn viewport_face_budget_is_global_across_visible_objects() {
        let mut filled = square(0.);
        filled.faces = vec![PolygonFace::Quad([0, 1, 2, 3]); MAX_VIEWPORT_FACES];
        let second = PolygonMesh {
            vertices: square(0.).vertices.iter().map(|v| Vec3::new(v.x + 10., v.y, v.z)).collect(),
            faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
        };
        let under_second = Vec2::new(1100., 100.);
        assert!(pick([(1, &filled), (2, &second)], under_second).is_none(), "undrawn faces must not be pickable");
        assert_eq!(pick([(2, &second), (1, &filled)], under_second).map(|hit| hit.object_id), Some(2));
    }

    #[test]
    fn returns_none_for_no_visible_mesh_objects() {
        let empty: [(u64, &PolygonMesh); 0] = [];
        assert!(pick(empty, Vec2::new(10., 10.)).is_none());
    }
}
