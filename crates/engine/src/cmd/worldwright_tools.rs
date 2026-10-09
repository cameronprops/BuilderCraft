//! Worldwright CAD/API commands for shared kernel operations.
//! These are numeric, headless commands: no geometry algorithm is duplicated.
use super::*;
use buildercraft_kernel::{ToolRequest, ToolValue, SHARED_TOOLS, execute_shared_tool};
use serde_json::json;
use std::collections::BTreeMap;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("worldwright.tool.run", "Run Shared Native Tool", run)
            .params("{operation,inputs:{port:{kind,value},...}}")
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.tool.list", "List Paired CAD/Orb Weaver Tools", list)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.point.distance", "Distance Between 3D Points", point_distance)
            .params("{inputs:{a:{kind:point,value:{x,y,z}},b:{kind:point,value:{x,y,z}}}}")
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.point.midpoint", "Midpoint of 3D Points", point_midpoint)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.point.interpolate", "Interpolate Between 3D Points", point_interpolate)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.length", "Vector Length", vector_length)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.normalize", "Normalize Vector", vector_normalize)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.dot", "Vector Dot Product", vector_dot)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.vector.cross", "Vector Cross Product", vector_cross)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.length", "Polyline Length", polyline_length)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.divide_count", "Divide Polyline by Count", polyline_divide_count)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.polyline.divide_distance", "Divide Polyline by Spacing", polyline_divide_distance)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.validate", "Validate Data Tree", tree_validate)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.flatten", "Flatten Data Tree", tree_flatten)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.graft", "Graft Data Tree", tree_graft)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.simplify", "Simplify Data Tree", tree_simplify)
            .enabled(always).noundo(),
        CommandSpec::new("worldwright.tree.match", "Match Two Data Trees", tree_match)
            .enabled(always).noundo(),
    ]
}

fn invalid(message: &str) -> crate::EngineError {
    crate::EngineError::Other(message.into())
}
fn invoke(operation: &str, p: &Value) -> Result<Value> {
    let inputs: BTreeMap<String, ToolValue> = serde_json::from_value(
        p.get("inputs").cloned().ok_or_else(|| invalid("inputs required"))?,
    ).map_err(|e| invalid(&e.to_string()))?;
    let output = execute_shared_tool(&ToolRequest {
        operation: operation.into(),
        inputs,
    }).map_err(|e| invalid(&e.to_string()))?;
    Ok(json!({"operation":operation,"output":output}))
}
fn run(_: &mut Session, p: &Value) -> Result<Value> {
    let operation = p.get("operation").and_then(Value::as_str)
        .ok_or_else(|| invalid("operation required"))?;
    invoke(operation, p)
}
fn list(_: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"paired_tools": SHARED_TOOLS}))
}
macro_rules! paired_command {
    ($function:ident, $operation:literal) => {
        fn $function(_: &mut Session, p: &Value) -> Result<Value> {
            invoke($operation, p)
        }
    };
}
paired_command!(point_distance, "kernel.point.distance");
paired_command!(point_midpoint, "kernel.point.midpoint");
paired_command!(point_interpolate, "kernel.point.interpolate");
paired_command!(vector_length, "kernel.vector.length");
paired_command!(vector_normalize, "kernel.vector.normalize");
paired_command!(vector_dot, "kernel.vector.dot");
paired_command!(vector_cross, "kernel.vector.cross");
paired_command!(polyline_length, "kernel.polyline.length");
paired_command!(polyline_divide_count, "kernel.polyline.divide_count");
paired_command!(polyline_divide_distance, "kernel.polyline.divide_distance");
paired_command!(tree_validate, "kernel.tree.validate");
paired_command!(tree_flatten, "kernel.tree.flatten");
paired_command!(tree_graft, "kernel.tree.graft");
paired_command!(tree_simplify, "kernel.tree.simplify");
paired_command!(tree_match, "kernel.tree.match");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_and_generic_dispatch_execute_identical_point_distance() {
        let mut session = Session::new();
        let inputs = json!({
            "a":{"kind":"point","value":{"x":0.,"y":0.,"z":0.}},
            "b":{"kind":"point","value":{"x":3.,"y":4.,"z":0.}}
        });
        let direct = session.execute("worldwright.point.distance", &json!({"inputs":inputs})).unwrap();
        let generic = session.execute("worldwright.tool.run", &json!({
            "operation":"kernel.point.distance","inputs":inputs
        })).unwrap();
        assert_eq!(direct, generic);
        assert_eq!(direct["output"]["kind"], "number");
        assert_eq!(direct["output"]["value"], 5.);
    }
    #[test]
    fn numeric_modifier_uses_the_same_point_interpolator() {
        let mut session = Session::new();
        let result = session.execute("worldwright.point.interpolate", &json!({"inputs":{
            "a":{"kind":"point","value":{"x":0.,"y":0.,"z":0.}},
            "b":{"kind":"point","value":{"x":8.,"y":0.,"z":0.}},
            "t":{"kind":"number","value":0.25}
        }})).unwrap();
        assert_eq!(result["output"]["value"]["x"], 2.);
    }
    #[test]
    fn malformed_ports_and_unknown_operations_are_rejected() {
        let mut session = Session::new();
        assert!(session.execute("worldwright.point.distance",&json!({"inputs":{}})).is_err());
        assert!(session.execute("worldwright.tool.run",&json!({
            "operation":"kernel.missing","inputs":{}
        })).is_err());
    }
    #[test]
    fn graft_command_routes_to_the_same_native_kernel_as_orb_weaver() {
        let mut session = Session::new();
        let tree = json!({"kind":"tree","value":{"branches":[
            {"path":[0],"items":[{"kind":"number","value":3.0},{"kind":"number","value":7.0}]}
        ]}});
        let inputs = json!({"tree": tree});
        let direct = session.execute("worldwright.tree.graft", &json!({"inputs":inputs})).unwrap();
        let generic = session.execute("worldwright.tool.run", &json!({
            "operation":"orbweaver.tree.graft","inputs":inputs
        })).unwrap();
        assert_eq!(direct["output"], generic["output"]);
        assert_eq!(direct["output"]["value"]["branches"].as_array().map(Vec::len), Some(2));
    }
    #[test]
    fn matching_modifier_rejects_invalid_branch_sets_atomically() {
        let mut session = Session::new();
        let result = session.execute("worldwright.tree.match", &json!({"inputs":{
            "a":{"kind":"tree","value":{"branches":[{"path":[0],"items":[
                {"kind":"number","value":1.0}]}]}},
            "b":{"kind":"tree","value":{"branches":[{"path":[1],"items":[
                {"kind":"number","value":2.0}]}]}},
            "mode":{"kind":"match_mode","value":"longest"}
        }}));
        assert!(result.is_err());
    }
    #[test]
    fn discovery_contains_shared_node_and_command_pairs() {
        let mut session = Session::new();
        let result = session.execute("worldwright.tool.list",&json!({})).unwrap();
        assert_eq!(result["paired_tools"].as_array().map(Vec::len),Some(15));
        assert_eq!(result["paired_tools"][0]["orbweaver_node"],"orbweaver.point.distance");
    }
}
