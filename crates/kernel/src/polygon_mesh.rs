//! Editable triangle/quad face storage. Faces retain original polygon identity;
//! triangulation is a derived view, not a destructive representation change.
use crate::{KernelError, Result, TriangleMesh};
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const MAX_ELEMENTS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolygonFace {
    Triangle([u32; 3]),
    Quad([u32; 4]),
}

impl PolygonFace {
    pub fn indices(self) -> Vec<u32> {
        match self {
            Self::Triangle(v) => v.to_vec(),
            Self::Quad(v) => v.to_vec(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolygonMesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<PolygonFace>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolygonTriangulation {
    pub mesh: TriangleMesh,
    /// For each triangle, the original polygon face index.
    pub source_face_indices: Vec<u32>,
}

fn triangle_normal(points: &[Vec3], a: u32, b: u32, c: u32) -> Result<Vec3> {
    let p = points[a as usize];
    let q = points[b as usize];
    let r = points[c as usize];
    let u = q - p;
    let v = r - p;
    let cross = u.cross(v);
    let length = cross.x.hypot(cross.y).hypot(cross.z);
    let scale = u.x.hypot(u.y).hypot(u.z).max(v.x.hypot(v.y).hypot(v.z)).max((r - q).x.hypot((r - q).y).hypot((r - q).z));
    if !length.is_finite() || scale == 0.0 || length <= 1e-12 * scale * scale {
        return Err(KernelError::Invalid("degenerate polygon triangle"));
    }
    Ok(cross * (1.0 / length))
}

fn face_triangles(face: PolygonFace) -> Vec<[u32; 3]> {
    match face {
        PolygonFace::Triangle(t) => vec![t],
        PolygonFace::Quad([a, b, c, d]) => vec![[a, b, c], [a, c, d]],
    }
}

/// Validate indexed triangles and quads. Quad diagonal 0-2 must produce
/// two nondegenerate, similarly oriented triangles. Concave and bow-tie
/// quads are rejected; nonplanar quads are permitted as faceted surfaces.
/// Does not yet check mesh-wide manifoldness or intersecting distant faces.
pub fn polygon_mesh_validate(mesh: &PolygonMesh) -> Result<()> {
    if mesh.vertices.len() > MAX_ELEMENTS || mesh.faces.len() > MAX_ELEMENTS {
        return Err(KernelError::Budget);
    }
    if mesh.vertices.iter().any(|p| !p.is_finite() || [p.x, p.y, p.z].iter().any(|v| v.abs() > 1e12)) {
        return Err(KernelError::Invalid("polygon coordinate"));
    }
    for face in &mesh.faces {
        let indices = face.indices();
        let mut unique = BTreeSet::new();
        for &index in &indices {
            if index as usize >= mesh.vertices.len() {
                return Err(KernelError::Invalid("polygon vertex index"));
            }
            if !unique.insert(index) {
                return Err(KernelError::Invalid("repeated polygon vertex"));
            }
        }
        let triangles = face_triangles(*face);
        let first = triangles[0];
        let normal = triangle_normal(&mesh.vertices, first[0], first[1], first[2])?;
        if let PolygonFace::Quad([a, b, c, d]) = *face {
            let n2 = triangle_normal(&mesh.vertices, a, c, d)?;
            if normal.dot(n2) <= 1e-10 {
                return Err(KernelError::Invalid("folded or concave quad"));
            }
            // For nonplanar quads, discourage a folded twist that passes
            // the chosen diagonal but reverses the other diagonal.
            let n3 = triangle_normal(&mesh.vertices, b, c, d)?;
            let n4 = triangle_normal(&mesh.vertices, b, d, a)?;
            if n3.dot(n4) <= 1e-10 {
                return Err(KernelError::Invalid("folded or concave quad"));
            }
        }
    }
    Ok(())
}

/// Lift a triangle mesh into editable polygon storage without changing
/// face order or vertex indices.
pub fn polygon_mesh_from_triangles(mesh: &TriangleMesh) -> Result<PolygonMesh> {
    let poly = PolygonMesh { vertices: mesh.vertices.clone(), faces: mesh.triangles.iter().copied().map(PolygonFace::Triangle).collect() };
    polygon_mesh_validate(&poly)?;
    Ok(poly)
}

/// Produce an indexed triangle view for existing kernel tools and renderers.
/// Source face mapping preserves quad identity in editing workflows.
pub fn polygon_mesh_triangulate(mesh: &PolygonMesh) -> Result<PolygonTriangulation> {
    polygon_mesh_validate(mesh)?;
    let face_budget = mesh
        .faces
        .iter()
        .try_fold(0usize, |sum, face| sum.checked_add(if matches!(face, PolygonFace::Quad(_)) { 2 } else { 1 }))
        .ok_or(KernelError::Budget)?;
    if face_budget > MAX_ELEMENTS {
        return Err(KernelError::Budget);
    }
    let mut triangles = Vec::new();
    let mut source_face_indices = Vec::new();
    triangles.try_reserve_exact(face_budget).map_err(|_| KernelError::Budget)?;
    source_face_indices.try_reserve_exact(face_budget).map_err(|_| KernelError::Budget)?;
    for (index, &face) in mesh.faces.iter().enumerate() {
        let source = u32::try_from(index).map_err(|_| KernelError::Budget)?;
        for triangle in face_triangles(face) {
            triangles.push(triangle);
            source_face_indices.push(source);
        }
    }
    Ok(PolygonTriangulation { mesh: TriangleMesh { vertices: mesh.vertices.clone(), triangles }, source_face_indices })
}

/// Return a new mesh with one native quad. The input is unchanged if invalid.
/// Duplicate polygon corner sets are rejected even with different winding.
/// Shared-edge manifold checks are a later topology milestone.
pub fn polygon_mesh_add_quad(mesh: &PolygonMesh, corners: [u32; 4]) -> Result<PolygonMesh> {
    if mesh.faces.len() >= MAX_ELEMENTS {
        return Err(KernelError::Budget);
    }
    polygon_mesh_validate(mesh)?;
    let mut candidate = mesh.clone();
    candidate.faces.push(PolygonFace::Quad(corners));
    polygon_mesh_validate(&candidate)?;
    let mut key = corners;
    key.sort_unstable();
    if mesh.faces.iter().any(|f| {
        if let PolygonFace::Quad(mut prior) = *f {
            prior.sort_unstable();
            prior == key
        } else {
            false
        }
    }) {
        return Err(KernelError::Invalid("duplicate quad face"));
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
        }
    }
    #[test]
    fn native_quad_stays_quad_and_triangulates_with_source_mapping() {
        let source = square();
        let result = polygon_mesh_triangulate(&source);
        assert!(result.is_ok_and(|r| r.mesh.triangles == vec![[0, 1, 2], [0, 2, 3]] && r.source_face_indices == vec![0, 0]));
        assert_eq!(source.faces.len(), 1);
        assert!(matches!(source.faces[0], PolygonFace::Quad(_)));
    }
    #[test]
    fn triangle_conversion_preserves_source_order() {
        let mesh = TriangleMesh { vertices: square().vertices, triangles: vec![[0, 1, 2], [0, 2, 3]] };
        let poly = polygon_mesh_from_triangles(&mesh);
        assert!(poly.is_ok_and(|p| p.faces == vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 2, 3])]));
    }
    #[test]
    fn adds_quad_without_mutating_source() {
        let mut source = square();
        source.faces.clear();
        let copy = source.clone();
        let added = polygon_mesh_add_quad(&source, [0, 1, 2, 3]);
        assert!(added.is_ok_and(|p| p.faces == vec![PolygonFace::Quad([0, 1, 2, 3])]));
        assert_eq!(source, copy);
    }
    #[test]
    fn rejects_duplicate_bowtie_and_bad_indices() {
        assert!(polygon_mesh_add_quad(&square(), [0, 1, 2, 3]).is_err());
        assert!(polygon_mesh_add_quad(&PolygonMesh { faces: vec![], ..square() }, [0, 2, 1, 3]).is_err());
        assert!(polygon_mesh_add_quad(&PolygonMesh { faces: vec![], ..square() }, [0, 1, 2, 44]).is_err());
        assert!(polygon_mesh_add_quad(&PolygonMesh { faces: vec![], ..square() }, [0, 1, 1, 3]).is_err());
    }
    #[test]
    fn accepts_nonplanar_quad_with_consistent_triangles() {
        let mut m = square();
        m.vertices[2].z = 0.1;
        assert!(polygon_mesh_validate(&m).is_ok());
    }
    #[test]
    fn rejects_degenerate_triangle_and_nonfinite_coordinate() {
        let mut m = square();
        m.faces = vec![PolygonFace::Triangle([0, 0, 2])];
        assert!(polygon_mesh_validate(&m).is_err());
        m = square();
        m.vertices[0].x = f64::NAN;
        assert!(polygon_mesh_validate(&m).is_err());
    }
}
