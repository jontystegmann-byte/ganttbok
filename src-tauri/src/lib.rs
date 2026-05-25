pub mod calendar;
pub mod chaser;
pub mod claude_config;
pub mod commands;
pub mod db;
pub mod deps;
pub mod error;
pub mod patches;
pub mod repo;

pub use error::{GbError, GbResult};

use commands::Db;
use std::path::PathBuf;
use tauri::Manager;

pub(crate) fn db_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .expect("no data_local_dir")
        .join("Gantt Bok");
    std::fs::create_dir_all(&dir).expect("could not create data dir");
    dir.join("ganttbok.db")
}

pub fn run() {
    let conn = db::connection::open(&db_path()).expect("failed to open db");

    // Sweep any proposed patches older than 30 days to 'expired'.
    // This runs synchronously before the window opens, so the Inbox
    // never shows stale rows.
    if let Err(e) = commands::patches::expire_stale_patches_inner(&conn) {
        eprintln!("warn: expire_stale_patches on startup failed: {e}");
    }

    let db = Db::new(conn);

    let mut builder = tauri::Builder::default();
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    builder
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            commands::job::list_jobs,
            commands::job::list_templates,
            commands::job::list_archived,
            commands::job::get_job,
            commands::job::create_job,
            commands::job::update_job,
            commands::job::archive_job,
            commands::job::delete_job,
            commands::job::set_job_auto_shift,
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
            commands::task::set_task_status,
            commands::drag::drag_task,
            commands::dependency::list_dependencies,
            commands::dependency::create_dependency,
            commands::dependency::update_dependency_lag,
            commands::dependency::delete_dependency,
            commands::no_work_day::list_no_work_days,
            commands::no_work_day::add_manual_no_work_day,
            commands::no_work_day::delete_no_work_day,
            commands::no_work_day::sync_sa_holidays,
            commands::no_work_day::sync_holidays,
            commands::meta::startup_info,
            commands::meta::mark_clean_shutdown,
            commands::meta::set_last_open_job,
            commands::meta::set_sidebar_width,
            commands::meta::touch_last_save,
            commands::meta::print_window,
            commands::meta::print_window_portrait,
            commands::meta::set_duration_unit,
            commands::meta::set_holidays_block_work_default,
            commands::meta::set_include_weekends,
            commands::meta::set_ui_scale,
            commands::meta::set_region_default,
            commands::meta::set_meta_value,
            commands::meta::get_meta_value,
            commands::meta::bundle_rename_needed,
            commands::meta::rename_bundle_and_restart,
            commands::chaser::list_contacts,
            commands::chaser::create_contact,
            commands::chaser::update_contact,
            commands::chaser::delete_contact,
            commands::chaser::assign_task_contact,
            commands::chaser::send_chaser,
            commands::chaser::test_telegram,
            commands::chaser::run_chaser_check,
            commands::sync::resync_job_state,
            commands::patches::list_pending_patches,
            commands::patches::get_pending_patch,
            commands::patches::accept_patch,
            commands::patches::reject_patch,
            commands::patches::clear_resolved_patches,
            commands::patches::expire_stale_patches,
            commands::claude::detect_claude_surfaces,
            commands::claude::connect_to_claude,
            commands::claude::disconnect_from_claude,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<Db>() {
                    let conn = state.0.lock().unwrap();
                    let _ = crate::db::models::meta_set(&conn, "clean_shutdown", "1");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
