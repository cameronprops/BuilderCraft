//! First integrated mesh edit workflow through shared Worldwright scene ownership.
use buildercraft_kernel::*;
use cadcraft_geom::Vec3;

fn id(n: u128) -> Id {
    Id::new(n).unwrap()
}
fn frame() -> Frame {
    Frame { unit: LengthUnit::Metre, axes: Axes::RightHandedZUp }
}
fn ring() -> PolygonMesh {
    PolygonMesh {
        vertices: vec![
            Vec3::new(0., 0., 0.),
            Vec3::new(4., 0., 0.),
            Vec3::new(4., 4., 0.),
            Vec3::new(0., 4., 0.),
            Vec3::new(1., 1., 0.),
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
fn polygon(scene: &Scene) -> &PolygonMesh {
    let geometry = scene.object(id(2)).unwrap().geometry.as_ref().unwrap();
    let GeometryData::PolygonMesh(mesh) = geometry.data() else {
        panic!("polygon mesh missing from retained scene");
    };
    mesh
}
fn fixture(budget: std::sync::Arc<GeometryBudget>) -> Scene {
    let lease = budget.retain(GeometryData::PolygonMesh(ring())).unwrap();
    let mut scene = Scene::new(id(1), frame(), budget, 20);
    let object =
        SceneObject { id: id(2), name: "Scenic mesh panel".into(), layer: "Rockwork".into(), parent: None, visible: true, geometry: Some(lease) };
    scene.apply(0, vec![SceneCommand::Insert(object)], &Cancellation::default()).unwrap();
    scene
}
fn inner_loop(mesh: &PolygonMesh) -> u32 {
    let report = polygon_mesh_boundary_loops(mesh).unwrap();
    u32::try_from(report.closed_loops.iter().position(|item| item.vertices.iter().all(|&index| index >= 4)).unwrap()).unwrap()
}

#[test]
fn native_quads_are_retained_with_geometry_budget_and_manifest() {
    let budget = GeometryBudget::new(1_000_000, 100);
    let scene = fixture(budget.clone());
    assert_eq!(scene.revision(), 1);
    assert_eq!(scene.manifest().objects[0].geometry_kind.as_deref(), Some("polygonMesh"));
    assert_eq!(polygon(&scene).faces.len(), 4);
    assert_eq!(budget.used(), scene.manifest().retained_geometry_bytes);
    assert!(budget.used() > 0);
    drop(scene);
    assert_eq!(budget.used(), 0);
}

#[test]
fn filling_a_hole_is_one_undoable_scene_transaction() {
    let budget = GeometryBudget::new(1_000_000, 100);
    let mut scene = fixture(budget.clone());
    let snapshot = scene.snapshot();
    let loop_index = inner_loop(polygon(&scene));
    let updated = scene
        .apply(
            1,
            vec![SceneCommand::EditPolygon(id(2), PolygonSceneEdit::FillPlanarHole { selected_revision: 1, loop_index })],
            &Cancellation::default(),
        )
        .unwrap();
    assert_eq!(updated, 2);
    assert_eq!(polygon(&scene).faces.len(), 6);
    let loops_after = polygon_mesh_boundary_loops(polygon(&scene)).unwrap();
    assert_eq!(loops_after.closed_loops.len(), 1);
    assert_eq!(scene.manifest().objects[0].geometry_kind.as_deref(), Some("polygonMesh"));
    assert!(budget.used() > 0);

    scene.restore(2, &snapshot, &Cancellation::default()).unwrap();
    assert_eq!(scene.revision(), 3);
    assert_eq!(polygon(&scene).faces.len(), 4);
    assert_eq!(polygon_mesh_boundary_loops(polygon(&scene)).unwrap().closed_loops.len(), 2);
    drop(snapshot);
    drop(scene);
    assert_eq!(budget.used(), 0);
}

#[test]
fn deletion_and_edge_triangle_are_revision_bound_and_atomic() {
    let mut scene = fixture(GeometryBudget::new(1_000_000, 100));
    scene
        .apply(
            1,
            vec![SceneCommand::EditPolygon(id(2), PolygonSceneEdit::DeleteFaces { selected_revision: 1, selected_faces: vec![0] })],
            &Cancellation::default(),
        )
        .unwrap();
    assert_eq!(scene.revision(), 2);
    assert_eq!(polygon(&scene).faces.len(), 3);

    let failed = scene.apply(
        2,
        vec![SceneCommand::EditPolygon(
            id(2),
            PolygonSceneEdit::AddTriangleFromEdge { selected_revision: 1, edge_vertices: [0, 1], point_vertex: 4 },
        )],
        &Cancellation::default(),
    );
    assert_eq!(failed.unwrap_err(), KernelError::Conflict { expected: 1, actual: 2 });
    assert_eq!(scene.revision(), 2);
    assert_eq!(polygon(&scene).faces.len(), 3);
}

#[test]
fn malformed_edit_rolls_back_prior_commands_in_same_batch() {
    let budget = GeometryBudget::new(1_000_000, 100);
    let mut scene = fixture(budget.clone());
    let bytes = budget.used();
    let failed = scene.apply(
        1,
        vec![
            SceneCommand::Rename(id(2), "Unwanted rename".into()),
            SceneCommand::EditPolygon(id(2), PolygonSceneEdit::DeleteFaces { selected_revision: 1, selected_faces: vec![999] }),
        ],
        &Cancellation::default(),
    );
    assert!(failed.is_err());
    assert_eq!(scene.object(id(2)).unwrap().name, "Scenic mesh panel");
    assert_eq!(polygon(&scene).faces.len(), 4);
    assert_eq!(scene.revision(), 1);
    assert_eq!(budget.used(), bytes);
}

#[test]
fn budget_exhaustion_rejects_edit_without_modifying_scene() {
    let sizing = GeometryBudget::new(1_000_000, 100);
    let byte_size = sizing.retain(GeometryData::PolygonMesh(ring())).unwrap().estimated_bytes();
    let budget = GeometryBudget::new(byte_size, 100);
    let mut scene = fixture(budget.clone());
    assert_eq!(budget.used(), byte_size);
    let failed = scene.apply(
        1,
        vec![SceneCommand::EditPolygon(id(2), PolygonSceneEdit::DeleteFaces { selected_revision: 1, selected_faces: vec![0] })],
        &Cancellation::default(),
    );
    assert_eq!(failed.unwrap_err(), KernelError::Budget);
    assert_eq!(scene.revision(), 1);
    assert_eq!(polygon(&scene).faces.len(), 4);
    assert_eq!(budget.used(), byte_size);
}

#[test]
fn invalid_polygon_geometry_does_not_consume_memory() {
    let budget = GeometryBudget::new(1_000_000, 100);
    let mut broken = ring();
    broken.faces[0] = PolygonFace::Triangle([0, 1, 999]);
    assert!(budget.retain(GeometryData::PolygonMesh(broken)).is_err());
    assert_eq!(budget.used(), 0);
}

#[test]
fn serialized_scene_edit_has_explicit_kind_and_revision() {
    let edit = PolygonSceneEdit::DeleteFaces { selected_revision: 18, selected_faces: vec![2, 6] };
    let json = serde_json::to_value(&edit).unwrap();
    assert_eq!(json["kind"], "delete_faces");
    assert_eq!(json["selected_revision"], 18);
    assert_eq!(serde_json::from_value::<PolygonSceneEdit>(json).unwrap(), edit);
}

#[test]
fn native_polygon_json_roundtrip_preserves_quads_and_indices() {
    let geometry = GeometryData::PolygonMesh(ring());
    let json = serde_json::to_string(&geometry).unwrap();
    let restored: GeometryData = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, geometry);
    let GeometryData::PolygonMesh(mesh) = restored else {
        panic!("polygon mesh was lost during JSON roundtrip");
    };
    assert_eq!(mesh.faces.len(), 4);
    assert!(matches!(mesh.faces[0], PolygonFace::Quad(_)));
}

#[test]
fn curvature_fill_is_one_revisioned_undoable_scene_edit() {
    let budget = GeometryBudget::new(2_000_000, 100);
    let mut scene = fixture(budget.clone());
    let snapshot = scene.snapshot();
    let source = polygon(&scene).clone();
    let loop_index = inner_loop(&source);
    let revision = scene.revision();
    let edit = PolygonSceneEdit::FillHole {
        selected_revision: revision,
        loop_index,
        mode: PolygonPatchMode::CurvatureSmooth { refinement_levels: 2, smoothing_iterations: 12, tangent_weight: 0.4, max_interior_offset: 0.5 },
    };
    let advanced_revision = scene.apply(revision, vec![SceneCommand::EditPolygon(id(2), edit.clone())], &Cancellation::default()).unwrap();
    assert_eq!(advanced_revision, revision + 1);
    assert_eq!(polygon(&scene).faces.len(), source.faces.len() + 18);
    assert_eq!(&polygon(&scene).vertices[..source.vertices.len()], &source.vertices);
    assert!(scene.apply(advanced_revision, vec![SceneCommand::EditPolygon(id(2), edit)], &Cancellation::default()).is_err());
    scene.restore(advanced_revision, &snapshot, &Cancellation::default()).unwrap();
    assert_eq!(polygon(&scene), &source);
    drop(snapshot);
    drop(scene);
    assert_eq!(budget.used(), 0);
}
