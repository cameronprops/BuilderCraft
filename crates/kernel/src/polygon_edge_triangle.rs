//! Extend a native polygon mesh from one boundary edge and an existing vertex.
//! All picks are revision-bound; the input mesh is immutable.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_topology, polygon_mesh_validate};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonEdgeTriangleResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    pub new_face_index: u32,
    /// Oriented to oppose the existing boundary edge.
    pub new_face: [u32; 3],
    /// Canonical endpoints of the selected boundary edge.
    pub selected_edge: [u32; 2],
}

/// Add a triangle adjoining an existing boundary edge, using a third existing
/// vertex. The edge is an unordered pair; winding is derived from the unique
/// incident polygon half-edge. Reject invalid/stale picks, degenerate triangles,
/// duplicate faces, non-manifold edges and inconsistent winding.
///
/// This does not yet perform full 3D self-intersection or face-overlap tests.
/// An existing vertex on the adjacent polygon is disallowed to avoid the
/// simplest case of placing a triangle inside its parent face.
pub fn polygon_mesh_add_triangle_from_edge(
    mesh: &PolygonMesh,
    current_revision: u64,
    picked_revision: u64,
    edge_vertices: [u32; 2],
    point_vertex: u32,
) -> Result<PolygonEdgeTriangleResult> {
    if picked_revision != current_revision {
        return Err(KernelError::Conflict { expected: picked_revision, actual: current_revision });
    }
    if mesh.faces.len() >= 1_000_000 {
        return Err(KernelError::Budget);
    }
    polygon_mesh_validate(mesh)?;
    if edge_vertices[0] == edge_vertices[1]
        || point_vertex == edge_vertices[0]
        || point_vertex == edge_vertices[1]
        || point_vertex as usize >= mesh.vertices.len()
    {
        return Err(KernelError::Invalid("invalid edge and point selection"));
    }
    let before = polygon_mesh_topology(mesh)?;
    if !before.non_manifold_edges.is_empty() || !before.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("repair existing polygon topology first"));
    }

    let edge_key = [edge_vertices[0].min(edge_vertices[1]), edge_vertices[0].max(edge_vertices[1])];
    let boundary = before
        .boundary_edges
        .iter()
        .find_map(|&edge_id| {
            let edge = &before.edges[edge_id as usize];
            if edge.vertices == edge_key { edge.halfedges.first().and_then(|&halfedge_id| before.halfedges.get(halfedge_id as usize)) } else { None }
        })
        .ok_or(KernelError::Invalid("selected edge is not a boundary edge"))?;

    let adjacent_face = mesh.faces[boundary.face as usize];
    if adjacent_face.indices().contains(&point_vertex) {
        return Err(KernelError::Invalid("new point lies on the adjacent face"));
    }

    let oriented = [boundary.to, boundary.from, point_vertex];
    let new_face_index = u32::try_from(mesh.faces.len()).map_err(|_| KernelError::Budget)?;
    let revision = current_revision.checked_add(1).ok_or(KernelError::Budget)?;
    let mut result = mesh.clone();
    result.faces.push(PolygonFace::Triangle(oriented));
    // Catches nonfinite, repeated, zero-area and invalid face geometry.
    polygon_mesh_validate(&result)?;
    let after = polygon_mesh_topology(&result)?;
    if !after.non_manifold_edges.is_empty() || !after.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("triangle creates invalid edge topology"));
    }
    Ok(PolygonEdgeTriangleResult { mesh: result, revision, new_face_index, new_face: oriented, selected_edge: edge_key })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn patch() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(2.0, 0.5, 0.0),
                Vec3::new(3.0, 0.5, 0.0),
            ],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
        }
    }

    #[test]
    fn adds_triangle_from_boundary_edge_with_correct_winding() {
        let source = patch();
        let original = source.clone();
        let result = polygon_mesh_add_triangle_from_edge(&source, 12, 12, [1, 2], 4);
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.new_face, [2, 1, 4]);
            assert_eq!(result.new_face_index, 1);
            assert_eq!(result.revision, 13);
            assert_eq!(result.mesh.faces, vec![PolygonFace::Quad([0, 1, 2, 3]), PolygonFace::Triangle([2, 1, 4]),]);
            let topology = polygon_mesh_topology(&result.mesh);
            assert!(topology.is_ok_and(|t| t.non_manifold_edges.is_empty()
                && t.inconsistent_winding_edges.is_empty()
                && t.boundary_edges.len() == 5
                && t.face_neighbors[0][1] == Some(1)));
        }
        assert_eq!(source, original);
    }

    #[test]
    fn either_endpoint_order_selects_same_edge() {
        let a = polygon_mesh_add_triangle_from_edge(&patch(), 1, 1, [1, 2], 4);
        let b = polygon_mesh_add_triangle_from_edge(&patch(), 1, 1, [2, 1], 4);
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_stale_revision_and_out_of_bounds_point() {
        assert_eq!(polygon_mesh_add_triangle_from_edge(&patch(), 9, 8, [1, 2], 4), Err(KernelError::Conflict { expected: 8, actual: 9 }));
        assert!(polygon_mesh_add_triangle_from_edge(&patch(), 1, 1, [1, 2], 99).is_err());
    }

    #[test]
    fn rejects_nonexistent_edges_and_parent_corners() {
        assert!(polygon_mesh_add_triangle_from_edge(&patch(), 1, 1, [0, 2], 4).is_err());
        assert!(polygon_mesh_add_triangle_from_edge(&patch(), 1, 1, [1, 2], 3).is_err());
        assert!(polygon_mesh_add_triangle_from_edge(&patch(), 1, 1, [1, 1], 4).is_err());
    }

    #[test]
    fn rejects_attaching_a_third_face_to_an_interior_edge() {
        let first = polygon_mesh_add_triangle_from_edge(&patch(), 0, 0, [1, 2], 4);
        assert!(first.is_ok());
        if let Ok(first) = first {
            assert!(polygon_mesh_add_triangle_from_edge(&first.mesh, 1, 1, [1, 2], 5).is_err());
        }
    }

    #[test]
    fn rejects_collinear_third_point() {
        let mut source = patch();
        source.vertices[4] = Vec3::new(1.0, 0.5, 0.0);
        assert!(polygon_mesh_add_triangle_from_edge(&source, 0, 0, [1, 2], 4).is_err());
    }

    #[test]
    fn rejects_existing_invalid_topology() {
        let mut source = patch();
        source.faces.push(PolygonFace::Triangle([1, 2, 4]));
        assert!(polygon_mesh_add_triangle_from_edge(&source, 0, 0, [0, 1], 5).is_err());
    }

    #[test]
    fn rejects_revision_overflow() {
        assert_eq!(polygon_mesh_add_triangle_from_edge(&patch(), u64::MAX, u64::MAX, [1, 2], 4), Err(KernelError::Budget));
    }
}
