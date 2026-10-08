//! Indexed polygon half-edge adjacency for native triangle/quad meshes.
//! This is a read-only topology view. The polygon faces remain authoritative.
use crate::{KernelError, PolygonMesh, Result, polygon_mesh_validate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonHalfEdge {
    pub from: u32,
    pub to: u32,
    pub face: u32,
    /// Next oriented edge around the same face.
    pub next: u32,
    /// Previous oriented edge around the same face.
    pub previous: u32,
    /// Opposite half-edge only when exactly two faces have opposite winding.
    pub opposite: Option<u32>,
    pub edge: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonEdge {
    /// Canonical, sorted endpoints.
    pub vertices: [u32; 2],
    /// All incident half-edge indices (one per polygon side).
    pub halfedges: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonTopology {
    pub halfedges: Vec<PolygonHalfEdge>,
    pub edges: Vec<PolygonEdge>,
    /// In face order, one start halfedge index per face.
    pub face_first_halfedge: Vec<u32>,
    /// One face-neighbor entry for every side in native polygon winding order.
    /// None for boundary, non-manifold or inconsistent-winding edges.
    pub face_neighbors: Vec<Vec<Option<u32>>>,
    /// Edge IDs referenced by exactly one polygon side.
    pub boundary_edges: Vec<u32>,
    /// Edge IDs referenced by three or more polygon sides.
    pub non_manifold_edges: Vec<u32>,
    /// Two-face edges whose halfedges run in the same direction.
    pub inconsistent_winding_edges: Vec<u32>,
}

/// Build a deterministic half-edge adjacency view from triangles and quads.
/// Shared edges with >2 incident faces, or matching instead of opposite
/// orientation, are diagnosed and left unpaired. No automatic topology
/// mutation, hole filling or assumption of manifoldness is performed.
pub fn polygon_mesh_topology(mesh: &PolygonMesh) -> Result<PolygonTopology> {
    polygon_mesh_validate(mesh)?;
    let side_count = mesh.faces.iter().try_fold(0usize, |total, face| {
        total.checked_add(face.indices().len())
    }).ok_or(KernelError::Budget)?;
    if side_count > 4_000_000 { return Err(KernelError::Budget); }

    let mut halfedges = Vec::<PolygonHalfEdge>::new();
    halfedges.try_reserve_exact(side_count).map_err(|_| KernelError::Budget)?;
    let mut face_first_halfedge = Vec::new();
    face_first_halfedge.try_reserve_exact(mesh.faces.len()).map_err(|_| KernelError::Budget)?;
    let mut face_neighbors = Vec::new();
    face_neighbors.try_reserve_exact(mesh.faces.len()).map_err(|_| KernelError::Budget)?;
    let mut incidents: BTreeMap<(u32, u32), Vec<u32>> = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().enumerate() {
        let corners = face.indices();
        let base = u32::try_from(halfedges.len()).map_err(|_| KernelError::Budget)?;
        let face_id = u32::try_from(face_index).map_err(|_| KernelError::Budget)?;
        face_first_halfedge.push(base);
        face_neighbors.push(vec![None; corners.len()]);
        for side in 0..corners.len() {
            let from = corners[side];
            let to = corners[(side + 1) % corners.len()];
            let id = u32::try_from(halfedges.len()).map_err(|_| KernelError::Budget)?;
            let next = base + ((side + 1) % corners.len()) as u32;
            let previous = base + ((side + corners.len() - 1) % corners.len()) as u32;
            halfedges.push(PolygonHalfEdge {
                from, to, face: face_id, next, previous,
                opposite: None, edge: u32::MAX,
            });
            incidents.entry((from.min(to), from.max(to))).or_default().push(id);
        }
    }
    let mut edges = Vec::new();
    let mut boundary_edges = Vec::new();
    let mut non_manifold_edges = Vec::new();
    let mut inconsistent_winding_edges = Vec::new();
    edges.try_reserve_exact(incidents.len()).map_err(|_| KernelError::Budget)?;
    for ((a,b), members) in incidents {
        let edge_id = u32::try_from(edges.len()).map_err(|_| KernelError::Budget)?;
        for &id in &members {
            halfedges[id as usize].edge = edge_id;
        }
        if members.len() == 1 {
            boundary_edges.push(edge_id);
        } else if members.len() > 2 {
            non_manifold_edges.push(edge_id);
        } else {
            let a_id = members[0] as usize;
            let b_id = members[1] as usize;
            if halfedges[a_id].from == halfedges[b_id].to
                && halfedges[a_id].to == halfedges[b_id].from {
                let a_face = halfedges[a_id].face;
                let b_face = halfedges[b_id].face;
                let a_side = (members[0] - face_first_halfedge[a_face as usize]) as usize;
                let b_side = (members[1] - face_first_halfedge[b_face as usize]) as usize;
                halfedges[a_id].opposite = Some(members[1]);
                halfedges[b_id].opposite = Some(members[0]);
                face_neighbors[a_face as usize][a_side] = Some(b_face);
                face_neighbors[b_face as usize][b_side] = Some(a_face);
            } else {
                inconsistent_winding_edges.push(edge_id);
            }
        }
        edges.push(PolygonEdge {
            vertices: [a,b], halfedges: members,
        });
    }
    Ok(PolygonTopology {
        halfedges, edges, face_first_halfedge, face_neighbors,
        boundary_edges, non_manifold_edges, inconsistent_winding_edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PolygonFace;
    use cadcraft_geom::Vec3;

    fn vertices() -> Vec<Vec3> {
        vec![
            Vec3::ZERO, Vec3::new(1.0,0.0,0.0),
            Vec3::new(1.0,1.0,0.0), Vec3::new(0.0,1.0,0.0),
            Vec3::new(2.0,0.0,0.0), Vec3::new(2.0,1.0,0.0),
            Vec3::new(0.5,0.5,1.0),
        ]
    }
    #[test]
    fn adjacent_quads_pair_opposite_halfedges() {
        let mesh = PolygonMesh { vertices:vertices(),
            faces: vec![
                PolygonFace::Quad([0,1,2,3]),
                PolygonFace::Quad([1,4,5,2]),
            ],
        };
        let t = polygon_mesh_topology(&mesh);
        assert!(t.is_ok());
        if let Ok(t) = t {
            assert_eq!(t.halfedges.len(),8);
            assert_eq!(t.edges.len(),7);
            assert_eq!(t.boundary_edges.len(),6);
            assert_eq!(t.face_neighbors[0], vec![None,Some(1),None,None]);
            assert_eq!(t.face_neighbors[1], vec![None,None,None,Some(0)]);
            assert_eq!(t.halfedges[1].opposite,Some(7));
            assert_eq!(t.halfedges[7].opposite,Some(1));
            assert_eq!(t.halfedges[1].next,2);
            assert_eq!(t.halfedges[1].previous,0);
        }
    }
    #[test]
    fn triangle_and_quad_sharing_edge() {
        let mesh=PolygonMesh { vertices:vertices(),
            faces:vec![
                PolygonFace::Quad([0,1,2,3]),
                PolygonFace::Triangle([2,1,6]),
            ],
        };
        assert!(polygon_mesh_topology(&mesh).is_ok_and(|t|
            t.halfedges.len()==7 && t.edges.len()==6
            && t.face_neighbors[0][1]==Some(1)));
    }
    #[test]
    fn same_winding_detected_without_false_neighbors() {
        let mesh=PolygonMesh { vertices:vertices(),
            faces:vec![
                PolygonFace::Triangle([0,1,2]),
                PolygonFace::Triangle([1,2,6]),
            ],
        };
        assert!(polygon_mesh_topology(&mesh).is_ok_and(|t|
            t.inconsistent_winding_edges.len()==1
            && t.face_neighbors[0].iter().all(Option::is_none)));
    }
    #[test]
    fn non_manifold_edge_is_reported_not_paired() {
        let mesh=PolygonMesh { vertices:vertices(),
            faces:vec![
                PolygonFace::Triangle([0,1,2]),
                PolygonFace::Triangle([1,0,3]),
                PolygonFace::Triangle([0,1,6]),
            ],
        };
        assert!(polygon_mesh_topology(&mesh).is_ok_and(|t|
            t.non_manifold_edges.len()==1
            && t.face_neighbors.iter().flatten().all(Option::is_none)));
    }
    #[test]
    fn empty_mesh_is_valid_and_invalid_indices_are_rejected() {
        let empty=PolygonMesh { vertices:vec![], faces:vec![] };
        assert!(polygon_mesh_topology(&empty).is_ok_and(|t|
            t.halfedges.is_empty() && t.edges.is_empty()));
        let broken=PolygonMesh { vertices:vertices(),
            faces:vec![PolygonFace::Triangle([0,1,99])] };
        assert!(polygon_mesh_topology(&broken).is_err());
    }
    #[test]
    fn shared_edge_ids_are_deterministic() {
        let m=PolygonMesh {vertices:vertices(),faces:vec![
            PolygonFace::Quad([0,1,2,3]),
            PolygonFace::Quad([1,4,5,2]),
        ]};
        let a=polygon_mesh_topology(&m);
        let b=polygon_mesh_topology(&m);
        assert_eq!(a,b);
    }
}
