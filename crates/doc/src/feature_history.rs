//! Host document checks for optional parametric feature-history scopes.
//! This ensures that .dftba histories never refer to nonexistent blocks or
//! organization nodes, including after document and block deletion.
use crate::Drawing;
use buildercraft_kernel::{FeatureScope, KernelError, Result, validate_feature_timelines};

impl Drawing {
    pub fn validate_feature_histories(&self) -> Result<()> {
        validate_feature_timelines(&self.feature_timelines)?;
        for history in &self.feature_timelines {
            match &history.scope {
                FeatureScope::Document => {}
                FeatureScope::ModelNode(id) => {
                    if !self.organization.nodes.iter().any(|node| node.id == *id) {
                        return Err(KernelError::Invalid("feature history refers to missing model node"));
                    }
                }
                FeatureScope::BlockDefinition(name) => {
                    if self.block(name).is_none() {
                        return Err(KernelError::Invalid("feature history refers to missing block"));
                    }
                }
            }
        }
        Ok(())
    }
}
