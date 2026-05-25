use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Job, NewJob};
use crate::repo::job as job_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreateJobArgs {
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
    #[serde(default = "default_true")]
    pub holidays_block_work: bool,
    #[serde(default = "default_region")]
    pub region: String,
}

fn default_true() -> bool { true }
fn default_region() -> String { "ZA".into() }

#[tauri::command]
pub fn list_jobs(db: State<Db>) -> GbResult<Vec<Job>> {
    let conn = db.0.lock().unwrap();
    job_repo::list_active(&conn)
}

#[tauri::command]
pub fn list_templates(db: State<Db>) -> GbResult<Vec<Job>> {
    let conn = db.0.lock().unwrap();
    job_repo::list_templates(&conn)
}

#[tauri::command]
pub fn list_archived(db: State<Db>) -> GbResult<Vec<Job>> {
    let conn = db.0.lock().unwrap();
    job_repo::list_archived(&conn)
}

#[tauri::command]
pub fn get_job(db: State<Db>, id: i64) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    job_repo::get(&conn, id)
}

#[tauri::command]
pub fn create_job(db: State<Db>, args: CreateJobArgs) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    job_repo::create(&conn, &NewJob {
        name: args.name, client: args.client, address: args.address,
        project_start_date: args.project_start_date, is_template: args.is_template,
        holidays_block_work: args.holidays_block_work,
        region: args.region,
        auto_shift_dependents: true,
    })
}

#[tauri::command]
pub fn update_job(db: State<Db>, job: Job) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    job_repo::update(&conn, &job)
}

#[tauri::command]
pub fn archive_job(db: State<Db>, id: i64, archived: bool) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    job_repo::set_archived(&conn, id, archived)
}

#[tauri::command]
pub fn delete_job(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    job_repo::delete(&conn, id)
}

#[tauri::command]
pub fn set_job_auto_shift(db: State<Db>, id: i64, enabled: bool) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    set_job_auto_shift_inner(&conn, id, enabled)
}

pub(crate) fn set_job_auto_shift_inner(
    conn: &rusqlite::Connection,
    id: i64,
    enabled: bool,
) -> GbResult<()> {
    let mut job = crate::repo::job::get(conn, id)?;
    job.auto_shift_dependents = enabled;
    crate::repo::job::update(conn, &job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::commands::Db;

    fn fresh() -> Db { Db::new(open_in_memory().unwrap()) }

    /// In-memory DB seeded with one job at id=1.
    fn create_test_db() -> Db {
        let db = fresh();
        {
            let conn = db.0.lock().unwrap();
            job_repo::create(&conn, &NewJob {
                name: "Test".into(), client: None, address: None,
                project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
                is_template: false, holidays_block_work: true, region: "ZA".into(),
                auto_shift_dependents: true,
            }).unwrap();
        }
        db
    }

    #[test]
    fn set_job_auto_shift_toggles_correctly() {
        let db = create_test_db();

        set_job_auto_shift_inner(&db.0.lock().unwrap(), 1, false).unwrap();
        let job = crate::repo::job::get(&db.0.lock().unwrap(), 1).unwrap();
        assert!(!job.auto_shift_dependents);

        set_job_auto_shift_inner(&db.0.lock().unwrap(), 1, true).unwrap();
        let job = crate::repo::job::get(&db.0.lock().unwrap(), 1).unwrap();
        assert!(job.auto_shift_dependents);
    }

    #[test]
    fn create_then_list() {
        let db = fresh();
        // Bypass tauri::State by calling the inner repo through the same lock.
        let conn = db.0.lock().unwrap();
        let job = job_repo::create(&conn, &NewJob {
            name: "Sea Point".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false, holidays_block_work: true, region: "ZA".into(),
            auto_shift_dependents: true,
        }).unwrap();
        let active = job_repo::list_active(&conn).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, job.id);
    }
}
