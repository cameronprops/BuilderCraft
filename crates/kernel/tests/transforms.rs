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
    let cancel = Cancellation::default();
    cancel.cancel();
    assert_eq!(transform_exact(&original, &operation, &cancel, 1_000_000), Err(KernelError::Cancelled));
    assert_eq!(transform_exact(&original, &operation, &Cancellation::default(), 0), Err(KernelError::Budget));
    assert!(transform_exact(&original, &Transform::Scale { origin: [1e12, 0., 0.], factor: 1e9 }, &Cancellation::default(), 1_000_000).is_err());
    assert_eq!(a.rows[0].control[0], Vec3::ZERO);
    assert!(Transform::Mirror { origin: [0.; 3], normal: [0.; 3] }.matrix().is_err());
    assert!(Transform::Scale { origin: [0.; 3], factor: f64::NAN }.matrix().is_err());
}
