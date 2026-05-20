use std::collections::HashMap;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(id: i64, pre: i64, suc: i64, lag: i64) -> Dependency {
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
