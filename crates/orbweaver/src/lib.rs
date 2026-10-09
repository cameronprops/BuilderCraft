//! OrbWeaver is Worldwright's headless, deterministic, typed parametric graph.
//! Nodes execute the SAME validated algorithms as direct Worldwright CAD/API
//! commands. Typed trees, branch-structure modifiers and explicit list matching
//! now execute through the same DAG and kernel dispatcher. Native math
//! broadcasts over typed tree branches; implicit Grasshopper path alignment,
//! bake/preview and the visual canvas come later.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

use buildercraft_kernel::{
    KernelError, SHARED_TOOLS, SharedToolContract, ToolRequest, ToolValue, TreeMatchPolicy, execute_shared_tool_with_matching,
    shared_tool, shared_tool_value_cost, tool_output_may_match_port, tool_value_matches_port,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(test)]
use buildercraft_kernel::execute_shared_tool;

pub const GRAPH_SCHEMA_VERSION: u32 = 1;
pub const MAX_GRAPH_NODES: usize = 512;
pub const MAX_GRAPH_CONNECTIONS: usize = 4096;
/// Upper bound on all retained graph literal and evaluated value items.
pub const MAX_GRAPH_VALUE_ITEMS: usize = 250_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputBinding {
    Constant { value: ToolValue },
    Output { node: u64 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: u64,
    /// Native OrbWeaver node ID, e.g. "orbweaver.point.distance".
    pub component: String,
    pub inputs: BTreeMap<String, InputBinding>,
    /// Branch-wise item matching; additive default keeps version-1 graphs
    /// readable without rewriting existing serialized nodes.
    #[serde(default)]
    pub matching: TreeMatchPolicy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub version: u32,
    pub nodes: Vec<Node>,
    /// Result node IDs; an empty selection evaluates and returns all nodes.
    pub outputs: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphResult {
    /// Deterministic, node-ID-keyed values. No document mutation occurs.
    pub values: BTreeMap<u64, ToolValue>,
    pub evaluated_node_count: usize,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("unsupported OrbWeaver graph schema version")]
    Version,
    #[error("OrbWeaver graph resource budget exceeded")]
    Budget,
    #[error("invalid or duplicated graph node identity: {0}")]
    NodeId(u64),
    #[error("unsupported native OrbWeaver component: {0}")]
    Component(String),
    #[error("missing linked node: {0}")]
    MissingNode(u64),
    #[error("invalid component port on graph node: {0}")]
    Port(u64),
    #[error("typed connection or constant mismatch on node {node}, port {port}")]
    Type { node: u64, port: String },
    #[error("OrbWeaver graph contains a cycle")]
    Cycle,
    #[error("shared geometry kernel rejected graph operation: {0}")]
    Kernel(#[from] KernelError),
}

pub type Result<T> = std::result::Result<T, GraphError>;

pub fn native_components() -> &'static [SharedToolContract] {
    SHARED_TOOLS
}

/// This evaluator handles tagged scalars and native tree values, including
/// per-node Shortest/Longest/CrossReference broadcasting. It never implicitly
/// aligns different branch paths. Failures return no partial result.
/// Strict node and edge ceilings bound the quadratic deterministic scheduler.
pub fn evaluate(graph: &Graph) -> Result<GraphResult> {
    if graph.version != GRAPH_SCHEMA_VERSION {
        return Err(GraphError::Version);
    }
    if graph.nodes.len() > MAX_GRAPH_NODES || graph.outputs.len() > MAX_GRAPH_NODES {
        return Err(GraphError::Budget);
    }
    let mut nodes = BTreeMap::new();
    for node in &graph.nodes {
        if node.id == 0 || nodes.insert(node.id, node).is_some() {
            return Err(GraphError::NodeId(node.id));
        }
    }
    let mut edge_count = 0usize;
    let mut retained_items = 0usize;
    for node in nodes.values() {
        let contract = require_component(&node.component)?;
        if node.inputs.len() != contract.inputs.len() {
            return Err(GraphError::Port(node.id));
        }
        for port in contract.inputs {
            let binding = node.inputs.get(port.name).ok_or(GraphError::Port(node.id))?;
            match binding {
                InputBinding::Constant { value } => {
                    retained_items = retained_items.checked_add(graph_value_cost(value)?).ok_or(GraphError::Budget)?;
                    if retained_items > MAX_GRAPH_VALUE_ITEMS {
                        return Err(GraphError::Budget);
                    }
                    if tool_value_matches_port(port.kind, value).is_err() {
                        return Err(GraphError::Type { node: node.id, port: port.name.into() });
                    }
                }
                InputBinding::Output { node: predecessor } => {
                    edge_count = edge_count.checked_add(1).ok_or(GraphError::Budget)?;
                    if edge_count > MAX_GRAPH_CONNECTIONS {
                        return Err(GraphError::Budget);
                    }
                    let source = nodes.get(predecessor).ok_or(GraphError::MissingNode(*predecessor))?;
                    let source_contract = require_component(&source.component)?;
                    if !tool_output_may_match_port(port.kind, source_contract.output) {
                        return Err(GraphError::Type { node: node.id, port: port.name.into() });
                    }
                }
            }
        }
    }
    for &id in &graph.outputs {
        if !nodes.contains_key(&id) {
            return Err(GraphError::MissingNode(id));
        }
    }

    let mut values: BTreeMap<u64, ToolValue> = BTreeMap::new();
    // All graph inputs are complete and typed; evaluate only nodes whose
    // predecessors have completed. Node iteration order is stable by ID.
    while values.len() < nodes.len() {
        let mut progressed = false;
        for (&id, node) in &nodes {
            if values.contains_key(&id) {
                continue;
            }
            let contract = require_component(&node.component)?;
            let mut ready = true;
            let mut inputs = BTreeMap::new();
            for port in contract.inputs {
                let binding = node.inputs.get(port.name).ok_or(GraphError::Port(id))?;
                let value = match binding {
                    InputBinding::Constant { value } => value.clone(),
                    InputBinding::Output { node: source } => {
                        let Some(value) = values.get(source) else {
                            ready = false;
                            break;
                        };
                        value.clone()
                    }
                };
                inputs.insert(port.name.into(), value);
            }
            if !ready {
                continue;
            }
            // Upstream operators may dynamically lift scalar output types to
            // trees. Validate the ACTUAL runtime value before executing a
            // downstream node, not only the source's static output type.
            for port in contract.inputs {
                let value = inputs.get(port.name).ok_or(GraphError::Port(id))?;
                if let Err(error) = tool_value_matches_port(port.kind, value) {
                    return Err(match error {
                        KernelError::Budget => GraphError::Budget,
                        _ => GraphError::Type { node: id, port: port.name.into() },
                    });
                }
            }
            let value = execute_shared_tool_with_matching(&ToolRequest { operation: contract.operation.into(), inputs }, node.matching)?;
            retained_items = retained_items.checked_add(graph_value_cost(&value)?).ok_or(GraphError::Budget)?;
            if retained_items > MAX_GRAPH_VALUE_ITEMS {
                return Err(GraphError::Budget);
            }
            values.insert(id, value);
            progressed = true;
        }
        if !progressed {
            return Err(GraphError::Cycle);
        }
    }
    let evaluated_node_count = values.len();
    if !graph.outputs.is_empty() {
        values.retain(|id, _| graph.outputs.contains(id));
    }
    Ok(GraphResult { values, evaluated_node_count })
}

/// Shared value-budget validation mapped to the graph's public resource error.
fn graph_value_cost(value: &ToolValue) -> Result<usize> {
    shared_tool_value_cost(value).map_err(|error| match error {
        KernelError::Budget => GraphError::Budget,
        other => GraphError::Kernel(other),
    })
}

fn require_component(id: &str) -> Result<&'static SharedToolContract> {
    let spec = shared_tool(id).ok_or_else(|| GraphError::Component(id.into()))?;
    if spec.orbweaver_node != id {
        return Err(GraphError::Component(id.into()));
    }
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildercraft_kernel::ToolValue::{Count, Number, Point, Polyline, Vector};
    use cadcraft_geom::Vec3;

    fn constant(value: ToolValue) -> InputBinding {
        InputBinding::Constant { value }
    }
    fn dist(id: u64, a: Vec3, b: Vec3) -> Node {
        Node {
            id,
            matching: TreeMatchPolicy::Shortest,
            component: "orbweaver.point.distance".into(),
            inputs: BTreeMap::from([("a".into(), constant(Point(a))), ("b".into(), constant(Point(b)))]),
        }
    }
    #[test]
    fn distance_node_matches_direct_cad_and_kernel_dispatch() {
        let a = Vec3::ZERO;
        let b = Vec3::new(3., 4., 12.);
        let graph = Graph { version: 1, nodes: vec![dist(1, a, b)], outputs: vec![1] };
        let from_graph = evaluate(&graph).unwrap().values.get(&1).cloned();
        let direct = execute_shared_tool(&ToolRequest {
            operation: "worldwright.point.distance".into(),
            inputs: BTreeMap::from([("a".into(), Point(a)), ("b".into(), Point(b))]),
        })
        .ok();
        assert_eq!(from_graph, direct);
        assert_eq!(from_graph, Some(Number(13.)));
    }
    #[test]
    fn chained_vector_nodes_use_one_kernel_implementation() {
        let normalized = Node {
            id: 3,
            matching: TreeMatchPolicy::Shortest,
            component: "orbweaver.vector.normalize".into(),
            inputs: BTreeMap::from([("v".into(), constant(Vector(Vec3::new(3., 4., 0.))))]),
        };
        let length = Node {
            id: 9,
            matching: TreeMatchPolicy::Shortest,
            component: "orbweaver.vector.length".into(),
            inputs: BTreeMap::from([("v".into(), InputBinding::Output { node: 3 })]),
        };
        let graph = Graph { version: 1, nodes: vec![length, normalized], outputs: vec![9] };
        let result = evaluate(&graph).unwrap();
        assert_eq!(result.evaluated_node_count, 2);
        assert_eq!(result.values.len(), 1);
        assert!(result.values.get(&9).is_some_and(|v| matches!(v, Number(x) if (*x - 1.).abs() < 1e-12)));
    }
    #[test]
    fn division_count_is_a_modifier_not_a_new_sampling_engine() {
        let graph = Graph {
            version: 1,
            nodes: vec![Node {
                id: 4,
                matching: TreeMatchPolicy::Shortest,
                component: "orbweaver.polyline.divide_count".into(),
                inputs: BTreeMap::from([
                    ("points".into(), constant(Polyline(vec![Vec3::ZERO, Vec3::new(8., 0., 0.)]))),
                    ("count".into(), constant(Count(4))),
                ]),
            }],
            outputs: vec![],
        };
        let result = evaluate(&graph).unwrap();
        assert_eq!(
            result.values.get(&4),
            Some(&Polyline(vec![Vec3::ZERO, Vec3::new(2., 0., 0.), Vec3::new(4., 0., 0.), Vec3::new(6., 0., 0.), Vec3::new(8., 0., 0.),]))
        );
    }
    #[test]
    fn cycles_and_dangling_references_are_errors() {
        let n = |id, predecessor| Node {
            id,
            matching: TreeMatchPolicy::Shortest,
            component: "orbweaver.vector.normalize".into(),
            inputs: BTreeMap::from([("v".into(), InputBinding::Output { node: predecessor })]),
        };
        let cyclic = Graph { version: 1, nodes: vec![n(1, 2), n(2, 1)], outputs: vec![] };
        assert_eq!(evaluate(&cyclic), Err(GraphError::Cycle));
        let dangling = Graph { version: 1, nodes: vec![n(1, 99)], outputs: vec![] };
        assert_eq!(evaluate(&dangling), Err(GraphError::MissingNode(99)));
    }
    #[test]
    fn connection_type_checks_run_before_evaluation() {
        let n = Node {
            id: 2,
            matching: TreeMatchPolicy::Shortest,
            component: "orbweaver.vector.length".into(),
            inputs: BTreeMap::from([("v".into(), InputBinding::Output { node: 1 })]),
        };
        let graph = Graph { version: 1, nodes: vec![dist(1, Vec3::ZERO, Vec3::new(1., 0., 0.)), n], outputs: vec![] };
        assert_eq!(evaluate(&graph), Err(GraphError::Type { node: 2, port: "v".into() }));
    }
    #[test]
    fn invalid_modifier_propagates_kernel_error_atomically() {
        let graph = Graph {
            version: 1,
            nodes: vec![
                dist(1, Vec3::ZERO, Vec3::new(1., 0., 0.)),
                Node {
                    id: 2,
                    matching: TreeMatchPolicy::Shortest,
                    component: "orbweaver.polyline.divide_count".into(),
                    inputs: BTreeMap::from([
                        ("points".into(), constant(Polyline(vec![Vec3::ZERO, Vec3::new(3., 0., 0.)]))),
                        ("count".into(), constant(Count(0))),
                    ]),
                },
            ],
            outputs: vec![1],
        };
        assert!(matches!(evaluate(&graph), Err(GraphError::Kernel(_))));
    }
    #[test]
    fn rejects_duplicate_ids_and_invalid_schema() {
        let g = Graph { version: 1, nodes: vec![dist(7, Vec3::ZERO, Vec3::Z), dist(7, Vec3::ZERO, Vec3::Z)], outputs: vec![] };
        assert_eq!(evaluate(&g), Err(GraphError::NodeId(7)));
        let g = Graph { version: 2, nodes: vec![], outputs: vec![] };
        assert_eq!(evaluate(&g), Err(GraphError::Version));
    }
    #[test]
    fn bounded_input_collection_rejects_large_graph_values() {
        let samples = vec![Vec3::ZERO; MAX_GRAPH_VALUE_ITEMS + 1];
        let graph = Graph {
            version: 1,
            nodes: vec![Node {
                id: 5,
                matching: TreeMatchPolicy::Shortest,
                component: "orbweaver.polyline.length".into(),
                inputs: BTreeMap::from([("points".into(), constant(Polyline(samples)))]),
            }],
            outputs: vec![],
        };
        assert_eq!(evaluate(&graph), Err(GraphError::Budget));
    }

    #[test]
    fn native_tree_nodes_graft_then_flatten_without_losing_values() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreePath};
        let start =
            ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0, 4]), items: vec![Number(1.), Number(2.), Number(3.)] }] });
        let graph = Graph {
            version: 1,
            nodes: vec![
                Node {
                    id: 2,
                    matching: TreeMatchPolicy::Shortest,
                    component: "orbweaver.tree.flatten".into(),
                    inputs: BTreeMap::from([("tree".into(), InputBinding::Output { node: 1 })]),
                },
                Node {
                    id: 1,
                    matching: TreeMatchPolicy::Shortest,
                    component: "orbweaver.tree.graft".into(),
                    inputs: BTreeMap::from([("tree".into(), constant(start))]),
                },
            ],
            outputs: vec![2],
        };
        let result = evaluate(&graph).unwrap();
        assert_eq!(result.evaluated_node_count, 2);
        assert_eq!(
            result.values.get(&2),
            Some(&ToolValue::Tree(DataTree {
                branches: vec![TreeBranch { path: TreePath(vec![0]), items: vec![Number(1.), Number(2.), Number(3.)] }],
            }))
        );
    }
    #[test]
    fn native_tree_matching_respects_mode_modifier_and_budget() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreeMatchPolicy, TreePath};
        let tree = |values: Vec<ToolValue>| ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items: values }] });
        let graph = Graph {
            version: 1,
            nodes: vec![Node {
                id: 7,
                matching: TreeMatchPolicy::Shortest,
                component: "orbweaver.tree.match".into(),
                inputs: BTreeMap::from([
                    ("a".into(), constant(tree(vec![Number(1.), Number(2.)]))),
                    ("b".into(), constant(tree(vec![Number(8.)]))),
                    ("mode".into(), constant(ToolValue::MatchMode(TreeMatchPolicy::Longest))),
                ]),
            }],
            outputs: vec![7],
        };
        let result = evaluate(&graph).unwrap();
        let Some(ToolValue::Tree(tree)) = result.values.get(&7) else {
            assert!(false, "matching must return a tree");
            return;
        };
        assert_eq!(
            tree.branches[0].items,
            vec![ToolValue::Pair(Box::new((Number(1.), Number(8.)))), ToolValue::Pair(Box::new((Number(2.), Number(8.)))),]
        );
    }
    #[test]
    fn tree_input_types_are_validated_before_graph_execution() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreePath};
        let graph = Graph {
            version: 1,
            nodes: vec![Node {
                id: 1,
                matching: TreeMatchPolicy::Shortest,
                component: "orbweaver.tree.flatten".into(),
                inputs: BTreeMap::from([(
                    "tree".into(),
                    constant(ToolValue::Tree(DataTree {
                        branches: vec![TreeBranch {
                            path: TreePath(vec![0]),
                            items: vec![ToolValue::Polyline(vec![Vec3::ZERO; MAX_GRAPH_VALUE_ITEMS + 1])],
                        }],
                    })),
                )]),
            }],
            outputs: vec![1],
        };
        assert_eq!(evaluate(&graph), Err(GraphError::Budget));
    }
    #[test]
    fn linked_numerical_tree_output_flows_into_structural_node() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreePath};
        let source = ToolValue::Tree(DataTree {
            branches: vec![TreeBranch { path: TreePath(vec![0, 5]), items: vec![Point(Vec3::new(3., 4., 0.)), Point(Vec3::new(6., 8., 0.))] }],
        });
        // Node 1 applies existing distance math across two points. Node 2
        // consumes its tree output even though Distance's scalar output type
        // is Number. Both graph node orders must behave identically.
        let distance = Node {
            id: 1,
            component: "orbweaver.point.distance".into(),
            matching: TreeMatchPolicy::Shortest,
            inputs: BTreeMap::from([("a".into(), constant(source)), ("b".into(), constant(Point(Vec3::ZERO)))]),
        };
        let flatten = Node {
            id: 2,
            component: "orbweaver.tree.flatten".into(),
            matching: TreeMatchPolicy::Shortest,
            inputs: BTreeMap::from([("tree".into(), InputBinding::Output { node: 1 })]),
        };
        let graph = Graph { version: 1, nodes: vec![flatten, distance], outputs: vec![2] };
        let result = evaluate(&graph).unwrap();
        assert_eq!(result.evaluated_node_count, 2);
        assert_eq!(
            result.values.get(&2),
            Some(&ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items: vec![Number(5.), Number(10.)] }] }))
        );
    }

    #[test]
    fn cross_reference_modifier_is_applied_by_both_graph_and_direct_cad() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreePath};
        let make = |xs: Vec<f64>| {
            ToolValue::Tree(DataTree {
                branches: vec![TreeBranch { path: TreePath(vec![0]), items: xs.into_iter().map(|x| Point(Vec3::new(x, 0., 0.))).collect() }],
            })
        };
        let inputs: BTreeMap<String, ToolValue> = BTreeMap::from([("a".into(), make(vec![1., 2.])), ("b".into(), make(vec![4., 8., 16.]))]);
        let node = Node {
            id: 11,
            component: "orbweaver.point.distance".into(),
            matching: TreeMatchPolicy::CrossReference,
            inputs: inputs.iter().map(|(name, value)| (name.clone(), constant(value.clone()))).collect(),
        };
        let graph = Graph { version: 1, nodes: vec![node], outputs: vec![11] };
        let computed = evaluate(&graph).unwrap();
        let direct = execute_shared_tool_with_matching(
            &ToolRequest { operation: "worldwright.point.distance".into(), inputs },
            TreeMatchPolicy::CrossReference,
        )
        .unwrap();
        assert_eq!(computed.values.get(&11), Some(&direct));
        let ToolValue::Tree(tree) = direct else {
            assert!(false, "cross-reference must return a tree");
            return;
        };
        assert_eq!(tree.branches[0].items, vec![Number(3.), Number(7.), Number(15.), Number(2.), Number(6.), Number(14.),]);
    }

    #[test]
    fn old_version_one_graph_nodes_default_to_shortest_matching() {
        let graph = Graph { version: 1, nodes: vec![dist(7, Vec3::ZERO, Vec3::new(0., 3., 4.))], outputs: vec![7] };
        let mut encoded = serde_json::to_value(&graph).unwrap();
        let nodes = encoded["nodes"].as_array_mut().unwrap();
        nodes[0].as_object_mut().unwrap().remove("matching");
        let decoded: Graph = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.nodes[0].matching, TreeMatchPolicy::Shortest);
        assert_eq!(evaluate(&graph), evaluate(&decoded));
    }

    #[test]
    fn linked_output_with_an_incompatible_runtime_leaf_reports_a_type_error() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreePath};
        let number_tree = ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items: vec![Number(12.)] }] });
        let source = Node {
            id: 1,
            component: "orbweaver.tree.flatten".into(),
            matching: TreeMatchPolicy::Shortest,
            inputs: BTreeMap::from([("tree".into(), constant(number_tree))]),
        };
        let consumer = Node {
            id: 2,
            component: "orbweaver.point.distance".into(),
            matching: TreeMatchPolicy::Shortest,
            inputs: BTreeMap::from([("a".into(), InputBinding::Output { node: 1 }), ("b".into(), constant(Point(Vec3::ZERO)))]),
        };
        let graph = Graph { version: 1, nodes: vec![consumer, source], outputs: vec![2] };
        assert_eq!(evaluate(&graph), Err(GraphError::Type { node: 2, port: "a".into() }));
    }

    #[test]
    fn numeric_node_may_feed_a_tree_structural_port_when_lifted() {
        use buildercraft_kernel::{DataTree, TreeBranch, TreePath};
        let points = ToolValue::Tree(DataTree {
            branches: vec![TreeBranch { path: TreePath(vec![0, 4]), items: vec![Point(Vec3::new(0., 3., 4.)), Point(Vec3::new(0., 0., 2.))] }],
        });
        let distance = Node {
            id: 1,
            component: "orbweaver.point.distance".into(),
            matching: TreeMatchPolicy::Shortest,
            inputs: BTreeMap::from([("a".into(), constant(points)), ("b".into(), constant(Point(Vec3::ZERO)))]),
        };
        let count = Node {
            id: 2,
            component: "orbweaver.tree.validate".into(),
            matching: TreeMatchPolicy::Shortest,
            inputs: BTreeMap::from([("tree".into(), InputBinding::Output { node: 1 })]),
        };
        let graph = Graph { version: 1, nodes: vec![count, distance], outputs: vec![2] };
        assert_eq!(evaluate(&graph).unwrap().values.get(&2), Some(&Count(2)));
    }

    #[test]
    fn roundtrip_and_node_order_are_deterministic() {
        let graph = Graph {
            version: 1,
            nodes: vec![dist(11, Vec3::ZERO, Vec3::new(0., 3., 4.)), dist(2, Vec3::ZERO, Vec3::new(3., 4., 0.))],
            outputs: vec![],
        };
        let json = serde_json::to_string(&graph).unwrap();
        let restored: Graph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, restored);
        let mut reordered = restored.clone();
        reordered.nodes.reverse();
        assert_eq!(evaluate(&restored), evaluate(&reordered));
    }
}
