//! Derived vertex-disk and face-fan diagnostics for editable polygon meshes.
//! Inspired by BMesh's disk/loop/radial connectivity concepts, not a port of
//! Blender source code. Authoritative geometry remains in PolygonMesh.
//! Only shared, oppositely wound polygon edges connect a vertex's face fans.
use crate::{KernelError, PolygonMesh, Result, polygon_mesh_topology};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonVertexFan {
    pub vertex: u32,
    /// Face indices containing this vertex, in ascending order.
    pub incident_faces: Vec<u32>,
    /// Unique topology edge IDs meeting at this vertex, in ascending order.
    pub incident_edges: Vec<u32>,
    /// Face-connected components around this vertex; each fan is sorted.
    pub face_fans: Vec<Vec<u32>>,
    /// Incident edges used by only one polygon side.
    pub boundary_edges: Vec<u32>,
    /// False for disconnected fans, branched boundaries, invalid edge
    /// multiplicity or inconsistent winding around this vertex.
    pub is_manifold: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonVertexFanReport {
    /// One entry per referenced vertex, ordered by vertex index.
    pub vertices: Vec<PolygonVertexFan>,
    /// Vertices whose incident polygon faces do not form one valid disk/fan.
    pub non_manifold_vertices: Vec<u32>,
    /// Stored vertices not referenced by any face; not treated as defects.
    pub isolated_vertices: Vec<u32>,
}

/// Analyze polygon vertices without triangulating quads or modifying source.
///
/// A vertex is manifold when its incident faces form exactly one connected
/// fan across *valid paired* edges, and it has either zero boundary edges
/// (interior vertex) or exactly two (surface-boundary vertex). A bow-tie
/// vertex may have entirely valid edges but two disconnected fans; it is
/// non-manifold. Coincident, unwelded vertices remain separate by design.
///
/// This is a topological test, not a geometric self-intersection, orientation
/// of an entire shell, or watertight-solid guarantee.
pub fn polygon_mesh_vertex_fans(mesh: &PolygonMesh) -> Result<PolygonVertexFanReport> {
    let topology = polygon_mesh_topology(mesh)?;
    let mut corners: Vec<Vec<u32>> = Vec::new();
    corners.try_reserve_exact(mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
    corners.resize_with(mesh.vertices.len(), Vec::new);
    for (index, halfedge) in topology.halfedges.iter().enumerate() {
        let id = u32::try_from(index).map_err(|_| KernelError::Budget)?;
        corners[halfedge.from as usize].push(id);
    }

    let boundary: BTreeSet<u32> = topology.boundary_edges.iter().copied().collect();
    let invalid_edges: BTreeSet<u32> = topology.non_manifold_edges.iter()
        .chain(topology.inconsistent_winding_edges.iter())
        .copied()
        .collect();
    let mut vertices = Vec::new();
    let mut isolated_vertices = Vec::new();
    let mut non_manifold_vertices = Vec::new();

    for (index, incident_corners) in corners.iter().enumerate() {
        let vertex = u32::try_from(index).map_err(|_| KernelError::Budget)?;
        if incident_corners.is_empty() {
            isolated_vertices.push(vertex);
            continue;
        }

        let mut face_to_corner = BTreeMap::<u32, u32>::new();
        let mut edge_ids = BTreeSet::new();
        for &corner_id in incident_corners {
            let h = &topology.halfedges[corner_id as usize];
            let previous = &topology.halfedges[h.previous as usize];
            edge_ids.insert(h.edge);
            edge_ids.insert(previous.edge);
            face_to_corner.insert(h.face, corner_id);
        }

        let mut remaining: BTreeSet<u32> = face_to_corner.keys().copied().collect();
        let mut face_fans = Vec::new();
        while let Some(&seed) = remaining.first() {
            remaining.remove(&seed);
            let mut stack = vec![seed];
            let mut fan = Vec::new();
            while let Some(face) = stack.pop() {
                fan.push(face);
                let corner_id = *face_to_corner.get(&face).ok_or(KernelError::Invalid("vertex corner index"))?;
                let h = &topology.halfedges[corner_id as usize];
                let previous = &topology.halfedges[h.previous as usize];
                // Crossing either edge that meets this corner visits the next
                // polygon in the same vertex fan. Invalid edges have no pair.
                for paired in [h.opposite, previous.opposite].into_iter().flatten() {
                    let other_face = topology.halfedges[paired as usize].face;
                    if remaining.remove(&other_face) {
                        stack.push(other_face);
                    }
                }
            }
            fan.sort_unstable();
            face_fans.push(fan);
        }
        face_fans.sort();
        let incident_edges: Vec<u32> = edge_ids.iter().copied().collect();
        let boundary_edges: Vec<u32> = edge_ids.iter().filter(|id| boundary.contains(id)).copied().collect();
        let is_manifold = face_fans.len() == 1
            && (boundary_edges.is_empty() || boundary_edges.len() == 2)
            && !edge_ids.iter().any(|id| invalid_edges.contains(id));
        if !is_manifold {
            non_manifold_vertices.push(vertex);
        }
        vertices.push(PolygonVertexFan {
            vertex,
            incident_faces: face_to_corner.keys().copied().collect(),
            incident_edges,
            face_fans,
            boundary_edges,
            is_manifold,
        });
    }
    Ok(PolygonVertexFanReport { vertices, non_manifold_vertices, isolated_vertices })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PolygonFace;
    use cadcraft_geom::Vec3;

    fn points() -> Vec<Vec3> {
        vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        ]
    }

    #[test]
    fn lone_triangle_has_three_valid_boundary_fans_and_isolated_vertices() {
        let mesh = PolygonMesh { vertices: points(), faces: vec![PolygonFace::Triangle([0, 1, 2])] };
        let r = polygon_mesh_vertex_fans(&mesh);
        assert!(r.is_ok_and(|r| r.non_manifold_vertices.is_empty()
            && r.isolated_vertices == vec![3, 4, 5]
            && r.vertices.len() == 3
            && r.vertices.iter().all(|v| v.is_manifold && v.boundary_edges.len() == 2 && v.face_fans == vec![vec![0]])));
    }

    #[test]
    fn bow_tie_detected_even_when_no_edge_is_non_manifold() {
        let mesh = PolygonMesh {
            vertices: points(),
            faces: vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 3, 4])],
        };
        let report = polygon_mesh_vertex_fans(&mesh);
        assert!(report.is_ok_and(|r| r.non_manifold_vertices == vec![0]
            && r.vertices[0].face_fans == vec![vec![0], vec![1]]
            && r.vertices[0].boundary_edges.len() == 4));
    }

    #[test]
    fn connected_quads_preserve_native_faces_and_shared_vertex_fans() {
        let mesh = PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.), Vec3::new(1., 0., 0.),
                Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.),
                Vec3::new(2., 0., 0.), Vec3::new(2., 1., 0.),
            ],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3]), PolygonFace::Quad([1, 4, 5, 2])],
        };
        let r = polygon_mesh_vertex_fans(&mesh);
        assert!(r.is_ok_and(|r| r.non_manifold_vertices.is_empty()
            && r.vertices[1].incident_faces == vec![0, 1]
            && r.vertices[1].face_fans == vec![vec![0, 1]]
            && r.vertices[1].boundary_edges.len() == 2));
    }

    #[test]
    fn closed_tetrahedron_has_interior_manifold_vertices() {
        let mesh = PolygonMesh {
            vertices: points(),
            faces: vec![
                PolygonFace::Triangle([0, 2, 1]), PolygonFace::Triangle([0, 1, 5]),
                PolygonFace::Triangle([1, 2, 5]), PolygonFace::Triangle([2, 0, 5]),
            ],
        };
        let r = polygon_mesh_vertex_fans(&mesh);
        assert!(r.is_ok_and(|r| r.non_manifold_vertices.is_empty()
            && r.vertices.iter().all(|v| v.is_manifold && v.boundary_edges.is_empty() && v.face_fans.len() == 1)));
    }

    #[test]
    fn shared_edge_wrong_winding_marks_its_vertices_invalid() {
        let mesh = PolygonMesh {
            vertices: points(),
            faces: vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([1, 2, 5])],
        };
        let r = polygon_mesh_vertex_fans(&mesh);
        assert!(r.is_ok_and(|r| r.non_manifold_vertices.contains(&1)
            && r.non_manifold_vertices.contains(&2)
            && !r.non_manifold_vertices.contains(&0)));
    }

    #[test]
    fn three_faces_on_one_edge_marks_both_endpoints_invalid() {
        let mesh = PolygonMesh {
            vertices: points(),
            faces: vec![
                PolygonFace::Triangle([0, 1, 2]),
                PolygonFace::Triangle([1, 0, 3]),
                PolygonFace::Triangle([0, 1, 5]),
            ],
        };
        let r = polygon_mesh_vertex_fans(&mesh);
        assert!(r.is_ok_and(|r| r.non_manifold_vertices.contains(&0)
            && r.non_manifold_vertices.contains(&1)));
    }

    #[test]
    fn empty_mesh_is_supported_and_bad_indices_are_rejected() {
        let empty = PolygonMesh { vertices: vec![], faces: vec![] };
        assert!(polygon_mesh_vertex_fans(&empty).is_ok_and(|r| r.vertices.is_empty()
            && r.non_manifold_vertices.is_empty() && r.isolated_vertices.is_empty()));
        let invalid = PolygonMesh { vertices: points(), faces: vec![PolygonFace::Triangle([0, 1, 99])] };
        assert!(polygon_mesh_vertex_fans(&invalid).is_err());
    }

    #[test]
    fn deterministic_and_non_destructive() {
        let mesh = PolygonMesh {
            vertices: points(),
            faces: vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 3, 4])],
        };
        let original = mesh.clone();
        assert_eq!(polygon_mesh_vertex_fans(&mesh), polygon_mesh_vertex_fans(&mesh));
        assert_eq!(mesh, original);
    }
}
