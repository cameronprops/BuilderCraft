//! Original synthetic floor/wall massing fixture; no customer geometry.
use cadcraft_doc::{
    Drawing,
    organization::{GeometryObject, Shape},
};
use cadcraft_geom::{Vec3, nurbs3d};
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).ok_or("provide output.bcraft")?;
    let mut d = Drawing::new_metric();
    for (id, name, rows) in [
        (20, "Massing floor", [[Vec3::ZERO, Vec3::new(6000., 0., 0.)], [Vec3::new(0., 4000., 0.), Vec3::new(6000., 4000., 0.)]]),
        (21, "Massing wall", [[Vec3::ZERO, Vec3::new(6000., 0., 0.)], [Vec3::new(0., 0., 3000.), Vec3::new(6000., 0., 3000.)]]),
    ] {
        let rows = rows
            .into_iter()
            .map(|control| nurbs3d::Curve { degree: 1, control: control.to_vec(), weights: vec![1., 1.], knots: nurbs3d::uniform_knots(2, 1) })
            .collect();
        d.geometry3d.push(GeometryObject {
            id,
            name: name.into(),
            layer: "0".into(),
            visible: true,
            shape: Shape::Surface(nurbs3d::Surface { rows, degree_v: 1, knots_v: nurbs3d::uniform_knots(2, 1) }.into()),
        });
    }
    std::fs::write(output, cadcraft_io::write(&d, "fixture.bcraft")?)?;
    Ok(())
}
