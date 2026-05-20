pub mod calendar;
pub mod commands;
pub mod db;
pub mod deps;
pub mod error;
pub mod repo;

pub use error::{GbError, GbResult};

use commands::Db;
use std::path::PathBuf;

fn db_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .expect("no data_local_dir")
        .join("Gantt Bok");
    std::fs::create_dir_all(&dir).expect("could not create data dir");
    dir.join("ganttbok.db")
}

pub fn run() {
    let conn = db::connection::open(&db_path()).expect("failed to open db");
    let db = Db::new(conn);

    tauri::Builder::default()
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            commands::job::list_jobs,
            commands::job::list_templates,
            commands::job::get_job,
            commands::job::create_job,
            commands::job::update_job,
            commands::job::archive_job,
            commands::job::delete_job,
            commands::template::save_as_template,
            commands::template::instantiate_template,
            commands::phase::list_phases,
            commands::phase::create_phase,
            commands::phase::update_phase,
            commands::phase::delete_phase,
            commands::phase::reorder_phases,
            commands::task::list_tasks,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::delete_task,
            commands::task::reorder_tasks,
            commands::drag::drag_task,
            commands::dependency::list_dependencies,
            commands::dependency::create_dependency,
            commands::dependency::update_dependency_lag,
            commands::dependency::delete_dependency,
            commands::no_work_day::list_no_work_days,
            commands::no_work_day::add_manual_no_work_day,
            commands::no_work_day::delete_no_work_day,
            commands::no_work_day::sync_sa_holidays,
            commands::meta::startup_info,
            commands::meta::mark_clean_shutdown,
            commands::meta::set_last_open_job,
            commands::meta::set_sidebar_width,
            commands::meta::touch_last_save,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
