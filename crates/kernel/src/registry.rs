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
        id: "kernel.mesh.duplicate_faces",
        label: "Duplicate Mesh Faces",
        category: "mesh",
        inputs: &["mesh"],
        outputs: &["duplicates"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.remove_unused_vertices",
        label: "Remove Unused Mesh Vertices",
        category: "mesh",
        inputs: &["mesh"],
        outputs: &["mesh", "old_to_new", "removed_vertex_indices"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.validation_report",
        label: "Mesh Validation Report",
        category: "mesh",
        inputs: &["mesh", "relative_area_tolerance"],
        outputs: &["vertex_count", "face_count", "unused_vertex_indices", "duplicate_faces", "degenerate_face_indices", "edge_report"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.weld",
        label: "Weld Mesh Vertices",
        category: "mesh",
        inputs: &["mesh", "tolerance", "collapsed_face_policy"],
        outputs: &["mesh", "old_to_new", "retained_face_indices", "removed_face_indices"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.vertex_weld_map",
        label: "Duplicate Vertex Weld Map",
        category: "mesh",
        inputs: &["vertices", "tolerance"],
        outputs: &["representative", "duplicates"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.mesh_vertex_weld_map",
        label: "Mesh Vertex Weld Map",
        category: "mesh",
        inputs: &["mesh", "tolerance"],
        outputs: &["representative", "duplicates"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.edge_report",
        label: "Mesh Edge Topology Report",
        category: "mesh",
        inputs: &["mesh"],
        outputs: &["boundary_edges", "non_manifold_edges", "inconsistent_winding_edges", "boundary_loops", "unresolved_boundary_edges"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.face_analysis",
        label: "Mesh Face Analysis",
        category: "mesh",
        inputs: &["mesh", "relative_area_tolerance"],
        outputs: &["face_analysis"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.mesh.degenerate_faces",
        label: "Degenerate Mesh Faces",
        category: "mesh",
        inputs: &["mesh", "relative_area_tolerance"],
        outputs: &["face_indices"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.bounds.from_points",
        label: "3D Bounding Box",
        category: "bounds",
        inputs: &["points"],
        outputs: &["bounds"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.bounds.dimensions",
        label: "Bounding Box Dimensions",
        category: "bounds",
        inputs: &["bounds"],
        outputs: &["dimensions"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.bounds.center",
        label: "Bounding Box Center",
        category: "bounds",
        inputs: &["bounds"],
        outputs: &["center"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.polyline.segment_lengths",
        label: "Polyline Segment Lengths",
        category: "polyline",
        inputs: &["points"],
        outputs: &["lengths"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.polyline.length",
        label: "Polyline Total Length",
        category: "polyline",
        inputs: &["points"],
        outputs: &["length"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.polyline.divide_count",
        label: "Divide Polyline by Count",
        category: "polyline",
        inputs: &["points", "count"],
        outputs: &["points"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.polyline.divide_distance",
        label: "Divide Polyline by Distance",
        category: "polyline",
        inputs: &["points", "spacing"],
        outputs: &["points"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.vector.normalize",
        label: "Normalize Vector",
        category: "vector",
        inputs: &["vector"],
        outputs: &["unit_vector"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.vector.dot",
        label: "Vector Dot Product",
        category: "vector",
        inputs: &["vector_a", "vector_b"],
        outputs: &["scalar"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.vector.cross",
        label: "Vector Cross Product",
        category: "vector",
        inputs: &["vector_a", "vector_b"],
        outputs: &["vector"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.vector.angle",
        label: "Angle Between Vectors",
        category: "vector",
        inputs: &["vector_a", "vector_b"],
        outputs: &["angle_radians"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.point.distance",
        label: "Distance Between Points",
        category: "point",
        inputs: &["point_a", "point_b"],
        outputs: &["distance"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.point.midpoint",
        label: "Point Midpoint",
        category: "point",
        inputs: &["point_a", "point_b"],
        outputs: &["point"],
        status: OperationStatus::Implemented,
    },
    OperationDescriptor {
        id: "kernel.point.interpolate",
        label: "Interpolate Points",
        category: "point",
        inputs: &["point_a", "point_b", "parameter"],
        outputs: &["point"],
        status: OperationStatus::Implemented,
    },
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
