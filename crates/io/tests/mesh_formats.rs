use cadcraft_geom::Vec3;
use cadcraft_io::mesh_formats::{
    MAX_MESH_INPUT_BYTES, NamedTriangleMesh, read_meshes, read_obj_meshes, read_stl_mesh, write_stl_mesh,
};
use buildercraft_kernel::TriangleMesh;

fn triangle() -> TriangleMesh {
    TriangleMesh {
        vertices: vec![
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ],
        triangles: vec![[0, 1, 2]],
    }
}

#[test]
fn binary_stl_roundtrip_preserves_orientation_and_coordinates() {
    let source = triangle();
    let bytes = write_stl_mesh(&source).unwrap();
    assert_eq!(bytes.len(), 134);
    let back = read_stl_mesh(&bytes).unwrap();
    assert_eq!(back, source);
    let report = read_meshes(&bytes, "part.STL").unwrap();
    assert_eq!(report.objects.len(), 1);
    assert!(!report.diagnostics.is_empty());
}

#[test]
fn ascii_stl_decodes_without_changing_winding() {
    let source = b"solid test\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid test\n";
    let imported = read_stl_mesh(source).unwrap();
    assert_eq!(imported, triangle());
}

#[test]
fn obj_preserves_named_objects_and_double_precision() {
    let source = br#"o Frame
v 0 0 0
v 1.000000000000001 0 0
v 0 1 0
f 1 2 3
o Panel
v 0 0 1
v 1 0 1
v 0 1 1
f 4 5 6
"#;
    let loaded = read_obj_meshes(source).unwrap();
    assert_eq!(loaded.objects.len(), 2);
    assert_eq!(loaded.objects[0].name, "Frame");
    assert_eq!(loaded.objects[1].name, "Panel");
    assert!((loaded.objects[0].mesh.vertices[1].x - 1.000000000000001).abs() < 1e-15);
    assert_eq!(loaded.objects[1].mesh.triangles.len(), 1);
    assert!(loaded.diagnostics.iter().any(|msg| msg.contains("units")));
}

#[test]
fn obj_triangles_and_quads_become_triangle_views() {
    let source = b"o Slab\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nf 1 2 3 4\n";
    let loaded = read_meshes(source, "slab.obj").unwrap();
    assert_eq!(loaded.objects[0].mesh.triangles.len(), 2);
}

#[test]
fn invalid_meshes_and_hostile_sizes_fail_closed() {
    assert!(read_stl_mesh(&[]).is_err());
    assert!(read_obj_meshes(b"v 0 0 0\n").is_err());
    assert!(read_meshes(b"hello", "other.dwg").is_err());
    assert!(read_meshes(&vec![0u8; MAX_MESH_INPUT_BYTES + 1], "oversize.stl").is_err());
    let mut malformed = triangle();
    malformed.vertices[0].x = f64::NAN;
    assert!(write_stl_mesh(&malformed).is_err());
    malformed = triangle();
    malformed.triangles = vec![[0, 0, 1]];
    assert!(write_stl_mesh(&malformed).is_err());
    assert_eq!(triangle().vertices.len(), 3);
}

#[test]
fn imported_mesh_does_not_masquerade_as_a_persisted_document() {
    let mesh = NamedTriangleMesh { name: "Preview".into(), mesh: triangle() };
    let source = cadcraft_doc::Drawing::new_metric();
    assert_eq!(mesh.name, "Preview");
    assert!(source.geometry3d.is_empty());
    // No unsupported mesh type can silently be routed through Drawing::write.
    assert!(cadcraft_io::write(&source, "source.stl").is_err());
}
