use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Dependency, NewDependency};
use crate::repo::dependency as dep_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreateDepArgs {
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub lag_days: i64,
}

#[tauri::command]
pub fn list_dependencies(db: State<Db>, job_id: i64) -> GbResult<Vec<Dependency>> {
    let conn = db.0.lock().unwrap();
    dep_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn create_dependency(db: State<Db>, args: CreateDepArgs) -> GbResult<Dependency> {
    let conn = db.0.lock().unwrap();
    dep_repo::create(&conn, &NewDependency {
        predecessor_id: args.predecessor_id,
        successor_id: args.successor_id,
        lag_days: args.lag_days,
    })
}

#[tauri::command]
pub fn update_dependency_lag(db: State<Db>, id: i64, lag_days: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    dep_repo::update_lag(&conn, id, lag_days)
}

#[tauri::command]
pub fn delete_dependency(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    dep_repo::delete(&conn, id)
}
