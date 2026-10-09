//! Typed, host-independent operation contracts shared by Worldwright commands
//! and Orb Weaver nodes. The algorithm lives in the existing kernel; this module
//! only validates named inputs and dispatches to that one implementation.
use crate::{
    KernelError, Result, point_distance, point_midpoint,
    point_interpolate, vector_length, vector_normalize, vector_dot,
    vector_cross, polyline_length, polyline_divide_count,
    polyline_divide_distance,
};
use cadcraft_geom::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Number,
    Count,
    Point,
    Vector,
    Polyline,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ToolValue {
    Number(f64),
    Count(u64),
    Point(Vec3),
    Vector(Vec3),
    Polyline(Vec<Vec3>),
}

impl ToolValue {
    pub fn kind(&self) -> ToolType {
        match self {
            Self::Number(_) => ToolType::Number,
            Self::Count(_) => ToolType::Count,
            Self::Point(_) => ToolType::Point,
            Self::Vector(_) => ToolType::Vector,
            Self::Polyline(_) => ToolType::Polyline,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ToolPort {
    pub name: &'static str,
    pub kind: ToolType,
    /// A modifier changes the algorithm's policy instead of defining a new
    /// lower-level geometry engine. It can be a graph literal or a wired port.
    pub modifier: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SharedToolContract {
    pub operation: &'static str,
    pub cad_command: &'static str,
    pub orbweaver_node: &'static str,
    pub dependency_group: &'static str,
    /// Lower-level kernel operations required by this service/algorithm.
    pub prerequisites: &'static [&'static str],
    pub inputs: &'static [ToolPort],
    pub output: ToolType,
}

const A_B_POINTS: &[ToolPort] = &[
    ToolPort { name: "a", kind: ToolType::Point, modifier: false },
    ToolPort { name: "b", kind: ToolType::Point, modifier: false },
];
const A_B_VECTORS: &[ToolPort] = &[
    ToolPort { name: "a", kind: ToolType::Vector, modifier: false },
    ToolPort { name: "b", kind: ToolType::Vector, modifier: false },
];
const ONE_VECTOR: &[ToolPort] = &[
    ToolPort { name: "v", kind: ToolType::Vector, modifier: false },
];
const POINT_INTERPOLATE: &[ToolPort] = &[
    ToolPort { name: "a", kind: ToolType::Point, modifier: false },
    ToolPort { name: "b", kind: ToolType::Point, modifier: false },
    ToolPort { name: "t", kind: ToolType::Number, modifier: true },
];
const POLYLINE_LENGTH: &[ToolPort] = &[
    ToolPort { name: "points", kind: ToolType::Polyline, modifier: false },
];
const POLYLINE_COUNT: &[ToolPort] = &[
    ToolPort { name: "points", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "count", kind: ToolType::Count, modifier: true },
];
const POLYLINE_DISTANCE: &[ToolPort] = &[
    ToolPort { name: "points", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "spacing", kind: ToolType::Number, modifier: true },
];

/// One record per shared, executable operation. No copy of any algorithm exists
/// in the CAD or Graph adapters.
pub const SHARED_TOOLS: &[SharedToolContract] = &[
    SharedToolContract {
        operation: "kernel.point.distance",
        cad_command: "worldwright.point.distance",
        orbweaver_node: "orbweaver.point.distance",
        dependency_group: "geometry.point",
        prerequisites: &[],
        inputs: A_B_POINTS, output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.point.midpoint",
        cad_command: "worldwright.point.midpoint",
        orbweaver_node: "orbweaver.point.midpoint",
        dependency_group: "geometry.point",
        prerequisites: &[],
        inputs: A_B_POINTS, output: ToolType::Point,
    },
    SharedToolContract {
        operation: "kernel.point.interpolate",
        cad_command: "worldwright.point.interpolate",
        orbweaver_node: "orbweaver.point.interpolate",
        dependency_group: "geometry.point",
        prerequisites: &[],
        inputs: POINT_INTERPOLATE, output: ToolType::Point,
    },
    SharedToolContract {
        operation: "kernel.vector.length",
        cad_command: "worldwright.vector.length",
        orbweaver_node: "orbweaver.vector.length",
        dependency_group: "math.vector",
        prerequisites: &[],
        inputs: ONE_VECTOR, output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.vector.normalize",
        cad_command: "worldwright.vector.normalize",
        orbweaver_node: "orbweaver.vector.normalize",
        dependency_group: "math.vector",
        prerequisites: &["kernel.vector.length"],
        inputs: ONE_VECTOR, output: ToolType::Vector,
    },
    SharedToolContract {
        operation: "kernel.vector.dot",
        cad_command: "worldwright.vector.dot",
        orbweaver_node: "orbweaver.vector.dot",
        dependency_group: "math.vector",
        prerequisites: &[],
        inputs: A_B_VECTORS, output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.vector.cross",
        cad_command: "worldwright.vector.cross",
        orbweaver_node: "orbweaver.vector.cross",
        dependency_group: "math.vector",
        prerequisites: &[],
        inputs: A_B_VECTORS, output: ToolType::Vector,
    },
    SharedToolContract {
        operation: "kernel.polyline.length",
        cad_command: "worldwright.polyline.length",
        orbweaver_node: "orbweaver.polyline.length",
        dependency_group: "geometry.polyline",
        prerequisites: &["kernel.point.distance"],
        inputs: POLYLINE_LENGTH, output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.polyline.divide_count",
        cad_command: "worldwright.polyline.divide_count",
        orbweaver_node: "orbweaver.polyline.divide_count",
        dependency_group: "geometry.polyline",
        prerequisites: &["kernel.polyline.length", "kernel.point.distance", "kernel.point.interpolate"],
        inputs: POLYLINE_COUNT, output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.polyline.divide_distance",
        cad_command: "worldwright.polyline.divide_distance",
        orbweaver_node: "orbweaver.polyline.divide_distance",
        dependency_group: "geometry.polyline",
        prerequisites: &["kernel.polyline.length", "kernel.point.distance", "kernel.point.interpolate"],
        inputs: POLYLINE_DISTANCE, output: ToolType::Polyline,
    },
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRequest {
    pub operation: String,
    pub inputs: BTreeMap<String, ToolValue>,
}

pub fn shared_tool(operation: &str) -> Option<&'static SharedToolContract> {
    SHARED_TOOLS.iter().find(|tool| {
        tool.operation == operation
            || tool.cad_command == operation
            || tool.orbweaver_node == operation
    })
}

fn point(inputs: &BTreeMap<String, ToolValue>, name: &str) -> Result<Vec3> {
    match inputs.get(name) {
        Some(ToolValue::Point(value)) => Ok(*value),
        _ => Err(KernelError::Invalid("point input")),
    }
}
fn vector(inputs: &BTreeMap<String, ToolValue>, name: &str) -> Result<Vec3> {
    match inputs.get(name) {
        Some(ToolValue::Vector(value)) => Ok(*value),
        _ => Err(KernelError::Invalid("vector input")),
    }
}
fn number(inputs: &BTreeMap<String, ToolValue>, name: &str) -> Result<f64> {
    match inputs.get(name) {
        Some(ToolValue::Number(value)) if value.is_finite() => Ok(*value),
        _ => Err(KernelError::Invalid("finite numeric modifier")),
    }
}
fn count(inputs: &BTreeMap<String, ToolValue>, name: &str) -> Result<usize> {
    match inputs.get(name) {
        Some(ToolValue::Count(value)) => usize::try_from(*value).map_err(|_| KernelError::Budget),
        _ => Err(KernelError::Invalid("count modifier")),
    }
}
fn polyline<'a>(inputs: &'a BTreeMap<String, ToolValue>, name: &str) -> Result<&'a [Vec3]> {
    match inputs.get(name) {
        Some(ToolValue::Polyline(value)) => Ok(value.as_slice()),
        _ => Err(KernelError::Invalid("polyline input")),
    }
}

/// Execute a pure shared operation. Missing, extra, incorrectly typed or
/// nonfinite inputs fail before host document mutation. The native geometry
/// functions apply their original bounds and degeneracy checks.
pub fn execute_shared_tool(request: &ToolRequest) -> Result<ToolValue> {
    let contract = shared_tool(&request.operation)
        .ok_or(KernelError::Invalid("unregistered shared operation"))?;
    if request.inputs.len() != contract.inputs.len() {
        return Err(KernelError::Invalid("missing or unexpected tool port"));
    }
    for port in contract.inputs {
        let value = request.inputs.get(port.name)
            .ok_or(KernelError::Invalid("missing tool port"))?;
        if value.kind() != port.kind {
            return Err(KernelError::Invalid("tool port type mismatch"));
        }
    }
    match contract.operation {
        "kernel.point.distance" => Ok(ToolValue::Number(point_distance(
            point(&request.inputs, "a")?, point(&request.inputs, "b")?,
        )?)),
        "kernel.point.midpoint" => Ok(ToolValue::Point(point_midpoint(
            point(&request.inputs, "a")?, point(&request.inputs, "b")?,
        )?)),
        "kernel.point.interpolate" => Ok(ToolValue::Point(point_interpolate(
            point(&request.inputs, "a")?, point(&request.inputs, "b")?,
            number(&request.inputs, "t")?,
        )?)),
        "kernel.vector.length" => Ok(ToolValue::Number(vector_length(
            vector(&request.inputs, "v")?,
        )?)),
        "kernel.vector.normalize" => Ok(ToolValue::Vector(vector_normalize(
            vector(&request.inputs, "v")?,
        )?)),
        "kernel.vector.dot" => Ok(ToolValue::Number(vector_dot(
            vector(&request.inputs, "a")?, vector(&request.inputs, "b")?,
        )?)),
        "kernel.vector.cross" => Ok(ToolValue::Vector(vector_cross(
            vector(&request.inputs, "a")?, vector(&request.inputs, "b")?,
        )?)),
        "kernel.polyline.length" => Ok(ToolValue::Number(polyline_length(
            polyline(&request.inputs, "points")?,
        )?)),
        "kernel.polyline.divide_count" => Ok(ToolValue::Polyline(polyline_divide_count(
            polyline(&request.inputs, "points")?, count(&request.inputs, "count")?,
        )?)),
        "kernel.polyline.divide_distance" => Ok(ToolValue::Polyline(polyline_divide_distance(
            polyline(&request.inputs, "points")?, number(&request.inputs, "spacing")?,
        )?)),
        _ => Err(KernelError::Invalid("shared operation implementation missing")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn point_input(a: Vec3, b: Vec3) -> BTreeMap<String, ToolValue> {
        BTreeMap::from([
            ("a".into(), ToolValue::Point(a)),
            ("b".into(), ToolValue::Point(b)),
        ])
    }
    #[test]
    fn contracts_reference_real_kernel_operations_and_are_unique() {
        let mut operations = std::collections::BTreeSet::new();
        let mut cad = std::collections::BTreeSet::new();
        let mut graph = std::collections::BTreeSet::new();
        for tool in SHARED_TOOLS {
            assert!(crate::operation_by_id(tool.operation).is_some());
            assert!(operations.insert(tool.operation));
            for prerequisite in tool.prerequisites {
                assert_ne!(*prerequisite, tool.operation);
                assert!(crate::operation_by_id(prerequisite).is_some());
            }
            assert!(cad.insert(tool.cad_command));
            assert!(graph.insert(tool.orbweaver_node));
            assert!(!tool.inputs.is_empty());
            let mut names = std::collections::BTreeSet::new();
            for port in tool.inputs {
                assert!(names.insert(port.name));
            }
        }
        assert_eq!(SHARED_TOOLS.len(), 10);
    }
    #[test]
    fn distance_is_shared_across_both_entry_points() {
        let inputs = point_input(Vec3::ZERO, Vec3::new(3., 4., 0.));
        let run = |operation: &str| execute_shared_tool(&ToolRequest {
            operation: operation.into(), inputs: inputs.clone(),
        });
        assert_eq!(run("kernel.point.distance"), Ok(ToolValue::Number(5.)));
        assert_eq!(run("orbweaver.point.distance"), run("worldwright.point.distance"));
    }
    #[test]
    fn interpolation_modifier_changes_one_algorithm() {
        let mut inputs = point_input(Vec3::ZERO, Vec3::new(8., 0., 0.));
        inputs.insert("t".into(), ToolValue::Number(0.25));
        let run = |i: BTreeMap<String, ToolValue>| execute_shared_tool(&ToolRequest {
            operation: "kernel.point.interpolate".into(), inputs: i,
        });
        assert_eq!(run(inputs.clone()), Ok(ToolValue::Point(Vec3::new(2., 0., 0.))));
        inputs.insert("t".into(), ToolValue::Number(0.5));
        assert_eq!(run(inputs.clone()), Ok(ToolValue::Point(Vec3::new(4., 0., 0.))));
        inputs.insert("t".into(), ToolValue::Number(2.));
        assert!(run(inputs).is_err());
    }
    #[test]
    fn strict_ports_prevent_hidden_coercion() {
        let a = Vec3::new(2., 0., 0.);
        let b = Vec3::new(1., 0., 0.);
        let mut inputs = point_input(a, b);
        let run = |i: BTreeMap<String, ToolValue>| execute_shared_tool(&ToolRequest {
            operation: "kernel.point.distance".into(), inputs: i,
        });
        inputs.insert("other".into(), ToolValue::Number(1.));
        assert!(run(inputs.clone()).is_err());
        inputs.remove("other");
        inputs.insert("a".into(), ToolValue::Vector(a));
        assert!(run(inputs.clone()).is_err());
        inputs.remove("b");
        assert!(run(inputs).is_err());
    }
    #[test]
    fn divide_modifier_and_alias_roundtrip() {
        let request = ToolRequest {
            operation: "orbweaver.polyline.divide_count".into(),
            inputs: BTreeMap::from([
                ("points".into(), ToolValue::Polyline(vec![
                    Vec3::ZERO, Vec3::new(10., 0., 0.),
                ])),
                ("count".into(), ToolValue::Count(2)),
            ]),
        };
        let parsed: ToolRequest = serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(execute_shared_tool(&parsed), Ok(ToolValue::Polyline(vec![
            Vec3::ZERO, Vec3::new(5., 0., 0.), Vec3::new(10., 0., 0.),
        ])));
    }
}
