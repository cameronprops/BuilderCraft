//! Bounded mesh exchange adapters. Shared triangle geometry stays authoritative.
use crate::{IoError, Result};
use buildercraft_kernel::{TriangleMesh, mesh_face_analysis, validate_triangle_mesh};
use cadcraft_geom::Vec3;
use std::{
    collections::HashMap,
    io::{Cursor, Write},
};

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_SAMPLES: usize = 65_536;
pub struct MeshImport {
    pub mesh: TriangleMesh,
    pub losses: Vec<String>,
}
pub struct MeshExport {
    pub bytes: Vec<u8>,
    pub max_coordinate_error: f64,
    pub losses: Vec<String>,
}
fn fail(message: impl Into<String>) -> IoError {
    IoError::Format(message.into())
}
fn validate(mesh: &TriangleMesh) -> Result<()> {
    if mesh.vertices.len() > MAX_SAMPLES || mesh.triangles.is_empty() || mesh.triangles.len() > MAX_SAMPLES {
        return Err(fail("Mesh exchange requires 1–65536 vertices and triangles"));
    }
    validate_triangle_mesh(mesh).map_err(|e| fail(e.to_string()))
}
/// No file/network access or external material loading. Units belong to the caller.
pub fn read_mesh(bytes: &[u8], format: &str) -> Result<MeshImport> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err(fail("Mesh input byte budget exceeded"));
    }
    let output = match format {
        "stl" => read_stl(bytes)?,
        "obj" => read_obj(bytes)?,
        _ => return Err(fail("Mesh format must be stl or obj")),
    };
    validate(&output.mesh)?;
    Ok(output)
}
pub(crate) fn read_stl(bytes: &[u8]) -> Result<MeshImport> {
    let mut cursor = Cursor::new(bytes);
    let reader = stl_io::create_stl_reader(&mut cursor).map_err(|e| fail(e.to_string()))?;
    if let Some(count) = reader.size_hint().1
        && count.checked_mul(50).and_then(|n| n.checked_add(84)) != Some(bytes.len())
    {
        return Err(fail("Binary STL length does not match its face count"));
    }
    if reader.size_hint().0 > MAX_SAMPLES {
        return Err(fail("STL face budget exceeded"));
    }
    let mut mesh = TriangleMesh { vertices: Vec::new(), triangles: Vec::new() };
    let mut vertices = HashMap::new();
    for triangle in reader {
        if mesh.triangles.len() >= MAX_SAMPLES {
            return Err(fail("STL face budget exceeded"));
        }
        let triangle = triangle.map_err(|e| fail(e.to_string()))?;
        let mut indices = [0; 3];
        for (slot, vertex) in indices.iter_mut().zip(triangle.vertices) {
            let coords = vertex.0;
            // Normalize signed zero for geometric vertex identity, preserve other f32 bits.
            let key = coords.map(|n| if n == 0. { 0 } else { n.to_bits() });
            *slot = if let Some(&index) = vertices.get(&key) {
                index
            } else {
                if mesh.vertices.len() >= MAX_SAMPLES {
                    return Err(fail("STL vertex budget exceeded"));
                }
                let index = u32::try_from(mesh.vertices.len()).map_err(|_| fail("Vertex index overflow"))?;
                vertices.try_reserve(1).map_err(|_| fail("Vertex map admission failed"))?;
                mesh.vertices.try_reserve(1).map_err(|_| fail("Vertex buffer admission failed"))?;
                mesh.vertices.push(Vec3::new(f64::from(coords[0]), f64::from(coords[1]), f64::from(coords[2])));
                vertices.insert(key, index);
                index
            };
        }
        mesh.triangles.try_reserve(1).map_err(|_| fail("Face buffer admission failed"))?;
        mesh.triangles.push(indices);
    }
    Ok(MeshImport { mesh, losses: vec!["STL contains f32 positions without units; facet normals/attributes are not retained".into()] })
}
fn read_obj(bytes: &[u8]) -> Result<MeshImport> {
    let text = std::str::from_utf8(bytes).map_err(|_| fail("OBJ must be UTF-8"))?;
    let mut counts = [0usize; 5];
    let mut metadata = false;
    for line in text.lines() {
        let mut words = line.split('#').next().unwrap_or_default().split_whitespace();
        let Some(kind) = words.next() else {
            continue;
        };
        let index = match kind {
            "v" => {
                if words.count() != 3 {
                    return Err(fail("OBJ vertices require exactly XYZ; colors/homogeneous coordinates are not supported"));
                }
                0
            }
            "vn" => {
                metadata = true;
                1
            }
            "vt" => {
                metadata = true;
                2
            }
            "f" => {
                if words.count() != 3 {
                    return Err(fail("OBJ exchange accepts triangular faces only"));
                }
                3
            }
            "o" | "g" => {
                metadata = true;
                4
            }
            "s" | "usemtl" => {
                metadata = true;
                continue;
            }
            "mtllib" => return Err(fail("External OBJ materials are not loaded; provide geometry without mtllib")),
            _ => return Err(fail(format!("Unsupported OBJ record: {kind}"))),
        };
        counts[index] += 1;
        let limit = if index == 4 { 256 } else { MAX_SAMPLES };
        if counts[index] > limit {
            return Err(fail("OBJ source record budget exceeded"));
        }
    }
    // The named-object codec remains the only OBJ parser. This command facade
    // applies stricter triangle admission and combines its explicit objects.
    let parsed = crate::mesh_formats::read_obj_meshes(bytes)?;
    let mut mesh = TriangleMesh { vertices: Vec::new(), triangles: Vec::new() };
    for object in parsed.objects {
        let source = object.mesh;
        if source.vertices.len() > MAX_SAMPLES - mesh.vertices.len() || source.triangles.len() > MAX_SAMPLES - mesh.triangles.len() {
            return Err(fail("OBJ output geometry budget exceeded"));
        }
        let offset = u32::try_from(mesh.vertices.len()).map_err(|_| fail("Vertex index overflow"))?;
        mesh.vertices.try_reserve_exact(source.vertices.len()).map_err(|_| fail("Vertex admission failed"))?;
        mesh.triangles.try_reserve_exact(source.triangles.len()).map_err(|_| fail("Face admission failed"))?;
        mesh.vertices.extend(source.vertices);
        for face in source.triangles {
            let mut output = [0; 3];
            for (slot, index) in output.iter_mut().zip(face) {
                *slot = index.checked_add(offset).ok_or_else(|| fail("Vertex index overflow"))?;
            }
            mesh.triangles.push(output);
        }
    }
    let mut losses = vec!["OBJ vertex indexing may be remapped and unreferenced vertices are not retained".into()];
    if metadata {
        losses.push("OBJ normals, UVs, names/groups and material assignments are not retained in the position-only mesh".into());
    }
    Ok(MeshImport { mesh, losses })
}
struct Buffer(Vec<u8>);
impl Write for Buffer {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if data.len() > MAX_BYTES - self.0.len() {
            return Err(std::io::Error::other("Mesh output byte budget exceeded"));
        }
        let required = self.0.len() + data.len();
        if required > self.0.capacity() {
            let capacity = MAX_BYTES.min((self.0.capacity().max(256) * 2).max(required));
            self.0.try_reserve_exact(capacity - self.0.len()).map_err(|_| std::io::Error::other("Output admission failed"))?;
        }
        self.0.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn write_mesh(mesh: &TriangleMesh, format: &str) -> Result<MeshExport> {
    validate(mesh)?;
    let mut output = Buffer(Vec::new());
    match format {
        "obj" => {
            for p in &mesh.vertices {
                writeln!(output, "v {:.17e} {:.17e} {:.17e}", p.x, p.y, p.z).map_err(|e| fail(e.to_string()))?;
            }
            for &[a, b, c] in &mesh.triangles {
                writeln!(output, "f {} {} {}", a + 1, b + 1, c + 1).map_err(|e| fail(e.to_string()))?;
            }
            Ok(MeshExport { bytes: output.0, max_coordinate_error: 0., losses: Vec::new() })
        }
        "stl" => {
            let mut converted = TriangleMesh { vertices: Vec::new(), triangles: mesh.triangles.clone() };
            converted.vertices.try_reserve_exact(mesh.vertices.len()).map_err(|_| fail("STL precision buffer admission failed"))?;
            let mut error: f64 = 0.;
            for p in &mesh.vertices {
                let point = Vec3::new(f64::from(p.x as f32), f64::from(p.y as f32), f64::from(p.z as f32));
                error = error.max((point.x - p.x).abs().max((point.y - p.y).abs()).max((point.z - p.z).abs()));
                converted.vertices.push(point);
            }
            let before = mesh_face_analysis(mesh, 0.).map_err(|e| fail(e.to_string()))?;
            let after = mesh_face_analysis(&converted, 0.).map_err(|e| fail(e.to_string()))?;
            if before.iter().zip(&after).any(|(a, b)| a.degenerate != b.degenerate || a.normal.zip(b.normal).is_some_and(|(a, b)| a.dot(b) <= 0.)) {
                return Err(fail("STL f32 conversion collapses or reverses a triangle; use OBJ or change coordinates"));
            }
            let mut triangles = Vec::new();
            triangles.try_reserve_exact(mesh.triangles.len()).map_err(|_| fail("STL face admission failed"))?;
            for (face, info) in converted.triangles.iter().zip(after) {
                let normal = info.normal.unwrap_or(Vec3::ZERO);
                let mut vertices = [stl_io::Vertex::new([0.; 3]); 3];
                for (slot, index) in vertices.iter_mut().zip(face) {
                    let p = converted.vertices.get(*index as usize).ok_or_else(|| fail("Invalid STL triangle index"))?;
                    *slot = stl_io::Vertex::new([p.x as f32, p.y as f32, p.z as f32]);
                }
                triangles.push(stl_io::Triangle { normal: stl_io::Normal::new([normal.x as f32, normal.y as f32, normal.z as f32]), vertices });
            }
            stl_io::write_stl(&mut output, triangles.iter()).map_err(|e| fail(e.to_string()))?;
            Ok(MeshExport {
                bytes: output.0,
                max_coordinate_error: error,
                losses: vec!["STL stores f32 positions, recomputed normals and no units; unused vertex/vertex index identity is not retained".into()],
            })
        }
        _ => Err(fail("Mesh format must be stl or obj")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> TriangleMesh {
        TriangleMesh { vertices: vec![Vec3::ZERO, Vec3::new(2., 0., 0.), Vec3::new(0., 2., 0.)], triangles: vec![[0, 1, 2]] }
    }
    #[test]
    fn mesh_exchange_round_trip_source_precision_and_shared_closest_query() {
        let source = fixture();
        for format in ["obj", "stl"] {
            let bytes = write_mesh(&source, format).unwrap();
            assert_eq!(source, fixture());
            let result = read_mesh(&bytes.bytes, format).unwrap();
            assert_eq!(result.mesh, source);
            let hit = buildercraft_kernel::mesh_closest_point(&result.mesh, Vec3::new(0.5, 0.5, 3.), None, &Default::default()).unwrap().unwrap();
            assert_eq!(hit.point, Vec3::new(0.5, 0.5, 0.));
        }
        let mut precise = source;
        precise.vertices[1].x = 1.00000000001;
        assert_eq!(read_mesh(&write_mesh(&precise, "obj").unwrap().bytes, "obj").unwrap().mesh, precise);
        assert!(write_mesh(&precise, "stl").unwrap().max_coordinate_error > 0.);
        let collapsed = TriangleMesh {
            vertices: vec![Vec3::new(1e10, 0., 0.), Vec3::new(1e10 + 1., 0., 0.), Vec3::new(1e10, 1., 0.)],
            triangles: vec![[0, 1, 2]],
        };
        assert!(write_mesh(&collapsed, "stl").is_err());
    }
    #[test]
    fn mesh_exchange_hostile_admission_and_external_reference_rejection() {
        let mut binary = vec![0u8; 84];
        binary[80..84].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read_mesh(&binary, "stl").is_err());
        for obj in ["mtllib ../private.mtl\n", "v NaN 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3", "v 0 0 0\nf 1 2 9", "v 0 0 0\nf 1 1 1 1", "curv 0 1 1 2"] {
            assert!(read_mesh(obj.as_bytes(), "obj").is_err());
        }
        let mut invalid = fixture();
        invalid.triangles[0][2] = u32::MAX;
        assert!(write_mesh(&invalid, "obj").is_err());
        assert!(read_mesh(&vec![0; MAX_BYTES + 1], "stl").is_err());
    }
}
