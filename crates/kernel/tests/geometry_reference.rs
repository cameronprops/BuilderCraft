//! Project- and revision-bound references for the shared scene.
use buildercraft_kernel::*;
use cadcraft_geom::Vec3;

fn id(value: u128) -> Id { Id::new(value).unwrap() }
fn budget() -> std::sync::Arc<GeometryBudget> {
    GeometryBudget::new(1024 * 1024, 4096)
}
fn scene_with_polyline(project: Id, object: Id) -> Scene {
    let budget = budget();
    let lease = budget.retain(GeometryData::Polyline(vec![
        Vec3::ZERO, Vec3::new(10., 0., 0.),
    ])).unwrap();
    let mut scene = Scene::new(project, Frame {
        unit: LengthUnit::Millimetre,
        axes: Axes::RightHandedZUp,
    }, budget, 128);
    assert_eq!(scene.apply(0, vec![SceneCommand::Insert(SceneObject {
        id: object,
        name: "Reference line".into(),
        layer: "Geometry".into(),
        parent: None,
        visible: true,
        geometry: Some(lease),
    })], &Cancellation::default()), Ok(1));
    scene
}
#[test]
fn captured_reference_resolves_to_shared_immutable_storage() {
    let scene = scene_with_polyline(id(1), id(2));
    let reference = scene.capture_geometry_reference(id(2)).unwrap();
    assert_eq!(reference.project_id, id(1));
    assert_eq!(reference.object_id, id(2));
    assert_eq!(reference.source_revision, 1);
    assert_eq!(reference.kind, GeometryKind::Polyline);
    let resolved = scene.resolve_geometry_reference(&reference).unwrap();
    let original = scene.object(id(2)).unwrap().geometry.as_ref().unwrap();
    assert!(resolved.shares_storage(original));
    assert!(matches!(resolved.data(), GeometryData::Polyline(_)));
}
#[test]
fn any_scene_mutation_invalidates_previous_reference_conservatively() {
    let mut scene = scene_with_polyline(id(1), id(2));
    let previous = scene.capture_geometry_reference(id(2)).unwrap();
    assert_eq!(scene.apply(1, vec![SceneCommand::Rename(id(2), "New name".into())],
        &Cancellation::default()), Ok(2));
    assert_eq!(scene.resolve_geometry_reference(&previous),
        Err(KernelError::Conflict{expected:1,actual:2}));
    let current = scene.capture_geometry_reference(id(2)).unwrap();
    assert_eq!(scene.resolve_geometry_reference(&current).unwrap().data().kind(),"polyline");
}
#[test]
fn foreign_projects_and_wrong_geometry_kinds_are_rejected() {
    let scene = scene_with_polyline(id(1), id(2));
    let foreign = scene_with_polyline(id(3), id(2));
    let reference = scene.capture_geometry_reference(id(2)).unwrap();
    assert!(matches!(foreign.resolve_geometry_reference(&reference),
        Err(KernelError::Invalid("foreign geometry project"))));
    let mut incorrect = reference;
    incorrect.kind = GeometryKind::PolygonMesh;
    assert!(matches!(scene.resolve_geometry_reference(&incorrect),
        Err(KernelError::Invalid("geometry representation changed"))));
}
#[test]
fn serialized_handle_is_typed_and_contains_no_geometry_copy() {
    let scene = scene_with_polyline(id(1), id(2));
    let reference = scene.capture_geometry_reference(id(2)).unwrap();
    let json = serde_json::to_string(&reference).unwrap();
    let decoded: GeometryReference = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, reference);
    assert!(json.contains("polyline"));
    assert!(!json.contains("vertices"));
    assert!(!json.contains("control"));
    let mut altered: serde_json::Value = serde_json::from_str(&json).unwrap();
    altered["unexpected"] = serde_json::json!(1);
    assert!(serde_json::from_value::<GeometryReference>(altered).is_err());
}
#[test]
fn objects_without_geometry_cannot_be_captured_as_geometry_refs() {
    let mut scene = scene_with_polyline(id(1), id(2));
    assert!(matches!(scene.capture_geometry_reference(id(3)), Err(KernelError::Object)));
    scene.apply(1, vec![SceneCommand::Insert(SceneObject {
        id:id(3), name:"Group".into(), layer:"A".into(),
        parent:None, visible:true, geometry:None,
    })], &Cancellation::default()).unwrap();
    assert!(matches!(scene.capture_geometry_reference(id(3)),
        Err(KernelError::Invalid("referenced object has no geometry"))));
}
