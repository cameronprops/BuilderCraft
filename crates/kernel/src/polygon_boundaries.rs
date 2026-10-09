//! Deterministic boundary-loop extraction for native triangle/quad meshes.
//! Branches, open chains and non-manifold boundary components are NOT holes.
use crate::{KernelError, PolygonMesh, Result, polygon_mesh_topology};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonBoundaryLoop {
    /// Ordered vertices along original boundary half-edge directions,
    /// without repeating the first vertex at the end.
    pub vertices: Vec<u32>,
    /// Half-edge IDs in the same traversal order.
    pub halfedges: Vec<u32>,
    /// Polygon topology edge IDs in traversal order.
    pub edges: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolygonBoundaryReport {
    pub closed_loops: Vec<PolygonBoundaryLoop>,
    /// Boundary topology edge IDs in ambiguous/open/branched components.
    pub unresolved_edges: Vec<u32>,
    /// Boundary vertices having other than one incoming and one outgoing
    /// boundary half-edge within their connected boundary component.
    pub ambiguous_vertices: Vec<u32>,
    /// Pre-existing nonmanifold or wrong-winding polygon topology edges.
    pub non_manifold_edges: Vec<u32>,
    pub inconsistent_winding_edges: Vec<u32>,
}

/// Group boundary edges into undirected connected components, then emit a
/// closed, oriented loop only if EVERY vertex in the component has precisely
/// one incoming and one outgoing boundary half-edge. Components with
/// branches, shared vertices, open chains, etc. stay unresolved rather than
/// producing a fabricated loop. This does not classify outer vs inner loops.
pub fn polygon_mesh_boundary_loops(mesh: &PolygonMesh) -> Result<PolygonBoundaryReport> {
    let topo = polygon_mesh_topology(mesh)?;
    let mut neighbors: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &edge_id in &topo.boundary_edges {
        let [a, b] = topo.edges[edge_id as usize].vertices;
        neighbors.entry(a).or_default().push(edge_id);
        neighbors.entry(b).or_default().push(edge_id);
    }
    let mut pending: BTreeSet<u32> = topo.boundary_edges.iter().copied().collect();
    let mut closed_loops = Vec::new();
    let mut unresolved_edges = Vec::new();
    let mut ambiguous_vertices = BTreeSet::new();
    while let Some(&seed) = pending.first() {
        // Find the whole connected component so a branch cannot be mistaken
        // for one or more independent holes.
        let mut component = BTreeSet::new();
        let mut stack = vec![seed];
        pending.remove(&seed);
        while let Some(edge_id) = stack.pop() {
            if !component.insert(edge_id) {
                continue;
            }
            let [a, b] = topo.edges[edge_id as usize].vertices;
            for vertex in [a, b] {
                if let Some(incident) = neighbors.get(&vertex) {
                    for &id in incident {
                        if pending.remove(&id) {
                            stack.push(id);
                        }
                    }
                }
            }
        }
        let mut in_degree: BTreeMap<u32, usize> = BTreeMap::new();
        let mut out_degree: BTreeMap<u32, usize> = BTreeMap::new();
        let mut successor: BTreeMap<u32, (u32, u32, u32)> = BTreeMap::new();
        for &edge_id in &component {
            let edge = &topo.edges[edge_id as usize];
            let halfedge_id = edge.halfedges[0];
            let h = &topo.halfedges[halfedge_id as usize];
            *in_degree.entry(h.to).or_default() += 1;
            *out_degree.entry(h.from).or_default() += 1;
            successor.insert(h.from, (h.to, halfedge_id, edge_id));
        }
        let mut valid = true;
        for vertex in neighbors.keys().filter(|v| neighbors.get(v).is_some_and(|incident| incident.iter().any(|id| component.contains(id)))) {
            if in_degree.get(vertex).copied().unwrap_or(0) != 1 || out_degree.get(vertex).copied().unwrap_or(0) != 1 {
                valid = false;
                ambiguous_vertices.insert(*vertex);
            }
        }
        if !valid || component.len() < 3 {
            unresolved_edges.extend(component);
            continue;
        }
        let first_edge = *component.first().ok_or(KernelError::Invalid("empty boundary component"))?;
        let first_halfedge_id = topo.edges[first_edge as usize].halfedges[0];
        let start = topo.halfedges[first_halfedge_id as usize].from;
        let mut vertex = start;
        let mut vertices = Vec::new();
        let mut halfedges = Vec::new();
        let mut edges = Vec::new();
        for _ in 0..component.len() {
            vertices.push(vertex);
            let &(next, halfedge_id, edge_id) = successor.get(&vertex).ok_or(KernelError::Invalid("missing boundary successor"))?;
            halfedges.push(halfedge_id);
            edges.push(edge_id);
            vertex = next;
        }
        if vertex != start || edges.len() != component.len() {
            return Err(KernelError::Invalid("boundary traversal inconsistency"));
        }
        closed_loops.push(PolygonBoundaryLoop { vertices, halfedges, edges });
    }
    unresolved_edges.sort_unstable();
    Ok(PolygonBoundaryReport {
        closed_loops,
        unresolved_edges,
        ambiguous_vertices: ambiguous_vertices.into_iter().collect(),
        non_manifold_edges: topo.non_manifold_edges,
        inconsistent_winding_edges: topo.inconsistent_winding_edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PolygonFace;
    use cadcraft_geom::Vec3;

    fn vertices() -> Vec<Vec3> {
        vec![
            Vec3::new(0., 0., 0.),
            Vec3::new(3., 0., 0.),
            Vec3::new(3., 3., 0.),
            Vec3::new(0., 3., 0.),
            Vec3::new(1., 1., 0.),
            Vec3::new(2., 1., 0.),
            Vec3::new(2., 2., 0.),
            Vec3::new(1., 2., 0.),
            Vec3::new(5., 0., 0.),
            Vec3::new(6., 0., 0.),
            Vec3::new(5., 1., 0.),
        ]
    }
    #[test]
    fn single_quad_is_one_closed_outer_boundary() {
        let mesh = PolygonMesh { vertices: vertices(), faces: vec![PolygonFace::Quad([0, 1, 2, 3])] };
        let r = polygon_mesh_boundary_loops(&mesh);
        assert!(r.is_ok_and(|r| r.closed_loops.len() == 1
            && r.closed_loops[0].vertices.len() == 4
            && r.unresolved_edges.is_empty()
            && r.ambiguous_vertices.is_empty()));
    }
    #[test]
    fn annulus_returns_outer_and_inner_loops() {
        let mesh = PolygonMesh {
            vertices: vertices(),
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        };
        let r = polygon_mesh_boundary_loops(&mesh);
        assert!(r.is_ok_and(|r| r.closed_loops.len() == 2 && r.closed_loops.iter().all(|l| l.vertices.len() == 4) && r.unresolved_edges.is_empty()));
    }
    #[test]
    fn disconnected_components_are_separate_loops() {
        let mesh = PolygonMesh { vertices: vertices(), faces: vec![PolygonFace::Triangle([0, 1, 3]), PolygonFace::Triangle([8, 9, 10])] };
        let r = polygon_mesh_boundary_loops(&mesh);
        assert!(r.is_ok_and(|r| r.closed_loops.len() == 2));
    }
    #[test]
    fn vertex_branch_is_not_misreported_as_two_holes() {
        let mesh = PolygonMesh { vertices: vertices(), faces: vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([0, 3, 4])] };
        let r = polygon_mesh_boundary_loops(&mesh);
        assert!(r.is_ok_and(|r| r.closed_loops.is_empty() && r.unresolved_edges.len() == 6 && r.ambiguous_vertices.contains(&0)));
    }
    #[test]
    fn reports_invalid_shared_edge_orientation() {
        let mesh = PolygonMesh { vertices: vertices(), faces: vec![PolygonFace::Triangle([0, 1, 2]), PolygonFace::Triangle([1, 2, 3])] };
        let r = polygon_mesh_boundary_loops(&mesh);
        assert!(r.is_ok_and(|r| r.inconsistent_winding_edges.len() == 1));
    }
    #[test]
    fn empty_mesh_and_bad_face_handling() {
        let m = PolygonMesh { vertices: vec![], faces: vec![] };
        assert!(polygon_mesh_boundary_loops(&m).is_ok_and(|r| r.closed_loops.is_empty() && r.unresolved_edges.is_empty()));
        let m = PolygonMesh { vertices: vertices(), faces: vec![PolygonFace::Triangle([0, 1, 99])] };
        assert!(polygon_mesh_boundary_loops(&m).is_err());
    }
}
