use chrono::Local;
use serde::Deserialize;
use tauri::State;

use crate::commands::Db;
use crate::db::models::{BoqItem, Procurement};
use crate::repo::boq as boq_repo;
use crate::{GbError, GbResult};

#[tauri::command]
pub fn list_boq_items(db: State<Db>, job_id: i64) -> GbResult<Vec<BoqItem>> {
    let conn = db.0.lock().unwrap();
    boq_repo::list_by_job(&conn, job_id)
}

#[tauri::command]
pub fn create_boq_item(db: State<Db>, job_id: i64) -> GbResult<BoqItem> {
    let conn = db.0.lock().unwrap();
    boq_repo::create(&conn, job_id)
}

/// Content update. Never changes procurement/delivered_date (repo guard).
#[tauri::command]
pub fn update_boq_item(db: State<Db>, args: BoqItem) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::update(&conn, &args)
}

#[derive(Debug, Deserialize)]
pub struct SetProcurementArgs {
    pub id: i64,
    /// "not_ordered" | "quoted" | "ordered" | "delivered"
    pub procurement: String,
    /// ISO date; only used when procurement == "delivered".
    /// If omitted while delivering, today's date is used.
    pub delivered_date: Option<String>,
}

#[tauri::command]
pub fn set_boq_procurement(db: State<Db>, args: SetProcurementArgs) -> GbResult<()> {
    let status = Procurement::from_db_str(&args.procurement)
        .map_err(GbError::Validation)?;
    let today = Local::now().naive_local().date().format("%Y-%m-%d").to_string();
    let delivered_date: Option<String> = if status == Procurement::Delivered {
        Some(args.delivered_date.unwrap_or(today))
    } else {
        None
    };
    let conn = db.0.lock().unwrap();
    boq_repo::set_procurement(&conn, args.id, status, delivered_date.as_deref())
}

#[derive(Debug, Deserialize)]
pub struct ReorderArgs {
    pub id: i64,
    pub order_index: i64,
}

#[tauri::command]
pub fn reorder_boq_item(db: State<Db>, args: ReorderArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::reorder(&conn, args.id, args.order_index)
}

#[tauri::command]
pub fn delete_boq_item(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::delete(&conn, id)
}

#[derive(Debug, Deserialize)]
pub struct SetBudgetArgs {
    pub job_id: i64,
    pub budget: Option<f64>,
}

#[tauri::command]
pub fn set_job_budget(db: State<Db>, args: SetBudgetArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::set_job_budget(&conn, args.job_id, args.budget)
}

#[tauri::command]
pub fn get_job_budget(db: State<Db>, job_id: i64) -> GbResult<Option<f64>> {
    let conn = db.0.lock().unwrap();
    boq_repo::get_job_budget(&conn, job_id)
}
