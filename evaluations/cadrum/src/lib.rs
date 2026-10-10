//! Isolation and acceptance tests for an OCCT-backed exact BRep candidate.
//! NOT a production kernel, installed backend, or proof of Rhino/STEP parity.

#![forbid(unsafe_code)]

pub mod adapter;

#[cfg(test)]
mod tests {
    use cadrum::{DVec3, Solid};

    fn near(label: &str, actual: f64, expected: f64, tolerance: f64) {
        assert!(
            actual.is_finite() && (actual - expected).abs() <= tolerance,
            "{label}: expected {expected} +/- {tolerance}, observed {actual}"
        );
    }

    fn face_count(solid: &Solid) -> usize {
        solid.iter_face().count()
    }

    #[test]
    fn cuboid_has_exact_brep_topology_and_metric_volume() {
        let solid = Solid::cube(DVec3::ZERO, DVec3::new(2.0, 3.0, 4.0));
        assert_eq!(face_count(&solid), 6);
        assert_eq!(solid.iter_edge().count(), 12);
        near("rectangular cuboid", solid.volume(), 24.0, 1e-9);
    }

    #[test]
    fn transverse_box_boolean_volume_and_topology() -> Result<(), cadrum::Error> {
        // A: [0,2]^3, B: [1,3]^3, overlap = unit cube.
        let a = Solid::cube(DVec3::ZERO, DVec3::splat(2.0));
        let b = Solid::cube(DVec3::ONE, DVec3::splat(3.0));
        let common = (&a * &b).build()?;
        let fused = (&a + &b).build()?;
        let difference = (&a - &b).build()?;
        near("intersection", common.volume(), 1.0, 1e-8);
        near("union", fused.volume(), 15.0, 1e-8);
        near("difference", difference.volume(), 7.0, 1e-8);
        for shape in [&common, &fused, &difference] {
            assert!(face_count(shape) >= 6, "boolean yielded incomplete boundary");
            assert!(shape.iter_edge().count() >= 12);
        }
        Ok(())
    }

    #[test]
    fn identical_operand_boolean_is_idempotent() -> Result<(), Box<dyn std::error::Error>> {
        // Regression for the native CellsBuilder error with repeated operands.
        // The WorldWright adapter must prove identity and preserve boolean
        // semantics without calling the failing foreign kernel in this case.
        use crate::adapter::{AbsoluteTolerance, CadrumBrepCandidate, ExactBoolean};
        let backend = CadrumBrepCandidate::new(AbsoluteTolerance::new(1e-7)?);
        let a = backend.box_from_corners(DVec3::ZERO, DVec3::splat(2.0))?;
        for op in [ExactBoolean::Union, ExactBoolean::Intersection] {
            let pieces = backend.boolean(op, &a, &a)?;
            assert_eq!(pieces.len(), 1);
            near("same solid volume", pieces[0].statistics().volume, 8.0, 1e-8);
        }
        let pieces = backend.boolean(ExactBoolean::Difference, &a, &a)?;
        assert!(pieces.is_empty(), "identical difference unexpectedly has a solid");
        Ok(())
    }

    #[test]
    fn planar_tangency_has_no_phantom_common_volume() -> Result<(), cadrum::Error> {
        let left = Solid::cube(DVec3::ZERO, DVec3::splat(2.0));
        let right = Solid::cube(DVec3::new(2.0, 0.0, 0.0), DVec3::new(4.0, 2.0, 2.0));
        near("face-contact union", (&left + &right).build()?.volume(), 16.0, 1e-8);
        // A 2D shared face has zero common 3D volume. Empty solids may be reported
        // as an empty vector or an explicit NotOne(0), never a fabricated solid.
        let pieces = (&left * &right).build_vec()?;
        for part in pieces {
            near("tangent common", part.volume(), 0.0, 1e-8);
        }
        Ok(())
    }

    #[test]
    fn closed_curved_primitive_metrics() {
        let sphere = Solid::sphere(2.0);
        let cylinder = Solid::cylinder(1.5, DVec3::Z * 5.0);
        near("sphere", sphere.volume(), 4.0 / 3.0 * std::f64::consts::PI * 8.0, 1e-7);
        near("cylinder", cylinder.volume(), std::f64::consts::PI * 1.5_f64.powi(2) * 5.0, 1e-7);
        assert!(face_count(&sphere) >= 1);
        assert!(face_count(&cylinder) >= 3);
    }

    #[test]
    fn through_hole_boolean_has_trimmed_solid_and_expected_volume() -> Result<(), cadrum::Error> {
        // Drill a circular through-hole through a cuboid. This specifically
        // exercises an interior trim wire rather than only planar overlap.
        let stock = Solid::cube(DVec3::new(-5.0, -5.0, 0.0), DVec3::new(5.0, 5.0, 10.0));
        let drill = Solid::cylinder(2.0, DVec3::Z * 14.0)
            .translate(DVec3::Z * -2.0);
        let drilled = (&stock - &drill).build()?;
        let expected = 1000.0 - std::f64::consts::PI * 4.0 * 10.0;
        near("drilled stock volume", drilled.volume(), expected, 1e-6);
        assert!(face_count(&drilled) >= 7, "hole sidewall/trim topology missing");
        Ok(())
    }

    #[test]
    fn step_roundtrip_preserves_exact_closed_solid_volume() -> Result<(), cadrum::Error> {
        let original = Solid::cube(DVec3::new(-2.0, 0.0, 0.0), DVec3::new(3.0, 4.0, 6.0));
        let mut data = Vec::new();
        Solid::write_step([&original], &mut data)?;
        assert!(data.len() > 100, "empty or implausible STEP output");
        let decoded = Solid::read_step(&mut std::io::Cursor::new(data))?;
        assert_eq!(decoded.len(), 1);
        assert_eq!(face_count(&decoded[0]), 6);
        near("STEP volume roundtrip", decoded[0].volume(), original.volume(), 1e-7);
        Ok(())
    }

    #[test]
    fn native_brep_binary_roundtrip_preserves_volume() -> Result<(), cadrum::Error> {
        let input = Solid::cube(DVec3::new(-4.0, 2.0, 0.5), DVec3::new(1.0, 4.0, 3.5));
        let mut data = Vec::new();
        Solid::write_brep([&input], &mut data)?;
        assert!(data.len() > 64, "empty or implausible BRep persistence");
        let decoded = Solid::read_brep(&mut std::io::Cursor::new(data))?;
        assert_eq!(decoded.len(), 1);
        assert_eq!(face_count(&decoded[0]), 6);
        near("BRep roundtrip", decoded[0].volume(), input.volume(), 1e-9);
        Ok(())
    }
}
