//! Typed, host-independent operation contracts shared by Worldwright commands
//! and OrbWeaver nodes. The algorithm lives in the existing kernel; this module
//! only validates named inputs and dispatches to that one implementation.
use crate::{
    DataTree, KernelError, MAX_TREE_ITEMS, Result, TreeBranch, TreeMatchPolicy, point_distance, point_interpolate, point_midpoint,
    polyline_divide_count, polyline_divide_distance, polyline_length, tree_flatten, tree_graft, tree_match, tree_simplify, tree_validate,
    vector_cross, vector_dot, vector_length, vector_normalize,
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
    Tree,
    MatchMode,
    Pair,
    Mesh,
    Surface,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ToolValue {
    Number(f64),
    Count(u64),
    Point(Vec3),
    Vector(Vec3),
    Polyline(Vec<Vec3>),
    /// Native ordered branches, compatible with direct Worldwright commands.
    Tree(DataTree<ToolValue>),
    MatchMode(TreeMatchPolicy),
    Pair(Box<(ToolValue, ToolValue)>),
    Mesh(crate::PolygonMesh),
    Surface(cadcraft_geom::nurbs3d::Surface),
}

impl ToolValue {
    pub fn kind(&self) -> ToolType {
        match self {
            Self::Number(_) => ToolType::Number,
            Self::Count(_) => ToolType::Count,
            Self::Point(_) => ToolType::Point,
            Self::Vector(_) => ToolType::Vector,
            Self::Polyline(_) => ToolType::Polyline,
            Self::Tree(_) => ToolType::Tree,
            Self::MatchMode(_) => ToolType::MatchMode,
            Self::Pair(_) => ToolType::Pair,
            Self::Mesh(_) => ToolType::Mesh,
            Self::Surface(_) => ToolType::Surface,
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

const POLY_STEP_COUNT: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "step", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "count", kind: ToolType::Count, modifier: true },
];
const RECT_ARRAY: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "x_step", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "y_step", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "z_step", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "nx", kind: ToolType::Count, modifier: true },
    ToolPort { name: "ny", kind: ToolType::Count, modifier: true },
    ToolPort { name: "nz", kind: ToolType::Count, modifier: true },
];
const POLAR_ARRAY: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "center", kind: ToolType::Point, modifier: true },
    ToolPort { name: "axis", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "sweep_degrees", kind: ToolType::Number, modifier: true },
    ToolPort { name: "count", kind: ToolType::Count, modifier: true },
];
const PATH_ARRAY: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "path", kind: ToolType::Polyline, modifier: true },
    ToolPort { name: "count", kind: ToolType::Count, modifier: true },
];
const PIPE_INPUTS: &[ToolPort] = &[
    ToolPort { name: "rail", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "up", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "start_radius", kind: ToolType::Number, modifier: true },
    ToolPort { name: "end_radius", kind: ToolType::Number, modifier: true },
    ToolPort { name: "wall", kind: ToolType::Number, modifier: true },
    ToolPort { name: "stations", kind: ToolType::Count, modifier: true },
    ToolPort { name: "sides", kind: ToolType::Count, modifier: true },
    ToolPort { name: "flat_caps", kind: ToolType::Count, modifier: true },
];
const SWEEP1_INPUTS: &[ToolPort] = &[
    ToolPort { name: "rail", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "up", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "profile", kind: ToolType::Polyline, modifier: true },
    ToolPort { name: "stations", kind: ToolType::Count, modifier: true },
    ToolPort { name: "closed_profile", kind: ToolType::Count, modifier: true },
];
const SWEEP2_INPUTS: &[ToolPort] = &[
    ToolPort { name: "rail_a", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "rail_b", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "section", kind: ToolType::Polyline, modifier: true },
    ToolPort { name: "stations", kind: ToolType::Count, modifier: true },
];
const PATH_ARRAY_ORIENTED: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "path", kind: ToolType::Polyline, modifier: true },
    ToolPort { name: "count", kind: ToolType::Count, modifier: true },
    ToolPort { name: "up", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "anchor", kind: ToolType::Point, modifier: true },
];
const PLANE_PROJECT: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "origin", kind: ToolType::Point, modifier: true },
    ToolPort { name: "normal", kind: ToolType::Vector, modifier: true },
    ToolPort { name: "direction", kind: ToolType::Vector, modifier: true },
];
const PATCH_FLOW: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "base", kind: ToolType::Polyline, modifier: true },
    ToolPort { name: "target", kind: ToolType::Polyline, modifier: true },
];
const MESH_PROJECT: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "target", kind: ToolType::Mesh, modifier: true },
    ToolPort { name: "direction", kind: ToolType::Vector, modifier: true },
];
const NURBS_PROJECT: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "target", kind: ToolType::Surface, modifier: true },
    ToolPort { name: "direction", kind: ToolType::Vector, modifier: true },
];
const NURBS_FLOW: &[ToolPort] = &[
    ToolPort { name: "geometry", kind: ToolType::Polyline, modifier: false },
    ToolPort { name: "base", kind: ToolType::Surface, modifier: true },
    ToolPort { name: "target", kind: ToolType::Surface, modifier: true },
];
const QUAD_PUSHPULL: &[ToolPort] =
    &[ToolPort { name: "face", kind: ToolType::Polyline, modifier: false }, ToolPort { name: "distance", kind: ToolType::Number, modifier: true }];
const A_B_POINTS: &[ToolPort] =
    &[ToolPort { name: "a", kind: ToolType::Point, modifier: false }, ToolPort { name: "b", kind: ToolType::Point, modifier: false }];
const A_B_VECTORS: &[ToolPort] =
    &[ToolPort { name: "a", kind: ToolType::Vector, modifier: false }, ToolPort { name: "b", kind: ToolType::Vector, modifier: false }];
const ONE_VECTOR: &[ToolPort] = &[ToolPort { name: "v", kind: ToolType::Vector, modifier: false }];
const POINT_INTERPOLATE: &[ToolPort] = &[
    ToolPort { name: "a", kind: ToolType::Point, modifier: false },
    ToolPort { name: "b", kind: ToolType::Point, modifier: false },
    ToolPort { name: "t", kind: ToolType::Number, modifier: true },
];
const POLYLINE_LENGTH: &[ToolPort] = &[ToolPort { name: "points", kind: ToolType::Polyline, modifier: false }];
const POLYLINE_COUNT: &[ToolPort] =
    &[ToolPort { name: "points", kind: ToolType::Polyline, modifier: false }, ToolPort { name: "count", kind: ToolType::Count, modifier: true }];
const ONE_TREE: &[ToolPort] = &[ToolPort { name: "tree", kind: ToolType::Tree, modifier: false }];
const TREE_MATCH: &[ToolPort] = &[
    ToolPort { name: "a", kind: ToolType::Tree, modifier: false },
    ToolPort { name: "b", kind: ToolType::Tree, modifier: false },
    ToolPort { name: "mode", kind: ToolType::MatchMode, modifier: true },
];
const POLYLINE_DISTANCE: &[ToolPort] =
    &[ToolPort { name: "points", kind: ToolType::Polyline, modifier: false }, ToolPort { name: "spacing", kind: ToolType::Number, modifier: true }];

/// One record per shared, executable operation. No copy of any algorithm exists
/// in the CAD or Graph adapters.
pub const SHARED_TOOLS: &[SharedToolContract] = &[
    SharedToolContract {
        operation: "kernel.point.distance",
        cad_command: "worldwright.point.distance",
        orbweaver_node: "orbweaver.point.distance",
        dependency_group: "geometry.point",
        prerequisites: &[],
        inputs: A_B_POINTS,
        output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.point.midpoint",
        cad_command: "worldwright.point.midpoint",
        orbweaver_node: "orbweaver.point.midpoint",
        dependency_group: "geometry.point",
        prerequisites: &[],
        inputs: A_B_POINTS,
        output: ToolType::Point,
    },
    SharedToolContract {
        operation: "kernel.point.interpolate",
        cad_command: "worldwright.point.interpolate",
        orbweaver_node: "orbweaver.point.interpolate",
        dependency_group: "geometry.point",
        prerequisites: &[],
        inputs: POINT_INTERPOLATE,
        output: ToolType::Point,
    },
    SharedToolContract {
        operation: "kernel.vector.length",
        cad_command: "worldwright.vector.length",
        orbweaver_node: "orbweaver.vector.length",
        dependency_group: "math.vector",
        prerequisites: &[],
        inputs: ONE_VECTOR,
        output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.vector.normalize",
        cad_command: "worldwright.vector.normalize",
        orbweaver_node: "orbweaver.vector.normalize",
        dependency_group: "math.vector",
        prerequisites: &["kernel.vector.length"],
        inputs: ONE_VECTOR,
        output: ToolType::Vector,
    },
    SharedToolContract {
        operation: "kernel.vector.dot",
        cad_command: "worldwright.vector.dot",
        orbweaver_node: "orbweaver.vector.dot",
        dependency_group: "math.vector",
        prerequisites: &[],
        inputs: A_B_VECTORS,
        output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.vector.cross",
        cad_command: "worldwright.vector.cross",
        orbweaver_node: "orbweaver.vector.cross",
        dependency_group: "math.vector",
        prerequisites: &[],
        inputs: A_B_VECTORS,
        output: ToolType::Vector,
    },
    SharedToolContract {
        operation: "kernel.polyline.length",
        cad_command: "worldwright.polyline.length",
        orbweaver_node: "orbweaver.polyline.length",
        dependency_group: "geometry.polyline",
        prerequisites: &["kernel.point.distance"],
        inputs: POLYLINE_LENGTH,
        output: ToolType::Number,
    },
    SharedToolContract {
        operation: "kernel.polyline.divide_count",
        cad_command: "worldwright.polyline.divide_count",
        orbweaver_node: "orbweaver.polyline.divide_count",
        dependency_group: "geometry.polyline",
        prerequisites: &["kernel.polyline.length", "kernel.point.distance", "kernel.point.interpolate"],
        inputs: POLYLINE_COUNT,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.polyline.divide_distance",
        cad_command: "worldwright.polyline.divide_distance",
        orbweaver_node: "orbweaver.polyline.divide_distance",
        dependency_group: "geometry.polyline",
        prerequisites: &["kernel.polyline.length", "kernel.point.distance", "kernel.point.interpolate"],
        inputs: POLYLINE_DISTANCE,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.array.linear",
        cad_command: "worldwright.array.linear",
        orbweaver_node: "orbweaver.array.linear",
        dependency_group: "geometry.transforms",
        prerequisites: &["kernel.geometry.transform_exact"],
        inputs: POLY_STEP_COUNT,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.array.rectangular",
        cad_command: "worldwright.array.rectangular",
        orbweaver_node: "orbweaver.array.rectangular",
        dependency_group: "geometry.transforms",
        prerequisites: &["kernel.geometry.transform_exact"],
        inputs: RECT_ARRAY,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.array.polar",
        cad_command: "worldwright.array.polar",
        orbweaver_node: "orbweaver.array.polar",
        dependency_group: "geometry.transforms",
        prerequisites: &["kernel.geometry.transform_exact"],
        inputs: POLAR_ARRAY,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.array.path",
        cad_command: "worldwright.array.path",
        orbweaver_node: "orbweaver.array.path",
        dependency_group: "geometry.transforms",
        prerequisites: &["kernel.polyline.divide_count"],
        inputs: PATH_ARRAY,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.array.path_oriented",
        cad_command: "worldwright.array.path_oriented",
        orbweaver_node: "orbweaver.array.path_oriented",
        dependency_group: "geometry.transforms",
        prerequisites: &["kernel.polyline.divide_count", "kernel.vector.cross", "kernel.vector.dot"],
        inputs: PATH_ARRAY_ORIENTED,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.project.plane",
        cad_command: "worldwright.project",
        orbweaver_node: "orbweaver.project.plane",
        dependency_group: "geometry.intersections",
        prerequisites: &["kernel.vector.dot", "kernel.vector.normalize"],
        inputs: PLANE_PROJECT,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.surface.flow_patch",
        cad_command: "worldwright.flow_along_srf",
        orbweaver_node: "orbweaver.surface.flow_patch",
        dependency_group: "geometry.surface",
        prerequisites: &["kernel.vector.cross", "kernel.vector.dot"],
        inputs: PATCH_FLOW,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.solid.pushpull_quad",
        cad_command: "worldwright.pushpull",
        orbweaver_node: "orbweaver.solid.pushpull_quad",
        dependency_group: "geometry.solid",
        prerequisites: &["kernel.polygon.validate", "kernel.vector.cross"],
        inputs: QUAD_PUSHPULL,
        output: ToolType::Mesh,
    },
    SharedToolContract {
        operation: "kernel.project.mesh",
        cad_command: "worldwright.project.mesh",
        orbweaver_node: "orbweaver.project.mesh",
        dependency_group: "geometry.intersections",
        prerequisites: &["kernel.polygon.triangulate", "kernel.vector.dot"],
        inputs: MESH_PROJECT,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.project.nurbs",
        cad_command: "worldwright.project.nurbs",
        orbweaver_node: "orbweaver.project.nurbs",
        dependency_group: "geometry.intersections",
        prerequisites: &["kernel.geometry.tessellate", "kernel.vector.dot"],
        inputs: NURBS_PROJECT,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.surface.flow_nurbs",
        cad_command: "worldwright.flow_along_nurbs",
        orbweaver_node: "orbweaver.surface.flow_nurbs",
        dependency_group: "geometry.surface",
        prerequisites: &["kernel.vector.cross", "kernel.vector.dot"],
        inputs: NURBS_FLOW,
        output: ToolType::Polyline,
    },
    SharedToolContract {
        operation: "kernel.pipe.mesh",
        cad_command: "worldwright.pipe",
        orbweaver_node: "orbweaver.pipe.mesh",
        dependency_group: "geometry.surface",
        prerequisites: &["kernel.rail.frames", "kernel.polygon.validate"],
        inputs: PIPE_INPUTS,
        output: ToolType::Mesh,
    },
    SharedToolContract {
        operation: "kernel.sweep1.mesh",
        cad_command: "worldwright.sweep1",
        orbweaver_node: "orbweaver.sweep1.mesh",
        dependency_group: "geometry.surface",
        prerequisites: &["kernel.rail.frames", "kernel.polygon.validate"],
        inputs: SWEEP1_INPUTS,
        output: ToolType::Mesh,
    },
    SharedToolContract {
        operation: "kernel.sweep2.mesh",
        cad_command: "worldwright.sweep2",
        orbweaver_node: "orbweaver.sweep2.mesh",
        dependency_group: "geometry.surface",
        prerequisites: &["kernel.rail.frames", "kernel.polygon.validate"],
        inputs: SWEEP2_INPUTS,
        output: ToolType::Mesh,
    },
    SharedToolContract {
        operation: "kernel.tree.validate",
        cad_command: "worldwright.tree.validate",
        orbweaver_node: "orbweaver.tree.validate",
        dependency_group: "graph.list_tree",
        prerequisites: &[],
        inputs: ONE_TREE,
        output: ToolType::Count,
    },
    SharedToolContract {
        operation: "kernel.tree.flatten",
        cad_command: "worldwright.tree.flatten",
        orbweaver_node: "orbweaver.tree.flatten",
        dependency_group: "graph.list_tree",
        prerequisites: &["kernel.tree.validate"],
        inputs: ONE_TREE,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.tree.graft",
        cad_command: "worldwright.tree.graft",
        orbweaver_node: "orbweaver.tree.graft",
        dependency_group: "graph.list_tree",
        prerequisites: &["kernel.tree.validate"],
        inputs: ONE_TREE,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.tree.simplify",
        cad_command: "worldwright.tree.simplify",
        orbweaver_node: "orbweaver.tree.simplify",
        dependency_group: "graph.list_tree",
        prerequisites: &["kernel.tree.validate"],
        inputs: ONE_TREE,
        output: ToolType::Tree,
    },
    SharedToolContract {
        operation: "kernel.tree.match",
        cad_command: "worldwright.tree.match",
        orbweaver_node: "orbweaver.tree.match",
        dependency_group: "graph.list_tree",
        prerequisites: &["kernel.tree.validate"],
        inputs: TREE_MATCH,
        output: ToolType::Tree,
    },
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRequest {
    pub operation: String,
    pub inputs: BTreeMap<String, ToolValue>,
}

pub fn shared_tool(operation: &str) -> Option<&'static SharedToolContract> {
    SHARED_TOOLS.iter().find(|tool| tool.operation == operation || tool.cad_command == operation || tool.orbweaver_node == operation)
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

fn mesh<'a>(inputs: &'a BTreeMap<String, ToolValue>, name: &str) -> Result<&'a crate::PolygonMesh> {
    match inputs.get(name) {
        Some(ToolValue::Mesh(mesh)) => Ok(mesh),
        _ => Err(KernelError::Invalid("native polygon mesh input")),
    }
}
fn surface<'a>(inputs: &'a BTreeMap<String, ToolValue>, name: &str) -> Result<&'a cadcraft_geom::nurbs3d::Surface> {
    match inputs.get(name) {
        Some(ToolValue::Surface(s)) => Ok(s),
        _ => Err(KernelError::Invalid("NURBS surface input")),
    }
}
fn tree<'a>(inputs: &'a BTreeMap<String, ToolValue>, name: &str) -> Result<&'a DataTree<ToolValue>> {
    match inputs.get(name) {
        Some(ToolValue::Tree(value)) => Ok(value),
        _ => Err(KernelError::Invalid("tree input")),
    }
}
fn match_mode(inputs: &BTreeMap<String, ToolValue>, name: &str) -> Result<TreeMatchPolicy> {
    match inputs.get(name) {
        Some(ToolValue::MatchMode(mode)) => Ok(*mode),
        _ => Err(KernelError::Invalid("tree matching modifier")),
    }
}
/// Count primitive storage units so a tree of heavy polylines cannot bypass
/// the normal item budget. Depth is capped to avoid arbitrarily nested trees.
fn value_cost(value: &ToolValue, depth: usize) -> Result<usize> {
    if depth > 8 {
        return Err(KernelError::Budget);
    }
    match value {
        ToolValue::Number(value) if !value.is_finite() => Err(KernelError::Invalid("nonfinite numeric value")),
        ToolValue::Point(v) | ToolValue::Vector(v) if !v.is_finite() || [v.x, v.y, v.z].iter().any(|x| x.abs() > 1e12) => {
            Err(KernelError::Invalid("nonfinite or oversized geometric value"))
        }
        ToolValue::Polyline(points) => {
            if points.len() > MAX_TREE_ITEMS || points.iter().any(|v| !v.is_finite() || [v.x, v.y, v.z].iter().any(|x| x.abs() > 1e12)) {
                Err(KernelError::Budget)
            } else {
                Ok(points.len().max(1))
            }
        }
        ToolValue::Surface(surface) => {
            if !surface.valid() {
                return Err(KernelError::Invalid("invalid rational surface input"));
            }
            let count = surface.rows.iter().try_fold(0usize, |n, row| n.checked_add(row.control.len()).ok_or(KernelError::Budget))?;
            if count > MAX_TREE_ITEMS { Err(KernelError::Budget) } else { Ok(count) }
        }
        ToolValue::Mesh(mesh) => {
            crate::polygon_mesh_validate(mesh)?;
            mesh.vertices.len().checked_add(mesh.faces.len()).filter(|n| *n <= MAX_TREE_ITEMS).ok_or(KernelError::Budget)
        }
        ToolValue::Pair(pair) => {
            let total = value_cost(&pair.0, depth + 1)?.checked_add(value_cost(&pair.1, depth + 1)?).ok_or(KernelError::Budget)?;
            if total > MAX_TREE_ITEMS { Err(KernelError::Budget) } else { Ok(total) }
        }
        ToolValue::Tree(tree) => {
            tree_validate(tree)?;
            let mut total = tree.branches.len();
            for branch in &tree.branches {
                for item in &branch.items {
                    total = total.checked_add(value_cost(item, depth + 1)?).ok_or(KernelError::Budget)?;
                    if total > MAX_TREE_ITEMS {
                        return Err(KernelError::Budget);
                    }
                }
            }
            Ok(total)
        }
        _ => Ok(1),
    }
}
/// Cost of nested typed values for both CAD/API and OrbWeaver graph limits.
/// Limits are abstract item units, not an RSS/byte guarantee.
pub fn shared_tool_value_cost(value: &ToolValue) -> Result<usize> {
    value_cost(value, 0)
}

fn match_tree_values(a: &DataTree<ToolValue>, b: &DataTree<ToolValue>, mode: TreeMatchPolicy) -> Result<DataTree<ToolValue>> {
    // Preflight cloned value units BEFORE allocating the Cartesian or
    // longest-list result, not merely checking the number of output pairs.
    tree_validate(a)?;
    tree_validate(b)?;
    if a.branches.len() != b.branches.len() {
        return Err(KernelError::Invalid("tree branch path mismatch"));
    }
    let mut budget = a.branches.len();
    for (left, right) in a.branches.iter().zip(&b.branches) {
        if left.path != right.path {
            return Err(KernelError::Invalid("tree branch path mismatch"));
        }
        let n = match mode {
            TreeMatchPolicy::Shortest => left.items.len().min(right.items.len()),
            TreeMatchPolicy::Longest => left.items.len().max(right.items.len()),
            TreeMatchPolicy::CrossReference => left.items.len().checked_mul(right.items.len()).ok_or(KernelError::Budget)?,
        };
        if matches!(mode, TreeMatchPolicy::Longest) && left.items.is_empty() != right.items.is_empty() {
            return Err(KernelError::Invalid("cannot repeat missing tree item"));
        }
        if n > MAX_TREE_ITEMS {
            return Err(KernelError::Budget);
        }
        for index in 0..n {
            let (ai, bi) = match mode {
                TreeMatchPolicy::CrossReference => (index / right.items.len(), index % right.items.len()),
                TreeMatchPolicy::Shortest => (index, index),
                TreeMatchPolicy::Longest => (index.min(left.items.len() - 1), index.min(right.items.len() - 1)),
            };
            budget = budget
                .checked_add(2)
                .and_then(|x| x.checked_add(value_cost(&left.items[ai], 1).ok()?))
                .and_then(|x| x.checked_add(value_cost(&right.items[bi], 1).ok()?))
                .ok_or(KernelError::Budget)?;
            if budget > MAX_TREE_ITEMS {
                return Err(KernelError::Budget);
            }
        }
    }
    let paired = tree_match(a, b, mode)?;
    let result = DataTree {
        branches: paired
            .branches
            .into_iter()
            .map(|branch| TreeBranch { path: branch.path, items: branch.items.into_iter().map(|(a, b)| ToolValue::Pair(Box::new((a, b)))).collect() })
            .collect(),
    };
    Ok(result)
}

/// Execute a pure shared operation. Missing, extra, incorrectly typed or
/// nonfinite inputs fail before host document mutation. The native geometry
/// functions apply their original bounds and degeneracy checks.
pub fn execute_shared_tool(request: &ToolRequest) -> Result<ToolValue> {
    execute_shared_tool_with_matching(request, TreeMatchPolicy::Shortest)
}

/// Explicit native list/branch matching modifier. Any scalar/point/vector/
/// polyline port may accept a typed tree of that leaf kind. Each item is
/// evaluated by EXACTLY the same scalar kernel operation as the direct CAD
/// command. No implicit path expansion is attempted.
pub fn execute_shared_tool_with_matching(request: &ToolRequest, matching: TreeMatchPolicy) -> Result<ToolValue> {
    let contract = shared_tool(&request.operation).ok_or(KernelError::Invalid("unregistered shared operation"))?;
    if request.inputs.len() != contract.inputs.len() {
        return Err(KernelError::Invalid("missing or unexpected tool port"));
    }
    let mut lifted = false;
    for port in contract.inputs {
        let value = request.inputs.get(port.name).ok_or(KernelError::Invalid("missing tool port"))?;
        crate::tool_broadcast::tool_value_matches_port(port.kind, value)?;
        if port.kind != ToolType::Tree && value.kind() == ToolType::Tree {
            lifted = true;
        }
    }
    if lifted {
        return crate::tool_broadcast::execute_lifted(request, contract, matching, dispatch_scalar);
    }
    dispatch_scalar(request)
}

/// The only implementation of native scalar/geometry operations. The lifted
/// adapter constructs strictly typed per-item requests and delegates here.
fn dispatch_scalar(request: &ToolRequest) -> Result<ToolValue> {
    let contract = shared_tool(&request.operation).ok_or(KernelError::Invalid("unregistered shared operation"))?;
    let array_output = |copies: Vec<Vec<Vec3>>| -> Result<ToolValue> {
        let branches = copies
            .into_iter()
            .enumerate()
            .map(|(i, points)| -> Result<TreeBranch<ToolValue>> {
                Ok(TreeBranch {
                    path: crate::TreePath(vec![u32::try_from(i).map_err(|_| KernelError::Budget)?]),
                    items: vec![ToolValue::Polyline(points)],
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let tree = DataTree { branches };
        tree_validate(&tree)?;
        Ok(ToolValue::Tree(tree))
    };
    match contract.operation {
        "kernel.array.linear" => array_output(crate::array_linear(
            polyline(&request.inputs, "geometry")?,
            vector(&request.inputs, "step")?,
            count(&request.inputs, "count")?,
        )?),
        "kernel.array.rectangular" => array_output(crate::array_rectangular(
            polyline(&request.inputs, "geometry")?,
            vector(&request.inputs, "x_step")?,
            vector(&request.inputs, "y_step")?,
            vector(&request.inputs, "z_step")?,
            count(&request.inputs, "nx")?,
            count(&request.inputs, "ny")?,
            count(&request.inputs, "nz")?,
        )?),
        "kernel.array.polar" => array_output(crate::array_polar(
            polyline(&request.inputs, "geometry")?,
            point(&request.inputs, "center")?,
            vector(&request.inputs, "axis")?,
            number(&request.inputs, "sweep_degrees")?,
            count(&request.inputs, "count")?,
        )?),
        "kernel.array.path" => array_output(crate::array_path(
            polyline(&request.inputs, "geometry")?,
            polyline(&request.inputs, "path")?,
            count(&request.inputs, "count")?,
        )?),
        "kernel.array.path_oriented" => array_output(crate::array_path_oriented(
            polyline(&request.inputs, "geometry")?,
            polyline(&request.inputs, "path")?,
            count(&request.inputs, "count")?,
            vector(&request.inputs, "up")?,
            point(&request.inputs, "anchor")?,
        )?),
        "kernel.project.plane" => Ok(ToolValue::Polyline(crate::project_to_plane(
            polyline(&request.inputs, "geometry")?,
            point(&request.inputs, "origin")?,
            vector(&request.inputs, "normal")?,
            vector(&request.inputs, "direction")?,
        )?)),
        "kernel.surface.flow_patch" => Ok(ToolValue::Polyline(crate::flow_along_patch(
            polyline(&request.inputs, "geometry")?,
            polyline(&request.inputs, "base")?,
            polyline(&request.inputs, "target")?,
        )?)),
        "kernel.solid.pushpull_quad" => {
            Ok(ToolValue::Mesh(crate::pushpull_quad(polyline(&request.inputs, "face")?, number(&request.inputs, "distance")?)?))
        }
        "kernel.project.mesh" => Ok(ToolValue::Polyline(crate::project_onto_mesh(
            polyline(&request.inputs, "geometry")?,
            mesh(&request.inputs, "target")?,
            vector(&request.inputs, "direction")?,
        )?)),
        "kernel.project.nurbs" => Ok(ToolValue::Polyline(crate::project_onto_nurbs(
            polyline(&request.inputs, "geometry")?,
            surface(&request.inputs, "target")?,
            vector(&request.inputs, "direction")?,
        )?)),
        "kernel.surface.flow_nurbs" => Ok(ToolValue::Polyline(crate::flow_along_nurbs(
            polyline(&request.inputs, "geometry")?,
            surface(&request.inputs, "base")?,
            surface(&request.inputs, "target")?,
        )?)),
        "kernel.pipe.mesh" => {
            let cap = count(&request.inputs, "flat_caps")?;
            if cap > 1 {
                return Err(KernelError::Invalid("pipe flat_caps must be zero or one"));
            }
            Ok(ToolValue::Mesh(crate::pipe_mesh(
                polyline(&request.inputs, "rail")?,
                vector(&request.inputs, "up")?,
                number(&request.inputs, "start_radius")?,
                number(&request.inputs, "end_radius")?,
                number(&request.inputs, "wall")?,
                count(&request.inputs, "stations")?,
                count(&request.inputs, "sides")?,
                cap == 1,
            )?))
        }
        "kernel.sweep1.mesh" => {
            let closed = count(&request.inputs, "closed_profile")?;
            if closed > 1 {
                return Err(KernelError::Invalid("closed_profile must be zero or one"));
            }
            Ok(ToolValue::Mesh(crate::sweep1_mesh(
                polyline(&request.inputs, "rail")?,
                vector(&request.inputs, "up")?,
                polyline(&request.inputs, "profile")?,
                count(&request.inputs, "stations")?,
                closed == 1,
            )?))
        }
        "kernel.sweep2.mesh" => Ok(ToolValue::Mesh(crate::sweep2_mesh(
            polyline(&request.inputs, "rail_a")?,
            polyline(&request.inputs, "rail_b")?,
            polyline(&request.inputs, "section")?,
            count(&request.inputs, "stations")?,
        )?)),
        "kernel.point.distance" => Ok(ToolValue::Number(point_distance(point(&request.inputs, "a")?, point(&request.inputs, "b")?)?)),
        "kernel.point.midpoint" => Ok(ToolValue::Point(point_midpoint(point(&request.inputs, "a")?, point(&request.inputs, "b")?)?)),
        "kernel.point.interpolate" => {
            Ok(ToolValue::Point(point_interpolate(point(&request.inputs, "a")?, point(&request.inputs, "b")?, number(&request.inputs, "t")?)?))
        }
        "kernel.vector.length" => Ok(ToolValue::Number(vector_length(vector(&request.inputs, "v")?)?)),
        "kernel.vector.normalize" => Ok(ToolValue::Vector(vector_normalize(vector(&request.inputs, "v")?)?)),
        "kernel.vector.dot" => Ok(ToolValue::Number(vector_dot(vector(&request.inputs, "a")?, vector(&request.inputs, "b")?)?)),
        "kernel.vector.cross" => Ok(ToolValue::Vector(vector_cross(vector(&request.inputs, "a")?, vector(&request.inputs, "b")?)?)),
        "kernel.polyline.length" => Ok(ToolValue::Number(polyline_length(polyline(&request.inputs, "points")?)?)),
        "kernel.polyline.divide_count" => {
            Ok(ToolValue::Polyline(polyline_divide_count(polyline(&request.inputs, "points")?, count(&request.inputs, "count")?)?))
        }
        "kernel.polyline.divide_distance" => {
            Ok(ToolValue::Polyline(polyline_divide_distance(polyline(&request.inputs, "points")?, number(&request.inputs, "spacing")?)?))
        }
        "kernel.tree.validate" => {
            Ok(ToolValue::Count(u64::try_from(tree_validate(tree(&request.inputs, "tree")?)?).map_err(|_| KernelError::Budget)?))
        }
        "kernel.tree.flatten" => Ok(ToolValue::Tree(tree_flatten(tree(&request.inputs, "tree")?)?)),
        "kernel.tree.graft" => Ok(ToolValue::Tree(tree_graft(tree(&request.inputs, "tree")?)?)),
        "kernel.tree.simplify" => Ok(ToolValue::Tree(tree_simplify(tree(&request.inputs, "tree")?)?)),
        "kernel.tree.match" => {
            Ok(ToolValue::Tree(match_tree_values(tree(&request.inputs, "a")?, tree(&request.inputs, "b")?, match_mode(&request.inputs, "mode")?)?))
        }
        _ => Err(KernelError::Invalid("shared operation implementation missing")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn point_input(a: Vec3, b: Vec3) -> BTreeMap<String, ToolValue> {
        BTreeMap::from([("a".into(), ToolValue::Point(a)), ("b".into(), ToolValue::Point(b))])
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
        assert_eq!(SHARED_TOOLS.len(), 29);
    }
    #[test]
    fn distance_is_shared_across_both_entry_points() {
        let inputs = point_input(Vec3::ZERO, Vec3::new(3., 4., 0.));
        let run = |operation: &str| execute_shared_tool(&ToolRequest { operation: operation.into(), inputs: inputs.clone() });
        assert_eq!(run("kernel.point.distance"), Ok(ToolValue::Number(5.)));
        assert_eq!(run("orbweaver.point.distance"), run("worldwright.point.distance"));
    }
    #[test]
    fn interpolation_modifier_changes_one_algorithm() {
        let mut inputs = point_input(Vec3::ZERO, Vec3::new(8., 0., 0.));
        inputs.insert("t".into(), ToolValue::Number(0.25));
        let run = |i: BTreeMap<String, ToolValue>| execute_shared_tool(&ToolRequest { operation: "kernel.point.interpolate".into(), inputs: i });
        assert_eq!(run(inputs.clone()), Ok(ToolValue::Point(Vec3::new(2., 0., 0.))));
        inputs.insert("t".into(), ToolValue::Number(0.5));
        assert_eq!(run(inputs.clone()), Ok(ToolValue::Point(Vec3::new(4., 0., 0.))));
        inputs.insert("t".into(), ToolValue::Number(2.));
        assert!(run(inputs).is_err());
    }
    #[test]
    fn tree_operations_remain_paired_and_roundtrip_tagged_values() {
        let tree = DataTree {
            branches: vec![TreeBranch {
                path: crate::TreePath(vec![0, 2]),
                items: vec![ToolValue::Point(Vec3::new(2., 0., 0.)), ToolValue::Point(Vec3::new(4., 0., 0.))],
            }],
        };
        let original = ToolValue::Tree(tree.clone());
        let decoded: ToolValue = serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(SHARED_TOOLS.len(), 29);
        let cmd = |op: &str| execute_shared_tool(&ToolRequest { operation: op.into(), inputs: BTreeMap::from([("tree".into(), original.clone())]) });
        let graft = cmd("worldwright.tree.graft").unwrap();
        assert_eq!(graft, cmd("orbweaver.tree.graft").unwrap());
        let ToolValue::Tree(grafted_tree) = graft else { panic!("expected tree") };
        assert_eq!(grafted_tree.branches.len(), 2);
    }
    #[test]
    fn tree_matching_modifier_keeps_branch_paths_and_pairs() {
        let make = |items| ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: crate::TreePath(vec![7]), items }] });
        let matched = execute_shared_tool(&ToolRequest {
            operation: "orbweaver.tree.match".into(),
            inputs: BTreeMap::from([
                ("a".into(), make(vec![ToolValue::Count(1)])),
                ("b".into(), make(vec![ToolValue::Count(3), ToolValue::Count(4)])),
                ("mode".into(), ToolValue::MatchMode(TreeMatchPolicy::Longest)),
            ]),
        })
        .unwrap();
        let ToolValue::Tree(result) = matched else { panic!("tree needed") };
        assert_eq!(result.branches[0].path.0, vec![7]);
        assert_eq!(
            result.branches[0].items,
            vec![
                ToolValue::Pair(Box::new((ToolValue::Count(1), ToolValue::Count(3)))),
                ToolValue::Pair(Box::new((ToolValue::Count(1), ToolValue::Count(4)))),
            ]
        );
    }
    #[test]
    fn strict_ports_prevent_hidden_coercion() {
        let a = Vec3::new(2., 0., 0.);
        let b = Vec3::new(1., 0., 0.);
        let mut inputs = point_input(a, b);
        let run = |i: BTreeMap<String, ToolValue>| execute_shared_tool(&ToolRequest { operation: "kernel.point.distance".into(), inputs: i });
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
                ("points".into(), ToolValue::Polyline(vec![Vec3::ZERO, Vec3::new(10., 0., 0.)])),
                ("count".into(), ToolValue::Count(2)),
            ]),
        };
        let parsed: ToolRequest = serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        assert_eq!(execute_shared_tool(&parsed), Ok(ToolValue::Polyline(vec![Vec3::ZERO, Vec3::new(5., 0., 0.), Vec3::new(10., 0., 0.),])));
    }
}
