//! Run with: cargo run -p buildercraft-kernel --example mesh_scene
//! Headless proof of editable mesh geometry, revision-aware repair and undo.

use buildercraft_kernel::*;
use cadcraft_geom::Vec3;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let polygon = PolygonMesh {
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
    };
    let report = polygon_mesh_boundary_loops(&polygon)?;
    let inner = report.closed_loops.iter()
        .position(|entry| entry.vertices.iter().all(|&id| id >= 4))
        .ok_or("inner boundary not found")?;
    let budget = GeometryBudget::new(1024 * 1024, 100);
    let lease = budget.retain(GeometryData::PolygonMesh(polygon))?;
    let cancel = Cancellation::default();
    let mut scene = Scene::new(
        Id::new(1)?,
        Frame { unit: LengthUnit::Metre, axes: Axes::RightHandedZUp },
        budget.clone(),
        16,
    );
    scene.apply(
        0,
        vec![SceneCommand::Insert(SceneObject {
            id: Id::new(2)?,
            name: "Worldwright mesh patch example".into(),
            layer: "Scenery".into(),
            parent: None,
            visible: true,
            geometry: Some(lease),
        })],
        &cancel,
    )?;
    let undo = scene.snapshot();
    println!("Before: {}", serde_json::to_string(&scene.manifest())?);
    scene.apply(
        1,
        vec![SceneCommand::EditPolygon(
            Id::new(2)?,
            PolygonSceneEdit::FillPlanarHole {
                selected_revision: 1,
                loop_index: u32::try_from(inner)?,
            },
        )],
        &cancel,
    )?;
    let after = scene.object(Id::new(2)?).ok_or("mesh object missing")?;
    let updated = after.geometry.as_ref().ok_or("mesh geometry missing")?;
    let GeometryData::PolygonMesh(mesh) = updated.data() else {
        return Err("edited geometry is not polygon mesh".into());
    };
    let loops = polygon_mesh_boundary_loops(mesh)?;
    assert_eq!(loops.closed_loops.len(), 1);
    println!("After patch: {}", serde_json::to_string(&scene.manifest())?);
    scene.restore(2, &undo, &cancel)?;
    println!("After undo: {}", serde_json::to_string(&scene.manifest())?);
    drop(undo);
    drop(scene);
    assert_eq!(budget.used(), 0);
    Ok(())
}
