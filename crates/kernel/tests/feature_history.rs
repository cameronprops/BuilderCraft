//! History engine regression tests. CAD document integration tests live in
//! cadcraft-io and engine; this suite is host- and UI-independent.
use buildercraft_kernel::*;
use cadcraft_geom::Vec3;
use std::collections::BTreeMap;

fn at(x: f64) -> ToolValue { ToolValue::Point(Vec3::new(x, 0., 0.)) }
fn constant(value: ToolValue) -> FeatureInput { FeatureInput::Constant { value } }
fn mid(id: u64, a: FeatureInput, b: FeatureInput) -> FeatureStep {
    FeatureStep {
        id, name: format!("Feature {id}"),
        operation: "kernel.point.midpoint".into(),
        inputs: BTreeMap::from([("a".into(), a), ("b".into(), b)]),
        matching: TreeMatchPolicy::Shortest, suppressed: false,
    }
}
fn append(t: &mut FeatureTimeline, step: FeatureStep) {
    let rev = t.revision;
    assert_eq!(t.apply(rev, FeatureHistoryEdit::Append { step }), Ok(rev+1));
}

#[test]
fn dependency_rebuild_reacts_to_local_dimension_change() {
    let mut t = FeatureTimeline::new(FeatureScope::ModelNode(18)).unwrap();
    t.apply(0, FeatureHistoryEdit::SetParameter { name:"width".into(), value:at(8.) }).unwrap();
    append(&mut t, mid(10, constant(at(0.)), FeatureInput::Parameter{name:"width".into()}));
    append(&mut t, mid(20, FeatureInput::PreviousFeature{id:10}, constant(at(10.))));
    assert_eq!(t.evaluate().unwrap().outputs.get(&20), Some(&at(7.)));
    t.apply(3, FeatureHistoryEdit::SetParameter {name:"width".into(),value:at(12.)}).unwrap();
    assert_eq!(t.evaluate().unwrap().outputs.get(&20), Some(&at(8.)));
}

#[test]
fn suppression_marks_dependents_blocked_without_deleting_steps() {
    let mut t = FeatureTimeline::new(FeatureScope::Document).unwrap();
    append(&mut t, mid(1, constant(at(0.)), constant(at(10.))));
    append(&mut t, mid(2, FeatureInput::PreviousFeature{id:1}, constant(at(20.))));
    t.apply(2, FeatureHistoryEdit::SetSuppressed{id:1,suppressed:true}).unwrap();
    let evaluation=t.evaluate().unwrap();
    assert!(evaluation.outputs.is_empty());
    assert_eq!(evaluation.states[0].status, FeatureStatus::Suppressed);
    assert_eq!(evaluation.states[1].status, FeatureStatus::Blocked{upstream:1});
    t.apply(3, FeatureHistoryEdit::SetSuppressed{id:1,suppressed:false}).unwrap();
    assert_eq!(t.evaluate().unwrap().outputs.get(&2),Some(&at(12.5)));
}

#[test]
fn rollback_is_nondestructive_and_recoverable() {
    let mut t = FeatureTimeline::new(FeatureScope::Document).unwrap();
    for id in 1..=3 {
        append(&mut t, mid(id,constant(at(0.)),constant(at(id as f64))));
    }
    t.apply(3, FeatureHistoryEdit::SetRollback{after:Some(1)}).unwrap();
    assert_eq!(t.evaluate().unwrap().outputs.len(),1);
    assert_eq!(t.evaluate().unwrap().states[2].status,FeatureStatus::RolledBack);
    t.apply(4, FeatureHistoryEdit::SetRollback{after:None}).unwrap();
    assert_eq!(t.evaluate().unwrap().outputs.len(),3);
}

#[test]
fn illegal_reorder_and_stale_revision_are_atomic() {
    let mut t=FeatureTimeline::new(FeatureScope::Document).unwrap();
    append(&mut t,mid(10,constant(at(0.)),constant(at(4.))));
    append(&mut t,mid(20,FeatureInput::PreviousFeature{id:10},constant(at(8.))));
    let snapshot=t.clone();
    assert!(t.apply(2, FeatureHistoryEdit::Reorder{id:20,before:Some(10)}).is_err());
    assert_eq!(t,snapshot);
    assert!(matches!(
        t.apply(1,FeatureHistoryEdit::SetRollback{after:None}),
        Err(KernelError::Conflict{expected:1,actual:2})
    ));
    assert_eq!(t,snapshot);
}

#[test]
fn scopes_and_serialized_recipes_are_independent() {
    let mut original=FeatureTimeline::new(FeatureScope::BlockDefinition("Bracket".into())).unwrap();
    append(&mut original,mid(1,constant(at(0.)),constant(at(5.))));
    let encoded=serde_json::to_string(&original).unwrap();
    assert_eq!(serde_json::from_str::<FeatureTimeline>(&encoded).unwrap(),original);
    let mut another=original.clone();
    another.scope=FeatureScope::BlockDefinition("Panel".into());
    assert!(validate_feature_timelines(&[original.clone(),another]).is_ok());
    assert!(validate_feature_timelines(&[original.clone(),original]).is_err());
    assert!(FeatureTimeline::new(FeatureScope::ModelNode(0)).is_err());
}

#[test]
fn block_scope_identity_is_case_insensitive_and_aggregate_history_is_bounded() {
    let upper = FeatureTimeline::new(FeatureScope::BlockDefinition("Bracket".into())).unwrap();
    let lower = FeatureTimeline::new(FeatureScope::BlockDefinition("bracket".into())).unwrap();
    assert_eq!(validate_feature_timelines(&[upper, lower]),
        Err(KernelError::Invalid("duplicate feature scope")));

    let mut oversized = FeatureTimeline::new(FeatureScope::Document).unwrap();
    // Each entry fits an individual value's budget, but together they must
    // not exceed the combined resource budget of the document.
    oversized.parameters.insert("largeA".into(), ToolValue::Polyline(vec![
        Vec3::ZERO; 150_000
    ]));
    oversized.parameters.insert("largeB".into(), ToolValue::Polyline(vec![
        Vec3::ZERO; 150_000
    ]));
    assert_eq!(validate_feature_timelines(&[oversized]), Err(KernelError::Budget));
}

#[test]
fn rejects_forward_links_and_unimplemented_solids_without_partial_edit() {
    let mut t=FeatureTimeline::new(FeatureScope::Document).unwrap();
    assert!(t.apply(0,FeatureHistoryEdit::Append{step:mid(
        1,FeatureInput::PreviousFeature{id:2},constant(at(10.))
    )}).is_err());
    assert!(t.steps.is_empty());
    assert!(t.apply(0,FeatureHistoryEdit::Append{step:FeatureStep{
        id:1,name:"Extrude".into(),operation:"kernel.solid.extrude".into(),
        inputs:BTreeMap::new(),matching:TreeMatchPolicy::Shortest,suppressed:false,
    }}).is_err());
    assert_eq!(t.revision,0);
}

#[test]
fn editing_input_recalculates_dependents_and_validates_types() {
    let mut t=FeatureTimeline::new(FeatureScope::Document).unwrap();
    append(&mut t,mid(1,constant(at(0.)),constant(at(10.))));
    let before=t.evaluate().unwrap().outputs.get(&1).cloned();
    t.apply(1,FeatureHistoryEdit::SetInput{
        id:1,port:"b".into(),input:constant(at(20.)),
    }).unwrap();
    assert_ne!(t.evaluate().unwrap().outputs.get(&1),before.as_ref());
    let snapshot=t.clone();
    assert!(t.apply(2,FeatureHistoryEdit::SetInput{
        id:1,port:"b".into(),input:constant(ToolValue::Number(5.)),
    }).is_err());
    assert_eq!(snapshot,t);
}
