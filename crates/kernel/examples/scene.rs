use buildercraft_kernel::*;
use cadcraft_geom::Vec3;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let budget = GeometryBudget::new(1024 * 1024, 10000);
    let geometry = budget.retain(GeometryData::Mesh(TriangleMesh {
        vertices: vec![Vec3::ZERO, Vec3::new(1., 0., 0.), Vec3::new(0., 0., 2.)],
        triangles: vec![[0, 1, 2]],
    }))?;
    let mut scene = Scene::new(Id::new(1)?, Frame { unit: LengthUnit::Metre, axes: Axes::RightHandedZUp }, budget, 100);
    scene.apply(
        0,
        vec![SceneCommand::Insert(SceneObject {
            id: Id::new(2)?,
            name: "Massing wall".into(),
            layer: "Scenery".into(),
            parent: None,
            visible: true,
            geometry: Some(geometry),
        })],
        &Cancellation::default(),
    )?;
    let snapshot = scene.snapshot();
    scene.apply(1, vec![SceneCommand::Rename(Id::new(2)?, "Edited wall".into())], &Cancellation::default())?;
    scene.restore(2, &snapshot, &Cancellation::default())?;
    println!("{}", serde_json::to_string_pretty(&scene.manifest())?);
    Ok(())
}
