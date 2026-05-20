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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
