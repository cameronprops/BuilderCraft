//! Read-only closest-point queries on native triangle meshes and point samples.
//! One shared numeric implementation serves CAD and future scan/metrology adapters.
use crate::{Cancellation, KernelError, Result, TriangleMesh};
use cadcraft_geom::{Vec3, closest3d::closest_point_triangle};

const MAX_SAMPLES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshClosest {
    pub point: Vec3,
    pub distance: f64,
    pub triangle_index: usize,
    pub barycentric: [f64; 3],
    pub degenerate: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudClosest {
    pub point: Vec3,
    pub distance: f64,
    pub point_index: usize,
}

fn bounded(point: Vec3) -> bool {
    point.is_finite() && point.x.abs().max(point.y.abs()).max(point.z.abs()) <= 1e12
}

fn radius(max_distance: Option<f64>) -> Result<f64> {
    match max_distance {
        None => Ok(f64::INFINITY),
        Some(value) if value.is_finite() && (0.0..=1e12).contains(&value) => Ok(value),
        _ => Err(KernelError::Invalid("closest-point maximum distance")),
    }
}

/// Linear, bounded scan over triangle indices. Does not claim a spatial
/// acceleration structure or a signed surface deviation.
pub fn mesh_closest_point(
    mesh: &TriangleMesh,
    query: Vec3,
    max_distance: Option<f64>,
    cancellation: &Cancellation,
) -> Result<Option<MeshClosest>> {
    cancellation.check()?;
    let maximum = radius(max_distance)?;
    if !bounded(query) {
        return Err(KernelError::Invalid("closest-point query"));
    }
    if mesh.vertices.is_empty()
        || mesh.triangles.is_empty()
        || mesh.vertices.len() > MAX_SAMPLES
        || mesh.triangles.len() > MAX_SAMPLES
    {
        return Err(KernelError::Budget);
    }
    if mesh.vertices.iter().any(|&p| !bounded(p)) {
        return Err(KernelError::Invalid("closest-point mesh vertex"));
    }
    let mut best: Option<MeshClosest> = None;
    for (index, triangle) in mesh.triangles.iter().enumerate() {
        if index % 256 == 0 {
            cancellation.check()?;
        }
        let &[a, b, c] = triangle;
        let (a, b, c) = (
            mesh.vertices.get(a as usize).copied().ok_or(KernelError::Invalid("closest-point mesh index"))?,
            mesh.vertices.get(b as usize).copied().ok_or(KernelError::Invalid("closest-point mesh index"))?,
            mesh.vertices.get(c as usize).copied().ok_or(KernelError::Invalid("closest-point mesh index"))?,
        );
        let hit = closest_point_triangle(query, a, b, c).ok_or(KernelError::Invalid("closest-point triangle"))?;
        if hit.distance <= maximum && best.is_none_or(|old| hit.distance < old.distance) {
            best = Some(MeshClosest {
                point: hit.point,
                distance: hit.distance,
                triangle_index: index,
                barycentric: hit.barycentric,
                degenerate: hit.degenerate,
            });
        }
    }
    cancellation.check()?;
    Ok(best)
}

/// Linear, bounded point-cloud nearest sample; no interpolated surface or
/// scan-to-CAD registration is implied. Equal distances select first index.
pub fn cloud_closest_point(
    points: &[Vec3],
    query: Vec3,
    max_distance: Option<f64>,
    cancellation: &Cancellation,
) -> Result<Option<CloudClosest>> {
    cancellation.check()?;
    let maximum = radius(max_distance)?;
    if points.is_empty() || points.len() > MAX_SAMPLES {
        return Err(KernelError::Budget);
    }
    if !bounded(query) {
        return Err(KernelError::Invalid("closest-point query"));
    }
    let mut best: Option<CloudClosest> = None;
    for (index, &point) in points.iter().enumerate() {
        if index % 256 == 0 {
            cancellation.check()?;
        }
        if !bounded(point) {
            return Err(KernelError::Invalid("point-cloud coordinate"));
        }
        let distance = (query - point).len();
        if distance <= maximum && best.is_none_or(|old| distance < old.distance) {
            best = Some(CloudClosest { point, distance, point_index: index });
        }
    }
    cancellation.check()?;
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
                Vec3::new(0.0, 2.0, 0.0),
                Vec3::new(0.0, 0.0, 4.0),
            ],
            triangles: vec![[0, 1, 2], [0, 1, 3]],
        }
    }

    #[test]
    fn nearest_face_and_stable_ties_preserve_barycentric_coordinates() {
        let source = mesh();
        let query = Vec3::new(0.5, 0.5, -1.0);
        let result = mesh_closest_point(&source, query, None, &Cancellation::default()).unwrap().unwrap();
        assert_eq!(result.triangle_index, 0);
        assert_eq!(result.point, Vec3::new(0.5, 0.5, 0.0));
        assert!((result.distance - 1.0).abs() < 1e-12);
        assert_eq!(result.barycentric, [0.5, 0.25, 0.25]);
        assert!(mesh_closest_point(&source, query, Some(0.9), &Cancellation::default()).unwrap().is_none());
        let tie = TriangleMesh { triangles: vec![[0, 1, 2], [0, 1, 2]], ..source };
        assert_eq!(
            mesh_closest_point(&tie, query, None, &Cancellation::default()).unwrap().map(|p| p.triangle_index),
            Some(0)
        );
    }

    #[test]
    fn cloud_query_rejects_bad_input_and_retains_stable_index() {
        let samples = [Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        let cancellation = Cancellation::default();
        let result = cloud_closest_point(&samples, Vec3::ZERO, None, &cancellation).unwrap().unwrap();
        assert_eq!(result.point_index, 0);
        assert_eq!(result.distance, 1.0);
        assert!(cloud_closest_point(&samples, Vec3::ZERO, Some(0.5), &cancellation).unwrap().is_none());
        assert!(cloud_closest_point(&samples, Vec3::ZERO, Some(-1.0), &cancellation).is_err());
        assert!(cloud_closest_point(&[Vec3::new(f64::NAN, 0.0, 0.0)], Vec3::ZERO, None, &cancellation).is_err());
    }

    #[test]
    fn cancellation_and_invalid_index_are_errors() {
        let cancellation = Cancellation::default();
        cancellation.cancel();
        assert_eq!(mesh_closest_point(&mesh(), Vec3::ZERO, None, &cancellation), Err(KernelError::Cancelled));
        let source = TriangleMesh { triangles: vec![[0, 1, 99]], ..mesh() };
        assert!(mesh_closest_point(&source, Vec3::ZERO, None, &Cancellation::default()).is_err());
        let source = TriangleMesh { triangles: vec![[0, 0, 0]], ..mesh() };
        assert!(mesh_closest_point(&source, Vec3::ZERO, None, &Cancellation::default()).unwrap().unwrap().degenerate);
    }
}
