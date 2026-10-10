//! Native OrbWeaver visual editor, backed by the one authoritative graph.
//!
//! Snarl owns only presentation nodes, wires and viewport interaction.
//! All semantic edits are validated by orbweaver::editor::EditorDocument,
//! and all mathematical execution uses orbweaver::evaluate.

use buildercraft_kernel::{SharedToolContract, ToolType, ToolValue};
use egui::{Color32, Id, Ui, pos2, vec2};
use egui_snarl::{
    InPin, InPinId, NodeId, OutPin, Snarl,
    ui::{PinInfo, SnarlViewer, SnarlWidget},
};
use orbweaver::{
    InputBinding,
    editor::{EditorDocument, NodePosition},
    native_components,
};

#[derive(Clone, Debug)]
pub struct CanvasNode {
    graph_id: u64,
    component: String,
}

pub struct OrbCanvas {
    pub semantic: EditorDocument,
    visual: Snarl<CanvasNode>,
    message: String,
}

fn spec(component: &str) -> Option<&'static SharedToolContract> {
    native_components().iter().find(|c| c.orbweaver_node == component)
}

fn pin_color(ty: ToolType) -> Color32 {
    match ty {
        ToolType::Point | ToolType::Vector => Color32::from_rgb(101, 198, 235),
        ToolType::Number | ToolType::Count => Color32::from_rgb(244, 189, 107),
        ToolType::Polyline => Color32::from_rgb(152, 206, 157),
        ToolType::Tree | ToolType::Pair | ToolType::MatchMode => Color32::from_rgb(175, 148, 229),
    }
}

impl Default for OrbCanvas {
    fn default() -> Self {
        let mut result = Self { semantic: EditorDocument::new(), visual: Snarl::new(), message: "Right-click canvas to add nodes".into() };
        result.add_node("orbweaver.point.midpoint", pos2(50.0, 80.0));
        result.add_node("orbweaver.point.distance", pos2(370.0, 100.0));
        let from = result.visual.nodes_ids_data().find(|(_, n)| n.value.component == "orbweaver.point.midpoint").map(|(_, n)| n.value.graph_id);
        let to = result.visual.nodes_ids_data().find(|(_, n)| n.value.component == "orbweaver.point.distance").map(|(_, n)| n.value.graph_id);
        if let (Some(from), Some(to)) = (from, to) {
            if result.semantic.connect(from, to, "a").is_ok() {
                let from_visual = result.visual.nodes_ids_data().find(|(_, n)| n.value.graph_id == from).map(|(id, _)| id);
                let to_visual = result.visual.nodes_ids_data().find(|(_, n)| n.value.graph_id == to).map(|(id, _)| id);
                if let (Some(a), Some(b)) = (from_visual, to_visual) {
                    result.visual.connect(egui_snarl::OutPinId { node: a, output: 0 }, InPinId { node: b, input: 0 });
                }
            }
        }
        result
    }
}

impl OrbCanvas {
    fn add_node(&mut self, component: &str, at: egui::Pos2) {
        match self.semantic.add(component, NodePosition { x: at.x, y: at.y }) {
            Ok(id) => {
                self.visual.insert_node(at, CanvasNode { graph_id: id, component: component.into() });
                self.message = format!("Added {component} (#{id})");
            }
            Err(e) => self.message = e.to_string(),
        }
    }

    /// A node removal or connection is never accepted in the visual graph
    /// unless the same operation succeeded in the authoritative domain graph.
    pub fn show(&mut self, ctx: &egui::Context, open: &mut bool) {
        egui::Window::new("OrbWeaver · Native Alpha").open(open).default_size(vec2(920.0, 610.0)).show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Add Point Midpoint").clicked() {
                    self.add_node("orbweaver.point.midpoint", pos2(80.0, 120.0));
                }
                if ui.button("Add Distance").clicked() {
                    self.add_node("orbweaver.point.distance", pos2(380.0, 120.0));
                }
                if ui.button("Evaluate").clicked() {
                    match self.semantic.evaluate() {
                        Ok(output) => {
                            self.message = format!("Evaluated {} nodes; {} output values", output.evaluated_node_count, output.values.len())
                        }
                        Err(e) => self.message = format!("Evaluation: {e}"),
                    }
                }
                if ui.button("Copy Graph JSON").clicked() {
                    match serde_json::to_string_pretty(&self.semantic) {
                        Ok(json) => {
                            ui.ctx().copy_text(json);
                            self.message = "Copied the graph and layout JSON to clipboard".into();
                        }
                        Err(e) => self.message = format!("Serialization: {e}"),
                    }
                }
            });
            ui.label("Drag nodes and typed wires; right-click empty canvas to add a node, or right-click a node to delete it.");
            ui.label(&self.message);
            ui.separator();
            let mut viewer = CanvasViewer { editor: &mut self.semantic, message: &mut self.message };
            SnarlWidget::new().id(Id::new("orbweaver_native_canvas")).show(&mut self.visual, &mut viewer, ui);

            // Presentation positions are copied into independent editor metadata.
            // There is no evaluation or document revision change on drag.
            for (_, visual_node) in self.visual.nodes_ids_data() {
                let position = NodePosition { x: visual_node.pos.x, y: visual_node.pos.y };
                if self.semantic.positions.get(&visual_node.value.graph_id) != Some(&position) {
                    if let Err(e) = self.semantic.move_node(visual_node.value.graph_id, position) {
                        self.message = format!("Node move rejected: {e}");
                    }
                }
            }
        });
    }
}

struct CanvasViewer<'a> {
    editor: &'a mut EditorDocument,
    message: &'a mut String,
}

impl CanvasViewer<'_> {
    fn input_name(&self, snarl: &Snarl<CanvasNode>, pin: InPinId) -> Option<&'static str> {
        spec(&snarl[pin.node].component)?.inputs.get(pin.input).map(|p| p.name)
    }

    fn remove_input_wire(&mut self, to: InPinId, snarl: &mut Snarl<CanvasNode>) {
        let Some(port) = self.input_name(snarl, to) else { return };
        let target = snarl[to.node].graph_id;
        match self.editor.disconnect(target, port) {
            Ok(()) => {
                snarl.drop_inputs(to);
                *self.message = format!("Disconnected {port} on node {target}");
            }
            Err(e) => *self.message = e.to_string(),
        }
    }
}

impl SnarlViewer<CanvasNode> for CanvasViewer<'_> {
    fn title(&mut self, node: &CanvasNode) -> String {
        format!("{} · #{}", node.component.trim_start_matches("orbweaver."), node.graph_id)
    }

    fn inputs(&mut self, node: &CanvasNode) -> usize {
        spec(&node.component).map_or(0, |s| s.inputs.len())
    }

    fn outputs(&mut self, node: &CanvasNode) -> usize {
        usize::from(spec(&node.component).is_some())
    }

    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<CanvasNode>) -> PinInfo {
        let n = &snarl[pin.id.node];
        let Some(port) = spec(&n.component).and_then(|s| s.inputs.get(pin.id.input)) else {
            ui.weak("Unknown port");
            return PinInfo::circle();
        };
        let id = n.graph_id;
        let name = port.name;
        let kind = port.kind;
        ui.label(name);
        if !pin.remotes.is_empty() {
            ui.weak("linked");
        } else {
            let original = self.editor.graph.nodes.iter().find(|node| node.id == id).and_then(|node| node.inputs.get(name)).cloned();
            if let Some(InputBinding::Constant { mut value }) = original {
                let changed = match &mut value {
                    ToolValue::Number(n) => ui.add(egui::DragValue::new(n).speed(0.1)).changed(),
                    ToolValue::Count(n) => ui.add(egui::DragValue::new(n).speed(1).range(1..=100_000)).changed(),
                    ToolValue::Point(v) | ToolValue::Vector(v) => {
                        ui.horizontal(|ui| {
                            let x = ui.add(egui::DragValue::new(&mut v.x).speed(0.1).max_decimals(2)).changed();
                            let y = ui.add(egui::DragValue::new(&mut v.y).speed(0.1).max_decimals(2)).changed();
                            let z = ui.add(egui::DragValue::new(&mut v.z).speed(0.1).max_decimals(2)).changed();
                            x || y || z
                        })
                        .inner
                    }
                    _ => {
                        ui.weak(format!("{kind:?} (editable via API)"));
                        false
                    }
                };
                if changed {
                    if let Err(e) = self.editor.set_literal(id, name, value) {
                        *self.message = format!("Input rejected: {e}");
                    }
                }
            }
        }
        PinInfo::circle().with_fill(pin_color(kind))
    }

    fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<CanvasNode>) -> PinInfo {
        let kind = spec(&snarl[pin.id.node].component).map(|s| s.output);
        if let Some(kind) = kind {
            ui.label(format!("{kind:?}"));
            PinInfo::circle().with_fill(pin_color(kind))
        } else {
            ui.weak("Unknown output");
            PinInfo::circle()
        }
    }

    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<CanvasNode>) {
        let Some(port) = self.input_name(snarl, to.id) else {
            *self.message = "Unknown target port".into();
            return;
        };
        let source = snarl[from.id.node].graph_id;
        let target = snarl[to.id.node].graph_id;
        match self.editor.connect(source, target, port) {
            Ok(()) => {
                for &old in &to.remotes {
                    snarl.disconnect(old, to.id);
                }
                snarl.connect(from.id, to.id);
                *self.message = format!("Connected node {source} to {target}.{port}");
            }
            Err(e) => *self.message = format!("Connection rejected: {e}"),
        }
    }

    fn disconnect(&mut self, _from: &OutPin, to: &InPin, snarl: &mut Snarl<CanvasNode>) {
        self.remove_input_wire(to.id, snarl);
    }

    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<CanvasNode>) {
        self.remove_input_wire(pin.id, snarl);
    }

    fn drop_outputs(&mut self, pin: &OutPin, snarl: &mut Snarl<CanvasNode>) {
        for &to in &pin.remotes {
            self.remove_input_wire(to, snarl);
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<CanvasNode>) -> bool {
        true
    }

    fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut Ui, snarl: &mut Snarl<CanvasNode>) {
        ui.label("Add OrbWeaver component");
        egui::ScrollArea::vertical().max_height(270.0).show(ui, |ui| {
            for component in native_components() {
                let label = component.orbweaver_node.trim_start_matches("orbweaver.");
                if ui.button(label).clicked() {
                    match self.editor.add(component.orbweaver_node, NodePosition { x: pos.x, y: pos.y }) {
                        Ok(id) => {
                            snarl.insert_node(pos, CanvasNode { graph_id: id, component: component.orbweaver_node.into() });
                            *self.message = format!("Added {label}");
                        }
                        Err(e) => *self.message = format!("Add rejected: {e}"),
                    }
                    ui.close();
                }
            }
        });
    }

    fn has_node_menu(&mut self, _node: &CanvasNode) -> bool {
        true
    }

    fn show_node_menu(&mut self, node: NodeId, _inputs: &[InPin], _outputs: &[OutPin], ui: &mut Ui, snarl: &mut Snarl<CanvasNode>) {
        if ui.button("Delete node").clicked() {
            let id = snarl[node].graph_id;
            match self.editor.remove(id) {
                Ok(()) => {
                    snarl.remove_node(node);
                    // The semantic editor detached all downstream links.
                    // Removing the matching Snarl source also removes its wires.
                    *self.message = format!("Removed node {id}");
                }
                Err(e) => *self.message = format!("Delete rejected: {e}"),
            }
            ui.close();
        }
    }
}
