//! Bounded, ordered, host-independent data trees for Worldwright and Orb Weaver.
//! No Grasshopper host is required. Matching here is intentionally STRICT by
//! branch path; Grasshopper's implicit branch/path alignment is a later layer.
use crate::{KernelError, Result};
use serde::{Deserialize, Serialize};

pub const MAX_TREE_BRANCHES: usize = 4_096;
pub const MAX_TREE_ITEMS: usize = 250_000;
pub const MAX_TREE_DEPTH: usize = 16;

/// An ordered tree path like {0;2;1}; the serialized form is [0,2,1].
/// No empty paths: the root/default data path is [0].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TreePath(pub Vec<u32>);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeBranch<T> {
    pub path: TreePath,
    pub items: Vec<T>,
}

/// Branches are strictly ordered by path. Empty branches are meaningful and
/// retained. A zero-branch tree represents no data, not a branch with no items.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataTree<T> {
    pub branches: Vec<TreeBranch<T>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeMatchPolicy {
    /// Pair only indices that exist in BOTH lists.
    #[default]
    Shortest,
    /// Repeat the final element of the shorter list.
    Longest,
    /// Cartesian product in left-major, right-minor order.
    CrossReference,
}

pub fn tree_validate<T>(tree: &DataTree<T>) -> Result<usize> {
    if tree.branches.len() > MAX_TREE_BRANCHES {
        return Err(KernelError::Budget);
    }
    let mut total = 0usize;
    let mut previous: Option<&TreePath> = None;
    for branch in &tree.branches {
        if branch.path.0.is_empty() || branch.path.0.len() > MAX_TREE_DEPTH {
            return Err(KernelError::Invalid("tree path depth"));
        }
        if previous.is_some_and(|p| p >= &branch.path) {
            return Err(KernelError::Invalid("tree paths must be strictly ordered"));
        }
        total = total.checked_add(branch.items.len()).ok_or(KernelError::Budget)?;
        if total > MAX_TREE_ITEMS {
            return Err(KernelError::Budget);
        }
        previous = Some(&branch.path);
    }
    Ok(total)
}

/// Flatten all branches to the single default path {0}, retaining order.
pub fn tree_flatten<T: Clone>(tree: &DataTree<T>) -> Result<DataTree<T>> {
    let total = tree_validate(tree)?;
    if tree.branches.is_empty() {
        return Ok(DataTree { branches: Vec::new() });
    }
    let mut items = Vec::new();
    items.try_reserve_exact(total).map_err(|_| KernelError::Budget)?;
    for branch in &tree.branches {
        items.extend_from_slice(&branch.items);
    }
    Ok(DataTree { branches: vec![TreeBranch { path: TreePath(vec![0]), items }] })
}

/// Graft each item to a unique child path, suffixing its source item index.
/// An empty branch yields one empty child at index 0, retaining its presence.
/// This simple explicit policy is NOT a full Grasshopper graft parity claim.
pub fn tree_graft<T: Clone>(tree: &DataTree<T>) -> Result<DataTree<T>> {
    let item_count = tree_validate(tree)?;
    let empty_count = tree.branches.iter().filter(|b| b.items.is_empty()).count();
    let branch_count = item_count.checked_add(empty_count).ok_or(KernelError::Budget)?;
    if branch_count > MAX_TREE_BRANCHES {
        return Err(KernelError::Budget);
    }
    let mut branches = Vec::new();
    branches.try_reserve_exact(branch_count).map_err(|_| KernelError::Budget)?;
    for branch in &tree.branches {
        if branch.path.0.len() >= MAX_TREE_DEPTH {
            return Err(KernelError::Budget);
        }
        let mut add = |index: usize, items: Vec<T>| -> Result<()> {
            let mut path = branch.path.0.clone();
            path.push(u32::try_from(index).map_err(|_| KernelError::Budget)?);
            branches.push(TreeBranch { path: TreePath(path), items });
            Ok(())
        };
        if branch.items.is_empty() {
            add(0, Vec::new())?;
        } else {
            for (index, item) in branch.items.iter().enumerate() {
                add(index, vec![item.clone()])?;
            }
        }
    }
    let result = DataTree { branches };
    tree_validate(&result)?;
    Ok(result)
}

/// Remove the longest COMMON leading path prefix, preserving at least one
/// coordinate per branch, and retaining all items and empty branches.
pub fn tree_simplify<T: Clone>(tree: &DataTree<T>) -> Result<DataTree<T>> {
    tree_validate(tree)?;
    let Some(first) = tree.branches.first() else {
        return Ok(DataTree { branches: Vec::new() });
    };
    let shortest_depth = tree.branches.iter().map(|branch| branch.path.0.len()).min().unwrap_or(1);
    let mut drop_prefix = 0usize;
    for index in 0..shortest_depth.saturating_sub(1) {
        if tree.branches.iter().all(|branch| branch.path.0[index] == first.path.0[index]) {
            drop_prefix += 1;
        } else {
            break;
        }
    }
    let branches = tree
        .branches
        .iter()
        .map(|branch| TreeBranch { path: TreePath(branch.path.0[drop_prefix..].to_vec()), items: branch.items.clone() })
        .collect();
    let result = DataTree { branches };
    tree_validate(&result)?;
    Ok(result)
}

/// Match two data trees STRICTLY by identical branch paths, then match items
/// by Shortest, Longest(last-item repeat) or CrossReference(cartesian) policy.
/// Missing branches and repeating from an empty list are explicit errors.
/// This deliberately avoids guessing Grasshopper path matching rules.
pub fn tree_match<A: Clone, B: Clone>(left: &DataTree<A>, right: &DataTree<B>, mode: TreeMatchPolicy) -> Result<DataTree<(A, B)>> {
    tree_validate(left)?;
    tree_validate(right)?;
    if left.branches.len() != right.branches.len() {
        return Err(KernelError::Invalid("tree branch path mismatch"));
    }
    let mut total = 0usize;
    let mut counts = Vec::new();
    counts.try_reserve_exact(left.branches.len()).map_err(|_| KernelError::Budget)?;
    for (a, b) in left.branches.iter().zip(&right.branches) {
        if a.path != b.path {
            return Err(KernelError::Invalid("tree branch path mismatch"));
        }
        let n = match mode {
            TreeMatchPolicy::Shortest => a.items.len().min(b.items.len()),
            TreeMatchPolicy::Longest => {
                if a.items.is_empty() != b.items.is_empty() {
                    return Err(KernelError::Invalid("cannot repeat missing tree item"));
                }
                a.items.len().max(b.items.len())
            }
            TreeMatchPolicy::CrossReference => a.items.len().checked_mul(b.items.len()).ok_or(KernelError::Budget)?,
        };
        total = total.checked_add(n).ok_or(KernelError::Budget)?;
        if total > MAX_TREE_ITEMS {
            return Err(KernelError::Budget);
        }
        counts.push(n);
    }
    let mut branches = Vec::new();
    branches.try_reserve_exact(left.branches.len()).map_err(|_| KernelError::Budget)?;
    for ((a, b), count) in left.branches.iter().zip(&right.branches).zip(counts) {
        let mut items = Vec::new();
        items.try_reserve_exact(count).map_err(|_| KernelError::Budget)?;
        match mode {
            TreeMatchPolicy::Shortest => {
                for (x, y) in a.items.iter().zip(&b.items) {
                    items.push((x.clone(), y.clone()));
                }
            }
            TreeMatchPolicy::Longest => {
                if !a.items.is_empty() {
                    for index in 0..count {
                        let ai = index.min(a.items.len() - 1);
                        let bi = index.min(b.items.len() - 1);
                        items.push((a.items[ai].clone(), b.items[bi].clone()));
                    }
                }
            }
            TreeMatchPolicy::CrossReference => {
                for x in &a.items {
                    for y in &b.items {
                        items.push((x.clone(), y.clone()));
                    }
                }
            }
        }
        branches.push(TreeBranch { path: a.path.clone(), items });
    }
    let result = DataTree { branches };
    tree_validate(&result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn branch<T>(path: &[u32], items: Vec<T>) -> TreeBranch<T> {
        TreeBranch { path: TreePath(path.to_vec()), items }
    }
    fn fixture() -> DataTree<i32> {
        DataTree { branches: vec![branch(&[0, 1], vec![1, 2]), branch(&[0, 2], vec![3])] }
    }
    #[test]
    fn flatten_preserves_order_and_tree_json_roundtrip() {
        let tree = fixture();
        let flattened = tree_flatten(&tree).unwrap();
        assert_eq!(flattened.branches, vec![branch(&[0], vec![1, 2, 3])]);
        assert_eq!(serde_json::from_str::<DataTree<i32>>(&serde_json::to_string(&tree).unwrap()).unwrap(), tree);
    }
    #[test]
    fn graft_each_item_and_preserve_empty_branch() {
        let tree = DataTree { branches: vec![branch(&[0], vec![4, 5]), branch(&[1], Vec::<i32>::new())] };
        let graft = tree_graft(&tree).unwrap();
        assert_eq!(graft.branches, vec![branch(&[0, 0], vec![4]), branch(&[0, 1], vec![5]), branch(&[1, 0], vec![]),]);
    }
    #[test]
    fn simplify_only_shared_leading_path() {
        let simplified = tree_simplify(&fixture()).unwrap();
        assert_eq!(simplified.branches, vec![branch(&[1], vec![1, 2]), branch(&[2], vec![3]),]);
        let unchanged = DataTree { branches: vec![branch(&[0, 1], vec![1]), branch(&[1, 1], vec![2])] };
        assert_eq!(tree_simplify(&unchanged).unwrap(), unchanged);
    }
    #[test]
    fn list_matching_supports_shortest_longest_and_cross() {
        let a = DataTree { branches: vec![branch(&[0], vec![1, 2])] };
        let b = DataTree { branches: vec![branch(&[0], vec![9, 10, 11])] };
        assert_eq!(tree_match(&a, &b, TreeMatchPolicy::Shortest).unwrap().branches[0].items, vec![(1, 9), (2, 10)]);
        assert_eq!(tree_match(&a, &b, TreeMatchPolicy::Longest).unwrap().branches[0].items, vec![(1, 9), (2, 10), (2, 11)]);
        assert_eq!(
            tree_match(&a, &b, TreeMatchPolicy::CrossReference).unwrap().branches[0].items,
            vec![(1, 9), (1, 10), (1, 11), (2, 9), (2, 10), (2, 11)]
        );
    }
    #[test]
    fn missing_or_unsorted_branches_rejected() {
        let bad = DataTree { branches: vec![branch(&[1], vec![1]), branch(&[0], vec![2])] };
        assert!(tree_validate(&bad).is_err());
        let repeat = DataTree { branches: vec![branch(&[0], vec![1]), branch(&[0], vec![2])] };
        assert!(tree_validate(&repeat).is_err());
        let different = DataTree { branches: vec![branch(&[3], vec![1])] };
        assert!(tree_match(&fixture(), &different, TreeMatchPolicy::Shortest).is_err());
    }
    #[test]
    fn longest_does_not_repeat_from_empty_list() {
        let a = DataTree { branches: vec![branch(&[0], Vec::<i32>::new())] };
        let b = DataTree { branches: vec![branch(&[0], vec![1])] };
        assert!(tree_match(&a, &b, TreeMatchPolicy::Longest).is_err());
        assert!(tree_match(&a, &b, TreeMatchPolicy::Shortest).unwrap().branches[0].items.is_empty());
    }
    #[test]
    fn strict_budgets_apply_before_materialization() {
        let huge = DataTree { branches: vec![branch(&[0], vec![0u32; MAX_TREE_ITEMS + 1])] };
        assert_eq!(tree_validate(&huge), Err(KernelError::Budget));
        let many = DataTree { branches: vec![branch(&[0], vec![1u32; MAX_TREE_BRANCHES + 1])] };
        assert_eq!(tree_graft(&many), Err(KernelError::Budget));
        let a = DataTree { branches: vec![branch(&[0], vec![1u32; 600])] };
        let b = DataTree { branches: vec![branch(&[0], vec![1u32; 600])] };
        assert_eq!(tree_match(&a, &b, TreeMatchPolicy::CrossReference), Err(KernelError::Budget));
    }
    #[test]
    fn empty_tree_is_distinct_from_empty_branch() {
        let empty: DataTree<u8> = DataTree { branches: vec![] };
        assert!(tree_flatten(&empty).unwrap().branches.is_empty());
        assert!(tree_graft(&empty).unwrap().branches.is_empty());
        assert!(tree_simplify(&empty).unwrap().branches.is_empty());
        assert!(tree_match(&empty, &empty, TreeMatchPolicy::Shortest).unwrap().branches.is_empty());
        let branch_only = DataTree { branches: vec![branch(&[0], Vec::<u8>::new())] };
        assert_eq!(tree_flatten(&branch_only).unwrap().branches.len(), 1);
    }
}
