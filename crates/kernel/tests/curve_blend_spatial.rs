//! Rigid-frame and uniform-unit invariance for the single shared NURBS blend
//! solver. These tests exercise the kernel rather than reproducing its math.
use buildercraft_kernel::blend_nurbs_curves;
use cadcraft_geom::{Vec3, nurbs3d::Curve};

fn rails() -> (Curve, Curve) {
    let a = Curve::from_control(vec![Vec3::new(-2.0, 0.25, 1.0), Vec3::new(-0.5, 1.75, 2.0), Vec3::new(1.25, 2.25, 0.5)], 2).unwrap();
    let b = Curve::from_control(vec![Vec3::new(3.5, 3.0, -1.5), Vec3::new(5.0, 3.75, 0.25), Vec3::new(6.0, 2.5, 1.5)], 2).unwrap();
    (a, b)
}

// (x,y,z) -> (y,z,x) is a proper orthogonal rotation (determinant +1).
// Translation and uniform scaling then exercise non-origin model coordinates
// and CAD unit changes without conflating them with non-uniform deformation.
fn transform(p: Vec3, scale: f64) -> Vec3 {
    Vec3::new(p.y * scale + 19.0, p.z * scale - 7.0, p.x * scale + 3.0)
}
fn rotate(p: Vec3) -> Vec3 {
    Vec3::new(p.y, p.z, p.x)
}
fn transformed_curve(source: &Curve, scale: f64) -> Curve {
    let mut out = source.clone();
    for p in &mut out.control {
        *p = transform(*p, scale);
    }
    out
}
fn near(actual: Vec3, expected: Vec3, tolerance: f64) {
    assert!((actual - expected).len() <= tolerance, "expected {expected:?}, got {actual:?}, tolerance {tolerance}");
}

#[test]
fn all_blend_orders_are_equivariant_under_3d_rigid_frame_and_unit_changes() {
    let (a, b) = rails();
    let original = (a.clone(), b.clone());
    for continuity in 0..=2 {
        for reverse_a in 0..=1 {
            for reverse_b in 0..=1 {
                let reference = blend_nurbs_curves(&a, &b, continuity, 0.3, 0.45, reverse_a, reverse_b).unwrap();
                for scale in [0.01, 1.0, 100.0] {
                    let ta = transformed_curve(&a, scale);
                    let tb = transformed_curve(&b, scale);
                    let result = blend_nurbs_curves(&ta, &tb, continuity, 0.3, 0.45, reverse_a, reverse_b).unwrap();
                    assert_eq!(result.degree, reference.degree);
                    assert_eq!(result.control.len(), reference.control.len());
                    for (actual, expected) in result.control.iter().zip(&reference.control) {
                        near(*actual, transform(*expected, scale), 1e-8 * scale.max(1.0));
                    }
                    for u in [0.0, 0.125, 0.5, 0.875, 1.0] {
                        near(result.evaluate(u).unwrap(), transform(reference.evaluate(u).unwrap(), scale), 1e-8 * scale.max(1.0));
                    }
                    if continuity >= 1 {
                        for u in [0.0, 1.0] {
                            near(result.tangent(u).unwrap(), rotate(reference.tangent(u).unwrap()), 1e-8);
                        }
                    }
                    if continuity == 2 {
                        for u in [0.0, 1.0] {
                            near(result.curvature_vector(u).unwrap() * scale, rotate(reference.curvature_vector(u).unwrap()), 1e-7);
                        }
                    }
                }
            }
        }
    }
    assert_eq!((a, b), original, "blend must preserve both source curves");
}
