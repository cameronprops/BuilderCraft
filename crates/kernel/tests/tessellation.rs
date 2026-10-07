use buildercraft_kernel::*;
use cadcraft_geom::{Vec3, nurbs3d};
fn curve() -> nurbs3d::Curve {
    nurbs3d::Curve {
        degree: 2,
        control: vec![Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.)],
        weights: vec![1., 0.5_f64.sqrt(), 1.],
        knots: nurbs3d::uniform_knots(3, 2),
    }
}
fn surface() -> ExactShape {
    let row = |y| nurbs3d::Curve {
        degree: 1,
        control: vec![Vec3::new(0., y, 0.), Vec3::new(2., y, 0.)],
        weights: vec![1., 1.],
        knots: nurbs3d::uniform_knots(2, 1),
    };
    ExactShape::Surface(nurbs3d::Surface { rows: vec![row(0.), row(3.)], degree_v: 1, knots_v: nurbs3d::uniform_knots(2, 1) }.into())
}
#[test]
fn rational_curve_preview_matches_source_and_preserves_exact_data() {
    let shape = ExactShape::Curve(curve().into());
    let original = shape.clone();
    let budget = GeometryBudget::new(100000, 1000);
    let result =
        tessellate(&shape, TessellationOptions { curve_segments: 4, ..Default::default() }, Default::default(), &budget, &Cancellation::default())
            .unwrap();
    let GeometryData::Polyline(points) = result.data() else { panic!() };
    assert_eq!(points.len(), 5);
    assert!((points[2].x - 0.5_f64.sqrt()).abs() < 1e-12);
    assert!((points[2].x * points[2].x + points[2].y * points[2].y - 1.).abs() < 1e-12);
    assert_eq!(shape, original);
    drop(result);
    assert_eq!(budget.used(), 0);
}
#[test]
fn surface_mesh_has_expected_extents_indices_and_winding() {
    let budget = GeometryBudget::new(100000, 1000);
    let result = tessellate(
        &surface(),
        TessellationOptions { surface_u: 2, surface_v: 3, ..Default::default() },
        Default::default(),
        &budget,
        &Cancellation::default(),
    )
    .unwrap();
    let GeometryData::Mesh(mesh) = result.data() else { panic!() };
    assert_eq!(mesh.vertices.len(), 12);
    assert_eq!(mesh.triangles.len(), 12);
    assert!((mesh.vertices.last().unwrap().x - 2.).abs() < 1e-9);
    assert!((mesh.vertices.last().unwrap().y - 3.).abs() < 1e-9);
    for face in &mesh.triangles {
        let a = mesh.vertices[face[0] as usize];
        let b = mesh.vertices[face[1] as usize];
        let c = mesh.vertices[face[2] as usize];
        assert!((b - a).cross(c - a).z > 0.);
    }
}
#[test]
fn hostile_resolution_and_work_limits_publish_nothing() {
    let budget = GeometryBudget::new(100000, 1000);
    for n in [0, 129, usize::MAX] {
        assert!(
            tessellate(&surface(), TessellationOptions { surface_u: n, ..Default::default() }, Default::default(), &budget, &Cancellation::default())
                .is_err()
        );
        assert_eq!(budget.used(), 0);
    }
    for limits in [
        TessellationLimits { max_buffer_bytes: 1, ..Default::default() },
        TessellationLimits { max_work_units: 1, ..Default::default() },
        TessellationLimits { max_samples: 1, ..Default::default() },
    ] {
        assert!(tessellate(&surface(), Default::default(), limits, &budget, &Cancellation::default()).is_err());
        assert_eq!(budget.used(), 0);
    }
}
#[test]
fn cancelled_or_unretained_preview_leaves_budget_unchanged() {
    let budget = GeometryBudget::new(1, 1000);
    assert!(tessellate(&surface(), Default::default(), Default::default(), &budget, &Cancellation::default()).is_err());
    assert_eq!(budget.used(), 0);
    let token = Cancellation::default();
    token.cancel();
    assert_eq!(tessellate(&surface(), Default::default(), Default::default(), &budget, &token).unwrap_err(), KernelError::Cancelled);
    assert_eq!(budget.used(), 0);
}
