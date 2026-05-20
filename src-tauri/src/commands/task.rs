use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Task, NewTask};
use crate::repo::task as task_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreateTaskArgs {
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
}

#[tauri::command]
pub fn list_tasks(db: State<Db>, job_id: i64) -> GbResult<Vec<Task>> {
    let conn = db.0.lock().unwrap();
    task_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn create_task(db: State<Db>, args: CreateTaskArgs) -> GbResult<Task> {
    let conn = db.0.lock().unwrap();
    let existing = task_repo::list_for_phase(&conn, args.phase_id)?;
    let next = existing.iter().map(|t| t.order_index).max().unwrap_or(-1) + 1;
    let dur = args.duration_workdays.max(1);
    task_repo::create(&conn, &NewTask {
        phase_id: args.phase_id, name: args.name,
        start_date: args.start_date, duration_workdays: dur,
        order_index: next, notes: None,
    })
}

#[tauri::command]
pub fn update_task(db: State<Db>, task: Task) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let mut t = task;
    t.duration_workdays = t.duration_workdays.max(1);
    task_repo::update(&conn, &t)
}

#[tauri::command]
pub fn delete_task(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    task_repo::delete(&conn, id)
}

#[tauri::command]
pub fn reorder_tasks(db: State<Db>, phase_id: i64, ordered_ids: Vec<i64>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    task_repo::reorder(&conn, phase_id, &ordered_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase};
    use crate::repo::{job, phase};

    fn setup() -> (rusqlite::Connection, i64) {
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
        (conn, p.id)
    }

    #[test]
    fn update_task_clamps_duration_to_one() {
        let (conn, phase_id) = setup();
        let t = task_repo::create(&conn, &NewTask {
            phase_id, name: "T".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        let mut t2 = t.clone();
        t2.duration_workdays = 0;
        t2.duration_workdays = t2.duration_workdays.max(1);
        task_repo::update(&conn, &t2).unwrap();
        let fetched = task_repo::get(&conn, t.id).unwrap();
        assert_eq!(fetched.duration_workdays, 1);
    }
}
