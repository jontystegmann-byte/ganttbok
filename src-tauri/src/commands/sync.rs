use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct ResyncArgs {
    pub job_id: i64,
    pub phases:        Vec<PhaseSnap>,
    pub tasks:         Vec<TaskSnap>,
    pub dependencies:  Vec<DepSnap>,
    pub no_work_days:  Vec<NwdSnap>,
}

#[derive(Debug, Deserialize)]
pub struct PhaseSnap {
    pub id: i64,
    pub name: String,
    pub colour: String,
    pub order_index: i64,
    pub collapsed: bool,
}

#[derive(Debug, Deserialize)]
pub struct TaskSnap {
    pub id: i64,
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
    pub order_index: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DepSnap {
    pub id: i64,
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub lag_days: i64,
}

#[derive(Debug, Deserialize)]
pub struct NwdSnap {
    pub id: i64,
    pub date: NaiveDate,
    pub reason: String,
    pub source: String,
}

pub fn resync(conn: &rusqlite::Connection, args: &ResyncArgs) -> GbResult<()> {
    let tx = conn.unchecked_transaction()?;

    // A structural resync (Cmd+S / save indicator / undo) carries scheduling
    // state only — it does NOT own task status. Status/completion_date are owned
    // by the dedicated status commands (set_task_status / mark_task_done_on_date /
    // mark_task_running_late). Capture them from the DB before the delete so the
    // delete+reinsert below preserves them instead of resetting to the column
    // default ('on_track', NULL). Without this, every save wiped Late/Done status.
    let mut preserved_status: std::collections::HashMap<i64, (String, Option<String>)> =
        std::collections::HashMap::new();
    {
        let mut stmt = tx.prepare(
            "SELECT t.id, t.status, t.completion_date
             FROM task t JOIN phase p ON p.id = t.phase_id
             WHERE p.job_id = ?1",
        )?;
        let rows = stmt.query_map([args.job_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
        })?;
        for row in rows {
            let (id, status, completion_date) = row?;
            preserved_status.insert(id, (status, completion_date));
        }
    }

    // Delete everything for this job, then reinsert from the snapshot.
    // task & dependency rows cascade from phase delete.
    tx.execute("DELETE FROM phase WHERE job_id = ?1", [args.job_id])?;
    tx.execute("DELETE FROM no_work_day WHERE job_id = ?1", [args.job_id])?;

    for p in &args.phases {
        tx.execute(
            "INSERT INTO phase (id, job_id, name, colour, order_index, collapsed)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![p.id, args.job_id, p.name, p.colour, p.order_index, p.collapsed as i64],
        )?;
    }
    for t in &args.tasks {
        // Preserve the task's existing status/completion_date across the reinsert.
        // New tasks (created this session, not in the pre-delete snapshot) fall back
        // to the on_track default.
        let (status, completion_date) = preserved_status
            .get(&t.id)
            .cloned()
            .unwrap_or_else(|| ("on_track".to_string(), None));
        tx.execute(
            "INSERT INTO task (id, phase_id, name, start_date, duration_workdays, order_index, notes, status, completion_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![t.id, t.phase_id, t.name, t.start_date.to_string(), t.duration_workdays, t.order_index, t.notes, status, completion_date],
        )?;
    }
    for d in &args.dependencies {
        tx.execute(
            "INSERT INTO dependency (id, predecessor_id, successor_id, type, lag_days)
             VALUES (?1, ?2, ?3, 'FS', ?4)",
            rusqlite::params![d.id, d.predecessor_id, d.successor_id, d.lag_days],
        )?;
    }
    for n in &args.no_work_days {
        tx.execute(
            "INSERT INTO no_work_day (id, job_id, date, reason, source)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![n.id, args.job_id, n.date.to_string(), n.reason, n.source],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[tauri::command]
pub fn resync_job_state(db: State<Db>, args: ResyncArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    resync(&conn, &args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase, NewTask, NewDependency, NewNoWorkDay, TaskStatus};
    use crate::repo::{job as job_repo, phase as phase_repo, task as task_repo, dependency as dep_repo, no_work_day as nwd_repo};

    #[test]
    fn resync_replaces_job_state_from_snapshot() {
        let conn = open_in_memory().unwrap();

        // Create a job with phases/tasks/deps/no-work
        let job = job_repo::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            is_template: false,
            holidays_block_work: true,
            region: "ZA".into(),
            auto_shift_dependents: true,
        }).unwrap();

        let p1 = phase_repo::create(&conn, &NewPhase { job_id: job.id, name: "P1".into(), colour: "#000".into(), order_index: 0, collapsed: false }).unwrap();
        let t1 = task_repo::create(&conn, &NewTask {
            phase_id: p1.id, name: "T1".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        let t2 = task_repo::create(&conn, &NewTask {
            phase_id: p1.id, name: "T2".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();
        let _d = dep_repo::create(&conn, &NewDependency { predecessor_id: t1.id, successor_id: t2.id, lag_days: 0 }).unwrap();
        let _n = nwd_repo::create(&conn, &NewNoWorkDay {
            job_id: job.id,
            date: NaiveDate::from_ymd_opt(2026, 6, 10).unwrap(),
            reason: "Test".into(), source: "manual".into(),
        }).unwrap();

        // Build a snapshot with only t1 (drop t2 and the dep, drop nwd)
        let args = ResyncArgs {
            job_id: job.id,
            phases: vec![PhaseSnap { id: p1.id, name: "P1-renamed".into(), colour: "#fff".into(), order_index: 0, collapsed: false }],
            tasks: vec![TaskSnap {
                id: t1.id, phase_id: p1.id, name: "T1".into(),
                start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
                duration_workdays: 3, order_index: 0, notes: None,
            }],
            dependencies: vec![],
            no_work_days: vec![],
        };

        resync(&conn, &args).unwrap();

        let phases = phase_repo::list_for_job(&conn, job.id).unwrap();
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].name, "P1-renamed");

        let tasks = task_repo::list_for_job(&conn, job.id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, t1.id);

        let deps = dep_repo::list_for_job(&conn, job.id).unwrap();
        assert_eq!(deps.len(), 0);

        let nwd = nwd_repo::list_for_job(&conn, job.id).unwrap();
        assert_eq!(nwd.len(), 0);
    }

    /// Regression: a structural resync (Cmd+S / undo / save indicator) must NOT
    /// wipe task status. Previously the reinsert omitted status/completion_date,
    /// so every Late/Done task silently reverted to on_track on the next save.
    #[test]
    fn resync_preserves_task_status_and_completion_date() {
        let conn = open_in_memory().unwrap();
        let job = job_repo::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            is_template: false, holidays_block_work: true,
            region: "ZA".into(), auto_shift_dependents: true,
        }).unwrap();
        let p1 = phase_repo::create(&conn, &NewPhase {
            job_id: job.id, name: "P1".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        // t_done: completed; t_late: running late; t_ok: on track.
        let t_done = task_repo::create(&conn, &NewTask {
            phase_id: p1.id, name: "Done".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 2).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t_late = task_repo::create(&conn, &NewTask {
            phase_id: p1.id, name: "Late".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 3).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        let t_ok = task_repo::create(&conn, &NewTask {
            phase_id: p1.id, name: "OnTrack".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 4).unwrap(),
            duration_workdays: 1, order_index: 2, notes: None,
        }).unwrap();

        let done_date = NaiveDate::from_ymd_opt(2026, 6, 2).unwrap();
        crate::commands::task::set_task_status_inner(&conn, t_done.id, "done", Some(done_date)).unwrap();
        crate::commands::task::set_task_status_inner(&conn, t_late.id, "late", None).unwrap();

        // Frontend's structural snapshot carries no status field (TaskSnap has none).
        let snap = |t: &crate::db::models::Task| TaskSnap {
            id: t.id, phase_id: t.phase_id, name: t.name.clone(),
            start_date: t.start_date, duration_workdays: t.duration_workdays,
            order_index: t.order_index, notes: t.notes.clone(),
        };
        let args = ResyncArgs {
            job_id: job.id,
            phases: vec![PhaseSnap { id: p1.id, name: "P1".into(), colour: "#000".into(), order_index: 0, collapsed: false }],
            tasks: vec![snap(&t_done), snap(&t_late), snap(&t_ok)],
            dependencies: vec![],
            no_work_days: vec![],
        };
        resync(&conn, &args).unwrap();

        let after = task_repo::list_for_job(&conn, job.id).unwrap();
        let by_id = |id: i64| after.iter().find(|t| t.id == id).unwrap();
        assert_eq!(by_id(t_done.id).status, TaskStatus::Done, "Done must survive resync");
        assert_eq!(by_id(t_done.id).completion_date, Some(done_date), "completion_date must survive");
        assert_eq!(by_id(t_late.id).status, TaskStatus::Late, "Late must survive resync");
        assert_eq!(by_id(t_ok.id).status, TaskStatus::OnTrack, "On-track stays on-track");
    }
}
