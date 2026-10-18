//! Direct-CAD and OrbWeaver share one geometry kernel and typed contracts.
use buildercraft_kernel::{ToolRequest, ToolValue, TreeMatchPolicy, execute_shared_tool};
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

#[test]
fn oriented_path_array_node_aligns_copies_and_keeps_source_geometry() {
    let shape = node(
        10,
        "orbweaver.array.path_oriented",
        vec![
            ("geometry", literal(ToolValue::Polyline(vec![p(0., 0., 0.), p(1., 0., 0.)]))),
            ("path", literal(ToolValue::Polyline(vec![p(0., 0., 0.), p(5., 0., 0.), p(5., 5., 0.)]))),
            ("count", literal(ToolValue::Count(3))),
            ("up", literal(ToolValue::Vector(Vec3::Z))),
            ("anchor", literal(ToolValue::Point(Vec3::ZERO))),
        ],
    );
    let graph = Graph { version: GRAPH_SCHEMA_VERSION, nodes: vec![shape], outputs: vec![10] };
    let evaluated = evaluate(&graph).unwrap();
    let Some(ToolValue::Tree(tree)) = evaluated.values.get(&10) else { panic!("expected oriented arrays") };
    assert_eq!(tree.branches.len(), 3);
    assert_eq!(tree.branches[0].items[0], ToolValue::Polyline(vec![p(0., 0., 0.), p(1., 0., 0.)]));
    let ToolValue::Polyline(bent) = &tree.branches[1].items[0] else { panic!("expected copy geometry") };
    assert!((bent[0] - p(5., 0., 0.)).len() < 1e-9);
    assert!((bent[1] - p(5., 1., 0.)).len() < 1e-9);
    let mut bad = graph;
    bad.nodes[0].inputs.insert("up".into(), literal(ToolValue::Vector(p(1., 0., 0.))));
    assert!(evaluate(&bad).is_err());
}

#[test]
fn cad_and_orbweaver_oriented_path_arrays_have_identical_outputs() {
    let geometry = ToolValue::Polyline(vec![p(0., 0., 0.), p(1., 0., 0.), p(0., 1., 0.)]);
    let path = ToolValue::Polyline(vec![p(0., 0., 0.), p(5., 0., 0.), p(5., 5., 0.)]);
    let values = std::collections::BTreeMap::from([
        ("geometry".into(), geometry.clone()),
        ("path".into(), path.clone()),
        ("count".into(), ToolValue::Count(3)),
        ("up".into(), ToolValue::Vector(Vec3::Z)),
        ("anchor".into(), ToolValue::Point(Vec3::ZERO)),
    ]);
    let direct = execute_shared_tool(&ToolRequest { operation: "worldwright.array.path_oriented".into(), inputs: values.clone() }).unwrap();
    let node = Node {
        id: 1,
        component: "orbweaver.array.path_oriented".into(),
        inputs: values.into_iter().map(|(k, v)| (k, InputBinding::Constant { value: v })).collect(),
        matching: TreeMatchPolicy::Shortest,
    };
    let graph = Graph { version: GRAPH_SCHEMA_VERSION, nodes: vec![node], outputs: vec![1] };
    let evaluation = evaluate(&graph).unwrap();
    assert_eq!(evaluation.values.get(&1), Some(&direct));
}
