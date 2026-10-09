//! Transactional editing and pure recomputation of feature-history recipes.
//! The same shared dispatcher is invoked by OrbWeaver and CAD commands.
use crate::feature_history::valid_history_parameter;
use crate::{
    FeatureEvaluation, FeatureHistoryEdit, FeatureInput, FeatureStatus, FeatureStepState, FeatureTimeline, KernelError, Result, ToolRequest,
    execute_shared_tool_with_matching, shared_tool_value_cost,
};
use std::collections::BTreeMap;

impl FeatureTimeline {
    /// Optimistic, atomic edit. A failed edit leaves the old recipe untouched.
    /// Undo/redo is provided by the CAD document's existing COW snapshots.
    pub fn apply(&mut self, expected_revision: u64, edit: FeatureHistoryEdit) -> Result<u64> {
        if self.revision != expected_revision {
            return Err(KernelError::Conflict { expected: expected_revision, actual: self.revision });
        }
        let next_revision = self.revision.checked_add(1).ok_or(KernelError::Invalid("feature timeline revision overflow"))?;
        let mut candidate = self.clone();
        match edit {
            FeatureHistoryEdit::Append { step } => candidate.steps.push(step),
            FeatureHistoryEdit::SetParameter { name, value } => {
                valid_history_parameter(&name)?;
                shared_tool_value_cost(&value)?;
                candidate.parameters.insert(name, value);
            }
            FeatureHistoryEdit::SetInput { id, port, input } => {
                let step = candidate.steps.iter_mut().find(|s| s.id == id).ok_or(KernelError::Invalid("unknown feature ID"))?;
                step.inputs.insert(port, input);
            }
            FeatureHistoryEdit::SetSuppressed { id, suppressed } => {
                let step = candidate.steps.iter_mut().find(|s| s.id == id).ok_or(KernelError::Invalid("unknown feature ID"))?;
                step.suppressed = suppressed;
            }
            FeatureHistoryEdit::Reorder { id, before } => {
                if Some(id) == before {
                    return Err(KernelError::Invalid("cannot reorder before self"));
                }
                let position = candidate.steps.iter().position(|s| s.id == id).ok_or(KernelError::Invalid("unknown feature ID"))?;
                let step = candidate.steps.remove(position);
                let index = match before {
                    Some(target) => candidate.steps.iter().position(|s| s.id == target).ok_or(KernelError::Invalid("unknown reorder target"))?,
                    None => candidate.steps.len(),
                };
                candidate.steps.insert(index, step);
            }
            FeatureHistoryEdit::SetRollback { after } => {
                candidate.rollback_after = after;
            }
        }
        candidate.validate()?;
        candidate.revision = next_revision;
        *self = candidate;
        Ok(next_revision)
    }

    /// Evaluate active prefix without mutating any CAD geometry or history.
    /// Suppressed and rolled-back steps retain IDs and recipe state. A
    /// dependent on a suppressed step becomes Blocked, not silently stale.
    pub fn evaluate(&self) -> Result<FeatureEvaluation> {
        self.validate()?;
        let limit = self.rollback_after.and_then(|id| self.steps.iter().position(|s| s.id == id)).unwrap_or(self.steps.len().saturating_sub(1));
        let mut outputs = BTreeMap::new();
        let mut states = Vec::new();
        states.try_reserve_exact(self.steps.len()).map_err(|_| KernelError::Budget)?;
        let mut output_cost = 0usize;
        for (index, step) in self.steps.iter().enumerate() {
            if self.rollback_after.is_some() && index > limit {
                states.push(FeatureStepState { id: step.id, status: FeatureStatus::RolledBack });
                continue;
            }
            if step.suppressed {
                states.push(FeatureStepState { id: step.id, status: FeatureStatus::Suppressed });
                continue;
            }
            let mut inputs = BTreeMap::new();
            let mut blocked_by = None;
            for (name, source) in &step.inputs {
                let value = match source {
                    FeatureInput::Constant { value } => value.clone(),
                    FeatureInput::Parameter { name } => {
                        self.parameters.get(name).ok_or(KernelError::Invalid("feature parameter disappeared"))?.clone()
                    }
                    FeatureInput::PreviousFeature { id } => {
                        let Some(value) = outputs.get(id) else {
                            blocked_by = Some(*id);
                            break;
                        };
                        value.clone()
                    }
                };
                inputs.insert(name.clone(), value);
            }
            if let Some(upstream) = blocked_by {
                states.push(FeatureStepState { id: step.id, status: FeatureStatus::Blocked { upstream } });
                continue;
            }
            let output = execute_shared_tool_with_matching(&ToolRequest { operation: step.operation.clone(), inputs }, step.matching)?;
            output_cost = output_cost.checked_add(shared_tool_value_cost(&output)?).ok_or(KernelError::Budget)?;
            if output_cost > crate::MAX_TREE_ITEMS {
                return Err(KernelError::Budget);
            }
            outputs.insert(step.id, output);
            states.push(FeatureStepState { id: step.id, status: FeatureStatus::Computed });
        }
        Ok(FeatureEvaluation { scope: self.scope.clone(), revision: self.revision, outputs, states })
    }
}
