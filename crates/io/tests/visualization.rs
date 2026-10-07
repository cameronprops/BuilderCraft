use buildercraft_kernel::*;
use cadcraft_doc::{
    Drawing,
    organization::{GeometryObject, Shape},
};
use cadcraft_geom::{Vec3, nurbs3d};
use cadcraft_io::visualization::*;
fn fixture() -> Drawing {
    let mut d = Drawing::new_metric();
    let row = |y| nurbs3d::Curve {
        degree: 1,
        control: vec![Vec3::new(0., y, 0.), Vec3::new(2000., y, 0.)],
        weights: vec![1., 1.],
        knots: nurbs3d::uniform_knots(2, 1),
    };
    d.geometry3d.push(GeometryObject {
        id: 20,
        name: "Massing floor".into(),
        layer: "0".into(),
        visible: true,
        shape: Shape::Surface(nurbs3d::Surface { rows: vec![row(0.), row(3000.)], degree_v: 1, knots_v: nurbs3d::uniform_knots(2, 1) }.into()),
    });
    d
}
fn id(n: u128) -> Id {
    Id::new(n).unwrap()
}
#[test]
fn exported_glb_validates_and_has_correct_meter_bounds_and_ids() {
    let d = fixture();
    let scene = snapshot(&d, id(1), 7, Default::default(), &Default::default()).unwrap();
    assert_eq!(scene.objects[0].id, id(21));
    assert_eq!(scene.source_revision, "7");
    let mesh = &scene.objects[0];
    let corner = mesh.positions.last().unwrap();
    assert!((corner[0] - 2.).abs() < 1e-9 && (corner[1] + 3.).abs() < 1e-9 && corner[2].abs() < 1e-9);
    let bytes = glb(&scene).unwrap();
    let parsed = gltf::Gltf::from_slice(&bytes).unwrap();
    assert_eq!(bytes.len(), u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize);
    let primitive = parsed.meshes().next().unwrap().primitives().next().unwrap();
    let positions = primitive.get(&gltf::Semantic::Positions).unwrap();
    assert_eq!(positions.count(), 289);
    assert_eq!(positions.min().unwrap(), serde_json::json!([0., 0., -3.]));
    assert_eq!(positions.max().unwrap(), serde_json::json!([2., 0., 0.]));
    assert_eq!(primitive.indices().unwrap().count(), 1536);
}
#[test]
fn geometry_fingerprint_is_stable_for_rename_and_changes_for_edit() {
    let mut d = fixture();
    let a = snapshot(&d, id(1), 1, Default::default(), &Default::default()).unwrap();
    d.geometry3d[0].name = "Renamed floor".into();
    let b = snapshot(&d, id(1), 2, Default::default(), &Default::default()).unwrap();
    assert_eq!(a.objects[0].id, b.objects[0].id);
    assert_eq!(a.objects[0].geometry_key, b.objects[0].geometry_key);
    let Shape::Surface(s) = &mut d.geometry3d[0].shape else { panic!() };
    std::sync::Arc::make_mut(s).rows[0].control[0].z = 100.;
    let c = snapshot(&d, id(1), 3, Default::default(), &Default::default()).unwrap();
    assert_ne!(a.objects[0].geometry_key, c.objects[0].geometry_key);
}
#[test]
fn empty_and_hidden_scenes_are_valid_portable_exports() {
    let mut d = fixture();
    d.geometry3d[0].visible = false;
    let hidden = snapshot(&d, id(1), 1, Default::default(), &Default::default()).unwrap();
    assert_eq!(gltf::Gltf::from_slice(&glb(&hidden).unwrap()).unwrap().meshes().count(), 0);
    d.geometry3d.clear();
    let empty = snapshot(&d, id(1), 2, Default::default(), &Default::default()).unwrap();
    assert!(empty.objects.is_empty());
    gltf::Gltf::from_slice(&glb(&empty).unwrap()).unwrap();
}
#[test]
fn production_records_roundtrip_and_travel_with_the_scene() {
    let mut d = fixture();
    d.production.records.push(ProductionRecord {
        id: id(100),
        kind: ProductionKind::Scene,
        name: "Scene 4: Geyser".into(),
        parent: None,
        attributes: Default::default(),
    });
    d.production.bindings.push(ProductionBinding { object: id(21), record: id(100), role: "scenic".into() });
    let bytes = cadcraft_io::write(&d, "fixture.bcraft").unwrap();
    let reopened = cadcraft_io::read(&bytes, "fixture.bcraft").unwrap();
    assert_eq!(reopened.production, d.production);
    let scene = snapshot(&reopened, id(1), 5, Default::default(), &Default::default()).unwrap();
    assert_eq!(scene.production, d.production);
}
#[test]
fn publication_is_monotonic_and_rejects_other_projects_and_writers() {
    let path = std::env::temp_dir().join(format!("buildercraft-feed-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    let d = fixture();
    assert_eq!(publish(&path, snapshot(&d, id(1), 1, Default::default(), &Default::default()).unwrap()).unwrap(), 1);
    let previous = std::fs::read(path.join("snapshot.json")).unwrap();
    assert!(publish(&path, snapshot(&d, id(2), 1, Default::default(), &Default::default()).unwrap()).is_err());
    assert_eq!(std::fs::read(path.join("snapshot.json")).unwrap(), previous);
    std::fs::write(path.join("writer.lock"), b"occupied").unwrap();
    assert!(publish(&path, snapshot(&d, id(1), 2, Default::default(), &Default::default()).unwrap()).is_err());
    std::fs::remove_file(path.join("writer.lock")).unwrap();
    assert_eq!(publish(&path, snapshot(&d, id(1), 2, Default::default(), &Default::default()).unwrap()).unwrap(), 2);
    let current: Snapshot = serde_json::from_slice(&std::fs::read(path.join("snapshot.json")).unwrap()).unwrap();
    assert_eq!(current.sequence, "2");
    assert!(path.join(&current.glb_file).exists());
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn external_snapshot_with_cycle_or_nonfinite_coordinate_is_rejected() {
    let mut scene = snapshot(&fixture(), id(1), 1, Default::default(), &Default::default()).unwrap();
    scene.objects[0].parent = Some(scene.objects[0].id);
    assert!(glb(&scene).is_err());
    scene.objects[0].parent = None;
    scene.objects[0].positions[0][0] = f64::NAN;
    assert!(glb(&scene).is_err());
}
