//! Headless example: evaluate OrbWeaver and direct CAD/API dispatch against
//! the same geometry kernel. Run: cargo run -p orbweaver --example paired_distance
use buildercraft_kernel::{ToolRequest, ToolValue, execute_shared_tool};
use cadcraft_geom::Vec3;
use orbweaver::{Graph, InputBinding, Node, evaluate};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let point_a = Vec3::ZERO;
    let point_b = Vec3::new(3., 4., 12.);
    let inputs = BTreeMap::from([
        ("a".to_string(), ToolValue::Point(point_a)),
        ("b".to_string(), ToolValue::Point(point_b)),
    ]);
    let cad_result = execute_shared_tool(&ToolRequest {
        operation: "worldwright.point.distance".into(),
        inputs: inputs.clone(),
    })?;
    let graph = Graph {
        version: 1,
        nodes: vec![Node {
            id: 1,
            component: "orbweaver.point.distance".into(),
            inputs: inputs.into_iter().map(|(key, value)| (
                key, InputBinding::Constant { value },
            )).collect(),
        }],
        outputs: vec![1],
    };
    let graph_result = evaluate(&graph)?;
    let graph_value = graph_result.values.get(&1).ok_or_else(|| std::io::Error::other("missing graph output"))?;
    if graph_value != &cad_result {
        return Err("CAD and OrbWeaver results differ".into());
    }
    println!("Worldwright CAD + OrbWeaver: {graph_value:?}");
    println!("1 shared kernel algorithm; zero duplicate implementations.");
    Ok(())
}
