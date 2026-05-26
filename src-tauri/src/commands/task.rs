use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Task, NewTask, TaskStatus};
use crate::repo::task as task_repo;
use crate::{GbError, GbResult};

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

#[tauri::command]
pub fn auto_transition_started_tasks(db: State<Db>, today: String) -> GbResult<usize> {
    let conn = db.0.lock().unwrap();
    let parsed = parse_date(&today)?;
    task_repo::auto_transition_started(&conn, parsed)
}

#[tauri::command]
pub fn set_task_status(
    db: State<Db>,
    id: i64,
    status: String,
    completion_date: Option<String>,
) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let parsed = completion_date.as_deref().map(parse_date).transpose()?;
    set_task_status_inner(&conn, id, &status, parsed)
}

fn parse_date(s: &str) -> Result<chrono::NaiveDate, GbError> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|e| GbError::Validation(format!("bad date {s}: {e}")))
}

pub(crate) fn set_task_status_inner(
    conn: &rusqlite::Connection,
    id: i64,
    status: &str,
    completion_date: Option<chrono::NaiveDate>,
) -> GbResult<()> {
    let status_enum = TaskStatus::from_db_str(status)
        .map_err(GbError::Validation)?;

    let mut task = crate::repo::task::get(conn, id)?;
    task.status = status_enum;
    task.completion_date = if status_enum == TaskStatus::Done {
        Some(completion_date.unwrap_or_else(|| chrono::Local::now().date_naive()))
    } else {
        None
    };
    crate::repo::task::update(conn, &task)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase, TaskStatus};
    use crate::repo::{job, phase};
    use std::sync::Mutex;

    fn setup() -> (rusqlite::Connection, i64) {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
            holidays_block_work: true,
            region: "ZA".into(),
            auto_shift_dependents: true,
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        (conn, p.id)
    }

    /// In-memory DB seeded with one job, one phase, one task — all at id=1.
    fn create_test_db() -> Db {
        let (conn, phase_id) = setup();
        task_repo::create(&conn, &NewTask {
            phase_id,
            name: "T".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 1,
            order_index: 0,
            notes: None,
        }).unwrap();
        Db(Mutex::new(conn))
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

    #[test]
    fn set_task_status_done_writes_completion_date() {
        let db = create_test_db();

        let now = chrono::Local::now().date_naive();
        set_task_status_inner(&db.0.lock().unwrap(), 1, "done", Some(now)).unwrap();

        let task = crate::repo::task::get(&db.0.lock().unwrap(), 1).unwrap();
        assert_eq!(task.status, TaskStatus::Done);
        assert_eq!(task.completion_date, Some(now));
    }

    #[test]
    fn new_task_defaults_to_not_started() {
        let (conn, phase_id) = setup();
        let t = task_repo::create(&conn, &NewTask {
            phase_id, name: "X".into(),
            start_date: NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(), // far future
            duration_workdays: 2, order_index: 0, notes: None,
        }).unwrap();
        assert_eq!(t.status, TaskStatus::NotStarted);
    }

    #[test]
    fn auto_transition_flips_only_started_not_started_tasks() {
        let (conn, phase_id) = setup();
        // Task A: starts yesterday, Not Started → should flip
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 7).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        // Task B: starts tomorrow, Not Started → must NOT flip
        let b = task_repo::create(&conn, &NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        // Task C: starts yesterday, Done → must NOT be touched
        let c = task_repo::create(&conn, &NewTask {
            phase_id, name: "C".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 7).unwrap(),
            duration_workdays: 1, order_index: 2, notes: None,
        }).unwrap();
        set_task_status_inner(&conn, c.id, "done", Some(NaiveDate::from_ymd_opt(2026, 6, 7).unwrap())).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
        let n = task_repo::auto_transition_started(&conn, today).unwrap();
        assert_eq!(n, 1, "only Task A should flip");

        assert_eq!(task_repo::get(&conn, a.id).unwrap().status, TaskStatus::OnTrack);
        assert_eq!(task_repo::get(&conn, b.id).unwrap().status, TaskStatus::NotStarted);
        assert_eq!(task_repo::get(&conn, c.id).unwrap().status, TaskStatus::Done);
    }

    #[test]
    fn set_task_status_clears_completion_date_when_not_done() {
        let db = create_test_db();
        let now = chrono::Local::now().date_naive();

        set_task_status_inner(&db.0.lock().unwrap(), 1, "done", Some(now)).unwrap();
        set_task_status_inner(&db.0.lock().unwrap(), 1, "on_track", None).unwrap();

        let task = crate::repo::task::get(&db.0.lock().unwrap(), 1).unwrap();
        assert_eq!(task.status, TaskStatus::OnTrack);
        assert_eq!(task.completion_date, None);
    }
}
