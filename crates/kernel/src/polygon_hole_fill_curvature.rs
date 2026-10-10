//! Curvature-guided mesh hole reconstruction.
//! Shared CAD/Scan/OrbWeaver kernel operation: no UI state or external math engine.
//! This deliberately preserves the boundary. It is NOT a global intersection
//! certificate or an exact NURBS curvature-continuity solver.
use crate::{
    KernelError, PolygonAdvancedFillResult, PolygonFace, PolygonMesh, PolygonPatchMode, Result, polygon_mesh_fill_hole_advanced,
    polygon_mesh_validate,
};
use cadcraft_geom::Vec3;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvatureFillOptions {
    pub refinement_levels: u8,
    pub smoothing_iterations: u16,
    pub tangent_weight: f64,
    pub max_interior_offset: f64,
}

fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}

fn normalized(v: Vec3) -> Result<Vec3> {
    let len = length(v);
    if !len.is_finite() || len <= 1e-14 {
        return Err(KernelError::Invalid("degenerate adjacent face normal"));
    }
    Ok(v * (1.0 / len))
}

/// Replace a simple inner opening by a refined triangle patch with fixed rim
/// and approximate tangent-plane continuation. Uses only bounded surface
/// triangulation, centroid refinement and a scalar harmonic smoothing step.
///
/// Refinement inserts vertices strictly inside triangles: no boundary T-joints.
/// The surface coordinate domain stays fixed and only interior height moves,
/// preserving face projected orientation. This does not certify distant
/// self-intersections or perfect C1 curvature continuity.
pub fn polygon_mesh_fill_hole_curvature(
    mesh: &PolygonMesh,
    revision: u64,
    picked_revision: u64,
    loop_index: u32,
    options: CurvatureFillOptions,
) -> Result<PolygonAdvancedFillResult> {
    let CurvatureFillOptions { refinement_levels, smoothing_iterations, tangent_weight, max_interior_offset } = options;
    if !(1..=3).contains(&refinement_levels)
        || !(1..=64).contains(&smoothing_iterations)
        || !tangent_weight.is_finite()
        || !(0.0..=1.0).contains(&tangent_weight)
        || !max_interior_offset.is_finite()
        || max_interior_offset < 0.0
    {
        return Err(KernelError::Invalid("invalid curvature patch parameters"));
    }
    // Delegate loop safety, orientation, projection and base triangulation to
    // the already shared fill operation. Surface mode preserves rim positions.
    let mut base = polygon_mesh_fill_hole_advanced(mesh, revision, picked_revision, loop_index, PolygonPatchMode::Surface)?;
    let original_vertex_count = base.mesh.vertices.len();
    let origin = base.plane.origin;
    let axis = normalized(base.plane.normal)?;
    // The base fill has already validated the source halfedge topology.
    // Scan source faces once to recover their normals at the selected
    // boundary edges, instead of building that large topology twice.
    let rim = &base.boundary_vertices;
    let mut pending = std::collections::BTreeSet::<(u32, u32)>::new();
    for i in 0..rim.len() {
        pending.insert((rim[i], rim[(i + 1) % rim.len()]));
    }
    let mut boundary_normals: BTreeMap<u32, Vec3> = BTreeMap::new();
    for face in &mesh.faces {
        let corners = face.indices();
        let mut matched = Vec::new();
        for i in 0..corners.len() {
            let from = corners[i];
            let to = corners[(i + 1) % corners.len()];
            if pending.remove(&(from, to)) {
                matched.push((from, to));
            }
        }
        if matched.is_empty() {
            continue;
        }
        let p = mesh.vertices[corners[0] as usize];
        let q = mesh.vertices[corners[1] as usize];
        let r = mesh.vertices[corners[2] as usize];
        let face_normal = normalized((q - p).cross(r - p))?;
        for (from, to) in matched {
            for vertex in [from, to] {
                let entry = boundary_normals.entry(vertex).or_insert(Vec3::ZERO);
                *entry = *entry + face_normal;
            }
        }
        if pending.is_empty() {
            break;
        }
    }
    if !pending.is_empty() {
        return Err(KernelError::Invalid("boundary face normals unavailable"));
    }

    // Each centroid split turns one triangle into three without ever splitting
    // an original boundary edge. Hard limit protects scan workloads.
    let mut active = base.new_face_indices.clone();
    let growth = 3usize.checked_pow(u32::from(refinement_levels)).ok_or(KernelError::Budget)?;
    let final_faces = active.len().checked_mul(growth).ok_or(KernelError::Budget)?;
    if final_faces > 8192 || mesh.faces.len().checked_add(final_faces).is_none_or(|total| total > 1_000_000) {
        return Err(KernelError::Budget);
    }
    let new_vertices = (final_faces - active.len()) / 2;
    if original_vertex_count.checked_add(new_vertices).is_none_or(|total| total > 1_000_000) {
        return Err(KernelError::Budget);
    }
    base.mesh.vertices.try_reserve_exact(new_vertices).map_err(|_| KernelError::Budget)?;
    base.mesh.faces.try_reserve_exact(final_faces - active.len()).map_err(|_| KernelError::Budget)?;
    for _ in 0..refinement_levels {
        let mut next = Vec::new();
        next.try_reserve_exact(active.len().checked_mul(3).ok_or(KernelError::Budget)?).map_err(|_| KernelError::Budget)?;
        for face_index in active {
            let [a, b, c] = match base.mesh.faces.get(face_index as usize) {
                Some(PolygonFace::Triangle(indices)) => *indices,
                _ => return Err(KernelError::Invalid("expected triangle patch face")),
            };
            let center = (base.mesh.vertices[a as usize] + base.mesh.vertices[b as usize] + base.mesh.vertices[c as usize]) * (1.0 / 3.0);
            if !center.is_finite() {
                return Err(KernelError::Invalid("nonfinite refined vertex"));
            }
            let center_id = u32::try_from(base.mesh.vertices.len()).map_err(|_| KernelError::Budget)?;
            base.mesh.vertices.push(center);
            base.mesh.faces[face_index as usize] = PolygonFace::Triangle([a, b, center_id]);
            next.push(face_index);
            for triangle in [[b, c, center_id], [c, a, center_id]] {
                let id = u32::try_from(base.mesh.faces.len()).map_err(|_| KernelError::Budget)?;
                next.push(id);
                base.mesh.faces.push(PolygonFace::Triangle(triangle));
            }
        }
        active = next;
    }
    let interior_count = base.mesh.vertices.len() - original_vertex_count;
    let mut adjacency = vec![Vec::<u32>::new(); interior_count];
    for face in base.mesh.faces.iter().skip(mesh.faces.len()) {
        let [a, b, c] = match face {
            PolygonFace::Triangle(ids) => *ids,
            PolygonFace::Quad(_) => return Err(KernelError::Invalid("quad in triangulated hole patch")),
        };
        for (from, to) in [(a, b), (b, c), (c, a)] {
            if (from as usize) >= original_vertex_count {
                adjacency[from as usize - original_vertex_count].push(to);
            }
            if (to as usize) >= original_vertex_count {
                adjacency[to as usize - original_vertex_count].push(from);
            }
        }
    }
    for neighbors in &mut adjacency {
        neighbors.sort_unstable();
        neighbors.dedup();
        if neighbors.len() < 3 {
            return Err(KernelError::Invalid("unconnected interior vertex"));
        }
    }

    let mut radius = 0.0_f64;
    for &id in &base.boundary_vertices {
        radius = radius.max(length(base.mesh.vertices[id as usize] - origin));
    }
    if !radius.is_finite() || radius <= 1e-12 {
        return Err(KernelError::Invalid("collapsed curvature patch region"));
    }
    let mut targets = Vec::new();
    targets.try_reserve_exact(interior_count).map_err(|_| KernelError::Budget)?;
    let mut weighted_neighbors = Vec::new();
    weighted_neighbors.try_reserve_exact(interior_count).map_err(|_| KernelError::Budget)?;
    for (offset, edges) in adjacency.iter().enumerate() {
        let p = base.mesh.vertices[original_vertex_count + offset];
        let mut projection = Vec::new();
        projection.try_reserve_exact(edges.len()).map_err(|_| KernelError::Budget)?;
        let mut total_edge_weight = 0.0;
        for &id in edges {
            let q = base.mesh.vertices[id as usize];
            let difference = p - q;
            let lateral = difference - axis * difference.dot(axis);
            let weight = 1.0 / length(lateral).max(1e-12 * radius);
            total_edge_weight += weight;
            projection.push((id as usize, weight));
        }
        if !total_edge_weight.is_finite() || total_edge_weight <= 0.0 {
            return Err(KernelError::Invalid("invalid curvature neighborhood"));
        }
        for (_, weight) in &mut projection {
            *weight /= total_edge_weight;
        }
        weighted_neighbors.push(projection);

        // Tangent-plane prediction from nearby original surface samples.
        // Inverse squared distance weights create a local slope near the rim.
        let mut predicted = 0.0;
        let mut sum_weight = 0.0;
        let mut closest = f64::INFINITY;
        for &rim_id in &base.boundary_vertices {
            let rim = base.mesh.vertices[rim_id as usize];
            let delta = p - rim;
            let lateral = delta - axis * delta.dot(axis);
            let dist = length(lateral);
            closest = closest.min(dist);
            let face_normal = match boundary_normals.get(&rim_id) {
                Some(normal) => normalized(*normal)?,
                None => return Err(KernelError::Invalid("missing adjacent face normal")),
            };
            let axis_alignment = face_normal.dot(axis);
            if axis_alignment.abs() < 0.25 {
                continue;
            }
            let rim_height = (rim - origin).dot(axis);
            let estimated_height = rim_height - face_normal.dot(lateral) / axis_alignment;
            if !estimated_height.is_finite() {
                return Err(KernelError::Invalid("nonfinite tangent estimate"));
            }
            let weight = 1.0 / (dist * dist + 1e-8 * radius * radius);
            predicted += weight * estimated_height;
            sum_weight += weight;
        }
        let blend = if sum_weight > 0.0 { tangent_weight * (-closest / (0.3 * radius)).exp() } else { 0.0 };
        let target = if sum_weight > 0.0 { predicted / sum_weight } else { (p - origin).dot(axis) };
        if !target.is_finite() || !blend.is_finite() {
            return Err(KernelError::Invalid("invalid tangent-plane blend"));
        }
        targets.push((target, blend));
    }

    let mut heights: Vec<f64> = base.mesh.vertices.iter().map(|p| (*p - origin).dot(axis)).collect();
    let mut next_heights = heights.clone();
    for _ in 0..smoothing_iterations {
        for (offset, neighbors) in weighted_neighbors.iter().enumerate() {
            let id = original_vertex_count + offset;
            let harmonic = neighbors.iter().map(|(neighbor, weight)| heights[*neighbor] * weight).sum::<f64>();
            let (target, blend) = targets[offset];
            next_heights[id] = harmonic * (1.0 - blend) + target * blend;
        }
        std::mem::swap(&mut heights, &mut next_heights);
    }
    for offset in 0..interior_count {
        let id = original_vertex_count + offset;
        let point = base.mesh.vertices[id];
        let displacement = heights[id] - (point - origin).dot(axis);
        if !displacement.is_finite() || displacement.abs() > max_interior_offset + 1e-12 * radius {
            return Err(KernelError::Invalid("curvature fill exceeds interior offset limit"));
        }
        base.mesh.vertices[id] = point + axis * displacement;
    }
    polygon_mesh_validate(&base.mesh)?;
    let result_topology = polygon_mesh_topology(&base.mesh)?;
    if !result_topology.non_manifold_edges.is_empty() || !result_topology.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("invalid curvature patch topology"));
    }
    base.new_face_indices =
        (mesh.faces.len()..base.mesh.faces.len()).map(|index| u32::try_from(index).map_err(|_| KernelError::Budget)).collect::<Result<Vec<u32>>>()?;
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_mesh_boundary_loops;

    fn ring() -> PolygonMesh {
        PolygonMesh {
            vertices: vec![
                Vec3::new(0., 0., 0.),
                Vec3::new(4., 0., 0.),
                Vec3::new(4., 4., 0.),
                Vec3::new(0., 4., 0.),
                Vec3::new(1., 1., 0.1),
                Vec3::new(3., 1., 0.),
                Vec3::new(3., 3., 0.),
                Vec3::new(1., 3., 0.),
            ],
            faces: vec![
                PolygonFace::Quad([0, 1, 5, 4]),
                PolygonFace::Quad([1, 2, 6, 5]),
                PolygonFace::Quad([2, 3, 7, 6]),
                PolygonFace::Quad([3, 0, 4, 7]),
            ],
        }
    }

    fn inner(source: &PolygonMesh) -> u32 {
        polygon_mesh_boundary_loops(source)
            .ok()
            .and_then(|r| r.closed_loops.iter().position(|b| b.vertices.iter().all(|&id| id >= 4)))
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(u32::MAX)
    }

    #[test]
    fn curvature_fill_keeps_outer_shell_and_rim_unchanged() {
        let source = ring();
        let r = polygon_mesh_fill_hole_curvature(
            &source,
            10,
            10,
            inner(&source),
            CurvatureFillOptions { refinement_levels: 2, smoothing_iterations: 16, tangent_weight: 0.4, max_interior_offset: 0.5 },
        );
        assert!(r.is_ok(), "{r:?}");
        if let Ok(patch) = r {
            assert_eq!(&patch.mesh.vertices[..source.vertices.len()], &source.vertices);
            assert_eq!(&patch.mesh.faces[..source.faces.len()], &source.faces);
            assert_eq!(patch.mesh.faces.len(), source.faces.len() + 18);
            assert_eq!(patch.mesh.vertices.len(), source.vertices.len() + 8);
            assert!(patch.moved_vertices.is_empty());
            assert!(polygon_mesh_boundary_loops(&patch.mesh).is_ok_and(|r| r.closed_loops.len() == 1));
        }
    }

    #[test]
    fn flat_rim_remains_flat_to_roundoff() {
        let mut source = ring();
        source.vertices[4].z = 0.0;
        let r = polygon_mesh_fill_hole_curvature(
            &source,
            0,
            0,
            inner(&source),
            CurvatureFillOptions { refinement_levels: 2, smoothing_iterations: 18, tangent_weight: 0.8, max_interior_offset: 0.01 },
        );
        assert!(r.is_ok(), "{r:?}");
        if let Ok(patch) = r {
            for vertex in patch.mesh.vertices.iter().skip(source.vertices.len()) {
                assert!(vertex.z.abs() < 1e-8);
            }
        }
    }

    #[test]
    fn rejects_excessive_offset_and_invalid_settings() {
        let source = ring();
        let i = inner(&source);
        assert!(
            polygon_mesh_fill_hole_curvature(
                &source,
                0,
                0,
                i,
                CurvatureFillOptions { refinement_levels: 4, smoothing_iterations: 8, tangent_weight: 0.4, max_interior_offset: 1. }
            )
            .is_err()
        );
        assert!(
            polygon_mesh_fill_hole_curvature(
                &source,
                0,
                0,
                i,
                CurvatureFillOptions { refinement_levels: 2, smoothing_iterations: 0, tangent_weight: 0.4, max_interior_offset: 1. }
            )
            .is_err()
        );
        assert!(
            polygon_mesh_fill_hole_curvature(
                &source,
                0,
                0,
                i,
                CurvatureFillOptions { refinement_levels: 2, smoothing_iterations: 8, tangent_weight: f64::NAN, max_interior_offset: 1. }
            )
            .is_err()
        );
        assert!(
            polygon_mesh_fill_hole_curvature(
                &source,
                0,
                0,
                i,
                CurvatureFillOptions { refinement_levels: 2, smoothing_iterations: 8, tangent_weight: 0.4, max_interior_offset: 0. }
            )
            .is_err()
        );
        assert_eq!(
            polygon_mesh_fill_hole_curvature(
                &source,
                2,
                1,
                i,
                CurvatureFillOptions { refinement_levels: 2, smoothing_iterations: 8, tangent_weight: 0.4, max_interior_offset: 1. }
            ),
            Err(KernelError::Conflict { expected: 1, actual: 2 })
        );
    }
}
