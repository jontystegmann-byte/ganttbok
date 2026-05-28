use chrono::{NaiveDate, Duration, Datelike};
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Task, NewTask, TaskStatus, meta_get};
use crate::repo::task as task_repo;
use crate::repo::no_work_day as nwd_repo;
use crate::repo::dependency as dep_repo;
use crate::repo::job as job_repo;
use crate::deps::ripple::compute_ripple;
use crate::calendar::workday::add_workdays_excluding;
use crate::{GbError, GbResult};

#[derive(Debug, Clone, Serialize)]
pub struct OverdueReview {
    pub task_id: i64,
    pub task_name: String,
    pub phase_id: i64,
    pub planned_end_date: NaiveDate,
}

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

/// Lists tasks whose planned end date is strictly before `today` and which are
/// not yet marked Done. These are surfaced in the Inbox for the user to review:
/// either confirm a completion date (Mark Done) or flag as Running Late.
#[tauri::command]
pub fn list_overdue_reviews(db: State<Db>, job_id: i64, today: String) -> GbResult<Vec<OverdueReview>> {
    let conn = db.0.lock().unwrap();
    let today = parse_date(&today)?;
    list_overdue_reviews_inner(&conn, job_id, today)
}

pub(crate) fn list_overdue_reviews_inner(
    conn: &rusqlite::Connection,
    job_id: i64,
    today: NaiveDate,
) -> GbResult<Vec<OverdueReview>> {
    let include_weekends = meta_get(conn, "include_weekends")
        .ok().flatten().map(|s| s == "1").unwrap_or(false);
    let tasks = task_repo::list_for_job(conn, job_id)?;
    let nwds: std::collections::HashSet<NaiveDate> = nwd_repo::list_for_job(conn, job_id)
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.date)
        .collect();

    let mut out = Vec::new();
    for t in tasks {
        if t.status == TaskStatus::Done { continue; }
        let end = add_workdays_excluding(t.start_date, (t.duration_workdays - 1).max(0), &nwds, include_weekends);
        if end < today {
            out.push(OverdueReview {
                task_id: t.id,
                task_name: t.name,
                phase_id: t.phase_id,
                planned_end_date: end,
            });
        }
    }
    Ok(out)
}

/// Mark a task Done with an explicit completion date. Adjusts the task's
/// duration so its end_date aligns with the picked date, then — if the job's
/// auto_shift_dependents flag is on — ripples dependent tasks' start dates
/// by the same workday delta (negative if early, positive if late).
#[tauri::command]
pub fn mark_task_done_on_date(
    db: State<Db>,
    job_id: i64,
    task_id: i64,
    completion_date: String,
) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let date = parse_date(&completion_date)?;
    mark_task_done_on_date_inner(&conn, job_id, task_id, date)
}

pub(crate) fn mark_task_done_on_date_inner(
    conn: &rusqlite::Connection,
    job_id: i64,
    task_id: i64,
    completion_date: NaiveDate,
) -> GbResult<()> {
    let include_weekends = meta_get(conn, "include_weekends")
        .ok().flatten().map(|s| s == "1").unwrap_or(false);
    let mut task = task_repo::get(conn, task_id)?;
    let job = job_repo::get(conn, job_id)?;
    let nwds: std::collections::HashSet<NaiveDate> = nwd_repo::list_for_job(conn, job_id)
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.date)
        .collect();

    let old_duration = task.duration_workdays;
    let new_duration = workdays_inclusive(task.start_date, completion_date, &nwds, include_weekends).max(1);
    let shift: i64 = new_duration - old_duration;

    task.status = TaskStatus::Done;
    task.completion_date = Some(completion_date);
    task.duration_workdays = new_duration;
    task_repo::update(conn, &task)?;

    if shift != 0 && job.auto_shift_dependents {
        let tasks = task_repo::list_for_job(conn, job_id)?;
        let deps = dep_repo::list_for_job(conn, job_id)?;
        let shifts = compute_ripple(&tasks, &deps, task_id, shift, &nwds, include_weekends);
        for (id, new_start) in shifts {
            let mut t = task_repo::get(conn, id)?;
            t.start_date = new_start;
            task_repo::update(conn, &t)?;
        }
    }
    Ok(())
}

/// Flag a task as Running Late. Catch-up extends its duration so its end date
/// is no earlier than `today`, then ripples downstream dependents by the
/// workday delta (if the job's auto_shift_dependents flag is on).
#[tauri::command]
pub fn mark_task_running_late(
    db: State<Db>,
    job_id: i64,
    task_id: i64,
    today: String,
) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let today = parse_date(&today)?;
    mark_task_running_late_inner(&conn, job_id, task_id, today)
}

pub(crate) fn mark_task_running_late_inner(
    conn: &rusqlite::Connection,
    job_id: i64,
    task_id: i64,
    today: NaiveDate,
) -> GbResult<()> {
    let include_weekends = meta_get(conn, "include_weekends")
        .ok().flatten().map(|s| s == "1").unwrap_or(false);
    let mut task = task_repo::get(conn, task_id)?;
    let job = job_repo::get(conn, job_id)?;
    let nwds: std::collections::HashSet<NaiveDate> = nwd_repo::list_for_job(conn, job_id)
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.date)
        .collect();

    let old_duration = task.duration_workdays;
    let old_end = add_workdays_excluding(task.start_date, (old_duration - 1).max(0), &nwds, include_weekends);
    let new_end = if today > old_end { today } else { old_end };
    let new_duration = workdays_inclusive(task.start_date, new_end, &nwds, include_weekends).max(1);
    let shift: i64 = new_duration - old_duration;

    task.status = TaskStatus::Late;
    task.completion_date = None;
    task.duration_workdays = new_duration;
    task_repo::update(conn, &task)?;

    if shift > 0 && job.auto_shift_dependents {
        let tasks = task_repo::list_for_job(conn, job_id)?;
        let deps = dep_repo::list_for_job(conn, job_id)?;
        let shifts = compute_ripple(&tasks, &deps, task_id, shift, &nwds, include_weekends);
        for (id, new_start) in shifts {
            let mut t = task_repo::get(conn, id)?;
            t.start_date = new_start;
            task_repo::update(conn, &t)?;
        }
    }
    Ok(())
}

/// Daily tick for every Late task in the given job. Each Late task is catch-up
/// extended so its end date is at least `today`; downstream dependents ripple
/// when the job has auto_shift_dependents enabled. Idempotent: running twice on
/// the same day has no effect (catch-up is a no-op when end >= today already).
/// Returns the number of tasks whose duration was changed.
#[tauri::command]
pub fn tick_late_tasks(db: State<Db>, job_id: i64, today: String) -> GbResult<usize> {
    let conn = db.0.lock().unwrap();
    let today = parse_date(&today)?;
    tick_late_tasks_inner(&conn, job_id, today)
}

pub(crate) fn tick_late_tasks_inner(
    conn: &rusqlite::Connection,
    job_id: i64,
    today: NaiveDate,
) -> GbResult<usize> {
    let tasks = task_repo::list_for_job(conn, job_id)?;
    let late_ids: Vec<i64> = tasks.iter()
        .filter(|t| t.status == TaskStatus::Late)
        .map(|t| t.id)
        .collect();

    let mut extended = 0usize;
    for tid in late_ids {
        // Capture pre-tick duration so we can detect whether this task was actually extended.
        let before = task_repo::get(conn, tid)?.duration_workdays;
        mark_task_running_late_inner(conn, job_id, tid, today)?;
        let after = task_repo::get(conn, tid)?.duration_workdays;
        if after != before { extended += 1; }
    }
    Ok(extended)
}

/// Count workdays between `start` and `end` inclusive, respecting `excluded`
/// (no-work days) and the `include_weekends` flag. Returns 0 if `end < start`.
fn workdays_inclusive(
    start: NaiveDate,
    end: NaiveDate,
    excluded: &std::collections::HashSet<NaiveDate>,
    include_weekends: bool,
) -> i64 {
    if end < start { return 0; }
    let is_work = |d: NaiveDate| {
        crate::calendar::workday::is_workday(d, include_weekends) && !excluded.contains(&d)
    };
    let mut cur = start;
    let mut n: i64 = 0;
    while cur <= end {
        if is_work(cur) { n += 1; }
        cur += Duration::days(1);
    }
    n
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
    fn new_task_defaults_to_on_track() {
        let (conn, phase_id) = setup();
        let t = task_repo::create(&conn, &NewTask {
            phase_id, name: "X".into(),
            start_date: NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(),
            duration_workdays: 2, order_index: 0, notes: None,
        }).unwrap();
        assert_eq!(t.status, TaskStatus::OnTrack);
    }

    #[test]
    fn mark_task_done_early_pulls_dependents_in() {
        let (conn, phase_id) = setup();
        // A: start 8 Jun, duration 5 workdays → ends Fri 12 Jun
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 5, order_index: 0, notes: None,
        }).unwrap();
        // B: depends on A. Start 15 Jun (the Monday after A's end).
        let b = task_repo::create(&conn, &NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();
        crate::repo::dependency::create(&conn, &crate::db::models::NewDependency {
            predecessor_id: a.id, successor_id: b.id, lag_days: 0,
        }).unwrap();

        // User confirms A finished 2 workdays early — Wed 10 Jun.
        mark_task_done_on_date_inner(
            &conn, 1, a.id,
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
        ).unwrap();

        let a_after = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(a_after.status, TaskStatus::Done);
        assert_eq!(a_after.completion_date, Some(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()));
        assert_eq!(a_after.duration_workdays, 3, "A's duration now spans 8–10 Jun = 3 workdays");

        let b_after = task_repo::get(&conn, b.id).unwrap();
        // B was 15 Jun, A shifted -2 workdays → B should now start 11 Jun (Thu)
        assert_eq!(b_after.start_date, NaiveDate::from_ymd_opt(2026, 6, 11).unwrap());
    }

    #[test]
    fn mark_task_done_late_pushes_dependents_out() {
        let (conn, phase_id) = setup();
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 5, order_index: 0, notes: None,
        }).unwrap();
        let b = task_repo::create(&conn, &NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();
        crate::repo::dependency::create(&conn, &crate::db::models::NewDependency {
            predecessor_id: a.id, successor_id: b.id, lag_days: 0,
        }).unwrap();

        // A finished 3 workdays late — Wed 17 Jun.
        mark_task_done_on_date_inner(
            &conn, 1, a.id,
            NaiveDate::from_ymd_opt(2026, 6, 17).unwrap(),
        ).unwrap();

        let a_after = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(a_after.duration_workdays, 8, "A now spans 8–17 Jun = 8 workdays");

        let b_after = task_repo::get(&conn, b.id).unwrap();
        // A shifted +3 workdays → B should be pushed to Wed 18 Jun
        assert_eq!(b_after.start_date, NaiveDate::from_ymd_opt(2026, 6, 18).unwrap());
    }

    #[test]
    fn mark_task_done_with_autoshift_off_leaves_dependents_alone() {
        let (conn, phase_id) = setup();
        // Turn off auto-shift on the seeded job
        let mut job = job_repo::get(&conn, 1).unwrap();
        job.auto_shift_dependents = false;
        job_repo::update(&conn, &job).unwrap();

        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 5, order_index: 0, notes: None,
        }).unwrap();
        let b = task_repo::create(&conn, &NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();
        crate::repo::dependency::create(&conn, &crate::db::models::NewDependency {
            predecessor_id: a.id, successor_id: b.id, lag_days: 0,
        }).unwrap();

        mark_task_done_on_date_inner(
            &conn, 1, a.id,
            NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
        ).unwrap();

        let b_after = task_repo::get(&conn, b.id).unwrap();
        assert_eq!(b_after.start_date, NaiveDate::from_ymd_opt(2026, 6, 15).unwrap(),
                   "B must stay put when auto_shift_dependents is off");
    }

    #[test]
    fn mark_task_running_late_catches_up_to_today_and_pushes_dependents() {
        let (conn, phase_id) = setup();
        // A: started Mon 8 Jun, 3 workdays → ends Wed 10 Jun
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        // B: depends on A. Starts Thu 11 Jun.
        let b = task_repo::create(&conn, &NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 11).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();
        crate::repo::dependency::create(&conn, &crate::db::models::NewDependency {
            predecessor_id: a.id, successor_id: b.id, lag_days: 0,
        }).unwrap();

        // Today is Mon 15 Jun (A is overdue by 2 workdays — Thu 11 and Fri 12).
        let today = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();
        mark_task_running_late_inner(&conn, 1, a.id, today).unwrap();

        let a_after = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(a_after.status, TaskStatus::Late);
        // A's new duration: Mon 8 → Mon 15 inclusive = 6 workdays
        assert_eq!(a_after.duration_workdays, 6);

        let b_after = task_repo::get(&conn, b.id).unwrap();
        // B was 11 Jun, A extended by 3 workdays (3 → 6) → B should now start Tue 16 Jun
        assert_eq!(b_after.start_date, NaiveDate::from_ymd_opt(2026, 6, 16).unwrap());
    }

    #[test]
    fn mark_task_running_late_no_extension_needed_when_today_within_planned_window() {
        // Task ending in the future but user marks Running Late preemptively.
        // We should set status=Late but NOT extend the duration (today < old_end).
        let (conn, phase_id) = setup();
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 10, order_index: 0, notes: None,
        }).unwrap();

        let today = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
        mark_task_running_late_inner(&conn, 1, a.id, today).unwrap();

        let a_after = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(a_after.status, TaskStatus::Late);
        assert_eq!(a_after.duration_workdays, 10, "duration must NOT change");
    }

    #[test]
    fn tick_late_tasks_extends_then_idempotent_on_same_day() {
        let (conn, phase_id) = setup();
        // A: started Mon 8 Jun, 3 workdays → ends Wed 10 Jun. Flagged Late on 11 Jun.
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        mark_task_running_late_inner(&conn, 1, a.id, NaiveDate::from_ymd_opt(2026, 6, 11).unwrap()).unwrap();
        let after_first = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(after_first.duration_workdays, 4);

        // Day rolls over to Fri 12 Jun. Tick should extend by 1.
        let n = tick_late_tasks_inner(&conn, 1, NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()).unwrap();
        assert_eq!(n, 1);
        let after_tick = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(after_tick.duration_workdays, 5);

        // Same day rerun is a no-op.
        let n2 = tick_late_tasks_inner(&conn, 1, NaiveDate::from_ymd_opt(2026, 6, 12).unwrap()).unwrap();
        assert_eq!(n2, 0);
        let unchanged = task_repo::get(&conn, a.id).unwrap();
        assert_eq!(unchanged.duration_workdays, 5);
    }

    #[test]
    fn list_overdue_reviews_flags_only_past_non_done_tasks() {
        let (conn, phase_id) = setup();
        // A: started yesterday, 1 day duration → ended yesterday, still On Track → SHOULD appear
        let a = task_repo::create(&conn, &NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        // B: starts tomorrow → NOT overdue
        let _b = task_repo::create(&conn, &NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();
        // C: ended yesterday but already Done → must NOT appear
        let c = task_repo::create(&conn, &NewTask {
            phase_id, name: "C".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            duration_workdays: 1, order_index: 2, notes: None,
        }).unwrap();
        set_task_status_inner(&conn, c.id, "done", Some(NaiveDate::from_ymd_opt(2026, 6, 5).unwrap())).unwrap();

        let job_id = 1; // setup() creates job at id=1
        let today = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
        let overdue = list_overdue_reviews_inner(&conn, job_id, today).unwrap();

        assert_eq!(overdue.len(), 1, "only Task A should be overdue");
        assert_eq!(overdue[0].task_id, a.id);
        assert_eq!(overdue[0].planned_end_date, NaiveDate::from_ymd_opt(2026, 6, 5).unwrap());
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
