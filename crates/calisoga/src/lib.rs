//! Calisoga is Worldwright's headless, deterministic, typed parametric graph.
//! Nodes execute the SAME validated algorithms as direct Worldwright CAD/API
//! commands. Graph lists, data trees, bake/preview and visual canvas follow
//! in later dependency layers; this first evaluator is deliberately scalar.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
#![forbid(unsafe_code)]

use buildercraft_kernel::{
    KernelError, SharedToolContract, ToolRequest, ToolValue,
    execute_shared_tool, shared_tool, SHARED_TOOLS,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GRAPH_SCHEMA_VERSION: u32 = 1;
pub const MAX_GRAPH_NODES: usize = 512;
pub const MAX_GRAPH_CONNECTIONS: usize = 4096;

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
    /// Native Calisoga node ID, e.g. "calisoga.point.distance".
    pub component: String,
    pub inputs: BTreeMap<String, InputBinding>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub version: u32,
    pub nodes: Vec<Node>,
    /// Result node IDs; an empty selection evaluates and returns all nodes.
    pub outputs: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphResult {
    /// Deterministic, node-ID-keyed values. No document mutation occurs.
    pub values: BTreeMap<u64, ToolValue>,
    pub evaluated_node_count: usize,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GraphError {
    #[error("unsupported Calisoga graph schema version")]
    Version,
    #[error("Calisoga graph resource budget exceeded")]
    Budget,
    #[error("invalid or duplicated graph node identity: {0}")]
    NodeId(u64),
    #[error("unsupported native Calisoga component: {0}")]
    Component(String),
    #[error("missing linked node: {0}")]
    MissingNode(u64),
    #[error("invalid component port on graph node: {0}")]
    Port(u64),
    #[error("typed connection or constant mismatch on node {node}, port {port}")]
    Type { node: u64, port: String },
    #[error("Calisoga graph contains a cycle")]
    Cycle,
    #[error("shared geometry kernel rejected graph operation: {0}")]
    Kernel(#[from] KernelError),
}

pub type Result<T> = std::result::Result<T, GraphError>;

pub fn native_components() -> &'static [SharedToolContract] {
    SHARED_TOOLS
}

/// The initial graph evaluator intentionally has no list/tree matching or
/// implicit conversions. All node outputs are single typed values, and graph
/// evaluation is atomic: a failed node returns no partial result to callers.
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
    for node in nodes.values() {
        let contract = require_component(&node.component)?;
        if node.inputs.len() != contract.inputs.len() {
            return Err(GraphError::Port(node.id));
        }
        for port in contract.inputs {
            let binding = node.inputs.get(port.name).ok_or(GraphError::Port(node.id))?;
            match binding {
                InputBinding::Constant { value } => {
                    if value.kind() != port.kind {
                        return Err(GraphError::Type {
                            node: node.id,
                            port: port.name.into(),
                        });
                    }
                }
                InputBinding::Output { node: predecessor } => {
                    edge_count = edge_count.checked_add(1).ok_or(GraphError::Budget)?;
                    if edge_count > MAX_GRAPH_CONNECTIONS { return Err(GraphError::Budget); }
                    let source = nodes.get(predecessor).ok_or(GraphError::MissingNode(*predecessor))?;
                    let source_contract = require_component(&source.component)?;
                    if source_contract.output != port.kind {
                        return Err(GraphError::Type {
                            node: node.id,
                            port: port.name.into(),
                        });
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

    let mut values = BTreeMap::new();
    // All graph inputs are complete and typed; evaluate only nodes whose
    // predecessors have completed. Node iteration order is stable by ID.
    while values.len() < nodes.len() {
        let mut progressed = false;
        for (&id, node) in &nodes {
            if values.contains_key(&id) { continue; }
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
            if !ready { continue; }
            let value = execute_shared_tool(&ToolRequest {
                operation: contract.operation.into(),
                inputs,
            })?;
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

fn require_component(id: &str) -> Result<&'static SharedToolContract> {
    let spec = shared_tool(id).ok_or_else(|| GraphError::Component(id.into()))?;
    if spec.calisoga_node != id {
        return Err(GraphError::Component(id.into()));
    }
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildercraft_kernel::ToolValue::{Count, Number, Point, Polyline, Vector};
    use buildercraft_kernel::execute_shared_tool;
    use buildercraft_kernel::ToolRequest;
    use cadcraft_geom::Vec3;

    fn constant(value: ToolValue) -> InputBinding {
        InputBinding::Constant { value }
    }
    fn dist(id: u64, a: Vec3, b: Vec3) -> Node {
        Node {
            id,
            component: "calisoga.point.distance".into(),
            inputs: BTreeMap::from([
                ("a".into(), constant(Point(a))),
                ("b".into(), constant(Point(b))),
            ]),
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
            inputs: BTreeMap::from([
                ("a".into(), Point(a)),
                ("b".into(), Point(b)),
            ]),
        }).ok();
        assert_eq!(from_graph, direct);
        assert_eq!(from_graph, Some(Number(13.)));
    }
    #[test]
    fn chained_vector_nodes_use_one_kernel_implementation() {
        let normalized = Node {
            id: 3,
            component: "calisoga.vector.normalize".into(),
            inputs: BTreeMap::from([
                ("v".into(), constant(Vector(Vec3::new(3., 4., 0.)))),
            ]),
        };
        let length = Node {
            id: 9,
            component: "calisoga.vector.length".into(),
            inputs: BTreeMap::from([
                ("v".into(), InputBinding::Output { node: 3 }),
            ]),
        };
        let graph = Graph {
            version: 1,
            nodes: vec![length, normalized],
            outputs: vec![9],
        };
        let result = evaluate(&graph).unwrap();
        assert_eq!(result.evaluated_node_count, 2);
        assert_eq!(result.values.len(), 1);
        assert!(result.values.get(&9).is_some_and(|v|
            matches!(v, Number(x) if (x - 1.).abs() < 1e-12)));
    }
    #[test]
    fn division_count_is_a_modifier_not_a_new_sampling_engine() {
        let graph = Graph {
            version: 1,
            nodes: vec![Node {
                id: 4,
                component: "calisoga.polyline.divide_count".into(),
                inputs: BTreeMap::from([
                    ("points".into(), constant(Polyline(vec![
                        Vec3::ZERO, Vec3::new(8., 0., 0.),
                    ]))),
                    ("count".into(), constant(Count(4))),
                ]),
            }],
            outputs: vec![],
        };
        let result = evaluate(&graph).unwrap();
        assert_eq!(result.values.get(&4), Some(&Polyline(vec![
            Vec3::ZERO,
            Vec3::new(2.,0.,0.),
            Vec3::new(4.,0.,0.),
            Vec3::new(6.,0.,0.),
            Vec3::new(8.,0.,0.),
        ])));
    }
    #[test]
    fn cycles_and_dangling_references_are_errors() {
        let n = |id, predecessor| Node {
            id,
            component: "calisoga.vector.normalize".into(),
            inputs: BTreeMap::from([
                ("v".into(), InputBinding::Output {node:predecessor}),
            ]),
        };
        let cyclic = Graph {version:1, nodes:vec![n(1,2),n(2,1)], outputs:vec![]};
        assert_eq!(evaluate(&cyclic), Err(GraphError::Cycle));
        let dangling = Graph {version:1, nodes:vec![n(1,99)], outputs:vec![]};
        assert_eq!(evaluate(&dangling), Err(GraphError::MissingNode(99)));
    }
    #[test]
    fn connection_type_checks_run_before_evaluation() {
        let n = Node {
            id: 2,
            component: "calisoga.vector.length".into(),
            inputs: BTreeMap::from([
                ("v".into(), InputBinding::Output { node: 1 }),
            ]),
        };
        let graph = Graph {
            version: 1,
            nodes: vec![dist(1, Vec3::ZERO, Vec3::new(1.,0.,0.)),n],
            outputs: vec![],
        };
        assert_eq!(evaluate(&graph), Err(GraphError::Type {
            node:2, port:"v".into()
        }));
    }
    #[test]
    fn invalid_modifier_propagates_kernel_error_atomically() {
        let graph = Graph {
            version: 1,
            nodes: vec![
                dist(1, Vec3::ZERO, Vec3::new(1., 0., 0.)),
                Node {
                    id: 2,
                    component: "calisoga.polyline.divide_count".into(),
                    inputs: BTreeMap::from([
                        ("points".into(), constant(Polyline(vec![
                            Vec3::ZERO, Vec3::new(3., 0., 0.),
                        ]))),
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
        let g = Graph {version:1, nodes:vec![
            dist(7,Vec3::ZERO,Vec3::Z),
            dist(7,Vec3::ZERO,Vec3::Z),
        ], outputs:vec![]};
        assert_eq!(evaluate(&g), Err(GraphError::NodeId(7)));
        let g = Graph {version:2,nodes:vec![],outputs:vec![]};
        assert_eq!(evaluate(&g), Err(GraphError::Version));
    }
    #[test]
    fn roundtrip_and_node_order_are_deterministic() {
        let graph = Graph { version:1, nodes:vec![
            dist(11,Vec3::ZERO,Vec3::new(0.,3.,4.)),
            dist(2,Vec3::ZERO,Vec3::new(3.,4.,0.)),
        ], outputs:vec![] };
        let json = serde_json::to_string(&graph).unwrap();
        let restored: Graph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, restored);
        let mut reordered=restored.clone();
        reordered.nodes.reverse();
        assert_eq!(evaluate(&restored), evaluate(&reordered));
    }
}
