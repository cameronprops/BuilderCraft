//! Direct-CAD and OrbWeaver share one geometry kernel and typed contracts.
use buildercraft_kernel::{ToolValue, TreeMatchPolicy};
use cadcraft_geom::{
    Vec3,
    nurbs3d::{Curve, Surface, uniform_knots},
};
use orbweaver::{GRAPH_SCHEMA_VERSION, Graph, InputBinding, Node, evaluate};

fn p(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new(x, y, z)
}
fn literal(value: ToolValue) -> InputBinding {
    InputBinding::Constant { value }
}
fn node(id: u64, component: &str, inputs: Vec<(&str, InputBinding)>) -> Node {
    Node { id, component: component.into(), inputs: inputs.into_iter().map(|(n, v)| (n.into(), v)).collect(), matching: TreeMatchPolicy::Shortest }
}
fn plane(z: f64) -> Surface {
    Surface {
        rows: vec![
            Curve { degree: 1, control: vec![p(0., 0., z), p(2., 0., z)], weights: vec![1., 1.], knots: uniform_knots(2, 1) },
            Curve { degree: 1, control: vec![p(0., 2., z), p(2., 2., z)], weights: vec![1., 1.], knots: uniform_knots(2, 1) },
        ],
        degree_v: 1,
        knots_v: uniform_knots(2, 1),
    }
}
#[test]
fn pushpull_mesh_output_can_be_wired_directly_to_mesh_project() {
    let shape = node(
        1,
        "orbweaver.solid.pushpull_quad",
        vec![
            ("face", literal(ToolValue::Polyline(vec![p(0., 0., 0.), p(2., 0., 0.), p(2., 2., 0.), p(0., 2., 0.)]))),
            ("distance", literal(ToolValue::Number(2.))),
        ],
    );
    let projected = node(
        2,
        "orbweaver.project.mesh",
        vec![
            ("geometry", literal(ToolValue::Polyline(vec![p(1., 1., 5.)]))),
            ("target", InputBinding::Output { node: 1 }),
            ("direction", literal(ToolValue::Vector(Vec3::Z))),
        ],
    );
    let g = Graph { version: GRAPH_SCHEMA_VERSION, nodes: vec![projected, shape], outputs: vec![2] };
    let result = evaluate(&g).unwrap();
    assert_eq!(result.evaluated_node_count, 2);
    assert_eq!(result.values.get(&2), Some(&ToolValue::Polyline(vec![p(1., 1., 2.)])));
}
#[test]
fn polar_array_graph_node_is_parameter_driven_and_deterministic() {
    let n = node(
        1,
        "orbweaver.array.polar",
        vec![
            ("geometry", literal(ToolValue::Polyline(vec![p(1., 0., 0.)]))),
            ("center", literal(ToolValue::Point(Vec3::ZERO))),
            ("axis", literal(ToolValue::Vector(Vec3::Z))),
            ("sweep_degrees", literal(ToolValue::Number(360.))),
            ("count", literal(ToolValue::Count(4))),
        ],
    );
    let g = Graph { version: GRAPH_SCHEMA_VERSION, nodes: vec![n], outputs: vec![1] };
    let a = evaluate(&g).unwrap();
    let b = evaluate(&g).unwrap();
    assert_eq!(a, b);
    let Some(ToolValue::Tree(tree)) = a.values.get(&1) else { panic!("expected instances") };
    assert_eq!(tree.branches.len(), 4);
    assert_eq!(tree.branches[0].items[0], ToolValue::Polyline(vec![p(1., 0., 0.)]));
}
#[test]
fn nurbs_flow_and_project_are_native_graph_operations() {
    let flow = node(
        1,
        "orbweaver.surface.flow_nurbs",
        vec![
            ("geometry", literal(ToolValue::Polyline(vec![p(0.5, 1., 1.)]))),
            ("base", literal(ToolValue::Surface(plane(0.)))),
            ("target", literal(ToolValue::Surface(plane(4.)))),
        ],
    );
    let proj = node(
        2,
        "orbweaver.project.nurbs",
        vec![
            ("geometry", InputBinding::Output { node: 1 }),
            ("target", literal(ToolValue::Surface(plane(0.)))),
            ("direction", literal(ToolValue::Vector(Vec3::Z))),
        ],
    );
    let graph = Graph { version: GRAPH_SCHEMA_VERSION, nodes: vec![flow, proj], outputs: vec![1, 2] };
    let result = evaluate(&graph).unwrap();
    let Some(ToolValue::Polyline(a)) = result.values.get(&1) else { panic!("flow not a polyline") };
    let Some(ToolValue::Polyline(b)) = result.values.get(&2) else { panic!("project not a polyline") };
    assert!((a[0] - p(0.5, 1., 5.)).len() < 1e-6);
    assert!((b[0] - p(0.5, 1., 0.)).len() < 1e-6);
}
#[test]
fn invalid_input_rejects_entire_graph() {
    let n = node(
        1,
        "orbweaver.array.linear",
        vec![
            ("geometry", literal(ToolValue::Polyline(vec![p(1., 0., 0.)]))),
            ("step", literal(ToolValue::Vector(Vec3::Z))),
            ("count", literal(ToolValue::Count(257))),
        ],
    );
    assert!(evaluate(&Graph { version: GRAPH_SCHEMA_VERSION, nodes: vec![n], outputs: vec![1] }).is_err());
}
