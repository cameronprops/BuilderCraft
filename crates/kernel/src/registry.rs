//! Static kernel operation discovery shared by future CAD, Graph and API adapters.
//! A registry entry is metadata, not evidence of Rhino/Grasshopper parity.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Implemented,
    Partial,
    Planned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct OperationDescriptor {
    /// Stable, namespaced API key; do not rename without a compatibility alias.
    pub id: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub inputs: &'static [&'static str],
    pub outputs: &'static [&'static str],
    pub status: OperationStatus,
}

/// Implemented means the listed kernel service exists, not that every UI option exists.
/// Deliberately small: add entries only alongside matching implementation/tests.
pub const OPERATIONS: &[OperationDescriptor] = &[
    OperationDescriptor {
        id: "kernel.frame.convert_point",
        label: "Convert Point Frame",
        category: "units",
        inputs: &["source_frame", "target_frame", "point"],
        outputs: &["point"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.geometry.transform_exact",
        label: "Transform Exact Geometry",
        category: "transform",
        inputs: &["exact_shape", "transform"],
        outputs: &["exact_shape"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.geometry.tessellate",
        label: "Tessellate Exact Geometry",
        category: "geometry",
        inputs: &["exact_shape", "settings"],
        outputs: &["preview_geometry"],
        status: OperationStatus::Implemented,
    },
];

pub fn operation_by_id(id: &str) -> Option<&'static OperationDescriptor> {
    OPERATIONS.iter().find(|op| op.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_unique_namespaced_ids() {
        let mut ids = std::collections::HashSet::new();
        for op in OPERATIONS {
            assert!(op.id.starts_with("kernel."));
            assert!(ids.insert(op.id), "duplicate operation: {}", op.id);
            assert!(!op.label.is_empty());
            assert!(!op.inputs.is_empty());
            assert!(!op.outputs.is_empty());
        }
    }

    #[test]
    fn lookup_matches_catalogue() {
        for op in OPERATIONS {
            assert_eq!(operation_by_id(op.id), Some(op));
        }
        assert_eq!(operation_by_id("kernel.nonexistent"), None);
    }

    #[test]
    fn registry_serializes_for_api_discovery() {
        let json = serde_json::to_string(OPERATIONS);
        assert!(json.is_ok());
        if let Ok(json) = json {
            assert!(json.contains("kernel.frame.convert_point"));
            assert!(json.contains("implemented"));
        }
    }
}
