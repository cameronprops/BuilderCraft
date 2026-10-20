//! Frozen symbolic Wolfram oracle data; CI does not need Wolfram installed.
//! Oracle generator: tools/wolfram/blend_oracle.wl
use buildercraft_kernel::blend_nurbs_curves;
use cadcraft_geom::{
    Vec3,
    nurbs3d::{Curve, uniform_knots},
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Oracle {
    schema_version: u32,
    generator: String,
    cases: Vec<BlendCase>,
}

#[derive(Deserialize)]
struct CurveInput {
    degree: usize,
    control: Vec<[f64; 3]>,
    weights: Vec<f64>,
}

#[derive(Deserialize)]
struct BlendCase {
    name: String,
    a: CurveInput,
    b: CurveInput,
    continuity: usize,
    strength_a: f64,
    strength_b: f64,
    reverse_a: usize,
    reverse_b: usize,
    expected: ExpectedBlend,
}

#[derive(Deserialize)]
struct ExpectedBlend {
    degree: usize,
    control: Vec<[f64; 3]>,
    stations: Vec<Station>,
    curvature: Option<CurvatureStations>,
}

#[derive(Deserialize)]
struct Station {
    t: f64,
    point: [f64; 3],
    derivative: [f64; 3],
}

#[derive(Deserialize)]
struct CurvatureStations {
    start: [f64; 3],
    mid: [f64; 3],
    end: [f64; 3],
}

fn vec3(coords: [f64; 3]) -> Vec3 {
    Vec3::new(coords[0], coords[1], coords[2])
}

fn curve(input: &CurveInput) -> Curve {
    Curve {
        degree: input.degree,
        control: input.control.iter().copied().map(vec3).collect(),
        weights: input.weights.clone(),
        knots: uniform_knots(input.control.len(), input.degree),
    }
}

fn close(actual: Vec3, reference: [f64; 3], case: &str, label: &str) {
    let expected = vec3(reference);
    let tolerance = 2e-8 * expected.len().max(1.0);
    assert!(
        actual.is_finite() && (actual - expected).len() <= tolerance,
        "{case} {label}: got {actual:?}; Wolfram expected {expected:?}; tolerance {tolerance}"
    );
}

#[test]
fn exact_wolfram_blend_reference_positions_derivatives_and_curvature() {
    let oracle: Oracle =
        serde_json::from_str(include_str!("fixtures/wolfram_blend_oracle.json")).expect("versioned Wolfram reference must remain valid JSON");
    assert_eq!(oracle.schema_version, 1);
    assert!(oracle.generator.contains("Wolfram"));
    assert_eq!(oracle.cases.len(), 4, "fixture changes require explicit review");
    for case in oracle.cases {
        let a = curve(&case.a);
        let b = curve(&case.b);
        assert!(a.valid() && b.valid(), "invalid oracle sources: {}", case.name);
        let source_a = a.clone();
        let source_b = b.clone();
        let result = blend_nurbs_curves(&a, &b, case.continuity, case.strength_a, case.strength_b, case.reverse_a, case.reverse_b)
            .unwrap_or_else(|error| panic!("{} failed to blend: {error}", case.name));
        assert_eq!(result.degree, case.expected.degree, "{}", case.name);
        assert!(result.valid(), "{} produced invalid rational NURBS", case.name);
        assert_eq!(result.control.len(), case.expected.control.len(), "{}", case.name);
        assert_eq!(result.weights, vec![1.0; result.control.len()], "{}", case.name);
        for (i, (&actual, &expected)) in result.control.iter().zip(&case.expected.control).enumerate() {
            close(actual, expected, &case.name, &format!("control[{i}]"));
        }
        assert_eq!(case.expected.stations.len(), 5, "{}", case.name);
        for station in &case.expected.stations {
            let point = result.evaluate(station.t).expect("valid oracle station");
            let derivative = result.derivative(station.t).expect("valid oracle derivative");
            close(point, station.point, &case.name, &format!("C({})", station.t));
            close(derivative, station.derivative, &case.name, &format!("C'({})", station.t));
        }
        if let Some(curv) = &case.expected.curvature {
            assert_eq!(case.continuity, 2);
            for (t, expected) in [(0.0, curv.start), (0.5, curv.mid), (1.0, curv.end)] {
                close(result.curvature_vector(t).expect("regular oracle curvature station"), expected, &case.name, &format!("K({t})"));
            }
        }
        assert_eq!(a, source_a, "{} mutated input curve A", case.name);
        assert_eq!(b, source_b, "{} mutated input curve B", case.name);
    }
}
