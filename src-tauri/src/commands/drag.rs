use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;
use crate::commands::Db;
use crate::calendar::workday::count_workdays;
use crate::db::models::{Dependency, Task};
use crate::deps::ripple::compute_ripple;
use crate::repo::{dependency as dep_repo, job as job_repo, no_work_day as nwd_repo, task as task_repo};
use crate::{GbError, GbResult};

#[derive(Debug, Deserialize)]
pub struct DragTaskArgs {
    pub job_id: i64,
    pub task_id: i64,
    pub new_start_date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct DragResult {
    pub updated_tasks: Vec<Task>,
}

#[tauri::command]
pub fn drag_task(db: State<Db>, args: DragTaskArgs) -> GbResult<DragResult> {
    let conn = db.0.lock().unwrap();
    drag_task_inner(&conn, args)
}

fn drag_task_inner(conn: &rusqlite::Connection, args: DragTaskArgs) -> GbResult<DragResult> {
    let tasks: Vec<Task> = task_repo::list_for_job(conn, args.job_id)?;
    let _deps: Vec<Dependency> = dep_repo::list_for_job(conn, args.job_id)?;
    let job = job_repo::get(conn, args.job_id)?;
    let _nwds: HashSet<NaiveDate> = nwd_repo::list_for_job(conn, args.job_id)?
        .into_iter()
        .filter(|n| job.holidays_block_work || !n.source.ends_with("_holiday") && n.source != "sa_public_holiday")
        .map(|n| n.date)
        .collect();

    let dragged = tasks.iter().find(|t| t.id == args.task_id)
        .ok_or_else(|| GbError::NotFound(format!("task {}", args.task_id)))?;

    let shift = if args.new_start_date >= dragged.start_date {
        count_workdays(dragged.start_date, args.new_start_date) - 1
    } else {
        -(count_workdays(args.new_start_date, dragged.start_date) - 1)
    };

    let tx = conn.unchecked_transaction()?;
    apply_ripple(&tx, args.job_id, args.task_id, shift)?;
    tx.commit()?;

    let updated: Vec<Task> = task_repo::list_for_job(conn, args.job_id)?
        .into_iter()
        .filter(|t| {
            tasks.iter().find(|old| old.id == t.id).map(|old| old.start_date) != Some(t.start_date)
        })
        .collect();

    Ok(DragResult { updated_tasks: updated })
}

/// Applies a workday shift to `task_id` and ripples the change through
/// all downstream tasks in the same job. Safe to call inside an existing
/// `unchecked_transaction` because it does not open a new one — callers
/// are responsible for their own transaction boundary.
///
/// `by_days` is signed workdays (positive = later, negative = earlier).
/// Internally mirrors the logic that `drag_task_inner` performs but
/// accepts a pre-computed shift rather than a new absolute date.
pub fn apply_ripple(
    conn: &rusqlite::Connection,
    job_id: i64,
    task_id: i64,
    by_days: i64,
) -> GbResult<()> {
    use crate::calendar::workday::add_workdays_excluding;

    let tasks = task_repo::list_for_job(conn, job_id)?;
    let deps = dep_repo::list_for_job(conn, job_id)?;
    let job = job_repo::get(conn, job_id)?;
    let nwds: HashSet<NaiveDate> = nwd_repo::list_for_job(conn, job_id)?
        .into_iter()
        .filter(|n| {
            job.holidays_block_work
                || (!n.source.ends_with("_holiday") && n.source != "sa_public_holiday")
        })
        .map(|n| n.date)
        .collect();

    let dragged = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| GbError::NotFound(format!("task {task_id}")))?;

    let new_start = add_workdays_excluding(dragged.start_date, by_days, &nwds);

    let mut ripples = compute_ripple(&tasks, &deps, task_id, by_days, &nwds);
    ripples.insert(task_id, new_start);

    for t in &tasks {
        if let Some(new_date) = ripples.get(&t.id) {
            conn.execute(
                "UPDATE task SET start_date = ?1 WHERE id = ?2",
                rusqlite::params![new_date.to_string(), t.id],
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase, NewTask, NewDependency};
    use crate::repo::{job, phase, task, dependency};

    #[test]
    fn drag_ripples_to_downstream_task() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
            holidays_block_work: true,
            region: "ZA".into(),
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        let t1 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T1".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t2 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T2".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        dependency::create(&conn, &NewDependency {
            predecessor_id: t1.id, successor_id: t2.id, lag_days: 0,
        }).unwrap();

        let r = drag_task_inner(&conn, DragTaskArgs {
            job_id: j.id, task_id: t1.id,
            new_start_date: NaiveDate::from_ymd_opt(2026,6,10).unwrap(),
        }).unwrap();

        assert_eq!(r.updated_tasks.len(), 2);
        let t1_new = r.updated_tasks.iter().find(|t| t.id == t1.id).unwrap();
        let t2_new = r.updated_tasks.iter().find(|t| t.id == t2.id).unwrap();
        assert_eq!(t1_new.start_date, NaiveDate::from_ymd_opt(2026,6,10).unwrap());
        assert_eq!(t2_new.start_date, NaiveDate::from_ymd_opt(2026,6,11).unwrap());
    }

    #[test]
    fn apply_ripple_shifts_task_and_downstream() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false, holidays_block_work: true, region: "ZA".into(),
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        let t1 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T1".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t2 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T2".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        dependency::create(&conn, &NewDependency {
            predecessor_id: t1.id, successor_id: t2.id, lag_days: 0,
        }).unwrap();

        // Apply a +2 workday shift to t1 using apply_ripple (not drag_task_inner).
        apply_ripple(&conn, j.id, t1.id, 2).unwrap();

        let t1_updated = task::get(&conn, t1.id).unwrap();
        let t2_updated = task::get(&conn, t2.id).unwrap();
        assert_eq!(t1_updated.start_date, NaiveDate::from_ymd_opt(2026,6,10).unwrap());
        assert_eq!(t2_updated.start_date, NaiveDate::from_ymd_opt(2026,6,11).unwrap());
    }
}
