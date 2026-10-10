//! Quad-preserving strip subdivision across opposite polygon edges.
//! Independently implemented in Rust using WorldWright's native halfedges.
//! This is not a Blender source-code translation.
use crate::{KernelError, PolygonFace, PolygonMesh, Result, polygon_mesh_topology, polygon_mesh_validate, polygon_mesh_vertex_fans};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const MAX_EDIT_ELEMENTS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolygonQuadStripSplitResult {
    pub mesh: PolygonMesh,
    pub revision: u64,
    pub selected_edge: [u32; 2],
    /// Canonical edges split across the complete connected quad strip.
    pub split_edges: Vec<[u32; 2]>,
    /// One new vertex per split edge, ordered like split_edges.
    pub new_vertex_indices: Vec<u32>,
    /// Original quad face indices replaced in place, in ascending order.
    pub affected_faces: Vec<u32>,
    /// Appended quad face indices, corresponding to affected_faces.
    pub new_face_indices: Vec<u32>,
}

fn edge_key(a: u32, b: u32) -> [u32; 2] {
    [a.min(b), a.max(b)]
}

/// Divide a complete quad strip into two quad strips, propagating through
/// opposite sides of every incident quad. Boundary and cyclic strips work;
/// mixed triangle/quad strips fail rather than creating hanging T-junctions.
///
/// The input fraction is measured from edge_vertices[0] to edge_vertices[1].
/// Old face IDs remain valid; new IDs are appended deterministically.
/// Vertex IDs are preserved and added in sorted canonical edge order.
/// Does not support concurrent perpendicular cuts through one quad,
/// geometric self-intersections, or nonmanifold/wrong-winding input.
pub fn polygon_mesh_split_quad_strip(
    mesh: &PolygonMesh,
    current_revision: u64,
    selected_revision: u64,
    edge_vertices: [u32; 2],
    fraction: f64,
) -> Result<PolygonQuadStripSplitResult> {
    if current_revision != selected_revision {
        return Err(KernelError::Conflict { expected: selected_revision, actual: current_revision });
    }
    if !fraction.is_finite() || !(1e-6..1.0 - 1e-6).contains(&fraction) {
        return Err(KernelError::Invalid("quad split fraction must lie inside (0, 1)"));
    }
    polygon_mesh_validate(mesh)?;
    if edge_vertices[0] == edge_vertices[1] || edge_vertices.iter().any(|&v| v as usize >= mesh.vertices.len()) {
        return Err(KernelError::Invalid("invalid selected quad edge"));
    }
    let topo = polygon_mesh_topology(mesh)?;
    let fans = polygon_mesh_vertex_fans(mesh)?;
    if !topo.non_manifold_edges.is_empty() || !topo.inconsistent_winding_edges.is_empty() || !fans.non_manifold_vertices.is_empty() {
        return Err(KernelError::Invalid("source polygon topology is not manifold and consistently wound"));
    }
    let lookup: BTreeMap<[u32; 2], usize> = topo.edges.iter().enumerate().map(|(i, edge)| (edge.vertices, i)).collect();
    let selected_edge = edge_key(edge_vertices[0], edge_vertices[1]);
    if !lookup.contains_key(&selected_edge) {
        return Err(KernelError::Invalid("selected quad edge does not exist"));
    }

    // All fractions are measured in canonical min->max vertex-ID direction.
    // Opposite sides in a quad must have complementary directed fractions.
    let canonical_fraction = if edge_vertices[0] < edge_vertices[1] { fraction } else { 1.0 - fraction };
    let mut cuts = BTreeMap::from([(selected_edge, canonical_fraction)]);
    let mut todo = VecDeque::from([selected_edge]);
    let mut faces_to_split = BTreeSet::new();
    while let Some(key) = todo.pop_front() {
        let edge_id = *lookup.get(&key).ok_or(KernelError::Invalid("missing propagated edge"))?;
        let edge = &topo.edges[edge_id];
        let stored_fraction = *cuts.get(&key).ok_or(KernelError::Invalid("missing propagated split fraction"))?;
        if edge.halfedges.is_empty() || edge.halfedges.len() > 2 {
            return Err(KernelError::Invalid("invalid quad edge incidence"));
        }
        for &halfedge_id in &edge.halfedges {
            let h = &topo.halfedges[halfedge_id as usize];
            let quad = match mesh.faces[h.face as usize] {
                PolygonFace::Quad(q) => q,
                PolygonFace::Triangle(_) => return Err(KernelError::Invalid("quad strip meets triangle face; mixed topology not supported")),
            };
            let side = (0..4).find(|&i| quad[i] == h.from && quad[(i + 1) % 4] == h.to).ok_or(KernelError::Invalid("quad halfedge mismatch"))?;
            faces_to_split.insert(h.face);
            let local_fraction = if h.from == key[0] { stored_fraction } else { 1.0 - stored_fraction };
            let opposite_from = quad[(side + 2) % 4];
            let opposite_to = quad[(side + 3) % 4];
            let opposite_key = edge_key(opposite_from, opposite_to);
            let opposite_local_fraction = 1.0 - local_fraction;
            let opposite_fraction = if opposite_from == opposite_key[0] { opposite_local_fraction } else { 1.0 - opposite_local_fraction };
            if let Some(previous) = cuts.get(&opposite_key) {
                if (previous - opposite_fraction).abs() > 1e-12 {
                    return Err(KernelError::Invalid("cyclic quad strip has inconsistent split fractions"));
                }
            } else {
                cuts.insert(opposite_key, opposite_fraction);
                todo.push_back(opposite_key);
                if cuts.len() > MAX_EDIT_ELEMENTS {
                    return Err(KernelError::Budget);
                }
            }
        }
    }
    if mesh.vertices.len().checked_add(cuts.len()).is_none_or(|n| n > MAX_EDIT_ELEMENTS)
        || mesh.faces.len().checked_add(faces_to_split.len()).is_none_or(|n| n > MAX_EDIT_ELEMENTS)
    {
        return Err(KernelError::Budget);
    }
    let revision = current_revision.checked_add(1).ok_or(KernelError::Budget)?;
    let mut result = mesh.clone();
    let mut index_by_edge = BTreeMap::new();
    let mut new_vertex_indices = Vec::new();
    for (&key, &t) in &cuts {
        let a = mesh.vertices[key[0] as usize];
        let b = mesh.vertices[key[1] as usize];
        let position = a + (b - a) * t;
        if !position.is_finite() {
            return Err(KernelError::Invalid("invalid quad split vertex position"));
        }
        let id = u32::try_from(result.vertices.len()).map_err(|_| KernelError::Budget)?;
        result.vertices.push(position);
        index_by_edge.insert(key, id);
        new_vertex_indices.push(id);
    }
    let mut affected_faces = Vec::new();
    let mut new_face_indices = Vec::new();
    let mut new_faces = Vec::new();
    for &face_id in &faces_to_split {
        let q = match mesh.faces[face_id as usize] {
            PolygonFace::Quad(q) => q,
            PolygonFace::Triangle(_) => return Err(KernelError::Invalid("unexpected non-quad face")),
        };
        let sides: Vec<usize> = (0..4).filter(|&side| cuts.contains_key(&edge_key(q[side], q[(side + 1) % 4]))).collect();
        if sides.len() != 2 || (sides[0] + 2) % 4 != sides[1] {
            return Err(KernelError::Invalid("quad face does not have exactly two opposite split edges"));
        }
        let side = sides[0];
        let a = q[side];
        let b = q[(side + 1) % 4];
        let c = q[(side + 2) % 4];
        let d = q[(side + 3) % 4];
        let first = *index_by_edge.get(&edge_key(a, b)).ok_or(KernelError::Invalid("missing first split vertex"))?;
        let opposite = *index_by_edge.get(&edge_key(c, d)).ok_or(KernelError::Invalid("missing opposite split vertex"))?;
        result.faces[face_id as usize] = PolygonFace::Quad([a, first, opposite, d]);
        let appended_id =
            u32::try_from(mesh.faces.len().checked_add(new_faces.len()).ok_or(KernelError::Budget)?).map_err(|_| KernelError::Budget)?;
        new_faces.push(PolygonFace::Quad([first, b, c, opposite]));
        affected_faces.push(face_id);
        new_face_indices.push(appended_id);
    }
    result.faces.extend(new_faces);
    polygon_mesh_validate(&result)?;
    let after = polygon_mesh_topology(&result)?;
    if !after.non_manifold_edges.is_empty() || !after.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("quad strip introduced invalid edge topology"));
    }
    if !polygon_mesh_vertex_fans(&result)?.non_manifold_vertices.is_empty() {
        return Err(KernelError::Invalid("quad strip introduced invalid vertex fan"));
    }
    Ok(PolygonQuadStripSplitResult {
        mesh: result,
        revision,
        selected_edge,
        split_edges: cuts.into_keys().collect(),
        new_vertex_indices,
        affected_faces,
        new_face_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn single_quad() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![Vec3::new(0., 0., 0.), Vec3::new(2., 0., 0.), Vec3::new(2., 2., 0.), Vec3::new(0., 2., 0.)],
            faces: vec![PolygonFace::Quad([0, 1, 2, 3])],
        }
    }

    fn adjacent_quads() -> PolygonMesh {
        let mut m = single_quad();
        m.vertices.push(Vec3::new(4., 0., 0.));
        m.vertices.push(Vec3::new(4., 2., 0.));
        m.faces.push(PolygonFace::Quad([1, 4, 5, 2]));
        m
    }

    #[test]
    fn one_quad_yields_two_quads_and_stable_original_face_id() {
        let original = single_quad();
        let after = polygon_mesh_split_quad_strip(&original, 4, 4, [0, 1], 0.5).unwrap();
        assert_eq!(after.revision, 5);
        assert_eq!(after.selected_edge, [0, 1]);
        assert_eq!(after.split_edges, vec![[0, 1], [2, 3]]);
        assert_eq!(after.new_vertex_indices, vec![4, 5]);
        assert_eq!(after.affected_faces, vec![0]);
        assert_eq!(after.new_face_indices, vec![1]);
        assert_eq!(after.mesh.faces, vec![PolygonFace::Quad([0, 4, 5, 3]), PolygonFace::Quad([4, 1, 2, 5]),]);
        assert_eq!(after.mesh.vertices[4], Vec3::new(1., 0., 0.));
        assert_eq!(after.mesh.vertices[5], Vec3::new(1., 2., 0.));
        assert_eq!(original.faces.len(), 1);
        assert!(polygon_mesh_vertex_fans(&after.mesh).is_ok_and(|r| r.non_manifold_vertices.is_empty()));
    }

    #[test]
    fn propagates_over_adjacent_quads_without_t_junctions() {
        let original = adjacent_quads();
        let after = polygon_mesh_split_quad_strip(&original, 7, 7, [3, 0], 0.5).unwrap();
        assert_eq!(after.affected_faces, vec![0, 1]);
        assert_eq!(after.new_face_indices, vec![2, 3]);
        assert_eq!(after.split_edges, vec![[0, 3], [1, 2], [4, 5]]);
        assert_eq!(after.mesh.vertices.len(), 9);
        assert_eq!(after.mesh.faces.len(), 4);
        assert!(after.mesh.faces.iter().all(|face| matches!(face, PolygonFace::Quad(_))));
        assert!(
            polygon_mesh_topology(&after.mesh)
                .is_ok_and(|t| t.boundary_edges.len() == 8 && t.non_manifold_edges.is_empty() && t.inconsistent_winding_edges.is_empty())
        );
    }

    #[test]
    fn closed_quad_strip_terminates_and_preserves_boundary_count() {
        let mesh = PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(1., 0., 0.),
                Vec3::new(1., 1., 0.),
                Vec3::new(0., 1., 0.),
                Vec3::new(0., 0., 2.),
                Vec3::new(1., 0., 2.),
                Vec3::new(1., 1., 2.),
                Vec3::new(0., 1., 2.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        };
        let result = polygon_mesh_split_quad_strip(&mesh, 0, 0, [0, 4], 0.5).unwrap();
        assert_eq!(result.affected_faces, vec![0, 1, 2, 3]);
        assert_eq!(result.split_edges.len(), 4);
        assert_eq!(result.mesh.faces.len(), 8);
        assert_eq!(result.mesh.vertices.len(), 12);
        assert!(result.mesh.faces.iter().all(|face| matches!(face, PolygonFace::Quad(_))));
        assert!(polygon_mesh_topology(&result.mesh).is_ok_and(|t| t.boundary_edges.len() == 8));
    }

    #[test]
    fn directed_fraction_flips_with_requested_endpoints() {
        let a = polygon_mesh_split_quad_strip(&single_quad(), 0, 0, [0, 1], 0.25).unwrap();
        let b = polygon_mesh_split_quad_strip(&single_quad(), 0, 0, [1, 0], 0.75).unwrap();
        assert_eq!(a.mesh, b.mesh);
        assert_eq!(a.mesh.vertices[4], Vec3::new(0.5, 0., 0.));
        assert_eq!(a.mesh.vertices[5], Vec3::new(0.5, 2., 0.));
    }

    #[test]
    fn rejects_triangle_neighbor_without_partial_mutation() {
        let mut original = single_quad();
        original.vertices.push(Vec3::new(3., 1., 0.));
        original.faces.push(PolygonFace::Triangle([2, 1, 4]));
        let snapshot = original.clone();
        assert!(polygon_mesh_split_quad_strip(&original, 0, 0, [3, 0], 0.5).is_err());
        assert_eq!(original, snapshot);
    }

    #[test]
    fn stale_picks_bad_edges_and_bad_fraction_are_rejected() {
        let mesh = single_quad();
        assert_eq!(polygon_mesh_split_quad_strip(&mesh, 4, 3, [0, 1], 0.5), Err(KernelError::Conflict { expected: 3, actual: 4 }));
        for t in [0., 1., f64::INFINITY, f64::NAN, -0.2, 0.00000001] {
            assert!(polygon_mesh_split_quad_strip(&mesh, 0, 0, [0, 1], t).is_err());
        }
        for edge in [[0, 0], [0, 2], [0, 99]] {
            assert!(polygon_mesh_split_quad_strip(&mesh, 0, 0, edge, 0.5).is_err());
        }
    }
}
