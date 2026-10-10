//! BMesh-inspired topological edge splitting for native polygon meshes.
//! This is an original Rust implementation over WorldWright's existing
//! index-based halfedges; it contains no Blender source code.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_topology, polygon_mesh_validate, polygon_mesh_vertex_fans};
use serde::{Deserialize, Serialize};

const MAX_EDIT_ELEMENTS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonEdgeSplitResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    /// Undirected canonical source edge.
    pub selected_edge: [u32; 2],
    /// The newly appended vertex, which remains selected after splitting.
    pub new_vertex_index: u32,
    /// Old polygon faces replaced in place (their face indices remain stable).
    pub affected_faces: Vec<u32>,
    /// Extra triangle faces appended in the same order as affected_faces.
    pub new_face_indices: Vec<u32>,
}

/// Split an edge shared by one or two native triangle faces.
///
/// `fraction` is measured from the caller's first `edge_vertices` endpoint
/// toward the second. Existing face IDs are preserved: each incident triangle
/// is replaced in place by one triangle and another is appended. This initial
/// operation deliberately rejects incident quads, rather than silently
/// triangulating and destroying original quad identity.
///
/// Requires a manifold, consistently oriented source mesh and an up-to-date
/// selection revision. Copies and validates the entire result before returning
/// it. Geometric face self-intersections are outside this primitive's scope.
pub fn polygon_mesh_split_edge(
    mesh: &PolygonMesh,
    current_revision: u64,
    selected_revision: u64,
    edge_vertices: [u32; 2],
    fraction: f64,
) -> Result<PolygonEdgeSplitResult> {
    if current_revision != selected_revision {
        return Err(KernelError::Conflict { expected: selected_revision, actual: current_revision });
    }
    if !fraction.is_finite() || !(1e-6..1.0 - 1e-6).contains(&fraction) {
        return Err(KernelError::Invalid("edge split fraction must lie inside (0, 1)"));
    }
    polygon_mesh_validate(mesh)?;
    if edge_vertices[0] == edge_vertices[1]
        || edge_vertices.iter().any(|&v| v as usize >= mesh.vertices.len())
    {
        return Err(KernelError::Invalid("invalid selected edge"));
    }
    if mesh.vertices.len() >= MAX_EDIT_ELEMENTS || mesh.faces.len() >= MAX_EDIT_ELEMENTS {
        return Err(KernelError::Budget);
    }

    let topology = polygon_mesh_topology(mesh)?;
    let vertex_fans = polygon_mesh_vertex_fans(mesh)?;
    if !topology.non_manifold_edges.is_empty()
        || !topology.inconsistent_winding_edges.is_empty()
        || !vertex_fans.non_manifold_vertices.is_empty()
    {
        return Err(KernelError::Invalid("repair non-manifold or inconsistent polygon topology before splitting"));
    }
    let canonical = [edge_vertices[0].min(edge_vertices[1]), edge_vertices[0].max(edge_vertices[1])];
    let edge = topology.edges.iter()
        .find(|edge| edge.vertices == canonical)
        .ok_or(KernelError::Invalid("selected edge does not exist"))?;
    if edge.halfedges.is_empty() || edge.halfedges.len() > 2 {
        return Err(KernelError::Invalid("selected edge has invalid incidence"));
    }
    if mesh.faces.len().checked_add(edge.halfedges.len()).is_none_or(|count| count > MAX_EDIT_ELEMENTS) {
        return Err(KernelError::Budget);
    }
    let next_revision = current_revision.checked_add(1).ok_or(KernelError::Budget)?;
    let new_vertex_index = u32::try_from(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    let start = mesh.vertices[edge_vertices[0] as usize];
    let end = mesh.vertices[edge_vertices[1] as usize];
    let new_vertex = start + (end - start) * fraction;
    if !new_vertex.is_finite() {
        return Err(KernelError::Invalid("interpolated vertex coordinate"));
    }

    // All changed and added faces are staged on a copy; failures are atomic.
    let mut result = mesh.clone();
    result.vertices.push(new_vertex);
    let mut affected_faces = Vec::new();
    let mut new_face_indices = Vec::new();
    let mut additions = Vec::new();
    for &halfedge_index in &edge.halfedges {
        let halfedge = &topology.halfedges[halfedge_index as usize];
        let corners = match mesh.faces[halfedge.face as usize] {
            PolygonFace::Triangle(corners) => corners,
            PolygonFace::Quad(_) => {
                return Err(KernelError::Invalid("quad edge split requires a quad-preserving subdivision operator"));
            }
        };
        let side = (0..3).find(|&i| corners[i] == halfedge.from && corners[(i + 1) % 3] == halfedge.to)
            .ok_or(KernelError::Invalid("edge and polygon winding mismatch"))?;
        let a = corners[side];
        let b = corners[(side + 1) % 3];
        let c = corners[(side + 2) % 3];
        result.faces[halfedge.face as usize] = PolygonFace::Triangle([a, new_vertex_index, c]);
        let new_face_index = u32::try_from(mesh.faces.len().checked_add(additions.len()).ok_or(KernelError::Budget)?)
            .map_err(|_| KernelError::Budget)?;
        additions.push(PolygonFace::Triangle([new_vertex_index, b, c]));
        affected_faces.push(halfedge.face);
        new_face_indices.push(new_face_index);
    }
    result.faces.extend(additions);
    polygon_mesh_validate(&result)?;
    let after = polygon_mesh_topology(&result)?;
    if !after.non_manifold_edges.is_empty() || !after.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("split produced invalid edge topology"));
    }
    let after_fans = polygon_mesh_vertex_fans(&result)?;
    if !after_fans.non_manifold_vertices.is_empty() {
        return Err(KernelError::Invalid("split produced non-manifold vertex fan"));
    }
    Ok(PolygonEdgeSplitResult {
        mesh: result,
        revision: next_revision,
        selected_edge: canonical,
        new_vertex_index,
        affected_faces,
        new_face_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn triangle() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![Vec3::new(0., 0., 0.), Vec3::new(2., 0., 0.), Vec3::new(1., 1., 0.)],
            faces: vec![PolygonFace::Triangle([0, 1, 2])],
        }
    }

    fn adjacent_triangles() -> PolygonMesh {
        let mut mesh = triangle();
        mesh.vertices.push(Vec3::new(1., -1., 0.));
        mesh.faces.push(PolygonFace::Triangle([1, 0, 3]));
        mesh
    }

    #[test]
    fn boundary_split_preserves_source_face_index_and_opens_no_new_holes() {
        let source = triangle();
        let result = polygon_mesh_split_edge(&source, 8, 8, [0, 1], 0.5);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.revision, 9);
            assert_eq!(result.selected_edge, [0, 1]);
            assert_eq!(result.new_vertex_index, 3);
            assert_eq!(result.affected_faces, vec![0]);
            assert_eq!(result.new_face_indices, vec![1]);
            assert_eq!(result.mesh.vertices[3], Vec3::new(1., 0., 0.));
            assert_eq!(result.mesh.faces, vec![
                PolygonFace::Triangle([0, 3, 2]),
                PolygonFace::Triangle([3, 1, 2]),
            ]);
            assert!(polygon_mesh_vertex_fans(&result.mesh).is_ok_and(|r| r.non_manifold_vertices.is_empty()));
        }
        assert_eq!(source.faces.len(), 1);
    }

    #[test]
    fn interior_split_updates_both_triangles_without_t_junction() {
        let source = adjacent_triangles();
        let result = polygon_mesh_split_edge(&source, 1, 1, [0, 1], 0.5);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.affected_faces, vec![0, 1]);
            assert_eq!(result.new_face_indices, vec![2, 3]);
            assert_eq!(result.mesh.faces.len(), 4);
            assert_eq!(result.mesh.vertices.len(), 5);
            let topo = polygon_mesh_topology(&result.mesh);
            assert!(topo.is_ok_and(|t| t.boundary_edges.len() == 4
                && t.inconsistent_winding_edges.is_empty() && t.non_manifold_edges.is_empty()));
            assert!(polygon_mesh_vertex_fans(&result.mesh).is_ok_and(|r| r.non_manifold_vertices.is_empty()));
        }
    }

    #[test]
    fn reversed_edge_order_interpolates_from_selected_start() {
        let r = polygon_mesh_split_edge(&triangle(), 0, 0, [1, 0], 0.25);
        assert!(r.is_ok_and(|r| r.mesh.vertices[3] == Vec3::new(1.5, 0., 0.) && r.selected_edge == [0, 1]));
    }

    #[test]
    fn rejects_stale_picks_and_invalid_fractions_without_mutating() {
        let source = triangle();
        let copy = source.clone();
        assert_eq!(polygon_mesh_split_edge(&source, 5, 4, [0, 1], 0.5),
            Err(KernelError::Conflict { expected: 4, actual: 5 }));
        for f in [0., 1., -1., 1e-8, f64::NAN, f64::INFINITY] {
            assert!(polygon_mesh_split_edge(&source, 0, 0, [0, 1], f).is_err());
        }
        assert_eq!(source, copy);
    }

    #[test]
    fn rejects_nonexistent_edge_and_bad_vertices() {
        let source = triangle();
        assert!(polygon_mesh_split_edge(&source, 0, 0, [0, 0], 0.5).is_err());
        assert!(polygon_mesh_split_edge(&source, 0, 0, [0, 77], 0.5).is_err());
        assert!(polygon_mesh_split_edge(&source, u64::MAX, u64::MAX, [0, 1], 0.5).is_err());
    }

    #[test]
    fn refuses_quad_edges_instead_of_destroying_quad_identity() {
        let source = PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.), Vec3::new(1., 0., 0.),
                Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.),
            ],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
        };
        let original = source.clone();
        assert!(polygon_mesh_split_edge(&source, 0, 0, [0, 1], 0.5).is_err());
        assert_eq!(source, original);
    }

    #[test]
    fn refuses_preexisting_bow_tie_and_invalid_winding() {
        let mut source = adjacent_triangles();
        source.faces = vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 1, 3])];
        assert!(polygon_mesh_split_edge(&source, 0, 0, [0, 1], 0.5).is_err());
        source.vertices.push(Vec3::new(-1., -1., 0.));
        source.faces = vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 3, 4])];
        assert!(polygon_mesh_split_edge(&source, 0, 0, [0, 1], 0.5).is_err());
    }
}
