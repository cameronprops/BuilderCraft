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
        CommandSpec::new("worldwright.tool.list", "List Paired CAD/Calisoga Tools", list)
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
    fn discovery_contains_shared_node_and_command_pairs() {
        let mut session = Session::new();
        let result = session.execute("worldwright.tool.list",&json!({})).unwrap();
        assert_eq!(result["paired_tools"].as_array().map(Vec::len),Some(10));
        assert_eq!(result["paired_tools"][0]["calisoga_node"],"calisoga.point.distance");
    }
}
