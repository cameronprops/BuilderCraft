//! Optional, scoped feature-history browser in the Worldwright 3D model view.
//! All changes go through undoable engine commands, never direct UI mutation.
use crate::CadApp;
use buildercraft_kernel::{FeatureScope, FeatureTimeline, ToolValue};
use serde_json::{json, Value};

fn scope_label(scope: &FeatureScope) -> String {
    match scope {
        FeatureScope::Document => "Document".into(),
        FeatureScope::ModelNode(id) => format!("Model node {id}"),
        FeatureScope::BlockDefinition(name) => format!("Block {name}"),
    }
}
fn edit(app: &mut CadApp, history: &FeatureTimeline, change: Value) {
    let _ = app.run("worldwright.history.edit", json!({
        "scope": history.scope,
        "expected_revision": history.revision,
        "change": change,
    }));
}

pub fn panel(app: &mut CadApp, ui: &mut egui::Ui) {
    egui::CollapsingHeader::new("Parametric feature timelines")
        .id_salt("worldwright_feature_histories")
        .default_open(false)
        .show(ui, |ui| {
            ui.small("Opt-in: direct-modeled objects do not need histories.");
            let Ok(d) = app.session.doc() else { return };
            let histories = d.feature_timelines.clone();
            let nodes: Vec<_> = d.organization.nodes.iter()
                .map(|node| (node.id, node.name.clone())).collect();
            let blocks: Vec<_> = d.blocks.values()
                .filter(|block| !block.anonymous)
                .map(|block| block.name.clone()).collect();

            if !histories.iter().any(|h| h.scope == FeatureScope::Document)
                && ui.button("Enable document timeline").clicked()
            {
                let _ = app.run("worldwright.history.create",
                    json!({"scope": FeatureScope::Document}));
            }
            ui.collapsing("Enable local timeline", |ui| {
                for (id, name) in nodes.iter().take(32) {
                    if histories.iter().any(|h| h.scope == FeatureScope::ModelNode(*id)) {
                        continue;
                    }
                    if ui.button(format!("Component/body: {name}")).clicked() {
                        let _ = app.run("worldwright.history.create", json!({
                            "scope": FeatureScope::ModelNode(*id),
                        }));
                    }
                }
                for name in blocks.iter().take(32) {
                    let scope = FeatureScope::BlockDefinition(name.clone());
                    if histories.iter().any(|h| h.scope == scope) {
                        continue;
                    }
                    if ui.button(format!("Reusable block: {name}")).clicked() {
                        let _ = app.run("worldwright.history.create", json!({
                            "scope": scope,
                        }));
                    }
                }
            });
            if histories.is_empty() {
                ui.small("No feature histories yet. Direct geometry stays editable.");
            }
            for history in &histories {
                egui::CollapsingHeader::new(format!(
                    "{} ({} steps)", scope_label(&history.scope), history.steps.len(),
                ))
                .id_salt(("feature_history", format!("{:?}", history.scope)))
                .show(ui, |ui| {
                    ui.label(format!("History revision {}", history.revision));
                    ui.horizontal(|ui| {
                        if ui.button("Recompute").clicked() {
                            let _ = app.run("worldwright.history.evaluate",
                                json!({"scope": history.scope}));
                        }
                        if history.rollback_after.is_some()
                            && ui.button("Run to end").clicked()
                        {
                            edit(app, history, json!({
                                "edit":"set_rollback", "after": null
                            }));
                        }
                    });
                    ui.collapsing("Local parameters", |ui| {
                        if history.parameters.is_empty() {
                            ui.small("No parameters yet. Define one with the history.edit API.");
                        }
                        for (name, source) in &history.parameters {
                            let mut updated = source.clone();
                            let mut changed = false;
                            ui.horizontal(|ui| {
                                ui.monospace(name);
                                match &mut updated {
                                    ToolValue::Number(value) => {
                                        changed = ui.add(egui::DragValue::new(value).speed(0.1)).changed();
                                    }
                                    ToolValue::Count(value) => {
                                        changed = ui.add(egui::DragValue::new(value)).changed();
                                    }
                                    ToolValue::Point(value) | ToolValue::Vector(value) => {
                                        for component in [&mut value.x, &mut value.y, &mut value.z] {
                                            changed |= ui.add(egui::DragValue::new(component).speed(0.1)).changed();
                                        }
                                    }
                                    _ => {
                                        ui.small("Structured value. Edit through history API.");
                                    }
                                }
                            });
                            if changed {
                                edit(app, history, json!({
                                    "edit":"set_parameter",
                                    "name":name,
                                    "value":updated
                                }));
                            }
                        }
                    });
                    if history.steps.is_empty() {
                        ui.small("Append a feature with worldwright.history.edit.");
                    }
                    for (index, step) in history.steps.iter().enumerate() {
                        ui.horizontal_wrapped(|ui| {
                            let mut active = !step.suppressed;
                            if ui.checkbox(&mut active,
                                format!("{}: {}", step.id, step.name)).changed()
                            {
                                edit(app, history, json!({
                                    "edit":"set_suppressed",
                                    "id":step.id,
                                    "suppressed":!active,
                                }));
                            }
                            ui.small(step.operation.as_str());
                            if index > 0 && ui.small_button("Up").clicked() {
                                edit(app, history, json!({
                                    "edit":"reorder",
                                    "id":step.id,
                                    "before":history.steps[index-1].id,
                                }));
                            }
                            if index + 1 < history.steps.len()
                                && ui.small_button("Down").clicked()
                            {
                                let before = history.steps.get(index+2).map(|s| s.id);
                                edit(app, history, json!({
                                    "edit":"reorder",
                                    "id":step.id,
                                    "before":before,
                                }));
                            }
                            let at = history.rollback_after == Some(step.id);
                            if ui.selectable_label(at, "Roll here").clicked() {
                                edit(app, history, json!({
                                    "edit":"set_rollback",
                                    "after":if at {None} else {Some(step.id)},
                                }));
                            }
                        });
                    }
                    ui.small("Recompute reports blocked or invalid dependencies. Preview/bake is not yet available.");
                });
            }
            ui.small("Sketch solids and the visual feature editor are later dependencies.");
        });
}
