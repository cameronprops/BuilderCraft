//! End-to-end regression for the shared G1 blend preflight.
//! The Wolfram-derived case is mathematically G1 at both endpoints but has
//! an interior zero velocity, so it must not be published as usable geometry.
use buildercraft_kernel::{KernelError, blend_nurbs_curves, cubic_bezier_stationary_point};
use cadcraft_geom::{Vec3, nurbs3d::Curve};

fn line(start: Vec3, end: Vec3) -> Curve {
    Curve::from_control(vec![start, end], 1).expect("valid source line")
}

#[test]
fn wolfram_reversal_case_rejects_unsafe_g1_and_preserves_sources() {
    let golden_parameter = (5.0_f64.sqrt() - 1.0) / 2.0;
    for scale in [0.01, 1.0, 100.0] {
        let a = line(Vec3::ZERO, Vec3::new(scale, 0.0, 0.0));
        let b = line(Vec3::new(1.5 * scale, 0.0, 0.0), Vec3::new(0.5 * scale, 0.0, 0.0));
        let original = (a.clone(), b.clone());

        let control = [
            Vec3::new(scale, 0.0, 0.0),
            Vec3::new(2.0 * scale, 0.0, 0.0),
            Vec3::new(2.5 * scale, 0.0, 0.0),
            Vec3::new(1.5 * scale, 0.0, 0.0),
        ];
        let hit = cubic_bezier_stationary_point(control).expect("Wolfram witness must be detected");
        assert!((hit.parameter - golden_parameter).abs() < 1e-12);
        let result = blend_nurbs_curves(&a, &b, 1, 2.0, 2.0, 0, 0);
        assert!(
            matches!(result, Err(KernelError::Invalid(message)) if message.contains("interior stationary point")),
            "stationary G1 curve must fail with an explicit diagnostic"
        );
        assert_eq!((a.clone(), b.clone()), original, "failed blend cannot mutate either source");

        let g0 = blend_nurbs_curves(&a, &b, 0, 2.0, 2.0, 0, 0).expect("G0 connector is regular");
        assert_eq!(g0.degree, 1);
        assert!(g0.valid());
    }
}

#[test]
fn ordinary_g1_curves_still_construct() {
    let a = line(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    let b = line(Vec3::new(1.5, 0.0, 0.0), Vec3::new(2.5, 0.0, 0.0));
    let result = blend_nurbs_curves(&a, &b, 1, 0.25, 0.25, 0, 0).expect("regular cubic");
    assert!(result.valid());
    assert_eq!(result.degree, 3);
    assert!(cubic_bezier_stationary_point([
        result.control[0],
        result.control[1],
        result.control[2],
        result.control[3],
    ])
    .is_none());
}

#[test]
fn axis_extremum_without_full_velocity_zero_is_not_rejected() {
    let a = line(Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0));
    let b = line(Vec3::new(1.5, 1.0, 0.0), Vec3::new(0.5, 2.0, 0.0));
    let result = blend_nurbs_curves(&a, &b, 1, 2.0, 2.0, 0, 0).expect("moving along Y at the X extremum");
    assert!(result.valid());
}
