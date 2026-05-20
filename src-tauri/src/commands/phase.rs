use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Phase, NewPhase};
use crate::repo::phase as phase_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreatePhaseArgs {
    pub job_id: i64,
    pub name: String,
    pub colour: String,
}

#[tauri::command]
pub fn list_phases(db: State<Db>, job_id: i64) -> GbResult<Vec<Phase>> {
    let conn = db.0.lock().unwrap();
    phase_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn create_phase(db: State<Db>, args: CreatePhaseArgs) -> GbResult<Phase> {
    let conn = db.0.lock().unwrap();
    let existing = phase_repo::list_for_job(&conn, args.job_id)?;
    let next_order = existing.iter().map(|p| p.order_index).max().unwrap_or(-1) + 1;
    phase_repo::create(&conn, &NewPhase {
        job_id: args.job_id, name: args.name, colour: args.colour,
        order_index: next_order, collapsed: false,
    })
}

#[tauri::command]
pub fn update_phase(db: State<Db>, phase: Phase) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    phase_repo::update(&conn, &phase)
}

#[tauri::command]
pub fn delete_phase(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    phase_repo::delete(&conn, id)
}

#[tauri::command]
pub fn reorder_phases(db: State<Db>, job_id: i64, ordered_ids: Vec<i64>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    phase_repo::reorder(&conn, job_id, &ordered_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::NewJob;
    use crate::repo::job;
    use chrono::NaiveDate;

    #[test]
    fn create_phase_auto_increments_order_index() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        let existing = phase_repo::list_for_job(&conn, j.id).unwrap();
        let next = existing.iter().map(|p| p.order_index).max().unwrap_or(-1) + 1;
        assert_eq!(next, 0);
        phase_repo::create(&conn, &NewPhase {
            job_id: j.id, name: "A".into(), colour: "#000".into(),
            order_index: next, collapsed: false,
        }).unwrap();
        let existing = phase_repo::list_for_job(&conn, j.id).unwrap();
        let next = existing.iter().map(|p| p.order_index).max().unwrap_or(-1) + 1;
        assert_eq!(next, 1);
    }
}
