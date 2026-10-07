use buildercraft_kernel::*;
use cadcraft_geom::{Vec3, nurbs3d};
use std::sync::{Arc, Barrier};

fn id(n: u128) -> Id {
    Id::new(n).unwrap()
}
fn frame() -> Frame {
    Frame { unit: LengthUnit::Metre, axes: Axes::RightHandedZUp }
}
fn point(x: f64) -> GeometryData {
    GeometryData::PointCloud(vec![Vec3::new(x, 0., 0.)])
}
fn object(id: Id, geometry: Option<GeometryLease>) -> SceneObject {
    SceneObject { id, name: "Scenic door".into(), layer: "Scenery".into(), parent: None, visible: true, geometry }
}

#[test]
fn identities_survive_json_without_integer_precision_loss() {
    let value = id(u128::MAX);
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "\"ffffffffffffffffffffffffffffffff\"");
    assert_eq!(serde_json::from_str::<Id>(&json).unwrap(), value);
    assert!(serde_json::from_str::<Id>("\"00000000000000000000000000000000\"").is_err());
    assert!(serde_json::from_str::<Id>("123").is_err());
}
#[test]
fn unit_and_axis_conversion_roundtrips() {
    for axes in [Axes::RightHandedZUp, Axes::LeftHandedZUp, Axes::RightHandedYUp] {
        let target = Frame { unit: LengthUnit::Millimetre, axes };
        let p = Vec3::new(1., 2., 3.);
        let converted = frame().convert_point(target, p).unwrap();
        let back = target.convert_point(frame(), converted).unwrap();
        assert!((back - p).len() < 1e-12);
    }
    let feet = Frame { unit: LengthUnit::Foot, axes: Axes::RightHandedZUp };
    assert!((feet.convert_point(frame(), Vec3::new(1., 0., 0.)).unwrap().x - 0.3048).abs() < 1e-12);
    assert!(frame().convert_point(frame(), Vec3::new(f64::NAN, 0., 0.)).is_err());
}
#[test]
fn shared_leases_charge_once_and_release_on_last_drop() {
    let budget = GeometryBudget::new(10000, 100);
    let lease = budget.retain(point(1.)).unwrap();
    let bytes = budget.used();
    let copy = lease.clone();
    assert!(lease.shares_storage(&copy));
    assert_eq!(budget.used(), bytes);
    drop(lease);
    assert_eq!(budget.used(), bytes);
    drop(copy);
    assert_eq!(budget.used(), 0);
}
#[test]
fn concurrent_retains_cannot_overcommit_the_geometry_budget() {
    let sizing = GeometryBudget::new(10000, 10);
    let bytes = sizing.retain(point(0.)).unwrap().estimated_bytes();
    let budget = GeometryBudget::new(bytes, 10);
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let b = budget.clone();
            let wait = barrier.clone();
            std::thread::spawn(move || {
                wait.wait();
                let lease = b.retain(point(1.));
                wait.wait();
                lease
            })
        })
        .collect();
    barrier.wait();
    barrier.wait();
    assert_eq!(budget.used(), bytes);
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    drop(results);
    assert_eq!(budget.used(), 0);
}
#[test]
fn bad_mesh_or_point_cloud_does_not_consume_budget() {
    let b = GeometryBudget::new(10000, 10);
    assert!(b.retain(GeometryData::Mesh(TriangleMesh { vertices: vec![Vec3::ZERO], triangles: vec![[0, 1, 2]] })).is_err());
    assert!(b.retain(GeometryData::PointCloud(vec![Vec3::new(f64::INFINITY, 0., 0.)])).is_err());
    assert_eq!(b.used(), 0);
}
#[test]
fn transaction_failure_preserves_scene_and_revision() {
    let mut scene = Scene::new(id(1), frame(), GeometryBudget::new(10000, 10), 10);
    scene.apply(0, vec![SceneCommand::Insert(object(id(2), None))], &Cancellation::default()).unwrap();
    let failed = scene.apply(1, vec![SceneCommand::Rename(id(2), "Changed".into()), SceneCommand::Remove(id(3))], &Cancellation::default());
    assert_eq!(failed.unwrap_err(), KernelError::Object);
    assert_eq!(scene.object(id(2)).unwrap().name, "Scenic door");
    assert_eq!(scene.revision(), 1);
    assert!(matches!(scene.apply(0, vec![], &Cancellation::default()), Err(KernelError::Conflict { .. })));
}
#[test]
fn hierarchy_cycles_and_orphaning_fail_atomically() {
    let mut scene = Scene::new(id(1), frame(), GeometryBudget::new(10000, 10), 10);
    let mut parent = object(id(2), None);
    parent.parent = Some(id(3));
    let mut child = object(id(3), None);
    child.parent = Some(id(2));
    assert!(scene.apply(0, vec![SceneCommand::Insert(parent), SceneCommand::Insert(child.clone())], &Cancellation::default()).is_err());
    assert!(scene.manifest().objects.is_empty());
    scene.apply(0, vec![SceneCommand::Insert(object(id(2), None)), SceneCommand::Insert(child)], &Cancellation::default()).unwrap();
    assert!(scene.apply(1, vec![SceneCommand::Remove(id(2))], &Cancellation::default()).is_err());
    assert_eq!(scene.manifest().objects.len(), 2);
}
#[test]
fn snapshot_restore_shares_geometry_and_releases_replaced_buffers() {
    let b = GeometryBudget::new(10000, 10);
    let mut scene = Scene::new(id(1), frame(), b.clone(), 10);
    scene.apply(0, vec![SceneCommand::Insert(object(id(2), Some(b.retain(point(1.)).unwrap())))], &Cancellation::default()).unwrap();
    let before = b.used();
    let snapshot = scene.snapshot();
    assert_eq!(b.used(), before);
    scene.apply(1, vec![SceneCommand::SetGeometry(id(2), b.retain(point(2.)).unwrap())], &Cancellation::default()).unwrap();
    assert_eq!(b.used(), 2 * before);
    scene.restore(2, &snapshot, &Cancellation::default()).unwrap();
    assert_eq!(b.used(), before);
    assert_eq!(scene.revision(), 3);
    drop(snapshot);
    drop(scene);
    assert_eq!(b.used(), 0);
}
#[test]
fn cancellation_and_foreign_budget_never_publish() {
    let b = GeometryBudget::new(10000, 10);
    let mut scene = Scene::new(id(1), frame(), b, 10);
    let token = Cancellation::default();
    token.cancel();
    assert_eq!(scene.apply(0, vec![SceneCommand::Insert(object(id(2), None))], &token).unwrap_err(), KernelError::Cancelled);
    let foreign = GeometryBudget::new(10000, 10).retain(point(0.)).unwrap();
    assert!(scene.apply(0, vec![SceneCommand::Insert(object(id(2), Some(foreign)))], &Cancellation::default()).is_err());
    assert_eq!(scene.revision(), 0);
}
#[test]
fn exact_shape_clone_shares_controls_but_edit_preserves_old_shape() {
    let curve = nurbs3d::Curve { degree: 1, control: vec![Vec3::ZERO, Vec3::new(1., 0., 0.)], weights: vec![1., 1.], knots: vec![0., 0., 1., 1.] };
    let original = ExactShape::Curve(Arc::new(curve));
    let mut edited = original.clone();
    let (ExactShape::Curve(a), ExactShape::Curve(b)) = (&original, &edited) else { panic!() };
    assert!(Arc::ptr_eq(a, b));
    let ExactShape::Curve(b) = &mut edited else { panic!() };
    Arc::make_mut(b).control[0].x = 5.;
    let ExactShape::Curve(a) = &original else { panic!() };
    assert_eq!(a.control[0].x, 0.);
    assert!(!Arc::ptr_eq(a, b));
}
#[test]
fn legacy_exact_shape_json_encoding_remains_compatible() {
    let legacy =
        r#"{"Curve":{"degree":1,"control":[{"x":0.0,"y":0.0,"z":0.0},{"x":1.0,"y":0.0,"z":0.0}],"weights":[1.0,1.0],"knots":[0.0,0.0,1.0,1.0]}}"#;
    let shape: ExactShape = serde_json::from_str(legacy).unwrap();
    assert!(shape.valid());
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&serde_json::to_string(&shape).unwrap()).unwrap(),
        serde_json::from_str::<serde_json::Value>(legacy).unwrap()
    );
}
