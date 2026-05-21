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
        tx.execute(
            "INSERT INTO task (id, phase_id, name, start_date, duration_workdays, order_index, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![t.id, t.phase_id, t.name, t.start_date.to_string(), t.duration_workdays, t.order_index, t.notes],
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
    use crate::db::models::{NewJob, NewPhase, NewTask, NewDependency, NewNoWorkDay};
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
}
