use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{NoWorkDay, NewNoWorkDay};
use crate::repo::no_work_day as nwd_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct AddManualArgs {
    pub job_id: i64,
    pub date: NaiveDate,
    pub reason: String,
}

#[tauri::command]
pub fn list_no_work_days(db: State<Db>, job_id: i64) -> GbResult<Vec<NoWorkDay>> {
    let conn = db.0.lock().unwrap();
    nwd_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn add_manual_no_work_day(db: State<Db>, args: AddManualArgs) -> GbResult<NoWorkDay> {
    let conn = db.0.lock().unwrap();
    nwd_repo::create(&conn, &NewNoWorkDay {
        job_id: args.job_id, date: args.date,
        reason: args.reason, source: "manual".into(),
    })
}

#[tauri::command]
pub fn delete_no_work_day(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    nwd_repo::delete(&conn, id)
}

#[derive(Debug, Deserialize)]
pub struct SyncSaArgs {
    pub job_id: i64,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[tauri::command]
pub fn sync_sa_holidays(db: State<Db>, args: SyncSaArgs) -> GbResult<i64> {
    let conn = db.0.lock().unwrap();
    nwd_repo::sync_sa_holidays(&conn, args.job_id, args.from, args.to)
}
