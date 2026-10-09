use buildercraft_kernel::*;
use cadcraft_geom::{
    Vec3,
    nurbs3d::{Curve, Surface, uniform_knots},
};
fn arch() -> Curve {
    Curve {
        degree: 2,
        control: vec![Vec3::new(0., 0., 0.), Vec3::new(2., 8., 0.), Vec3::new(4., 0., 0.)],
        weights: vec![1.; 3],
        knots: uniform_knots(3, 2),
    }
}
fn center(s: &ExactShape) -> Vec3 {
    exact_bounds_center(s, 1e-7, &Cancellation::default(), &mut 200_000).unwrap()
}
#[test]
fn arch_uses_curve_bounds_not_control_cage_and_preserves_shape() {
    let c = arch();
    let shape = ExactShape::Curve(c.clone().into());
    assert!((center(&shape) - Vec3::new(2., 2., 0.)).len() < 1e-6);
    for (mode, delta) in [
        (SpacingMode::ThreeD, Vec3::new(2., 2., 0.)),
        (SpacingMode::OneD { axis: [0., 1., 0.] }, Vec3::new(0., 2., 0.)),
        (SpacingMode::TwoD { normal: [1., 0., 0.] }, Vec3::new(0., 2., 0.)),
    ] {
        let out = transform_exact(
            &shape,
            &Transform::ScalePositions { origin: [0.; 3], factor: 2., mode, tolerance: 1e-7 },
            &Cancellation::default(),
            1_000_000,
        )
        .unwrap();
        let ExactShape::Curve(out) = out else { panic!() };
        assert_eq!(out.knots, c.knots);
        assert_eq!(out.weights, c.weights);
        for (a, b) in out.control.iter().zip(&c.control) {
            assert!((*a - *b - delta).len() < 1e-6);
        }
    }
}
#[test]
fn rational_multispan_and_tensor_surface_bounds() {
    let mut c = arch();
    c.weights[1] = 0.25;
    assert!((center(&ExactShape::Curve(c.clone().into())).y - 0.8).abs() < 1e-6);
    let mut upper = c.clone();
    for p in &mut upper.control {
        p.z = 6.;
    }
    let s = Surface { rows: vec![c, upper], degree_v: 1, knots_v: uniform_knots(2, 1) };
    assert!((center(&ExactShape::Surface(s.into())) - Vec3::new(2., 0.8, 3.)).len() < 1e-6);
    let mut c = arch();
    c.control.push(Vec3::new(6., 0., 0.));
    c.weights.push(1.);
    c.knots = uniform_knots(4, 2);
    let mid = center(&ExactShape::Curve(c.into()));
    assert!((mid - Vec3::new(3., 8. / 3., 0.)).len() < 1e-6);
}
#[test]
fn hostile_budget_tolerance_and_cancel_are_errors() {
    let shape = ExactShape::Curve(arch().into());
    assert!(exact_bounds_center(&shape, 1e-6, &Cancellation::default(), &mut 0).is_err());
    for tol in [0., -1., f64::NAN] {
        assert!(exact_bounds_center(&shape, tol, &Cancellation::default(), &mut 200_000).is_err());
    }
    let cancel = Cancellation::default();
    cancel.cancel();
    assert_eq!(exact_bounds_center(&shape, 1e-6, &cancel, &mut 200_000), Err(KernelError::Cancelled));
}

#[test]
fn repeated_knots_zero_factor_and_nonzero_origin() {
    let c = Curve {
        degree: 2,
        control: vec![Vec3::new(0., 0., 0.), Vec3::new(2., 8., 0.), Vec3::new(4., 0., 0.), Vec3::new(6., -8., 0.), Vec3::new(8., 0., 0.)],
        weights: vec![1.; 5],
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
    };
    let shape = ExactShape::Curve(c.into());
    assert!((center(&shape) - Vec3::new(4., 0., 0.)).len() < 1e-6);
    let result = transform_exact(
        &shape,
        &Transform::ScalePositions { origin: [7., 8., 9.], factor: 0., mode: SpacingMode::ThreeD, tolerance: 1e-7 },
        &Cancellation::default(),
        1_000_000,
    )
    .unwrap();
    assert!((center(&result) - Vec3::new(7., 8., 9.)).len() < 1e-6);
}

#[test]
fn interior_surface_peak_is_bounded_in_both_parameters() {
    let rows = (0..3)
        .map(|j| Curve {
            degree: 2,
            control: (0..3).map(|i| Vec3::new(i as f64 * 2., j as f64 * 2., if i == 1 && j == 1 { 8. } else { 0. })).collect(),
            weights: vec![1.; 3],
            knots: uniform_knots(3, 2),
        })
        .collect();
    let shape = ExactShape::Surface(Surface { rows, degree_v: 2, knots_v: uniform_knots(3, 2) }.into());
    assert!((center(&shape) - Vec3::new(2., 2., 1.)).len() < 1e-6);
}
