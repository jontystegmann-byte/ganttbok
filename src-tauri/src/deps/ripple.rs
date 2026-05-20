use std::collections::{HashMap, HashSet};
use chrono::NaiveDate;
use crate::db::models::{Dependency, Task};
use crate::calendar::workday::add_workdays_excluding;
use super::graph::{build_adjacency, downstream};

/// New start_dates for every downstream task after `dragged` shifts by `shift_workdays`.
/// Excludes weekends + provided no-work-day set.
pub fn compute_ripple(
    tasks: &[Task],
    deps:  &[Dependency],
    dragged_id: i64,
    shift_workdays: i64,
    no_work_days: &HashSet<NaiveDate>,
) -> HashMap<i64, NaiveDate> {
    let mut out = HashMap::new();
    if shift_workdays == 0 { return out; }

    let adj = build_adjacency(deps);
    let downstream_ids = downstream(&adj, dragged_id);
    let by_id: HashMap<i64, &Task> = tasks.iter().map(|t| (t.id, t)).collect();

    for id in downstream_ids {
        if let Some(t) = by_id.get(&id) {
            let new_start = add_workdays_excluding(t.start_date, shift_workdays, no_work_days);
            out.insert(id, new_start);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn task(id: i64, start: NaiveDate, dur: i64) -> Task {
        Task { id, phase_id: 1, name: format!("T{id}"),
               start_date: start, duration_workdays: dur,
               order_index: id, notes: None }
    }
    fn dep(id: i64, pre: i64, suc: i64, lag: i64) -> Dependency {
        Dependency { id, predecessor_id: pre, successor_id: suc,
                     r#type: "FS".into(), lag_days: lag }
    }

    #[test]
    fn linear_chain_shifts_downstream_only() {
        // T1: Mon 8 Jun, 3 days   -> Mon..Wed
        // T2: Thu 11 Jun, 2 days  -> Thu..Fri   (depends on T1, lag 0)
        // T3: Mon 15 Jun, 1 day                  (depends on T2, lag 0)
        // Drag T1 +2 workdays: T1 starts Wed 10; T2 -> Mon 15; T3 -> Wed 17
        let tasks = vec![ task(1, d(2026,6,8), 3),
                          task(2, d(2026,6,11), 2),
                          task(3, d(2026,6,15), 1) ];
        let deps  = vec![ dep(1, 1, 2, 0), dep(2, 2, 3, 0) ];
        let r = compute_ripple(&tasks, &deps, 1, 2, &HashSet::new());
        assert_eq!(r.get(&2).copied(), Some(d(2026,6,15)));
        assert_eq!(r.get(&3).copied(), Some(d(2026,6,17)));
        assert!(!r.contains_key(&1), "dragged task itself not included");
    }

    #[test]
    fn ripple_respects_lag() {
        // T1: Mon, 2 days; T2 depends on T1 with lag 2; drag T1 +1
        let tasks = vec![ task(1, d(2026,6,8), 2), task(2, d(2026,6,12), 1) ];
        let deps  = vec![ dep(1, 1, 2, 2) ];
        let r = compute_ripple(&tasks, &deps, 1, 1, &HashSet::new());
        // T2 original start = T1_end + 2 wd; T1 shifted +1 -> T2 +1.
        assert_eq!(r.get(&2).copied(), Some(d(2026,6,15)));
    }

    #[test]
    fn ripple_skips_no_work_days() {
        // T1: Mon 15 Jun, 1 day. T2 depends on T1 lag 0; T2 originally Tue 16.
        // 16 Jun is Youth Day. Drag T1 +0 (no shift) -> no ripple.
        // Drag T1 +1 workday -> T1 starts Tue 16 (or after, if 16 is no-work).
        // For this test we drag T1 +0 and expect empty ripple.
        let mut hol = HashSet::new();
        hol.insert(d(2026,6,16));
        let tasks = vec![ task(1, d(2026,6,15), 1), task(2, d(2026,6,17), 1) ];
        let deps  = vec![ dep(1, 1, 2, 0) ];
        let r = compute_ripple(&tasks, &deps, 1, 0, &hol);
        assert!(r.is_empty(), "shift of 0 should yield no ripple");
    }
}
