// SPDX-License-Identifier: LGPL-2.1-or-later
//! Bounded FreeCAD-derived document dependency traversal.
//!
//! Source: FreeCAD/FreeCAD, src/App/DocumentObject.cpp,
//! DocumentObject::getInListEx, revision ec6852e6caadf55045881a6003cc7ad038c8d2a7.
//! FreeCAD source copyright (c) 2011 Jürgen Riegel and Werner Mayer.
//! Ported and adapted to Rust with numeric IDs, strict memory budgets,
//! stable output ordering and no FreeCAD/Qt/Python dependencies,
//! Worldwright changes 2026-10-11.
//!
//! Only this component is LGPL; it does not change other source file headers.
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

/// Hard limits against untrusted or unexpectedly dense dependency graphs.
pub const MAX_GRAPH_NODES: usize = 8_192;
pub const MAX_GRAPH_EDGES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkError {
    InvalidNode,
    Budget,
}

/// Return the reachable source nodes and downstream dependents, sorted by
/// *numeric node index*. `edges` are (prerequisite, dependent) pairs.
///
/// Adapts FreeCAD's iterative stack + visited-set traversal (getInListEx).
/// The input edges are validated before walking; cycles terminate safely.
/// The caller remains responsible for rejecting dependency cycles where
/// their document semantics require a DAG.
///
/// Allocation is O(nodes + edges), with strict limits and fallible reserves.
/// No geometry is loaded, recomputed or copied.
pub fn affected_nodes(
    node_count: usize,
    edges: &[(usize, usize)],
    roots: &[usize],
) -> Result<Vec<usize>, WalkError> {
    if node_count > MAX_GRAPH_NODES
        || edges.len() > MAX_GRAPH_EDGES
        || roots.len() > node_count
    {
        return Err(WalkError::Budget);
    }

    let mut counts = Vec::new();
    counts
        .try_reserve_exact(node_count)
        .map_err(|_| WalkError::Budget)?;
    counts.resize(node_count, 0usize);
    for &(from, to) in edges {
        if from >= node_count || to >= node_count {
            return Err(WalkError::InvalidNode);
        }
        counts[from] = counts[from].checked_add(1).ok_or(WalkError::Budget)?;
    }
    for &root in roots {
        if root >= node_count {
            return Err(WalkError::InvalidNode);
        }
    }

    let mut successors: Vec<Vec<usize>> = Vec::new();
    successors
        .try_reserve_exact(node_count)
        .map_err(|_| WalkError::Budget)?;
    for count in counts {
        let mut neighbors = Vec::new();
        neighbors
            .try_reserve_exact(count)
            .map_err(|_| WalkError::Budget)?;
        successors.push(neighbors);
    }
    for &(from, to) in edges {
        successors[from].push(to);
    }

    let mut seen = Vec::new();
    seen.try_reserve_exact(node_count)
        .map_err(|_| WalkError::Budget)?;
    seen.resize(node_count, false);
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(edges.len().saturating_add(roots.len()))
        .map_err(|_| WalkError::Budget)?;
    pending.extend_from_slice(roots);

    // FreeCAD's getInListEx uses a LIFO stack and an inserted/visited set.
    // Here numeric nodes replace C++ DocumentObject pointers.
    while let Some(node) = pending.pop() {
        if seen[node] {
            continue;
        }
        seen[node] = true;
        for &dependent in &successors[node] {
            if !seen[dependent] {
                pending.push(dependent);
            }
        }
    }

    let mut result = Vec::new();
    result
        .try_reserve_exact(node_count)
        .map_err(|_| WalkError::Budget)?;
    for (index, reachable) in seen.into_iter().enumerate() {
        if reachable {
            result.push(index);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diamond_graph_has_no_duplicate_and_keeps_stable_order() {
        assert_eq!(
            affected_nodes(6, &[(0, 2), (0, 3), (2, 4), (3, 4)], &[0]),
            Ok(vec![0, 2, 3, 4])
        );
        assert_eq!(affected_nodes(6, &[(0, 2), (0, 3)], &[1]), Ok(vec![1]));
        assert_eq!(affected_nodes(6, &[(0, 2), (0, 3)], &[]), Ok(vec![]));
    }

    #[test]
    fn defensive_cycle_termination_and_duplicate_edges() {
        assert_eq!(
            affected_nodes(4, &[(0, 1), (1, 2), (2, 0), (1, 2)], &[0]),
            Ok(vec![0, 1, 2])
        );
        assert_eq!(affected_nodes(3, &[(1, 1)], &[1]), Ok(vec![1]));
    }

    #[test]
    fn input_validation_does_not_hide_invalid_references() {
        assert_eq!(affected_nodes(2, &[(0, 2)], &[0]), Err(WalkError::InvalidNode));
        assert_eq!(affected_nodes(2, &[], &[2]), Err(WalkError::InvalidNode));
        assert_eq!(affected_nodes(0, &[], &[]), Ok(vec![]));
        assert_eq!(
            affected_nodes(MAX_GRAPH_NODES + 1, &[], &[]),
            Err(WalkError::Budget)
        );
        assert_eq!(affected_nodes(1, &[], &[0, 0]), Err(WalkError::Budget));
    }

    #[test]
    fn upper_limit_chain_remains_iterative() {
        let count = MAX_GRAPH_NODES;
        let edges: Vec<_> = (1..count).map(|i| (i - 1, i)).collect();
        let result = affected_nodes(count, &edges, &[0]);
        assert_eq!(result.as_ref().map(Vec::len), Ok(count));
        assert_eq!(result.ok().and_then(|nodes| nodes.last().copied()), Some(count - 1));
    }
}
