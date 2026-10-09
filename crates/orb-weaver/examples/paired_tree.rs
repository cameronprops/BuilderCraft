//! Headless Orb Weaver data-tree graph. Run:
//! cargo run -p orb-weaver --example paired_tree
//! Demonstrates that a direct CAD tree operation and a graph node call exactly
//! the same native kernel function; no Rhino/Grasshopper host is required.
use buildercraft_kernel::{
    DataTree, ToolRequest, ToolValue, TreeBranch, TreePath, execute_shared_tool,
};
use orb_weaver::{Graph, InputBinding, Node, evaluate};
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tree = ToolValue::Tree(DataTree {
        branches: vec![
            TreeBranch { path: TreePath(vec![0, 1]), items: vec![
                ToolValue::Count(4), ToolValue::Count(8),
            ] },
            TreeBranch { path: TreePath(vec![0, 2]), items: vec![
                ToolValue::Count(12),
            ] },
        ],
    });
    let direct = execute_shared_tool(&ToolRequest {
        operation: "worldwright.tree.flatten".into(),
        inputs: BTreeMap::from([("tree".into(), tree.clone())]),
    })?;
    let graph = Graph {
        version: 1,
        nodes: vec![Node {
            id: 1,
            component: "orbweaver.tree.flatten".into(),
            inputs: BTreeMap::from([(
                "tree".into(),
                InputBinding::Constant { value: tree },
            )]),
        }],
        outputs: vec![1],
    };
    let result = evaluate(&graph)?;
    if result.values.get(&1) != Some(&direct) {
        return Err(std::io::Error::other("CAD/Orb Weaver tree flatten mismatch").into());
    }
    println!("Orb Weaver flatten: {direct:?}");
    println!("One kernel operation serves both Worldwright and Orb Weaver.");
    Ok(())
}
