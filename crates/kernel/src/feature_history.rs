//! Optional, scope-local parametric histories shared by CAD and OrbWeaver.
//! The history stores feature recipes; evaluated outputs do not silently
//! overwrite authoritative CAD geometry. Feature algorithms live in the
//! same kernel dispatcher as CAD commands and OrbWeaver graph nodes.
use crate::{
    KernelError, Result, ToolValue, TreeMatchPolicy, shared_tool, shared_tool_value_cost, tool_output_may_match_port, tool_value_matches_port,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_HISTORY_SCOPES: usize = 256;
pub const MAX_HISTORY_STEPS: usize = 512;
pub const MAX_HISTORY_PARAMETERS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum FeatureScope {
    Document,
    ModelNode(u64),
    BlockDefinition(String),
}
impl FeatureScope {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Document => Ok(()),
            Self::ModelNode(0) => Err(KernelError::Identity),
            Self::ModelNode(_) => Ok(()),
            Self::BlockDefinition(name) if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) => {
                Err(KernelError::Invalid("feature block name"))
            }
            Self::BlockDefinition(_) => Ok(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeatureInput {
    Constant { value: ToolValue },
    Parameter { name: String },
    PreviousFeature { id: u64 },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureStep {
    pub id: u64,
    pub name: String,
    /// Only canonical, already-implemented shared kernel operation IDs.
    pub operation: String,
    pub inputs: BTreeMap<String, FeatureInput>,
    #[serde(default)]
    pub matching: TreeMatchPolicy,
    #[serde(default)]
    pub suppressed: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureTimeline {
    pub scope: FeatureScope,
    pub revision: u64,
    /// Local parameters. Two different block definitions never share them.
    pub parameters: BTreeMap<String, ToolValue>,
    /// Ordered steps; references must address EARLIER steps in this scope.
    pub steps: Vec<FeatureStep>,
    #[serde(default)]
    pub rollback_after: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "edit", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeatureHistoryEdit {
    Append { step: FeatureStep },
    SetParameter { name: String, value: ToolValue },
    SetInput { id: u64, port: String, input: FeatureInput },
    SetSuppressed { id: u64, suppressed: bool },
    Reorder { id: u64, before: Option<u64> },
    SetRollback { after: Option<u64> },
}
/// Source of a prospective feature-history change. An impact report is
/// advisory: it does not evaluate, cache, or mutate document geometry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case", deny_unknown_fields)]
pub enum FeatureImpactSource {
    Parameter(String),
    Step(u64),
}

/// Potentially affected steps in deterministic timeline order.
/// Includes suppressed and rolled-back steps because they may be reactivated.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FeatureImpact {
    pub scope: FeatureScope,
    pub revision: u64,
    pub source: FeatureImpactSource,
    pub affected_steps: Vec<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FeatureStatus {
    Computed,
    Suppressed,
    RolledBack,
    Blocked { upstream: u64 },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureStepState {
    pub id: u64,
    pub status: FeatureStatus,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureEvaluation {
    pub scope: FeatureScope,
    pub revision: u64,
    pub outputs: BTreeMap<u64, ToolValue>,
    pub states: Vec<FeatureStepState>,
}

pub(crate) fn valid_history_parameter(name: &str) -> Result<()> {
    let mut chars = name.chars();
    let start_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    if !start_ok || name.len() > 128 || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(KernelError::Invalid("feature parameter name"));
    }
    Ok(())
}

impl FeatureTimeline {
    pub fn new(scope: FeatureScope) -> Result<Self> {
        scope.validate()?;
        Ok(Self { scope, revision: 0, parameters: BTreeMap::new(), steps: Vec::new(), rollback_after: None })
    }

    /// Inspect the transitive dependency cone before editing a parameter or
    /// feature. The history is already topologically ordered: every previous
    /// feature reference must point to an earlier step. This single forward
    /// pass is bounded by the existing 512-step history limit.
    ///
    /// An impact is *potential*, not a recomputation result. Suppressed and
    /// rolled-back descendants remain in the report so that changing their
    /// activation state cannot hide dependencies.
    pub fn impact(&self, source: FeatureImpactSource) -> Result<FeatureImpact> {
        self.validate()?;
        match &source {
            FeatureImpactSource::Parameter(name) if !self.parameters.contains_key(name) => {
                return Err(KernelError::Invalid("unknown feature parameter"));
            }
            FeatureImpactSource::Step(id) if !self.steps.iter().any(|step| step.id == *id) => {
                return Err(KernelError::Invalid("unknown feature ID"));
            }
            _ => {}
        }

        let mut affected = BTreeSet::new();
        for step in &self.steps {
            let direct = match &source {
                FeatureImpactSource::Parameter(name) => step
                    .inputs
                    .values()
                    .any(|input| matches!(input, FeatureInput::Parameter { name: parameter } if parameter == name)),
                FeatureImpactSource::Step(id) => step.id == *id,
            };
            let downstream = step.inputs.values().any(|input| {
                matches!(input, FeatureInput::PreviousFeature { id } if affected.contains(id))
            });
            if direct || downstream {
                affected.insert(step.id);
            }
        }
        let affected_steps = self
            .steps
            .iter()
            .filter_map(|step| affected.contains(&step.id).then_some(step.id))
            .collect();
        Ok(FeatureImpact {
            scope: self.scope.clone(),
            revision: self.revision,
            source,
            affected_steps,
        })
    }

    pub fn validate(&self) -> Result<()> {
        self.scope.validate()?;
        if self.steps.len() > MAX_HISTORY_STEPS || self.parameters.len() > MAX_HISTORY_PARAMETERS {
            return Err(KernelError::Budget);
        }
        for (name, value) in &self.parameters {
            valid_history_parameter(name)?;
            shared_tool_value_cost(value)?;
        }
        let mut earlier = BTreeMap::new();
        for step in &self.steps {
            if step.id == 0 || step.id == u64::MAX || earlier.contains_key(&step.id) {
                return Err(KernelError::Invalid("duplicate or invalid feature ID"));
            }
            if step.name.trim().is_empty() || step.name.len() > 256 {
                return Err(KernelError::Invalid("feature name"));
            }
            let op = shared_tool(&step.operation).ok_or(KernelError::Invalid("feature operation not yet implemented"))?;
            if op.operation != step.operation {
                return Err(KernelError::Invalid("feature requires canonical kernel operation ID"));
            }
            if op.inputs.len() != step.inputs.len() {
                return Err(KernelError::Invalid("feature port count"));
            }
            for port in op.inputs {
                let source = step.inputs.get(port.name).ok_or(KernelError::Invalid("missing feature input port"))?;
                match source {
                    FeatureInput::Constant { value } => {
                        tool_value_matches_port(port.kind, value)?;
                    }
                    FeatureInput::Parameter { name } => {
                        valid_history_parameter(name)?;
                        let value = self.parameters.get(name).ok_or(KernelError::Invalid("missing feature parameter"))?;
                        tool_value_matches_port(port.kind, value)?;
                    }
                    FeatureInput::PreviousFeature { id } => {
                        let output = earlier.get(id).ok_or(KernelError::Invalid("feature must reference earlier step"))?;
                        if !tool_output_may_match_port(port.kind, *output) {
                            return Err(KernelError::Invalid("feature output/port mismatch"));
                        }
                    }
                }
            }
            earlier.insert(step.id, op.output);
        }
        if self.rollback_after.is_some_and(|id| !earlier.contains_key(&id)) {
            return Err(KernelError::Invalid("unknown rollback step"));
        }
        Ok(())
    }
}

/// Validate independent, uniquely scoped histories. Host-specific scope
/// existence (block definition or model node) belongs in the CAD document.
pub fn validate_feature_timelines(histories: &[FeatureTimeline]) -> Result<()> {
    if histories.len() > MAX_HISTORY_SCOPES {
        return Err(KernelError::Budget);
    }
    let mut scopes = BTreeSet::new();
    let mut all_steps = 0usize;
    let mut retained_values = 0usize;
    for history in histories {
        history.validate()?;
        // Native block names are case-insensitive. Reject alias histories
        // targeting the same definition under different capitalization.
        let canonical_scope = match &history.scope {
            FeatureScope::BlockDefinition(name) => FeatureScope::BlockDefinition(name.to_ascii_lowercase()),
            other => other.clone(),
        };
        if !scopes.insert(canonical_scope) {
            return Err(KernelError::Invalid("duplicate feature scope"));
        }
        all_steps = all_steps.checked_add(history.steps.len()).ok_or(KernelError::Budget)?;
        if all_steps > 8192 {
            return Err(KernelError::Budget);
        }
        for value in history.parameters.values() {
            retained_values = retained_values.checked_add(shared_tool_value_cost(value)?).ok_or(KernelError::Budget)?;
        }
        for step in &history.steps {
            for source in step.inputs.values() {
                if let FeatureInput::Constant { value } = source {
                    retained_values = retained_values.checked_add(shared_tool_value_cost(value)?).ok_or(KernelError::Budget)?;
                }
            }
        }
        if retained_values > crate::MAX_TREE_ITEMS {
            return Err(KernelError::Budget);
        }
    }
    Ok(())
}

#[cfg(test)]
mod impact_tests {
    use super::*;
    use cadcraft_geom::Vec3;

    fn midpoint(id: u64, a: FeatureInput, b: FeatureInput) -> FeatureStep {
        FeatureStep {
            id,
            name: format!("Midpoint {id}"),
            operation: "kernel.point.midpoint".into(),
            inputs: BTreeMap::from([("a".into(), a), ("b".into(), b)]),
            matching: TreeMatchPolicy::default(),
            suppressed: false,
        }
    }

    fn constant() -> FeatureInput {
        FeatureInput::Constant {
            value: ToolValue::Point(Vec3::ZERO),
        }
    }

    #[test]
    fn impact_reports_only_transitive_dependents_in_timeline_order() {
        let mut history = FeatureTimeline::new(FeatureScope::Document).unwrap();
        history
            .parameters
            .insert("width".into(), ToolValue::Point(Vec3::ZERO));
        history
            .parameters
            .insert("unused".into(), ToolValue::Point(Vec3::ZERO));
        history.steps = vec![
            midpoint(
                10,
                FeatureInput::Parameter {
                    name: "width".into(),
                },
                constant(),
            ),
            midpoint(
                20,
                FeatureInput::PreviousFeature { id: 10 },
                constant(),
            ),
            midpoint(30, constant(), constant()),
            midpoint(
                40,
                FeatureInput::PreviousFeature { id: 20 },
                constant(),
            ),
        ];
        history.steps[1].suppressed = true;
        history.rollback_after = Some(20);

        let parameter = history
            .impact(FeatureImpactSource::Parameter("width".into()))
            .unwrap();
        assert_eq!(parameter.affected_steps, vec![10, 20, 40]);
        assert_eq!(parameter.revision, 0);
        assert_eq!(history.impact(FeatureImpactSource::Step(20)).unwrap().affected_steps, vec![20, 40]);
        assert_eq!(history.impact(FeatureImpactSource::Step(30)).unwrap().affected_steps, vec![30]);
        assert!(
            history
                .impact(FeatureImpactSource::Parameter("unused".into()))
                .unwrap()
                .affected_steps
                .is_empty()
        );
        assert!(history.impact(FeatureImpactSource::Parameter("missing".into())).is_err());
        assert!(history.impact(FeatureImpactSource::Step(999)).is_err());
        assert_eq!(history.steps.len(), 4);
        assert!(history.steps[1].suppressed);
    }
}
