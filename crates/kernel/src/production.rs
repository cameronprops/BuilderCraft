//! Production relationships are independent of geometric ownership and display layers.
use crate::{Id, KernelError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductionKind {
    Show,
    Act,
    Scene,
    Beat,
    Effect,
    Department,
    System,
    Equipment,
    Zone,
    Package,
    Cue,
    Asset,
    Script,
    Storyboard,
    Camera,
    RidePath,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductionRecord {
    pub id: Id,
    pub kind: ProductionKind,
    pub name: String,
    pub parent: Option<Id>,
    #[serde(default)]
    pub attributes: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductionBinding {
    pub object: Id,
    pub record: Id,
    pub role: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProductionLink {
    pub from: Id,
    pub to: Id,
    pub role: String,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProductionModel {
    #[serde(default)]
    pub records: Vec<ProductionRecord>,
    #[serde(default)]
    pub bindings: Vec<ProductionBinding>,
    #[serde(default)]
    pub links: Vec<ProductionLink>,
}
impl ProductionModel {
    pub fn validate(&self) -> Result<()> {
        if self.records.len() > 1024 || self.bindings.len() > 8192 || self.links.len() > 8192 {
            return Err(KernelError::Invalid("production count limit"));
        }
        let records: BTreeMap<_, _> = self.records.iter().map(|r| (r.id, r)).collect();
        if records.len() != self.records.len() {
            return Err(KernelError::Invalid("duplicate production ID"));
        }
        for r in &self.records {
            if r.name.trim().is_empty()
                || r.name.len() > 256
                || r.attributes.len() > 16
                || r.attributes.iter().any(|(k, v)| k.is_empty() || k.len() > 64 || v.len() > 1024)
            {
                return Err(KernelError::Invalid("production metadata"));
            }
            let mut seen = BTreeSet::from([r.id]);
            let mut parent = r.parent;
            while let Some(id) = parent {
                if seen.len() >= 64 || !seen.insert(id) {
                    return Err(KernelError::Invalid("production hierarchy"));
                }
                parent = records.get(&id).ok_or(KernelError::Invalid("missing production parent"))?.parent;
            }
        }
        let role = |r: &str| !r.trim().is_empty() && r.len() <= 64;
        for b in &self.bindings {
            if !records.contains_key(&b.record) || !role(&b.role) {
                return Err(KernelError::Invalid("production binding"));
            }
        }
        for l in &self.links {
            if !records.contains_key(&l.from) || !records.contains_key(&l.to) || !role(&l.role) {
                return Err(KernelError::Invalid("production relationship"));
            }
        }
        Ok(())
    }
}
