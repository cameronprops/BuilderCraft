//! Optional scoped parametric feature histories. The recipe is undoable CAD
//! document state; feature math is executed only by the shared kernel (also
//! used by OrbWeaver). No unsupported solid features are marked executable.
use super::*;
use buildercraft_kernel::{FeatureHistoryEdit, FeatureScope, FeatureTimeline};
use serde_json::{Value, json};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("worldwright.history.create", "Create Feature Timeline", create)
            .params("{scope:{kind:document|model_node|block_definition,id?}}"),
        CommandSpec::new("worldwright.history.edit", "Edit Feature Timeline", edit)
            .params("{scope,expected_revision,change:{edit:append|set_parameter|set_input|set_suppressed|reorder|set_rollback,...}}"),
        CommandSpec::new("worldwright.history.list", "List Feature Timelines", list).noundo(),
        CommandSpec::new("worldwright.history.inspect", "Inspect Feature Timeline", inspect).params("{scope}").noundo(),
        CommandSpec::new("worldwright.history.evaluate", "Recompute Feature Timeline", evaluate).params("{scope}").noundo(),
    ]
}

fn scope_from(session: &Session, p: &Value) -> Result<FeatureScope> {
    let value = p.get("scope").ok_or_else(|| bad("worldwright.history", "scope is required"))?;
    let scope: FeatureScope = serde_json::from_value(value.clone()).map_err(|e| bad("worldwright.history", e.to_string()))?;
    // CAD block names are case-insensitive. Normalize to the canonical
    // definition name before finding a local history or creating one.
    if let FeatureScope::BlockDefinition(name) = &scope
        && let Some(block) = session.doc()?.block(name)
    {
        return Ok(FeatureScope::BlockDefinition(block.name.clone()));
    }
    Ok(scope)
}
fn find<'a>(d: &'a cadcraft_doc::Drawing, scope: &FeatureScope) -> Result<&'a FeatureTimeline> {
    d.feature_timelines.iter().find(|history| &history.scope == scope).ok_or_else(|| bad("worldwright.history", "no timeline in requested scope"))
}

fn create(session: &mut Session, p: &Value) -> Result<Value> {
    let scope = scope_from(session, p)?;
    let history = FeatureTimeline::new(scope.clone()).map_err(|e| bad("worldwright.history.create", e.to_string()))?;
    // Validate ownership and duplicates BEFORE copy-on-write document edit.
    let mut probe = session.doc()?.clone();
    probe.feature_timelines.push(history.clone());
    probe.validate_feature_histories().map_err(|e| bad("worldwright.history.create", e.to_string()))?;
    session.doc_mut()?.feature_timelines.push(history);
    Ok(json!({"scope":scope,"revision":0}))
}

fn edit(session: &mut Session, p: &Value) -> Result<Value> {
    let scope = scope_from(session, p)?;
    let expected =
        p.get("expected_revision").and_then(Value::as_u64).ok_or_else(|| bad("worldwright.history.edit", "expected_revision is required"))?;
    let change: FeatureHistoryEdit =
        serde_json::from_value(p.get("change").cloned().ok_or_else(|| bad("worldwright.history.edit", "change is required"))?)
            .map_err(|e| bad("worldwright.history.edit", e.to_string()))?;
    let mut candidate = find(session.doc()?, &scope)?.clone();
    let revision = candidate.apply(expected, change).map_err(|e| bad("worldwright.history.edit", e.to_string()))?;
    // Scope is retained, but revalidate against the current drawing in case
    // a block or component was removed by another command.
    let current = session.doc()?;
    let index = current
        .feature_timelines
        .iter()
        .position(|h| h.scope == scope)
        .ok_or_else(|| bad("worldwright.history.edit", "timeline no longer exists"))?;
    let mut probe = current.clone();
    probe.feature_timelines[index] = candidate.clone();
    probe.validate_feature_histories().map_err(|e| bad("worldwright.history.edit", e.to_string()))?;
    session.doc_mut()?.feature_timelines[index] = candidate;
    Ok(json!({"scope":scope,"revision":revision}))
}
fn list(session: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"histories":session.doc()?.feature_timelines.iter().map(|h| json!({
        "scope":h.scope,
        "revision":h.revision,
        "steps":h.steps.len(),
        "parameters":h.parameters.len(),
        "rollback_after":h.rollback_after
    })).collect::<Vec<_>>()}))
}
fn inspect(session: &mut Session, p: &Value) -> Result<Value> {
    let scope = scope_from(session, p)?;
    serde_json::to_value(find(session.doc()?, &scope)?).map_err(|e| bad("worldwright.history.inspect", e.to_string()))
}
fn evaluate(session: &mut Session, p: &Value) -> Result<Value> {
    let scope = scope_from(session, p)?;
    let result = find(session.doc()?, &scope)?.evaluate().map_err(|e| bad("worldwright.history.evaluate", e.to_string()))?;
    serde_json::to_value(&result).map_err(|e| bad("worldwright.history.evaluate", e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn separate_histories_use_undoable_edit_and_kernel_midpoint() {
        let mut session = Session::new();
        let scope = json!({"kind":"document"});
        session.execute("worldwright.history.create", &json!({"scope":scope})).unwrap();
        let result = session
            .execute(
                "worldwright.history.edit",
                &json!({
                    "scope":scope, "expected_revision":0,
                    "change":{"edit":"append","step":{
                        "id":3,"name":"Driven Point","operation":"kernel.point.midpoint",
                        "inputs":{
                            "a":{"source":"constant","value":{"kind":"point","value":{"x":0.,"y":0.,"z":0.}}},
                            "b":{"source":"constant","value":{"kind":"point","value":{"x":10.,"y":0.,"z":0.}}}
                        }
                    }}
                }),
            )
            .unwrap();
        assert_eq!(result["revision"], 1);
        let output = session.execute("worldwright.history.evaluate", &json!({"scope":scope})).unwrap();
        assert_eq!(output["outputs"]["3"]["value"]["x"], 5.);
        let doc_revision = session.state().unwrap().revision;
        assert!(
            session
                .execute(
                    "worldwright.history.edit",
                    &json!({
                        "scope":scope,"expected_revision":0,
                        "change":{"edit":"set_rollback","after":null}
                    })
                )
                .is_err()
        );
        assert_eq!(session.state().unwrap().revision, doc_revision);
        session.undo().unwrap();
        assert_eq!(session.doc().unwrap().feature_timelines[0].steps.len(), 0);
    }
    #[test]
    fn block_history_is_case_insensitive_and_blocks_definition_replacement() {
        use cadcraft_doc::{Block, EntityKind, Point};
        use cadcraft_geom::{Vec2, Vec3};
        use std::sync::Arc;
        let mut session = Session::new();
        session.doc_mut().unwrap().blocks.insert("Bracket".into(), Arc::new(Block::new("Bracket")));
        session
            .execute(
                "worldwright.history.create",
                &json!({
                    "scope":{"kind":"block_definition","id":"BRACKET"}
                }),
            )
            .unwrap();
        let created = &session.doc().unwrap().feature_timelines[0];
        assert_eq!(created.scope, FeatureScope::BlockDefinition("Bracket".into()));
        // The command accepts different casing but resolves one local scope.
        assert!(
            session
                .execute(
                    "worldwright.history.create",
                    &json!({
                        "scope":{"kind":"block_definition","id":"bracket"}
                    })
                )
                .is_err()
        );
        let h = session.add_entity(EntityKind::Point(Point { p: Vec3::ZERO, angle: 0. })).unwrap();
        let before = session.doc().unwrap().clone();
        assert!(super::blocks::make_block(&mut session, "Bracket", Vec2::ZERO, &[h], "retain", "").is_err());
        assert_eq!(session.doc().unwrap(), &before);
    }

    #[test]
    fn failed_mutation_does_not_push_undo_or_advance_document_revision() {
        let mut session = Session::new();
        let scope = json!({"kind":"document"});
        session.execute("worldwright.history.create", &json!({"scope":scope})).unwrap();
        let before_doc = session.doc().unwrap().clone();
        let before_revision = session.state().unwrap().revision;
        let before_undo = session.state().unwrap().undo.len();
        assert!(
            session
                .execute(
                    "worldwright.history.edit",
                    &json!({
                        "scope":scope,"expected_revision":0,
                        "change":{"edit":"append","step":{
                            "id":9,"name":"Unsupported solid",
                            "operation":"kernel.solid.extrude","inputs":{}
                        }}
                    })
                )
                .is_err()
        );
        assert_eq!(session.doc().unwrap(), &before_doc);
        assert_eq!(session.state().unwrap().revision, before_revision);
        assert_eq!(session.state().unwrap().undo.len(), before_undo);
    }

    #[test]
    fn cannot_create_history_for_an_unknown_block() {
        let mut session = Session::new();
        assert!(
            session
                .execute(
                    "worldwright.history.create",
                    &json!({
                        "scope":{"kind":"block_definition","id":"NoSuchBlock"}
                    })
                )
                .is_err()
        );
        assert!(session.doc().unwrap().feature_timelines.is_empty());
    }
}
