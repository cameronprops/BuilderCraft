//! Conservative, deterministic quadric-error decimation for editable triangle meshes.
//! Inspired by the public Garland-Heckbert QEM method (not copied from OrcaSlicer).
//! The input is immutable. Only paired interior edges can collapse; boundaries,
//! optional creases, local winding, and local triangle degeneracy are protected.
//! Global self-intersection and Hausdorff bounds are NOT certified.
use crate::{
    KernelError, Result, TriangleMesh, mesh_degenerate_faces, mesh_duplicate_faces, mesh_edge_report, polygon_mesh_from_triangles,
    polygon_mesh_vertex_fans, validate_triangle_mesh,
};
use cadcraft_geom::Vec3;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

const MAX_VERTICES: usize = 100_000;
const MAX_FACES: usize = 100_000;
const MAX_HEAP: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshDecimateOptions {
    /// Requested maximum number of output faces. Legal collapses may stop early.
    pub target_faces: usize,
    /// Maximum sum of squared plane distances for a proposed collapse, in drawing units squared.
    pub max_quadric_error: f64,
    /// Maximum permitted angular deviation of each surviving affected triangle (0..89 degrees).
    pub max_normal_change_degrees: f64,
    /// When enabled, no original boundary vertex may be moved or removed.
    pub preserve_boundary: bool,
    /// When Some, vertices on original edges with dihedral angle above this
    /// threshold are protected from collapse (threshold in degrees, 0..180).
    pub preserve_creases_above_degrees: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshDecimateResult {
    pub mesh: TriangleMesh,
    pub removed_faces: usize,
    /// Each original vertex maps to an output vertex, or None if no surviving
    /// output triangle uses its merged representative.
    pub old_to_new: Vec<Option<u32>>,
    /// Actual target can be missed when every remaining collapse is unsafe.
    pub target_reached: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Quadric([[f64; 4]; 4]);

impl Quadric {
    fn from_plane(normal: Vec3, point: Vec3) -> Self {
        let plane = [normal.x, normal.y, normal.z, -dot(normal, point)];
        let mut q = Self::default();
        for row in 0..4 {
            for col in 0..4 {
                q.0[row][col] = plane[row] * plane[col];
            }
        }
        q
    }

    fn add(self, other: Self) -> Self {
        let mut sum = Self::default();
        for row in 0..4 {
            for col in 0..4 {
                sum.0[row][col] = self.0[row][col] + other.0[row][col];
            }
        }
        sum
    }

    fn error(self, v: Vec3) -> f64 {
        let p = [v.x, v.y, v.z, 1.0];
        let mut sum = 0.0;
        for i in 0..4 {
            for j in 0..4 {
                sum += p[i] * self.0[i][j] * p[j];
            }
        }
        sum.max(0.0)
    }

    fn optimal(self) -> Option<Vec3> {
        // Partial pivoting; singular/ill-conditioned quadrics use endpoints
        // and the midpoint instead of generating non-finite new coordinates.
        let mut a = [[0.0; 4]; 3];
        let mut scale: f64 = 0.0;
        for (i, row) in a.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate().take(3) {
                *cell = self.0[i][j];
                scale = scale.max(cell.abs());
            }
            row[3] = -self.0[i][3];
        }
        if scale == 0.0 || !scale.is_finite() {
            return None;
        }
        for col in 0..3 {
            let pivot = (col..3).max_by(|&l, &r| a[l][col].abs().total_cmp(&a[r][col].abs()))?;
            if a[pivot][col].abs() <= scale * 1e-12 {
                return None;
            }
            a.swap(pivot, col);
            let denom = a[col][col];
            for j in col..4 {
                a[col][j] /= denom;
            }
            for i in 0..3 {
                if i == col {
                    continue;
                }
                let factor = a[i][col];
                for j in col..4 {
                    a[i][j] -= factor * a[col][j];
                }
            }
        }
        let v = Vec3::new(a[0][3], a[1][3], a[2][3]);
        v.is_finite().then_some(v)
    }
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

fn length(v: Vec3) -> f64 {
    v.x.hypot(v.y).hypot(v.z)
}

fn normal(p: Vec3, q: Vec3, r: Vec3) -> Option<Vec3> {
    let e0 = q - p;
    let e1 = r - p;
    let scale = length(e0).max(length(e1));
    if scale == 0.0 {
        return None;
    }
    let cross = (e0 * (1.0 / scale)).cross(e1 * (1.0 / scale));
    let size = length(cross);
    if size <= 1e-14 || !size.is_finite() {
        return None;
    }
    Some(cross * (1.0 / size))
}

fn face_normal(face: [u32; 3], positions: &[Vec3]) -> Option<Vec3> {
    normal(positions[face[0] as usize], positions[face[1] as usize], positions[face[2] as usize])
}

#[derive(Clone, Debug)]
struct Candidate {
    a: u32,
    b: u32,
    pos: Vec3,
    error: f64,
    ver_a: u64,
    ver_b: u64,
}
impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        (self.a, self.b, self.ver_a, self.ver_b, self.error.to_bits())
            == (other.a, other.b, other.ver_a, other.ver_b, other.error.to_bits())
    }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other.error.total_cmp(&self.error)
            .then_with(|| other.a.cmp(&self.a))
            .then_with(|| other.b.cmp(&self.b))
            .then_with(|| other.ver_a.cmp(&self.ver_a))
            .then_with(|| other.ver_b.cmp(&self.ver_b))
    }
}

struct Work {
    positions: Vec<Vec3>,
    faces: Vec<Option<[u32; 3]>>,
    incident: Vec<BTreeSet<usize>>,
    active: Vec<bool>,
    protected: Vec<bool>,
    quadrics: Vec<Quadric>,
    versions: Vec<u64>,
    parents: Vec<u32>,
}

impl Work {
    fn neighbors(&self, vertex: usize) -> BTreeSet<u32> {
        let mut neighbors = BTreeSet::new();
        for &id in &self.incident[vertex] {
            if let Some(face) = self.faces[id] {
                for corner in face {
                    if corner as usize != vertex {
                        neighbors.insert(corner);
                    }
                }
            }
        }
        neighbors
    }

    fn candidate(&self, a: u32, b: u32) -> Option<Candidate> {
        if a == b || !self.active[a as usize] || !self.active[b as usize]
            || self.protected[a as usize] || self.protected[b as usize] {
            return None;
        }
        let quadric = self.quadrics[a as usize].add(self.quadrics[b as usize]);
        let p = self.positions[a as usize];
        let q = self.positions[b as usize];
        let mut best = (quadric.error(p), p);
        for test in [q, (p + q) * 0.5] {
            let error = quadric.error(test);
            if error < best.0 {
                best = (error, test);
            }
        }
        if let Some(optimal) = quadric.optimal() {
            let error = quadric.error(optimal);
            if error < best.0 {
                best = (error, optimal);
            }
        }
        if !best.0.is_finite() || !best.1.is_finite() || [best.1.x, best.1.y, best.1.z].iter().any(|v| v.abs() > 1e12) {
            return None;
        }
        Some(Candidate { a, b, pos: best.1, error: best.0, ver_a: self.versions[a as usize], ver_b: self.versions[b as usize] })
    }

    fn is_legal(&self, c: &Candidate, cos_limit: f64) -> bool {
        let a = c.a as usize;
        let b = c.b as usize;
        if !self.active[a] || !self.active[b] || self.protected[a] || self.protected[b] {
            return false;
        }
        // Closed interior edge only. This also excludes disconnected vertices.
        let shared: Vec<usize> = self.incident[a].intersection(&self.incident[b]).copied().collect();
        if shared.len() != 2 {
            return false;
        }
        let mut opposite = BTreeSet::new();
        for &id in &shared {
            let Some(face) = self.faces[id] else { return false; };
            for vertex in face {
                if vertex != c.a && vertex != c.b {
                    opposite.insert(vertex);
                }
            }
        }
        if opposite.len() != 2 {
            return false;
        }
        // Link condition: closing the one-ring may not merge unrelated fans
        // or form duplicate triangles/non-manifold edges.
        let na = self.neighbors(a);
        let nb = self.neighbors(b);
        if na.intersection(&nb).copied().collect::<BTreeSet<_>>() != opposite {
            return false;
        }
        for &id in self.incident[a].union(&self.incident[b]) {
            let Some(face) = self.faces[id] else { return false; };
            if face.contains(&c.a) && face.contains(&c.b) {
                continue;
            }
            let Some(before) = face_normal(face, &self.positions) else { return false; };
            let mapped = face.map(|v| if v == c.b { c.a } else { v });
            if mapped[0] == mapped[1] || mapped[1] == mapped[2] || mapped[0] == mapped[2] {
                return false;
            }
            let point = |v: u32| if v == c.a { c.pos } else { self.positions[v as usize] };
            let Some(after) = normal(point(mapped[0]), point(mapped[1]), point(mapped[2])) else { return false; };
            if dot(before, after) < cos_limit {
                return false;
            }
        }
        true
    }

    fn collapse(&mut self, c: &Candidate) -> Result<(usize, BTreeSet<u32>)> {
        let a = c.a as usize;
        let b = c.b as usize;
        let affected: Vec<usize> = self.incident[a].union(&self.incident[b]).copied().collect();
        let mut touched = BTreeSet::new();
        let mut removed = 0;
        for &id in &affected {
            if let Some(face) = self.faces[id] {
                for vertex in face {
                    self.incident[vertex as usize].remove(&id);
                    touched.insert(vertex);
                }
            }
        }
        for &id in &affected {
            if let Some(face) = self.faces[id] {
                if face.contains(&c.a) && face.contains(&c.b) {
                    self.faces[id] = None;
                    removed += 1;
                } else {
                    let mapped = face.map(|v| if v == c.b { c.a } else { v });
                    for vertex in mapped {
                        self.incident[vertex as usize].insert(id);
                        touched.insert(vertex);
                    }
                    self.faces[id] = Some(mapped);
                }
            }
        }
        self.positions[a] = c.pos;
        self.quadrics[a] = self.quadrics[a].add(self.quadrics[b]);
        self.active[b] = false;
        self.parents[b] = c.a;
        touched.insert(c.a);
        touched.insert(c.b);
        for &vertex in &touched {
            self.versions[vertex as usize] = self.versions[vertex as usize].checked_add(1).ok_or(KernelError::Budget)?;
        }
        Ok((removed, touched))
    }
}

fn add_edges(heap: &mut BinaryHeap<Candidate>, work: &Work, vertices: &BTreeSet<u32>, max_error: f64) -> Result<()> {
    let mut edges = BTreeSet::new();
    for &v in vertices {
        if !work.active[v as usize] {
            continue;
        }
        for neighbor in work.neighbors(v as usize) {
            edges.insert((v.min(neighbor), v.max(neighbor)));
        }
    }
    for (a, b) in edges {
        if let Some(c) = work.candidate(a, b) {
            if c.error <= max_error {
                if heap.len() >= MAX_HEAP {
                    return Err(KernelError::Budget);
                }
                heap.push(c);
            }
        }
    }
    Ok(())
}

fn root(parent: &[u32], mut vertex: u32) -> Option<usize> {
    for _ in 0..parent.len() {
        let next = *parent.get(vertex as usize)?;
        if next == vertex {
            return Some(vertex as usize);
        }
        vertex = next;
    }
    None
}

/// Non-destructive bounded QEM triangle reduction.
///
/// Legal collapses require two oppositely wound faces along an interior edge,
/// a valid vertex-link condition, and no inverted/degenerate affected faces.
/// Original boundaries and optional creases remain fixed. No face or vertex
/// attributes are stored in TriangleMesh, so attribute-aware decimation is a
/// separate later operation. The method does not certify absence of new global
/// intersections or a Hausdorff error bound.
pub fn mesh_quadric_decimate(mesh: &TriangleMesh, options: MeshDecimateOptions) -> Result<MeshDecimateResult> {
    if mesh.vertices.len() > MAX_VERTICES || mesh.triangles.len() > MAX_FACES {
        return Err(KernelError::Budget);
    }
    if !options.max_quadric_error.is_finite() || options.max_quadric_error < 0.0
        || !options.max_normal_change_degrees.is_finite() || !(0.0..89.0).contains(&options.max_normal_change_degrees)
        || options.preserve_creases_above_degrees.is_some_and(|t| !t.is_finite() || !(0.0..=180.0).contains(&t)) {
        return Err(KernelError::Invalid("mesh decimate options"));
    }
    validate_triangle_mesh(mesh)?;
    if !mesh_degenerate_faces(mesh, 0.0)?.is_empty() || !mesh_duplicate_faces(mesh)?.duplicates.is_empty() {
        return Err(KernelError::Invalid("decimation needs nondegenerate, unique input triangles"));
    }
    let edge_report = mesh_edge_report(mesh)?;
    if !edge_report.non_manifold_edges.is_empty() || !edge_report.inconsistent_winding_edges.is_empty() {
        return Err(KernelError::Invalid("decimation needs consistent manifold edges"));
    }

    // Two closed shells sharing one vertex have valid edges but a pinched,
    // non-manifold vertex. Reuse the native quad/triangle fan classifier here
    // rather than maintaining a competing mesh-topology predicate.
    let polygon = polygon_mesh_from_triangles(mesh)?;
    if !polygon_mesh_vertex_fans(&polygon)?.non_manifold_vertices.is_empty() {
        return Err(KernelError::Invalid("decimation needs manifold vertex fans"));
    }

    let n = mesh.vertices.len();
    let mut work = Work {
        positions: mesh.vertices.clone(),
        faces: mesh.triangles.iter().copied().map(Some).collect(),
        incident: vec![BTreeSet::new(); n],
        active: vec![true; n],
        protected: vec![false; n],
        quadrics: vec![Quadric::default(); n],
        versions: vec![0; n],
        parents: (0..n).map(|i| i as u32).collect(),
    };
    let mut edge_faces: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    let mut normals = Vec::new();
    normals.try_reserve_exact(mesh.triangles.len()).map_err(|_| KernelError::Budget)?;
    for (i, face) in mesh.triangles.iter().copied().enumerate() {
        let unit = face_normal(face, &mesh.vertices).ok_or(KernelError::Invalid("degenerate input triangle"))?;
        normals.push(unit);
        let quadric = Quadric::from_plane(unit, mesh.vertices[face[0] as usize]);
        for &v in &face {
            work.incident[v as usize].insert(i);
            work.quadrics[v as usize] = work.quadrics[v as usize].add(quadric);
        }
        for (a, b) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            edge_faces.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    if options.preserve_boundary {
        for edge in &edge_report.boundary_edges {
            work.protected[edge[0] as usize] = true;
            work.protected[edge[1] as usize] = true;
        }
    }
    if let Some(degrees) = options.preserve_creases_above_degrees {
        let cosine = degrees.to_radians().cos();
        for (&(a, b), faces) in &edge_faces {
            if faces.len() == 2 && dot(normals[faces[0]], normals[faces[1]]) < cosine {
                work.protected[a as usize] = true;
                work.protected[b as usize] = true;
            }
        }
    }
    let mut heap = BinaryHeap::new();
    for &(a, b) in edge_faces.keys() {
        if let Some(candidate) = work.candidate(a, b) {
            if candidate.error <= options.max_quadric_error {
                if heap.len() >= MAX_HEAP {
                    return Err(KernelError::Budget);
                }
                heap.push(candidate);
            }
        }
    }
    let mut remaining = mesh.triangles.len();
    let cosine_limit = options.max_normal_change_degrees.to_radians().cos();
    while remaining > options.target_faces {
        let Some(candidate) = heap.pop() else { break; };
        if work.versions[candidate.a as usize] != candidate.ver_a || work.versions[candidate.b as usize] != candidate.ver_b {
            continue;
        }
        if candidate.error > options.max_quadric_error || !work.is_legal(&candidate, cosine_limit) {
            continue;
        }
        // Collapsing a closed tetrahedron to a pair of coincident/opposite
        // faces is not a valid simplification. Closed manifolds need >=4 faces.
        if remaining < 6 {
            break;
        }
        let (removed, changed) = work.collapse(&candidate)?;
        remaining -= removed;
        add_edges(&mut heap, &work, &changed, options.max_quadric_error)?;
    }

    let mut vertices = Vec::new();
    let mut used = vec![false; n];
    for face in work.faces.iter().flatten() {
        for &v in face {
            used[v as usize] = true;
        }
    }
    let mut compact = vec![None; n];
    for (i, &is_used) in used.iter().enumerate() {
        if is_used {
            let idx = u32::try_from(vertices.len()).map_err(|_| KernelError::Budget)?;
            compact[i] = Some(idx);
            vertices.push(work.positions[i]);
        }
    }
    let mut triangles = Vec::new();
    triangles.try_reserve_exact(remaining).map_err(|_| KernelError::Budget)?;
    for face in work.faces.into_iter().flatten() {
        triangles.push([
            compact[face[0] as usize].ok_or(KernelError::Invalid("decimation vertex"))?,
            compact[face[1] as usize].ok_or(KernelError::Invalid("decimation vertex"))?,
            compact[face[2] as usize].ok_or(KernelError::Invalid("decimation vertex"))?,
        ]);
    }
    let mut old_to_new = Vec::new();
    old_to_new.try_reserve_exact(n).map_err(|_| KernelError::Budget)?;
    for i in 0..n {
        let representative = root(&work.parents, i as u32).ok_or(KernelError::Invalid("decimation parent cycle"))?;
        old_to_new.push(compact[representative]);
    }
    let output = TriangleMesh { vertices, triangles };
    let edges = mesh_edge_report(&output)?;
    if !output.triangles.is_empty() && !polygon_mesh_vertex_fans(&polygon_mesh_from_triangles(&output)?)?.non_manifold_vertices.is_empty() {
        return Err(KernelError::Invalid("decimation pinches vertex fans"));
    }
    if !edges.non_manifold_edges.is_empty() || !edges.inconsistent_winding_edges.is_empty()
        || !mesh_duplicate_faces(&output)?.duplicates.is_empty() || !mesh_degenerate_faces(&output, 0.0)?.is_empty() {
        return Err(KernelError::Invalid("decimation postcondition"));
    }
    Ok(MeshDecimateResult {
        removed_faces: mesh.triangles.len() - output.triangles.len(),
        target_reached: output.triangles.len() <= options.target_faces,
        mesh: output,
        old_to_new,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(target_faces: usize) -> MeshDecimateOptions {
        MeshDecimateOptions {
            target_faces,
            max_quadric_error: 1e9,
            max_normal_change_degrees: 85.0,
            preserve_boundary: true,
            preserve_creases_above_degrees: None,
        }
    }

    fn octahedron() -> TriangleMesh {
        TriangleMesh {
            vertices: vec![
                Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0),
            ],
            triangles: vec![
                [0, 2, 3], [0, 3, 4], [0, 4, 5], [0, 5, 2],
                [1, 3, 2], [1, 4, 3], [1, 5, 4], [1, 2, 5],
            ],
        }
    }

    #[test]
    fn reduces_octahedron_with_manifold_winding_and_stable_remap() {
        let source = octahedron();
        let original = source.clone();
        let result = mesh_quadric_decimate(&source, options(6));
        assert!(result.is_ok());
        if let Ok(result) = result {
            assert_eq!(result.mesh.triangles.len(), 6);
            assert_eq!(result.removed_faces, 2);
            assert!(result.target_reached);
            assert_eq!(result.old_to_new.len(), 6);
            assert!(mesh_edge_report(&result.mesh).is_ok_and(|report| {
                report.boundary_edges.is_empty() && report.non_manifold_edges.is_empty() && report.inconsistent_winding_edges.is_empty()
            }));
        }
        assert_eq!(source, original);
    }

    #[test]
    fn keeps_open_square_boundary_and_original_mesh_untouched() {
        let m = TriangleMesh {
            vertices: vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let result = mesh_quadric_decimate(&m, options(0));
        assert!(result.is_ok_and(|r| r.mesh == m && r.removed_faces == 0 && !r.target_reached));
    }

    #[test]
    fn preserves_sharp_feature_vertices() {
        let m = octahedron();
        let mut options = options(4);
        options.preserve_creases_above_degrees = Some(30.0);
        let r = mesh_quadric_decimate(&m, options);
        assert!(r.is_ok_and(|r| r.mesh == m && r.removed_faces == 0));
    }

    #[test]
    fn no_op_and_deterministic_repeated_runs() {
        let m = octahedron();
        let opts = options(8);
        let r = mesh_quadric_decimate(&m, opts);
        assert!(r.is_ok_and(|r| r.mesh == m && r.removed_faces == 0 && r.target_reached));
        assert_eq!(mesh_quadric_decimate(&m, options(6)), mesh_quadric_decimate(&m, options(6)));
    }

    #[test]
    fn rejects_pinched_vertex_even_when_edges_are_manifold() {
        let m = TriangleMesh {
            vertices: vec![
                Vec3::ZERO,
                Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(-1.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0),
            ],
            triangles: vec![[0, 1, 2], [0, 3, 4]],
        };
        let edges = mesh_edge_report(&m);
        assert!(edges.is_ok_and(|r| r.non_manifold_edges.is_empty()));
        assert!(mesh_quadric_decimate(&m, options(1)).is_err());
    }

    #[test]
    fn rejects_non_manifold_invalid_and_degenerate_inputs() {
        let mut m = octahedron();
        m.triangles.push([0, 2, 3]);
        assert!(mesh_quadric_decimate(&m, options(6)).is_err());
        let mut m = octahedron();
        m.triangles[0] = [0, 0, 3];
        assert!(mesh_quadric_decimate(&m, options(6)).is_err());
        let mut m = octahedron();
        m.triangles[0] = [0, 3, 2];
        assert!(mesh_quadric_decimate(&m, options(6)).is_err());
        let mut opts = options(6);
        opts.max_quadric_error = f64::NAN;
        assert!(mesh_quadric_decimate(&octahedron(), opts).is_err());
    }
}
