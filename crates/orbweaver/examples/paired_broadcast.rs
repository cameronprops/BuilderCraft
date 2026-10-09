//! Headless demonstration: a single native Distance algorithm maps across
//! multiple tree branches with explicit Longest matching through both APIs.
//! cargo run -p orbweaver --example paired_broadcast
use buildercraft_kernel::{
    DataTree, ToolRequest, ToolValue, TreeBranch, TreeMatchPolicy, TreePath,
    execute_shared_tool_with_matching,
};
use cadcraft_geom::Vec3;
use orbweaver::{evaluate, Graph, InputBinding, Node};
use std::collections::BTreeMap;

fn point_tree(items: &[(u32, Vec<f64>)]) -> ToolValue {
    ToolValue::Tree(DataTree { branches: items.iter().map(|(path, values)| {
        TreeBranch {
            path: TreePath(vec![0, *path]),
            items: values.iter()
                .map(|value| ToolValue::Point(Vec3::new(*value, 0., 0.)))
                .collect(),
        }
    }).collect() })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let inputs = BTreeMap::from([
        ("a".into(), point_tree(&[
            (0, vec![0., 1.]),
            (1, vec![3.]),
        ])),
        ("b".into(), point_tree(&[
            (0, vec![2., 4., 8.]),
            (1, vec![5.]),
        ])),
    ]);
    let direct = execute_shared_tool_with_matching(&ToolRequest {
        operation: "worldwright.point.distance".into(),
        inputs: inputs.clone(),
    }, TreeMatchPolicy::Longest)?;
    let graph = Graph {
        version: 1,
        nodes: vec![Node {
            id: 1,
            component: "orbweaver.point.distance".into(),
            matching: TreeMatchPolicy::Longest,
            inputs: inputs.into_iter().map(|(name, value)| (
                name,
                InputBinding::Constant { value },
            )).collect(),
        }],
        outputs: vec![1],
    };
    let result = evaluate(&graph)?;
    if result.values.get(&1) != Some(&direct) {
        return Err(std::io::Error::other(
            "Worldwright and OrbWeaver broadcast results differ",
        ).into());
    }
    let ToolValue::Tree(tree) = direct else {
        return Err(std::io::Error::other("expected data-tree output").into());
    };
    if tree.branches.len() != 2 ||
        tree.branches[0].items.len() != 3 ||
        tree.branches[1].items.len() != 1 {
        return Err(std::io::Error::other("unexpected branch/list shape").into());
    }
    println!("Worldwright/OrbWeaver tree broadcast: {tree:?}");
    println!("Matching: longest; 2 branches; existing point.distance algorithm only.");
    Ok(())
}
