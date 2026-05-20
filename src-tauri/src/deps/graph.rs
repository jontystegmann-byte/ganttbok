use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use crate::db::models::Dependency;

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub successor_id: i64,
    pub lag_days: i64,
}

/// Adjacency map: predecessor_id -> [(successor_id, lag)]
pub fn build_adjacency(deps: &[Dependency]) -> HashMap<i64, Vec<Edge>> {
    let mut m: HashMap<i64, Vec<Edge>> = HashMap::new();
    for d in deps {
        m.entry(d.predecessor_id).or_default().push(Edge {
            successor_id: d.successor_id,
            lag_days: d.lag_days,
        });
    }
    m
}

/// Returns true iff adding (pre -> suc) would create a cycle in the existing adjacency.
pub fn would_cycle(adj: &HashMap<i64, Vec<Edge>>, pre: i64, suc: i64) -> bool {
    if pre == suc { return true; }
    let mut stack = vec![suc];
    let mut visited: HashSet<i64> = HashSet::new();
    while let Some(node) = stack.pop() {
        if !visited.insert(node) { continue; }
        if let Some(edges) = adj.get(&node) {
            for e in edges {
                if e.successor_id == pre { return true; }
                stack.push(e.successor_id);
            }
        }
    }
    false
}

/// Tasks transitively reachable from `root`, in BFS-by-depth order.
/// `root` itself is NOT included.
pub fn downstream(adj: &HashMap<i64, Vec<Edge>>, root: i64) -> Vec<i64> {
    let mut out = Vec::new();
    let mut seen: HashSet<i64> = HashSet::new();
    let mut q: VecDeque<i64> = VecDeque::new();
    q.push_back(root);
    while let Some(node) = q.pop_front() {
        if let Some(edges) = adj.get(&node) {
            for e in edges {
                if seen.insert(e.successor_id) {
                    out.push(e.successor_id);
                    q.push_back(e.successor_id);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod downstream_tests {
    use super::*;
    use super::tests::*;

    #[test]
    fn no_outgoing_returns_empty() {
        let adj = build_adjacency(&[]);
        assert!(downstream(&adj, 10).is_empty());
    }

    #[test]
    fn linear_chain() {
        let adj = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 20, 30, 0),
            dep(3, 30, 40, 0),
        ]);
        assert_eq!(downstream(&adj, 10), vec![20, 30, 40]);
    }

    #[test]
    fn diamond_visits_each_once() {
        // 10 -> 20 -> 40
        // 10 -> 30 -> 40
        let adj = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 10, 30, 0),
            dep(3, 20, 40, 0),
            dep(4, 30, 40, 0),
        ]);
        let d = downstream(&adj, 10);
        let s: HashSet<i64> = d.iter().copied().collect();
        assert_eq!(s, HashSet::from([20, 30, 40]));
        assert_eq!(d.len(), 3);   // each node exactly once
    }
}

#[cfg(test)]
mod cycle_tests {
    use super::*;
    use super::tests::*;  // reuse the dep() helper above

    #[test]
    fn self_loop_is_a_cycle() {
        let adj = build_adjacency(&[]);
        assert!(would_cycle(&adj, 10, 10));
    }

    #[test]
    fn direct_back_edge_is_a_cycle() {
        // existing: 10 -> 20.  Adding 20 -> 10 closes the loop.
        let adj = build_adjacency(&[dep(1, 10, 20, 0)]);
        assert!(would_cycle(&adj, 20, 10));
    }

    #[test]
    fn deeper_back_edge_is_a_cycle() {
        // existing: 10 -> 20 -> 30 -> 40.  Adding 40 -> 10 closes.
        let adj = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 20, 30, 0),
            dep(3, 30, 40, 0),
        ]);
        assert!(would_cycle(&adj, 40, 10));
    }

    #[test]
    fn unrelated_edge_does_not_cycle() {
        let adj = build_adjacency(&[dep(1, 10, 20, 0)]);
        assert!(!would_cycle(&adj, 30, 40));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn dep(id: i64, pre: i64, suc: i64, lag: i64) -> Dependency {
        Dependency { id, predecessor_id: pre, successor_id: suc, r#type: "FS".into(), lag_days: lag }
    }

    #[test]
    fn empty_input_returns_empty_map() {
        let m = build_adjacency(&[]);
        assert!(m.is_empty());
    }

    #[test]
    fn single_edge() {
        let m = build_adjacency(&[dep(1, 10, 20, 0)]);
        assert_eq!(m.get(&10).unwrap(), &vec![Edge { successor_id: 20, lag_days: 0 }]);
    }

    #[test]
    fn many_successors_grouped() {
        let m = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 10, 30, 1),
            dep(3, 20, 40, 0),
        ]);
        let from_10 = m.get(&10).unwrap();
        assert_eq!(from_10.len(), 2);
        assert!(from_10.contains(&Edge { successor_id: 20, lag_days: 0 }));
        assert!(from_10.contains(&Edge { successor_id: 30, lag_days: 1 }));
        assert_eq!(m.get(&20).unwrap(), &vec![Edge { successor_id: 40, lag_days: 0 }]);
    }
}
