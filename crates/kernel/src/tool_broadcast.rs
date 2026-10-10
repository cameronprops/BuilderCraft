//! Typed branch-wise execution adapter for existing point, vector and polyline
//! operators. The math lives ONLY in the shared scalar dispatcher, so CAD
//! commands and OrbWeaver nodes produce the same numeric/tree results.
//!
//! Native policy is intentionally explicit: identical canonical branch paths,
//! scalar broadcasting, and Shortest/Longest/CrossReference item matching.
//! This is not a claim of Grasshopper's implicit path-alignment semantics.

use crate::{
    DataTree, KernelError, MAX_TREE_BRANCHES, MAX_TREE_ITEMS, Result, SharedToolContract, ToolRequest, ToolType, ToolValue, TreeBranch,
    TreeMatchPolicy, TreePath, shared_tool_value_cost, tree_validate,
};
use std::collections::BTreeMap;

/// Return whether a concrete value can satisfy an operator port. Trees may
/// replace scalar/geometry inputs if every leaf has precisely the required
/// type; they may never masquerade as tree policy, pair, or tree-native ports.
pub fn tool_value_matches_port(expected: ToolType, value: &ToolValue) -> Result<()> {
    let supports_lifting = matches!(expected, ToolType::Number | ToolType::Count | ToolType::Point | ToolType::Vector | ToolType::Polyline);
    if value.kind() == expected {
        shared_tool_value_cost(value)?;
        return Ok(());
    }
    if supports_lifting && let ToolValue::Tree(tree) = value {
        tree_validate(tree)?;
        shared_tool_value_cost(value)?;
        for branch in &tree.branches {
            for item in &branch.items {
                if item.kind() != expected {
                    return Err(KernelError::Invalid("tree leaf type does not match port"));
                }
            }
        }
        return Ok(());
    }
    Err(KernelError::Invalid("tool port type mismatch"))
}

/// Whether the declared output of an upstream operation MIGHT supply a wired
/// port at runtime. Tree broadcasting can wrap a declared scalar output in a
/// tree; a tree producing node may feed a scalar port only if its items match.
/// The actual result is always checked by tool_value_matches_port.
pub fn tool_output_may_match_port(expected: ToolType, upstream: ToolType) -> bool {
    if expected == upstream {
        return true;
    }
    if upstream == ToolType::Tree {
        return matches!(expected, ToolType::Number | ToolType::Count | ToolType::Point | ToolType::Vector | ToolType::Polyline);
    }
    expected == ToolType::Tree && matches!(upstream, ToolType::Number | ToolType::Count | ToolType::Point | ToolType::Vector | ToolType::Polyline)
}

/// Match each tree input STRICTLY by branch path and then by item position.
/// All scalar input ports are broadcast to every item in every branch.
/// Shortest uses minimum branch length, Longest repeats the final item, and
/// CrossReference uses Cartesian product with the rightmost input changing
/// fastest. Invalid input/overflow fails without publishing partial results.
pub(crate) fn execute_lifted(
    request: &ToolRequest,
    contract: &SharedToolContract,
    policy: TreeMatchPolicy,
    scalar: impl Fn(&ToolRequest) -> Result<ToolValue>,
) -> Result<ToolValue> {
    let mut tree_ports = Vec::<(&str, &DataTree<ToolValue>)>::new();
    for port in contract.inputs {
        if let Some(ToolValue::Tree(tree)) = request.inputs.get(port.name)
            && port.kind != ToolType::Tree
        {
            tree_validate(tree)?;
            tree_ports.push((port.name, tree));
        }
    }
    let Some((_, first)) = tree_ports.first() else {
        return Err(KernelError::Invalid("tree broadcasting requires a tree input"));
    };
    if first.branches.len() > MAX_TREE_BRANCHES {
        return Err(KernelError::Budget);
    }
    for (_, other) in tree_ports.iter().skip(1) {
        if other.branches.len() != first.branches.len() || other.branches.iter().zip(&first.branches).any(|(a, b)| a.path != b.path) {
            return Err(KernelError::Invalid("tree branch path mismatch"));
        }
    }

    // Preflight output counts and branch paths before invoking any algorithms.
    let mut output_count = first.branches.len();
    let mut sizes = Vec::new();
    sizes.try_reserve_exact(first.branches.len()).map_err(|_| KernelError::Budget)?;
    for branch_index in 0..first.branches.len() {
        let mut lengths = Vec::new();
        lengths.try_reserve_exact(tree_ports.len()).map_err(|_| KernelError::Budget)?;
        for (_, tree) in &tree_ports {
            lengths.push(tree.branches[branch_index].items.len());
        }
        let size = match policy {
            TreeMatchPolicy::Shortest => lengths.iter().copied().min().unwrap_or(0),
            TreeMatchPolicy::Longest => {
                let max = lengths.iter().copied().max().unwrap_or(0);
                if max > 0 && lengths.contains(&0) {
                    return Err(KernelError::Invalid("cannot repeat missing tree item"));
                }
                max
            }
            TreeMatchPolicy::CrossReference => {
                let mut product = 1usize;
                for &length in &lengths {
                    product = product.checked_mul(length).ok_or(KernelError::Budget)?;
                    if product > MAX_TREE_ITEMS {
                        return Err(KernelError::Budget);
                    }
                }
                product
            }
        };
        output_count = output_count.checked_add(size).ok_or(KernelError::Budget)?;
        if output_count > MAX_TREE_ITEMS {
            return Err(KernelError::Budget);
        }
        sizes.push(size);
    }

    let mut result_branches = Vec::new();
    result_branches.try_reserve_exact(first.branches.len()).map_err(|_| KernelError::Budget)?;
    let mut resident_value_cost = first.branches.len();
    let mut expanded_input_cost = 0usize;
    for (branch_index, output_size) in sizes.into_iter().enumerate() {
        let mut items = Vec::new();
        items.try_reserve_exact(output_size).map_err(|_| KernelError::Budget)?;
        for item_index in 0..output_size {
            let mut arguments = BTreeMap::new();
            for port in contract.inputs {
                let original = request.inputs.get(port.name).ok_or(KernelError::Invalid("missing tool port"))?;
                let selected = if let ToolValue::Tree(_) = original {
                    if port.kind == ToolType::Tree {
                        original
                    } else {
                        let position =
                            tree_ports.iter().position(|(name, _)| *name == port.name).ok_or(KernelError::Invalid("missing lifted tree port"))?;
                        let source = tree_ports[position].1;
                        let branch_items = &source.branches[branch_index].items;
                        let input_index = match policy {
                            TreeMatchPolicy::Shortest => item_index,
                            TreeMatchPolicy::Longest => item_index.min(branch_items.len() - 1),
                            TreeMatchPolicy::CrossReference => {
                                // Rightmost tree input varies fastest.
                                let mut index = item_index;
                                for (_, previous_tree) in tree_ports.iter().skip(position + 1) {
                                    // Remaining input lengths are nonzero if
                                    // output_size > 0 (preflight established).
                                    let length = previous_tree.branches[branch_index].items.len();
                                    index /= length;
                                }
                                index % branch_items.len()
                            }
                        };
                        branch_items.get(input_index).ok_or(KernelError::Invalid("tree item index"))?
                    }
                } else {
                    original
                };
                let cost = shared_tool_value_cost(selected)?;
                expanded_input_cost = expanded_input_cost.checked_add(cost).ok_or(KernelError::Budget)?;
                if expanded_input_cost > MAX_TREE_ITEMS {
                    return Err(KernelError::Budget);
                }
                arguments.insert(port.name.to_owned(), selected.clone());
            }
            let output = scalar(&ToolRequest { operation: contract.operation.to_owned(), inputs: arguments })?;
            resident_value_cost = resident_value_cost.checked_add(shared_tool_value_cost(&output)?).ok_or(KernelError::Budget)?;
            if resident_value_cost > MAX_TREE_ITEMS {
                return Err(KernelError::Budget);
            }
            items.push(output);
        }
        result_branches.push(TreeBranch { path: TreePath(first.branches[branch_index].path.0.clone()), items });
    }
    let output = DataTree { branches: result_branches };
    tree_validate(&output)?;
    Ok(ToolValue::Tree(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TreeBranch, TreePath, execute_shared_tool, execute_shared_tool_with_matching};
    use cadcraft_geom::Vec3;
    fn points(path: &[u32], xs: &[f64]) -> ToolValue {
        ToolValue::Tree(DataTree {
            branches: vec![TreeBranch { path: TreePath(path.to_vec()), items: xs.iter().map(|x| ToolValue::Point(Vec3::new(*x, 0., 0.))).collect() }],
        })
    }
    fn input(operation: &str, a: ToolValue, b: ToolValue) -> ToolRequest {
        ToolRequest { operation: operation.into(), inputs: BTreeMap::from([("a".into(), a), ("b".into(), b)]) }
    }
    fn tree_value(output: ToolValue) -> DataTree<ToolValue> {
        match output {
            ToolValue::Tree(value) => value,
            other => panic!("expected tree, got {other:?}"),
        }
    }

    #[test]
    fn point_distance_is_lifted_over_a_tree_without_rewriting_math() {
        let req = input("kernel.point.distance", points(&[0, 3], &[0., 3., 7.]), ToolValue::Point(Vec3::ZERO));
        let out = tree_value(execute_shared_tool(&req).unwrap());
        assert_eq!(out.branches[0].path.0, vec![0, 3]);
        assert_eq!(out.branches[0].items, vec![ToolValue::Number(0.), ToolValue::Number(3.), ToolValue::Number(7.),]);
    }
    #[test]
    fn exact_paths_and_nonmatching_tree_paths_are_handled_conservatively() {
        let req = input("kernel.point.distance", points(&[0, 3], &[1., 2.]), points(&[0, 4], &[3., 4.]));
        assert!(execute_shared_tool(&req).is_err());
    }
    #[test]
    fn matching_modifiers_change_iteration_not_the_core_kernel() {
        let req = input("kernel.point.distance", points(&[0], &[1., 4.]), points(&[0], &[11., 22., 33.]));
        let short = tree_value(execute_shared_tool(&req).unwrap());
        assert_eq!(short.branches[0].items, vec![ToolValue::Number(10.), ToolValue::Number(18.)]);
        let longest = tree_value(execute_shared_tool_with_matching(&req, TreeMatchPolicy::Longest).unwrap());
        assert_eq!(longest.branches[0].items, vec![ToolValue::Number(10.), ToolValue::Number(18.), ToolValue::Number(29.),]);
        let cross = tree_value(execute_shared_tool_with_matching(&req, TreeMatchPolicy::CrossReference).unwrap());
        assert_eq!(
            cross.branches[0].items,
            vec![
                ToolValue::Number(10.),
                ToolValue::Number(21.),
                ToolValue::Number(32.),
                ToolValue::Number(7.),
                ToolValue::Number(18.),
                ToolValue::Number(29.),
            ]
        );
    }
    #[test]
    fn third_interpolation_modifier_can_be_a_tree() {
        let request = ToolRequest {
            operation: "kernel.point.interpolate".into(),
            inputs: BTreeMap::from([
                ("a".into(), ToolValue::Point(Vec3::ZERO)),
                ("b".into(), ToolValue::Point(Vec3::new(10., 0., 0.))),
                (
                    "t".into(),
                    ToolValue::Tree(DataTree {
                        branches: vec![TreeBranch {
                            path: TreePath(vec![0]),
                            items: vec![ToolValue::Number(0.), ToolValue::Number(0.5), ToolValue::Number(1.)],
                        }],
                    }),
                ),
            ]),
        };
        assert_eq!(
            tree_value(execute_shared_tool(&request).unwrap()).branches[0].items,
            vec![ToolValue::Point(Vec3::ZERO), ToolValue::Point(Vec3::new(5., 0., 0.)), ToolValue::Point(Vec3::new(10., 0., 0.)),]
        );
    }
    #[test]
    fn empty_branches_are_preserved_with_shortest_matching() {
        let empty = ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items: vec![] }] });
        let request = input("kernel.point.distance", empty, ToolValue::Point(Vec3::ZERO));
        let out = tree_value(execute_shared_tool(&request).unwrap());
        assert_eq!(out.branches.len(), 1);
        assert!(out.branches[0].items.is_empty());
    }
    #[test]
    fn disallow_nested_or_mistyped_leaves_and_infinite_values() {
        let nested = ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items: vec![ToolValue::Number(2.)] }] });
        let req = input("kernel.point.distance", nested, ToolValue::Point(Vec3::ZERO));
        assert!(execute_shared_tool(&req).is_err());
        let nonfinite = points(&[0], &[f64::NAN]);
        let req = input("kernel.point.distance", nonfinite, ToolValue::Point(Vec3::ZERO));
        assert!(execute_shared_tool(&req).is_err());
    }
    #[test]
    fn three_lifted_ports_use_rightmost_fastest_cartesian_order() {
        let as_tree = ToolValue::Tree(DataTree {
            branches: vec![TreeBranch {
                path: TreePath(vec![2]),
                items: vec![ToolValue::Point(Vec3::new(0., 0., 0.)), ToolValue::Point(Vec3::new(10., 0., 0.))],
            }],
        });
        let bs_tree = ToolValue::Tree(DataTree {
            branches: vec![TreeBranch {
                path: TreePath(vec![2]),
                items: vec![ToolValue::Point(Vec3::new(100., 0., 0.)), ToolValue::Point(Vec3::new(200., 0., 0.))],
            }],
        });
        let ts_tree = ToolValue::Tree(DataTree {
            branches: vec![TreeBranch { path: TreePath(vec![2]), items: vec![ToolValue::Number(0.), ToolValue::Number(1.)] }],
        });
        let req = ToolRequest {
            operation: "kernel.point.interpolate".into(),
            inputs: BTreeMap::from([("a".into(), as_tree), ("b".into(), bs_tree), ("t".into(), ts_tree)]),
        };
        let out = tree_value(execute_shared_tool_with_matching(&req, TreeMatchPolicy::CrossReference).unwrap());
        assert_eq!(out.branches[0].path.0, vec![2]);
        assert_eq!(
            out.branches[0].items,
            vec![
                ToolValue::Point(Vec3::new(0., 0., 0.)),
                ToolValue::Point(Vec3::new(100., 0., 0.)),
                ToolValue::Point(Vec3::new(0., 0., 0.)),
                ToolValue::Point(Vec3::new(200., 0., 0.)),
                ToolValue::Point(Vec3::new(10., 0., 0.)),
                ToolValue::Point(Vec3::new(100., 0., 0.)),
                ToolValue::Point(Vec3::new(10., 0., 0.)),
                ToolValue::Point(Vec3::new(200., 0., 0.)),
            ]
        );
    }

    #[test]
    fn strict_paths_do_not_depend_on_matching_modifier() {
        let a = points(&[0, 1], &[1.]);
        let b = points(&[0, 2], &[2.]);
        let req = input("worldwright.point.distance", a, b);
        for policy in [TreeMatchPolicy::Shortest, TreeMatchPolicy::Longest, TreeMatchPolicy::CrossReference] {
            assert_eq!(execute_shared_tool_with_matching(&req, policy), Err(KernelError::Invalid("tree branch path mismatch")),);
        }
    }

    #[test]
    fn longest_does_not_repeat_from_empty_branch() {
        let empty = ToolValue::Tree(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items: Vec::new() }] });
        let present = points(&[0], &[2., 3.]);
        let req = input("kernel.point.distance", empty, present);
        assert_eq!(execute_shared_tool_with_matching(&req, TreeMatchPolicy::Longest), Err(KernelError::Invalid("cannot repeat missing tree item")),);
        let short = execute_shared_tool_with_matching(&req, TreeMatchPolicy::Shortest);
        assert!(matches!(short, Ok(ToolValue::Tree(_))));
    }

    #[test]
    fn preflights_cartesian_growth_beyond_tree_budget() {
        let request = input("kernel.point.distance", points(&[0], &vec![0.; 600]), points(&[0], &vec![1.; 600]));
        assert_eq!(execute_shared_tool_with_matching(&request, TreeMatchPolicy::CrossReference), Err(KernelError::Budget),);
    }
}
