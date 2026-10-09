//! Read-only topological edge diagnostics for indexed triangle meshes.
//! Vertex indices define connectivity; coincident but unwelded positions remain separate.
use crate::{KernelError, Result, TriangleMesh};
use std::collections::{BTreeMap, BTreeSet};

const MAX_VERTICES: usize = 1_000_000;
const MAX_FACES: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshEdgeReport {
    /// Edges referenced by exactly one triangle.
    pub boundary_edges: Vec<[u32; 2]>,
    /// Edges referenced by more than two triangles.
    pub non_manifold_edges: Vec<[u32; 2]>,
    /// Exactly two incident faces sharing an edge in the same direction.
    pub inconsistent_winding_edges: Vec<[u32; 2]>,
    /// Closed, unbranched index-space boundary loops; the first vertex is NOT repeated.
    pub boundary_loops: Vec<Vec<u32>>,
    /// Boundary edges in chains or branched components that are not closed loops.
    pub unresolved_boundary_edges: Vec<[u32; 2]>,
}

fn canonical(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// Classify triangle edges. Repeated vertex indices in a face are rejected:
/// run face diagnostics first, then repair such faces before topology analysis.
/// Non-manifold vertex fans and geometric self-intersections are not evaluated.
pub fn mesh_edge_report(mesh: &TriangleMesh) -> Result<MeshEdgeReport> {
    if mesh.vertices.len() > MAX_VERTICES || mesh.triangles.len() > MAX_FACES {
        return Err(KernelError::Budget);
    }
    // (incident count, signed winding sum). Opposite directed occurrences cancel.
    let mut incidences: BTreeMap<(u32, u32), (u32, i32)> = BTreeMap::new();
    for &[a, b, c] in &mesh.triangles {
        if [a, b, c].iter().any(|&v| v as usize >= mesh.vertices.len()) {
            return Err(KernelError::Invalid("mesh triangle index"));
        }
        if a == b || b == c || c == a {
            return Err(KernelError::Invalid("repeated triangle vertex index"));
        }
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let edge = canonical(from, to);
            let entry = incidences.entry(edge).or_insert((0, 0));
            entry.0 = entry.0.checked_add(1).ok_or(KernelError::Budget)?;
            entry.1 += if from == edge.0 { 1 } else { -1 };
        }
    }

    let mut boundary_edges = Vec::new();
    let mut non_manifold_edges = Vec::new();
    let mut inconsistent_winding_edges = Vec::new();
    for (&(a, b), &(count, winding)) in &incidences {
        if count == 1 {
            boundary_edges.push([a, b]);
        } else if count > 2 {
            non_manifold_edges.push([a, b]);
        } else if winding != 0 {
            inconsistent_winding_edges.push([a, b]);
        }
    }

    // Only degree-two boundary components can be safely identified as loops.
    // Chains and junctions remain unresolved rather than inventing a traversal.
    let mut adjacency: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for &[a, b] in &boundary_edges {
        adjacency.entry(a).or_default().push(b);
        adjacency.entry(b).or_default().push(a);
    }
    for neighbors in adjacency.values_mut() {
        neighbors.sort_unstable();
    }
    let mut visited = BTreeSet::new();
    let mut boundary_loops = Vec::new();
    let mut unresolved_boundary_edges = Vec::new();
    for &start in adjacency.keys() {
        if !visited.insert(start) { continue; }
        let mut stack = vec![start];
        let mut component = Vec::new();
        while let Some(vertex) = stack.pop() {
            component.push(vertex);
            if let Some(neighbors) = adjacency.get(&vertex) {
                for &next in neighbors {
                    if visited.insert(next) {
                        stack.push(next);
                    }
                }
            }
        }
        component.sort_unstable();
        let regular = component.iter().all(|v| adjacency.get(v).is_some_and(|n| n.len() == 2));
        if regular {
            let first = component[0];
            let mut sequence = vec![first];
            let mut previous = None;
            let mut current = first;
            loop {
                let neighbors = &adjacency[&current];
                let next = if Some(neighbors[0]) == previous { neighbors[1] } else { neighbors[0] };
                if next == first { break; }
                sequence.push(next);
                previous = Some(current);
                current = next;
            }
            boundary_loops.push(sequence);
        } else {
            let members: BTreeSet<u32> = component.into_iter().collect();
            unresolved_boundary_edges.extend(boundary_edges.iter().copied()
                .filter(|edge| members.contains(&edge[0]) && members.contains(&edge[1])));
        }
    }
    Ok(MeshEdgeReport {
        boundary_edges,
        non_manifold_edges,
        inconsistent_winding_edges,
        boundary_loops,
        unresolved_boundary_edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn mesh(triangles: Vec<[u32; 3]>) -> TriangleMesh {
        TriangleMesh { vertices: vec![Vec3::ZERO; 6], triangles }
    }

    #[test]
    fn open_square_has_one_boundary_loop() {
        let report = mesh_edge_report(&mesh(vec![[0,1,2], [0,2,3]]));
        assert!(report.is_ok());
        if let Ok(r) = report {
            assert_eq!(r.boundary_edges, vec![[0,1], [0,3], [1,2], [2,3]]);
            assert_eq!(r.boundary_loops.len(), 1);
            assert_eq!(r.boundary_loops[0].len(), 4);
            assert!(r.non_manifold_edges.is_empty());
            assert!(r.unresolved_boundary_edges.is_empty());
        }
    }

    #[test]
    fn closed_tetrahedron_has_no_boundary() {
        let r = mesh_edge_report(&mesh(vec![[0,2,1], [0,1,3], [1,2,3], [2,0,3]]));
        assert!(r.is_ok_and(|r| r.boundary_edges.is_empty()
            && r.non_manifold_edges.is_empty() && r.inconsistent_winding_edges.is_empty()));
    }

    #[test]
    fn three_faces_on_edge_are_non_manifold() {
        let r = mesh_edge_report(&mesh(vec![[0,1,2], [1,0,3], [0,1,4]]));
        assert!(r.is_ok_and(|r| r.non_manifold_edges == vec![[0,1]]));
    }

    #[test]
    fn shared_edge_same_direction_reports_winding() {
        let r = mesh_edge_report(&mesh(vec![[0,1,2], [0,1,3]]));
        assert!(r.is_ok_and(|r| r.inconsistent_winding_edges == vec![[0,1]]));
    }

    #[test]
    fn branched_boundary_is_not_falsely_reported_as_loop() {
        let r = mesh_edge_report(&mesh(vec![[0,1,2], [0,3,4]]));
        assert!(r.is_ok_and(|r| r.boundary_loops.is_empty()
            && r.unresolved_boundary_edges.len() == 6));
    }

    #[test]
    fn rejects_invalid_triangles() {
        assert!(mesh_edge_report(&mesh(vec![[0,0,1]])).is_err());
        assert!(mesh_edge_report(&mesh(vec![[0,1,99]])).is_err());
    }
}
