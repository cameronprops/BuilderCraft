//! Transactional, UI-independent editing for an OrbWeaver graph.
//!
//! This module stores only graph semantics and optional node locations.
//! It can be used by an egui canvas, scripts, headless tests and APIs without
//! creating an alternate geometry evaluator or tying the graph to a CAD host.

use std::collections::{BTreeMap, BTreeSet};

use buildercraft_kernel::{DataTree, ToolType, ToolValue, TreeMatchPolicy, tool_output_may_match_port, tool_value_matches_port};
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};

use crate::{
    GRAPH_SCHEMA_VERSION, Graph, GraphError, GraphResult, InputBinding, MAX_GRAPH_CONNECTIONS, MAX_GRAPH_NODES, Node, evaluate, native_components,
};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodePosition {
    pub x: f32,
    pub y: f32,
}

impl NodePosition {
    fn valid(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.x.abs() <= 1_000_000.0 && self.y.abs() <= 1_000_000.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorDocument {
    pub graph: Graph,
    /// Canvas positions are presentation-only and never modify evaluation.
    pub positions: BTreeMap<u64, NodePosition>,
    /// Monotonically increasing ID seed. Deleted IDs are never reused.
    #[serde(default = "first_node_id")]
    pub next_id: u64,
}

fn first_node_id() -> u64 {
    1
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EditorError {
    #[error("unsupported or invalid graph schema")]
    Schema,
    #[error("OrbWeaver edit exceeds graph resource limits")]
    Budget,
    #[error("node identity missing or duplicated: {0}")]
    Node(u64),
    #[error("unknown OrbWeaver component: {0}")]
    Component(String),
    #[error("invalid input port on node {node}: {port}")]
    Port { node: u64, port: String },
    #[error("input or output type mismatch on node {node}: {port}")]
    Type { node: u64, port: String },
    #[error("the graph would contain a dependency cycle")]
    Cycle,
    #[error("invalid canvas node location")]
    Position,
    #[error("graph evaluation failed: {0}")]
    Evaluation(#[from] GraphError),
}

pub type Result<T> = std::result::Result<T, EditorError>;

impl Default for EditorDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorDocument {
    pub fn new() -> Self {
        Self { graph: Graph { version: GRAPH_SCHEMA_VERSION, nodes: Vec::new(), outputs: Vec::new() }, positions: BTreeMap::new(), next_id: 1 }
    }

    pub fn from_graph(graph: Graph) -> Result<Self> {
        let next_id = graph.nodes.iter().map(|n| n.id).max().unwrap_or(0).checked_add(1).ok_or(EditorError::Budget)?;
        let document = Self { graph, positions: BTreeMap::new(), next_id };
        document.validate()?;
        Ok(document)
    }

    /// Evaluate the actual OrbWeaver graph through its existing shared kernel.
    pub fn evaluate(&self) -> Result<GraphResult> {
        self.validate()?;
        Ok(evaluate(&self.graph)?)
    }

    /// Editing is all-or-nothing. A failed change does not alter graph, positions or outputs.
    fn transact<T>(&mut self, edit: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let mut draft = self.clone();
        let output = edit(&mut draft)?;
        draft.validate()?;
        *self = draft;
        Ok(output)
    }

    pub fn add(&mut self, component: &str, position: NodePosition) -> Result<u64> {
        if !position.valid() {
            return Err(EditorError::Position);
        }
        let contract = native_components().iter().find(|p| p.orbweaver_node == component).ok_or_else(|| EditorError::Component(component.into()))?;
        self.transact(|draft| {
            if draft.graph.nodes.len() >= MAX_GRAPH_NODES {
                return Err(EditorError::Budget);
            }
            let id = draft.next_id;
            draft.next_id = draft.next_id.checked_add(1).ok_or(EditorError::Budget)?;
            let mut inputs = BTreeMap::new();
            for port in contract.inputs {
                let value = default_value(port.kind).ok_or_else(|| EditorError::Port { node: id, port: port.name.into() })?;
                inputs.insert(port.name.into(), InputBinding::Constant { value });
            }
            draft.graph.nodes.push(Node { id, component: component.into(), inputs, matching: TreeMatchPolicy::Shortest });
            draft.graph.outputs.push(id);
            draft.positions.insert(id, position);
            Ok(id)
        })
    }

    pub fn move_node(&mut self, id: u64, position: NodePosition) -> Result<()> {
        if !position.valid() {
            return Err(EditorError::Position);
        }
        self.transact(|draft| {
            if !draft.graph.nodes.iter().any(|n| n.id == id) {
                return Err(EditorError::Node(id));
            }
            draft.positions.insert(id, position);
            Ok(())
        })
    }

    pub fn set_literal(&mut self, id: u64, port: &str, value: ToolValue) -> Result<()> {
        self.transact(|draft| {
            let node = draft.graph.nodes.iter_mut().find(|n| n.id == id).ok_or(EditorError::Node(id))?;
            let spec = native_components()
                .iter()
                .find(|p| p.orbweaver_node == node.component)
                .ok_or_else(|| EditorError::Component(node.component.clone()))?;
            let input = spec.inputs.iter().find(|p| p.name == port).ok_or_else(|| EditorError::Port { node: id, port: port.into() })?;
            if tool_value_matches_port(input.kind, &value).is_err() {
                return Err(EditorError::Type { node: id, port: port.into() });
            }
            node.inputs.insert(port.into(), InputBinding::Constant { value });
            Ok(())
        })
    }

    pub fn connect(&mut self, from: u64, to: u64, port: &str) -> Result<()> {
        self.transact(|draft| {
            let source = draft.graph.nodes.iter().find(|n| n.id == from).ok_or(EditorError::Node(from))?;
            let source_spec = native_components()
                .iter()
                .find(|p| p.orbweaver_node == source.component)
                .ok_or_else(|| EditorError::Component(source.component.clone()))?;
            let target = draft.graph.nodes.iter_mut().find(|n| n.id == to).ok_or(EditorError::Node(to))?;
            let target_spec = native_components()
                .iter()
                .find(|p| p.orbweaver_node == target.component)
                .ok_or_else(|| EditorError::Component(target.component.clone()))?;
            let destination = target_spec.inputs.iter().find(|p| p.name == port).ok_or_else(|| EditorError::Port { node: to, port: port.into() })?;
            if !tool_output_may_match_port(destination.kind, source_spec.output) {
                return Err(EditorError::Type { node: to, port: port.into() });
            }
            target.inputs.insert(port.into(), InputBinding::Output { node: from });
            Ok(())
        })
    }

    pub fn disconnect(&mut self, id: u64, port: &str) -> Result<()> {
        self.transact(|draft| {
            let node = draft.graph.nodes.iter_mut().find(|n| n.id == id).ok_or(EditorError::Node(id))?;
            let spec = native_components()
                .iter()
                .find(|p| p.orbweaver_node == node.component)
                .ok_or_else(|| EditorError::Component(node.component.clone()))?;
            let input = spec.inputs.iter().find(|p| p.name == port).ok_or_else(|| EditorError::Port { node: id, port: port.into() })?;
            let value = default_value(input.kind).ok_or_else(|| EditorError::Port { node: id, port: port.into() })?;
            node.inputs.insert(port.into(), InputBinding::Constant { value });
            Ok(())
        })
    }

    /// Removing a node detaches downstream wires to typed editable literals.
    pub fn remove(&mut self, id: u64) -> Result<()> {
        self.transact(|draft| {
            let before = draft.graph.nodes.len();
            draft.graph.nodes.retain(|n| n.id != id);
            if draft.graph.nodes.len() == before {
                return Err(EditorError::Node(id));
            }
            for node in &mut draft.graph.nodes {
                let spec = native_components()
                    .iter()
                    .find(|p| p.orbweaver_node == node.component)
                    .ok_or_else(|| EditorError::Component(node.component.clone()))?;
                for input in spec.inputs {
                    if matches!(node.inputs.get(input.name), Some(InputBinding::Output { node: predecessor }) if *predecessor == id) {
                        let value = default_value(input.kind).ok_or_else(|| EditorError::Port { node: node.id, port: input.name.into() })?;
                        node.inputs.insert(input.name.into(), InputBinding::Constant { value });
                    }
                }
            }
            draft.positions.remove(&id);
            draft.graph.outputs.retain(|output| *output != id);
            Ok(())
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.graph.version != GRAPH_SCHEMA_VERSION {
            return Err(EditorError::Schema);
        }
        if self.graph.nodes.len() > MAX_GRAPH_NODES || self.graph.outputs.len() > MAX_GRAPH_NODES {
            return Err(EditorError::Budget);
        }
        if self.next_id == 0 || self.graph.nodes.iter().any(|n| n.id >= self.next_id) {
            return Err(EditorError::Schema);
        }
        let mut ids = BTreeMap::new();
        for node in &self.graph.nodes {
            if node.id == 0 || ids.insert(node.id, node).is_some() {
                return Err(EditorError::Node(node.id));
            }
            let spec = native_components()
                .iter()
                .find(|p| p.orbweaver_node == node.component)
                .ok_or_else(|| EditorError::Component(node.component.clone()))?;
            if node.inputs.len() != spec.inputs.len() {
                return Err(EditorError::Port { node: node.id, port: "inputs".into() });
            }
        }
        let mut edge_count = 0usize;
        for node in &self.graph.nodes {
            let spec = native_components()
                .iter()
                .find(|p| p.orbweaver_node == node.component)
                .ok_or_else(|| EditorError::Component(node.component.clone()))?;
            for input in spec.inputs {
                let binding = node.inputs.get(input.name).ok_or_else(|| EditorError::Port { node: node.id, port: input.name.into() })?;
                match binding {
                    InputBinding::Constant { value } => {
                        if tool_value_matches_port(input.kind, value).is_err() {
                            return Err(EditorError::Type { node: node.id, port: input.name.into() });
                        }
                    }
                    InputBinding::Output { node: source } => {
                        edge_count = edge_count.checked_add(1).ok_or(EditorError::Budget)?;
                        if edge_count > MAX_GRAPH_CONNECTIONS {
                            return Err(EditorError::Budget);
                        }
                        let predecessor = ids.get(source).ok_or(EditorError::Node(*source))?;
                        let source_spec = native_components()
                            .iter()
                            .find(|p| p.orbweaver_node == predecessor.component)
                            .ok_or_else(|| EditorError::Component(predecessor.component.clone()))?;
                        if !tool_output_may_match_port(input.kind, source_spec.output) {
                            return Err(EditorError::Type { node: node.id, port: input.name.into() });
                        }
                    }
                }
            }
        }
        if self.positions.iter().any(|(id, p)| !ids.contains_key(id) || !p.valid()) {
            return Err(EditorError::Position);
        }
        for id in &self.graph.outputs {
            if !ids.contains_key(id) {
                return Err(EditorError::Node(*id));
            }
        }
        // Bounded, iterative topological check: no recursion or hidden geometry evaluation.
        let mut completed = BTreeSet::new();
        for _ in 0..ids.len() {
            let mut progress = false;
            for (id, node) in &ids {
                if completed.contains(id) {
                    continue;
                }
                if node.inputs.values().all(|b| match b {
                    InputBinding::Constant { .. } => true,
                    InputBinding::Output { node: source } => completed.contains(source),
                }) {
                    completed.insert(*id);
                    progress = true;
                }
            }
            if completed.len() == ids.len() {
                break;
            }
            if !progress {
                return Err(EditorError::Cycle);
            }
        }
        Ok(())
    }
}

/// Defaults initialize a typed editable port. They are not a promise that a
/// component's numerical operation can evaluate without user-supplied inputs.
fn default_value(kind: ToolType) -> Option<ToolValue> {
    match kind {
        ToolType::Number => Some(ToolValue::Number(1.0)),
        ToolType::Count => Some(ToolValue::Count(2)),
        ToolType::Point => Some(ToolValue::Point(Vec3::ZERO)),
        ToolType::Vector => Some(ToolValue::Vector(Vec3::new(1.0, 0.0, 0.0))),
        ToolType::Polyline => Some(ToolValue::Polyline(vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)])),
        ToolType::Tree => Some(ToolValue::Tree(DataTree { branches: Vec::new() })),
        ToolType::MatchMode => Some(ToolValue::MatchMode(TreeMatchPolicy::Shortest)),
        ToolType::Pair => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn place(x: f32) -> NodePosition {
        NodePosition { x, y: 10.0 }
    }

    #[test]
    fn add_evaluate_and_move_does_not_change_numerics() {
        let mut d = EditorDocument::new();
        let id = d.add("orbweaver.point.distance", place(10.0)).unwrap();
        d.set_literal(id, "a", ToolValue::Point(Vec3::new(3.0, 4.0, 0.0))).unwrap();
        let result = d.evaluate().unwrap();
        assert_eq!(result.values.get(&id), Some(&ToolValue::Number(5.0)));
        d.move_node(id, place(120.0)).unwrap();
        assert_eq!(d.evaluate().unwrap(), result);
        assert_eq!(d.positions.get(&id), Some(&place(120.0)));
    }

    #[test]
    fn connects_and_disconnects_real_typed_kernel_nodes() {
        let mut d = EditorDocument::new();
        let id = d.add("orbweaver.point.midpoint", place(0.0)).unwrap();
        d.set_literal(id, "b", ToolValue::Point(Vec3::new(10.0, 0.0, 0.0))).unwrap();
        let target = d.add("orbweaver.point.distance", place(100.0)).unwrap();
        d.connect(id, target, "a").unwrap();
        d.set_literal(target, "b", ToolValue::Point(Vec3::new(0.0, 0.0, 0.0))).unwrap();
        assert_eq!(d.evaluate().unwrap().values.get(&target), Some(&ToolValue::Number(5.0)));
        d.disconnect(target, "a").unwrap();
        assert_eq!(d.evaluate().unwrap().values.get(&target), Some(&ToolValue::Number(0.0)));
    }

    #[test]
    fn type_and_cycle_errors_roll_back_atomically() {
        let mut d = EditorDocument::new();
        let a = d.add("orbweaver.point.midpoint", place(1.0)).unwrap();
        let b = d.add("orbweaver.point.midpoint", place(2.0)).unwrap();
        let before = d.clone();
        assert_eq!(d.set_literal(a, "a", ToolValue::Number(8.0)), Err(EditorError::Type { node: a, port: "a".into() }));
        assert_eq!(d, before);
        d.connect(a, b, "a").unwrap();
        let before = d.clone();
        assert_eq!(d.connect(b, a, "b"), Err(EditorError::Cycle));
        assert_eq!(d, before);
        assert!(d.move_node(a, NodePosition { x: f32::NAN, y: 0.0 }).is_err());
        assert_eq!(d, before);
    }

    #[test]
    fn delete_repairs_dependents_and_roundtrips_layout() {
        let mut d = EditorDocument::new();
        let a = d.add("orbweaver.point.midpoint", place(1.0)).unwrap();
        let b = d.add("orbweaver.point.distance", place(2.0)).unwrap();
        d.connect(a, b, "a").unwrap();
        d.remove(a).unwrap();
        assert_eq!(d.graph.nodes.len(), 1);
        assert_eq!(d.graph.nodes[0].inputs.get("a"), Some(&InputBinding::Constant { value: ToolValue::Point(Vec3::ZERO) }));
        assert!(!d.positions.contains_key(&a));
        assert_eq!(d.graph.outputs, vec![b]);
        let c = d.add("orbweaver.point.midpoint", place(3.0)).unwrap();
        assert!(c > b, "deleted node identities must not be recycled");
        let encoded = serde_json::to_string(&d).unwrap();
        let restored: EditorDocument = serde_json::from_str(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored, d);
    }

    #[test]
    fn budget_and_bad_component_are_rejected() {
        let mut d = EditorDocument::new();
        assert_eq!(d.add("orbweaver.missing", place(0.0)), Err(EditorError::Component("orbweaver.missing".into())));
        assert_eq!(d.remove(42), Err(EditorError::Node(42)));
        assert_eq!(d, EditorDocument::new());
        let mut invalid = Graph { version: GRAPH_SCHEMA_VERSION + 1, nodes: Vec::new(), outputs: Vec::new() };
        assert_eq!(EditorDocument::from_graph(invalid.clone()), Err(EditorError::Schema));
        invalid.version = GRAPH_SCHEMA_VERSION;
        assert!(EditorDocument::from_graph(invalid).is_ok());
    }
}
