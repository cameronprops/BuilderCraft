//! Bounded mesh interchange adapters. Geometry always returns Worldwright's
//! own TriangleMesh, never an external crate's mesh or scene representation.
//!
//! STL has no units/IDs/materials; OBJ units are unspecified. Callers must
//! explicitly set import scale/units and persist objects in a document.
//! These APIs do NOT imply Drawing::read or user-facing CAD import support.

use crate::{IoError, Result};
use buildercraft_kernel::{TriangleMesh, mesh_face_analysis};
use cadcraft_geom::Vec3;
use std::io::{BufReader, Cursor};

/// Limit parsed untrusted input before external parsers allocate memory.
pub const MAX_MESH_INPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MESH_VERTICES: usize = 1_000_000;
pub const MAX_MESH_TRIANGLES: usize = 1_000_000;

/// A source mesh plus object name. Named OBJ objects remain independent.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedTriangleMesh {
    pub name: String,
    pub mesh: TriangleMesh,
}

/// Explicit import diagnostics for data the first alpha mesh schema cannot
/// retain (materials, normals, texture coordinates, freeform OBJ data).
#[derive(Clone, Debug, PartialEq)]
pub struct MeshImport {
    pub objects: Vec<NamedTriangleMesh>,
    pub diagnostics: Vec<String>,
}

fn bad(message: impl Into<String>) -> IoError {
    IoError::Format(message.into())
}

fn guard_input(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_MESH_INPUT_BYTES {
        return Err(bad("mesh input empty or exceeds 16 MiB limit"));
    }
    Ok(())
}

fn validate(mesh: &TriangleMesh) -> Result<()> {
    if mesh.vertices.is_empty() || mesh.triangles.is_empty() || mesh.vertices.len() > MAX_MESH_VERTICES || mesh.triangles.len() > MAX_MESH_TRIANGLES {
        return Err(bad("mesh vertex/face count invalid or exceeds limit"));
    }
    // Reuse the shared kernel's index/finite/coordinate validation and
    // analysis rather than creating another mesh validation engine.
    mesh_face_analysis(mesh, 0.0).map_err(|e| bad(e.to_string()))?;
    Ok(())
}

/// Read ASCII or binary STL. Vertex positions are only f32 precise on disk.
/// This is mesh geometry only, not proof of water-tightness or solidness.
pub fn read_stl_mesh(bytes: &[u8]) -> Result<TriangleMesh> {
    guard_input(bytes)?;
    let mut cursor = Cursor::new(bytes);
    let raw = stl_io::read_stl(&mut cursor).map_err(|e| bad(format!("STL: {e}")))?;
    if raw.vertices.len() > MAX_MESH_VERTICES || raw.faces.len() > MAX_MESH_TRIANGLES {
        return Err(bad("STL mesh element limit"));
    }
    let mut vertices = Vec::new();
    vertices.try_reserve_exact(raw.vertices.len()).map_err(|_| bad("STL vertex allocation"))?;
    for point in raw.vertices {
        vertices.push(Vec3::new(f64::from(point.0[0]), f64::from(point.0[1]), f64::from(point.0[2])));
    }
    let mut triangles = Vec::new();
    triangles.try_reserve_exact(raw.faces.len()).map_err(|_| bad("STL triangle allocation"))?;
    for face in raw.faces {
        let a = u32::try_from(face.vertices[0]).map_err(|_| bad("STL triangle index overflow"))?;
        let b = u32::try_from(face.vertices[1]).map_err(|_| bad("STL triangle index overflow"))?;
        let c = u32::try_from(face.vertices[2]).map_err(|_| bad("STL triangle index overflow"))?;
        triangles.push([a, b, c]);
    }
    let result = TriangleMesh { vertices, triangles };
    validate(&result)?;
    Ok(result)
}

/// Export native triangle mesh to binary STL using shared kernel normals.
/// Refuses degenerate triangles; the mesh remains unmodified on error.
/// Output coordinates are f32, as required by STL.
pub fn write_stl_mesh(mesh: &TriangleMesh) -> Result<Vec<u8>> {
    validate(mesh)?;
    let analysis = mesh_face_analysis(mesh, 0.0).map_err(|e| bad(e.to_string()))?;
    let mut faces = Vec::new();
    faces.try_reserve_exact(mesh.triangles.len()).map_err(|_| bad("STL output allocation"))?;
    for (&indices, info) in mesh.triangles.iter().zip(analysis.iter()) {
        let normal = info.normal.ok_or_else(|| bad("STL cannot export a degenerate triangle"))?;
        let to_vertex = |i: u32| {
            let p = mesh.vertices[i as usize];
            stl_io::Vertex::new([p.x as f32, p.y as f32, p.z as f32])
        };
        faces.push(stl_io::Triangle {
            normal: stl_io::Normal::new([normal.x as f32, normal.y as f32, normal.z as f32]),
            vertices: [to_vertex(indices[0]), to_vertex(indices[1]), to_vertex(indices[2])],
        });
    }
    let capacity = mesh.triangles.len().checked_mul(50).and_then(|n| n.checked_add(84)).ok_or_else(|| bad("STL byte budget overflow"))?;
    if capacity > MAX_MESH_INPUT_BYTES {
        return Err(bad("STL output exceeds 16 MiB limit"));
    }
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| bad("STL output allocation"))?;
    stl_io::write_stl(&mut output, faces.iter()).map_err(|e| bad(format!("STL write: {e}")))?;
    Ok(output)
}

/// Read OBJ objects as distinct triangle meshes. The f64 parser option avoids
/// premature precision truncation. Materials and UVs are reported, not stored
/// silently. Unsupported external MTL files are never fetched automatically.
pub fn read_obj_meshes(bytes: &[u8]) -> Result<MeshImport> {
    guard_input(bytes)?;
    let mut reader = BufReader::new(Cursor::new(bytes));
    let options = tobj::LoadOptions { triangulate: true, single_index: true, ignore_points: true, ignore_lines: true };
    let (models, _) = tobj::load_obj_buf(&mut reader, &options, |_| Ok((Vec::new(), Default::default()))).map_err(|e| bad(format!("OBJ: {e}")))?;
    if models.is_empty() || models.len() > 256 {
        return Err(bad("OBJ object count invalid or exceeds 256"));
    }
    let mut objects = Vec::new();
    let mut diagnostics = Vec::new();
    let mut total_vertices = 0usize;
    let mut total_triangles = 0usize;
    for (index, model) in models.into_iter().enumerate() {
        let raw = model.mesh;
        if raw.positions.len() % 3 != 0 || raw.indices.len() % 3 != 0 {
            return Err(bad("OBJ non-triangle topology after triangulation"));
        }
        let vertex_count = raw.positions.len() / 3;
        let triangle_count = raw.indices.len() / 3;
        total_vertices = total_vertices.checked_add(vertex_count).ok_or_else(|| bad("OBJ vertex overflow"))?;
        total_triangles = total_triangles.checked_add(triangle_count).ok_or_else(|| bad("OBJ face overflow"))?;
        if total_vertices > MAX_MESH_VERTICES || total_triangles > MAX_MESH_TRIANGLES {
            return Err(bad("OBJ aggregate mesh element limit"));
        }
        if !raw.normals.is_empty() || !raw.texcoords.is_empty() || !raw.vertex_color.is_empty() || raw.material_id.is_some() {
            diagnostics.push(format!("OBJ object {}: material, normal, UV or vertex color attributes require a richer mesh carrier", index + 1));
        }
        if vertex_count == 0 || triangle_count == 0 {
            diagnostics.push(format!("OBJ object {} contains no triangle geometry", index + 1));
            continue;
        }
        let mut vertices = Vec::new();
        vertices.try_reserve_exact(vertex_count).map_err(|_| bad("OBJ vertex allocation"))?;
        for point in raw.positions.chunks_exact(3) {
            vertices.push(Vec3::new(point[0], point[1], point[2]));
        }
        let mut triangles = Vec::new();
        triangles.try_reserve_exact(triangle_count).map_err(|_| bad("OBJ triangle allocation"))?;
        for face in raw.indices.chunks_exact(3) {
            triangles.push([face[0], face[1], face[2]]);
        }
        let mesh = TriangleMesh { vertices, triangles };
        validate(&mesh)?;
        let name = if model.name.is_empty() { format!("Mesh {}", index + 1) } else { model.name };
        objects.push(NamedTriangleMesh { name, mesh });
    }
    if objects.is_empty() {
        return Err(bad("OBJ has no triangle geometry"));
    }
    diagnostics.push("OBJ coordinate units are unspecified; assign source units on import".into());
    Ok(MeshImport { objects, diagnostics })
}

/// Select a mesh codec explicitly; do not pass mesh data to Drawing::read.
pub fn read_meshes(bytes: &[u8], name: &str) -> Result<MeshImport> {
    let extension = std::path::Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
    match extension.as_str() {
        "stl" => Ok(MeshImport {
            objects: vec![NamedTriangleMesh { name: "STL mesh".into(), mesh: read_stl_mesh(bytes)? }],
            diagnostics: vec!["STL has no source units, layers, materials, object IDs or topology guarantees".into()],
        }),
        "obj" => read_obj_meshes(bytes),
        _ => Err(IoError::Unsupported(extension)),
    }
}
