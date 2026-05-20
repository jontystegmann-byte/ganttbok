use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;
use crate::commands::Db;
use crate::calendar::workday::count_workdays;
use crate::db::models::{Dependency, Task};
use crate::deps::ripple::compute_ripple;
use crate::repo::{dependency as dep_repo, no_work_day as nwd_repo, task as task_repo};
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
    let deps: Vec<Dependency> = dep_repo::list_for_job(conn, args.job_id)?;
    let nwds: HashSet<NaiveDate> = nwd_repo::list_for_job(conn, args.job_id)?
        .into_iter().map(|n| n.date).collect();

    let dragged = tasks.iter().find(|t| t.id == args.task_id)
        .ok_or_else(|| GbError::NotFound(format!("task {}", args.task_id)))?;

    let shift = if args.new_start_date >= dragged.start_date {
        count_workdays(dragged.start_date, args.new_start_date) - 1
    } else {
        -(count_workdays(args.new_start_date, dragged.start_date) - 1)
    };

    let mut ripples = compute_ripple(&tasks, &deps, args.task_id, shift, &nwds);

    ripples.insert(args.task_id, args.new_start_date);

    let tx = conn.unchecked_transaction()?;
    let mut updated: Vec<Task> = Vec::new();
    for t in &tasks {
        if let Some(new_start) = ripples.get(&t.id) {
            let mut nt = t.clone();
            nt.start_date = *new_start;
            tx.execute(
                "UPDATE task SET start_date = ?1 WHERE id = ?2",
                rusqlite::params![nt.start_date.to_string(), nt.id],
            )?;
            updated.push(nt);
        }
    }
    tx.commit()?;

    Ok(DragResult { updated_tasks: updated })
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
}
