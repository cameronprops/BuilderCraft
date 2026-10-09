//! Non-destructive duplicate-vertex discovery and representative index mapping.
//! Within-tolerance clustering uses the first matching representative, never
//! transitive chaining. Welding faces/vertices is a separate mutation step.
use crate::{KernelError, Result, TriangleMesh};
use cadcraft_geom::Vec3;
use std::collections::BTreeMap;

const MAX_VERTICES: usize = 1_000_000;
const MAX_COORDINATE: f64 = 1e12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexWeldMap {
    /// Each original vertex index maps to the first retained original index.
    pub representative: Vec<u32>,
    /// [duplicate index, retained index], ordered by duplicate index.
    pub duplicates: Vec<[u32; 2]>,
}

fn valid(p: Vec3) -> bool {
    p.is_finite() && [p.x, p.y, p.z].iter().all(|v| v.abs() <= MAX_COORDINATE)
}

fn canonical_bits(v: f64) -> u64 {
    if v == 0.0 { 0 } else { v.to_bits() }
}

fn cell(v: f64, tolerance: f64) -> Result<i64> {
    let value = (v / tolerance).floor();
    // Leave room for +-1 neighbor probing without integer overflow.
    if !value.is_finite() || value < (i64::MIN + 2) as f64 || value >= (i64::MAX - 2) as f64 {
        return Err(KernelError::Invalid("vertex tolerance too small for coordinate range"));
    }
    Ok(value as i64)
}

/// Find exact duplicates at tolerance=0, or Euclidean-close duplicates for
/// a positive absolute tolerance in drawing units. Deterministic: the earliest
/// retained input vertex is the representative. Does not change the mesh.
///
/// Positive tolerance uses spatial bins and checks all 27 neighboring bins.
/// Very small tolerances that overflow the grid coordinate range are rejected.
pub fn vertex_weld_map(vertices: &[Vec3], tolerance: f64) -> Result<VertexWeldMap> {
    if vertices.len() > MAX_VERTICES {
        return Err(KernelError::Budget);
    }
    if !tolerance.is_finite() || !(0.0..=MAX_COORDINATE).contains(&tolerance) {
        return Err(KernelError::Invalid("vertex weld tolerance"));
    }
    let mut representative = Vec::new();
    representative.try_reserve_exact(vertices.len()).map_err(|_| KernelError::Budget)?;
    let mut duplicates = Vec::new();
    let mut exact: BTreeMap<(u64, u64, u64), u32> = BTreeMap::new();
    let mut spatial: BTreeMap<(i64, i64, i64), Vec<u32>> = BTreeMap::new();
    for (index, &p) in vertices.iter().enumerate() {
        if !valid(p) {
            return Err(KernelError::Invalid("vertex coordinate"));
        }
        let index = u32::try_from(index).map_err(|_| KernelError::Budget)?;
        let retained = if tolerance == 0.0 {
            let key = (canonical_bits(p.x), canonical_bits(p.y), canonical_bits(p.z));
            *exact.entry(key).or_insert(index)
        } else {
            let key = (cell(p.x, tolerance)?, cell(p.y, tolerance)?, cell(p.z, tolerance)?);
            let mut best: Option<u32> = None;
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        if let Some(indices) = spatial.get(&(key.0 + dx, key.1 + dy, key.2 + dz)) {
                            for &candidate in indices {
                                let q = vertices[candidate as usize];
                                let delta = p - q;
                                if delta.x.hypot(delta.y).hypot(delta.z) <= tolerance && best.is_none_or(|previous| candidate < previous) {
                                    best = Some(candidate);
                                }
                            }
                        }
                    }
                }
            }
            if best.is_none() {
                spatial.entry(key).or_default().push(index);
            }
            best.unwrap_or(index)
        };
        representative.push(retained);
        if retained != index {
            duplicates.push([index, retained]);
        }
    }
    Ok(VertexWeldMap { representative, duplicates })
}

/// Validate triangle indices before producing a map; original mesh stays unchanged.
pub fn mesh_vertex_weld_map(mesh: &TriangleMesh, tolerance: f64) -> Result<VertexWeldMap> {
    if mesh.triangles.iter().flatten().any(|&index| index as usize >= mesh.vertices.len()) {
        return Err(KernelError::Invalid("mesh triangle index"));
    }
    vertex_weld_map(&mesh.vertices, tolerance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_duplicates_and_signed_zero() {
        let v = [Vec3::ZERO, Vec3::new(-0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)];
        let map = vertex_weld_map(&v, 0.0);
        assert_eq!(map, Ok(VertexWeldMap { representative: vec![0, 0, 2, 2], duplicates: vec![[1, 0], [3, 2]] }));
    }

    #[test]
    fn detects_neighbors_across_cell_boundaries() {
        let v = [Vec3::new(0.99, 0.0, 0.0), Vec3::new(1.01, 0.0, 0.0), Vec3::new(-0.01, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0)];
        let map = vertex_weld_map(&v, 0.1);
        assert!(map.is_ok_and(|m| m.representative == vec![0, 0, 2, 3]));
    }

    #[test]
    fn does_not_chain_beyond_representative_tolerance() {
        let v = [Vec3::ZERO, Vec3::new(0.09, 0.0, 0.0), Vec3::new(0.18, 0.0, 0.0)];
        let map = vertex_weld_map(&v, 0.1);
        assert!(map.is_ok_and(|m| m.representative == vec![0, 0, 2]));
    }

    #[test]
    fn empty_and_all_unique() {
        assert!(vertex_weld_map(&[], 0.0).is_ok_and(|m| m.representative.is_empty()));
        assert!(vertex_weld_map(&[Vec3::ZERO, Vec3::Z], 0.0).is_ok_and(|m| m.duplicates.is_empty() && m.representative == vec![0, 1]));
    }

    #[test]
    fn rejects_bad_tolerance_coordinates_and_indices() {
        assert!(vertex_weld_map(&[Vec3::ZERO], -1.0).is_err());
        assert!(vertex_weld_map(&[Vec3::ZERO], f64::NAN).is_err());
        assert!(vertex_weld_map(&[Vec3::new(f64::INFINITY, 0.0, 0.0)], 0.0).is_err());
        assert!(vertex_weld_map(&[Vec3::new(1e12, 0.0, 0.0)], 1e-12).is_err());
        let mesh = TriangleMesh { vertices: vec![Vec3::ZERO], triangles: vec![[0, 1, 0]] };
        assert!(mesh_vertex_weld_map(&mesh, 0.0).is_err());
    }
}
