use buildercraft_kernel::*;
use cadcraft_geom::{Vec3, nurbs3d};
fn curve() -> ExactShape {
    ExactShape::Curve(
        nurbs3d::Curve {
            degree: 2,
            control: vec![Vec3::new(1., 0., 0.), Vec3::new(1., 1., 0.), Vec3::new(0., 1., 0.)],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            knots: nurbs3d::uniform_knots(3, 2),
        }
        .into(),
    )
}
#[test]
fn affine_edits_preserve_exact_rational_evaluation_and_source() {
    let original = curve();
    let source = original.clone();
    for operation in [
        Transform::Move { delta: [4., -3., 2.] },
        Transform::Rotate { origin: [2., 3., 4.], axis: [1., 2., 3.], angle_degrees: 37. },
        Transform::Scale { origin: [2., 3., 4.], factor: 2.5 },
        Transform::Scale1d { origin: [2., 3., 4.], axis: [1., 2., 3.], factor: 2.5 },
        Transform::Scale2d { origin: [2., 3., 4.], normal: [1., 2., 3.], factor: 2.5 },
        Transform::ScaleNu { origin: [2., 3., 4.], factors: [2., 3., 4.] },
        Transform::ScaleByPlane { origin: [2., 3., 4.], x_axis: [1., 1., 0.], y_axis: [-1., 1., 0.], factors: [2., 3.] },
        Transform::Shear { origin: [2., 3., 4.], direction: [1., 1., 0.], normal: [0., 0., 1.], angle_degrees: -30. },
        Transform::Mirror { origin: [2., 3., 4.], normal: [1., 2., 3.] },
    ] {
        let result = transform_exact(&original, &operation, &Cancellation::default(), 1_000_000).unwrap();
        let (ExactShape::Curve(a), ExactShape::Curve(b)) = (&original, &result) else { panic!() };
        assert_eq!(a.weights, b.weights);
        assert_eq!(a.knots, b.knots);
        assert_eq!(a.degree, b.degree);
        for t in [0., 0.2, 0.5, 0.8, 1.] {
            let expected = operation.matrix().unwrap().apply(a.evaluate(t).unwrap());
            let actual = b.evaluate(t).unwrap();
            assert!((actual - expected).len() < 1e-9);
        }
    }
    assert_eq!(original, source);
}
#[test]
fn orientation_and_off_origin_rotation() {
    let op = Transform::Rotate { origin: [1., 1., 0.], axis: [0., 0., 5.], angle_degrees: 90. };
    let p = op.matrix().unwrap().apply(Vec3::new(2., 1., 0.));
    assert!((p - Vec3::new(1., 2., 0.)).len() < 1e-12);
    let mirror = Transform::Mirror { origin: [2., 0., 0.], normal: [1., 0., 0.] };
    assert_eq!(mirror.matrix().unwrap().apply(Vec3::new(3., 4., 5.)), Vec3::new(1., 4., 5.));
}
#[test]
fn surface_transform_and_failure_preserve_sources() {
    let row = |y| nurbs3d::Curve {
        degree: 1,
        control: vec![Vec3::new(0., y, 0.), Vec3::new(2., y, 0.)],
        weights: vec![1., 1.],
        knots: nurbs3d::uniform_knots(2, 1),
    };
    let original = ExactShape::Surface(nurbs3d::Surface { rows: vec![row(0.), row(3.)], degree_v: 1, knots_v: nurbs3d::uniform_knots(2, 1) }.into());
    let operation = Transform::Move { delta: [1., 2., 3.] };
    let result = transform_exact(&original, &operation, &Cancellation::default(), 1_000_000).unwrap();
    let (ExactShape::Surface(a), ExactShape::Surface(b)) = (&original, &result) else { panic!() };
    for (u, v) in [(0., 0.), (0.25, 0.8), (1., 1.)] {
        assert!((b.evaluate(u, v).unwrap() - operation.matrix().unwrap().apply(a.evaluate(u, v).unwrap())).len() < 1e-9);
    }
    for op in [
        Transform::Scale1d { origin: [1., 2., 3.], axis: [1., 2., 3.], factor: 0. },
        Transform::Scale2d { origin: [1., 2., 3.], normal: [1., 2., 3.], factor: 2. },
        Transform::Shear { origin: [1., 2., 3.], direction: [1., 0., 0.], normal: [0., 0., 1.], angle_degrees: 45. },
        Transform::ScaleNu { origin: [1., 2., 3.], factors: [2., 3., 4.] },
        Transform::ScaleByPlane { origin: [1., 2., 3.], x_axis: [1., 1., 0.], y_axis: [-1., 1., 0.], factors: [2., 3.] },
    ] {
        let result = transform_exact(&original, &op, &Cancellation::default(), 1_000_000).unwrap();
        let ExactShape::Surface(scaled) = result else { panic!() };
        for (u, v) in [(0., 0.), (0.25, 0.8), (1., 1.)] {
            assert!((scaled.evaluate(u, v).unwrap() - op.matrix().unwrap().apply(a.evaluate(u, v).unwrap())).len() < 1e-9);
        }
        assert_eq!(scaled.knots_v, a.knots_v);
        assert_eq!(scaled.rows[0].weights, a.rows[0].weights);
    }
    let cancel = Cancellation::default();
    cancel.cancel();
    assert_eq!(transform_exact(&original, &operation, &cancel, 1_000_000), Err(KernelError::Cancelled));
    assert_eq!(transform_exact(&original, &operation, &Cancellation::default(), 0), Err(KernelError::Budget));
    assert!(transform_exact(&original, &Transform::Scale { origin: [1e12, 0., 0.], factor: 1e9 }, &Cancellation::default(), 1_000_000).is_err());
    assert_eq!(a.rows[0].control[0], Vec3::ZERO);
    assert!(Transform::Mirror { origin: [0.; 3], normal: [0.; 3] }.matrix().is_err());
    assert!(Transform::Scale { origin: [0.; 3], factor: f64::NAN }.matrix().is_err());
}

#[test]
fn directional_scale_preserves_complement_and_flattens() {
    let origin = [1., 2., 3.];
    let p = Vec3::new(4., 6., 8.);
    let one = Transform::Scale1d { origin, axis: [0., 0., 7.], factor: 2. };
    assert_eq!(one.matrix().unwrap().apply(p), Vec3::new(4., 6., 13.));
    let two = Transform::Scale2d { origin, normal: [0., 0., 7.], factor: 2. };
    assert_eq!(two.matrix().unwrap().apply(p), Vec3::new(7., 10., 8.));
    let oblique = Transform::Scale1d { origin: [0.; 3], axis: [1., 1., 0.], factor: 2. };
    assert!((oblique.matrix().unwrap().apply(Vec3::new(2., 0., 3.)) - Vec3::new(3., 1., 3.)).len() < 1e-12);
    let oblique_plane = Transform::Scale2d { origin: [0.; 3], normal: [1., 1., 0.], factor: 2. };
    assert!((oblique_plane.matrix().unwrap().apply(Vec3::new(2., 0., 3.)) - Vec3::new(3., -1., 6.)).len() < 1e-12);
    let flat = Transform::Scale1d { origin, axis: [0., 0., 1.], factor: 0. };
    assert_eq!(flat.matrix().unwrap().apply(p), Vec3::new(4., 6., 3.));
    for factor in [-1., f64::NAN, f64::INFINITY, 1e10] {
        assert!(Transform::Scale1d { origin, axis: [1., 0., 0.], factor }.matrix().is_err());
        assert!(Transform::Scale2d { origin, normal: [1., 0., 0.], factor }.matrix().is_err());
    }
    assert!(Transform::Scale2d { origin, normal: [0.; 3], factor: 2. }.matrix().is_err());
}

#[test]
fn nonuniform_scale_and_explicit_plane_have_known_results() {
    let world = Transform::ScaleNu { origin: [1., 2., 3.], factors: [2., 3., 4.] };
    assert_eq!(world.matrix().unwrap().apply(Vec3::new(4., 6., 8.)), Vec3::new(7., 14., 23.));
    let plane = Transform::ScaleByPlane { origin: [1., 2., 3.], x_axis: [2., 2., 0.], y_axis: [-4., 4., 0.], factors: [2., 3.] };
    assert!((plane.matrix().unwrap().apply(Vec3::new(3., 2., 7.)) - Vec3::new(6., 1., 7.)).len() < 1e-12);
    let tilted = Transform::ScaleByPlane { origin: [0.; 3], x_axis: [1., 0., 1.], y_axis: [0., 1., 0.], factors: [2., 3.] };
    assert!((tilted.matrix().unwrap().apply(Vec3::new(2., 3., 4.)) - Vec3::new(5., 9., 7.)).len() < 1e-12);
    let drift = Transform::ScaleByPlane { origin: [0.; 3], x_axis: [1., 0., 0.], y_axis: [1e-10, 1., 0.], factors: [2., 3.] };
    assert_eq!(drift.matrix().unwrap().apply(Vec3::new(2., 3., 4.)), Vec3::new(4., 9., 4.));
    for y_axis in [[0.; 3], [1., 1., 0.], [0., 1., 0.]] {
        assert!(Transform::ScaleByPlane { origin: [0.; 3], x_axis: [1., 1., 0.], y_axis, factors: [2., 3.] }.matrix().is_err());
    }
    for factor in [-1., f64::NAN, f64::INFINITY, 1e10] {
        assert!(Transform::ScaleNu { origin: [0.; 3], factors: [1., factor, 1.] }.matrix().is_err());
        assert!(Transform::ScaleByPlane { origin: [0.; 3], x_axis: [1., 0., 0.], y_axis: [0., 1., 0.], factors: [1., factor] }.matrix().is_err());
    }
}

#[test]
fn shear_fixes_base_plane_is_invertible_and_rejects_invalid_frames() {
    let op = |angle| Transform::Shear { origin: [1., 2., 3.], direction: [5., 0., 0.], normal: [0., 0., 2.], angle_degrees: angle };
    let matrix = op(45.).matrix().unwrap();
    let fixed = Vec3::new(8., 9., 3.);
    assert_eq!(matrix.apply(fixed), fixed);
    let p = Vec3::new(2., 4., 5.);
    assert!((matrix.apply(p) - Vec3::new(4., 4., 5.)).len() < 1e-12);
    assert!((op(-45.).matrix().unwrap().apply(matrix.apply(p)) - p).len() < 1e-12);
    let oblique = Transform::Shear { origin: [0.; 3], direction: [1., 1., 0.], normal: [1., -1., 0.], angle_degrees: 45. };
    assert!((oblique.matrix().unwrap().apply(Vec3::new(1., 0., 3.)) - Vec3::new(1.5, 0.5, 3.)).len() < 1e-12);
    for angle in [89., -89., 90., f64::NAN, f64::INFINITY] {
        assert!(op(angle).matrix().is_err());
    }
    for direction in [[0.; 3], [0., 0., 1.], [1., 0., 1.]] {
        assert!(Transform::Shear { origin: [0.; 3], direction, normal: [0., 0., 1.], angle_degrees: 45. }.matrix().is_err());
    }
    assert!(Transform::Shear { origin: [0.; 3], direction: [1., 0., 0.], normal: [0.; 3], angle_degrees: 45. }.matrix().is_err());
}
