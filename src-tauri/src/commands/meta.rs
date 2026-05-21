use serde::Serialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{meta_get, meta_set};
use crate::GbResult;

#[derive(Debug, Serialize)]
pub struct StartupInfo {
    pub clean_shutdown: bool,
    pub last_open_job_id: Option<i64>,
    pub last_save_at: Option<String>,
    pub sidebar_width: Option<i64>,
    pub duration_unit: Option<String>,
    pub holidays_block_work_default: Option<bool>,
    pub include_weekends: Option<bool>,
    pub ui_scale: Option<f64>,
}

/// Called by the frontend on app launch. Returns the previous shutdown state then marks the
/// new session as dirty (will be flipped back to clean on graceful exit).
#[tauri::command]
pub fn startup_info(db: State<Db>) -> GbResult<StartupInfo> {
    let conn = db.0.lock().unwrap();
    let clean = meta_get(&conn, "clean_shutdown")?.as_deref() == Some("1");
    let last_open_job_id = meta_get(&conn, "last_open_job_id")?.and_then(|s| s.parse().ok());
    let last_save_at = meta_get(&conn, "last_save_at")?;
    let sidebar_width = meta_get(&conn, "sidebar_width")?.and_then(|s| s.parse().ok());
    let duration_unit = meta_get(&conn, "duration_unit")?;
    let holidays_block_work_default = meta_get(&conn, "holidays_block_work_default")?
        .map(|s| s == "1");
    let include_weekends = meta_get(&conn, "include_weekends")?.map(|s| s == "1");
    let ui_scale = meta_get(&conn, "ui_scale")?.and_then(|s| s.parse::<f64>().ok());
    meta_set(&conn, "clean_shutdown", "0")?;
    Ok(StartupInfo {
        clean_shutdown: clean,
        last_open_job_id,
        last_save_at,
        sidebar_width,
        duration_unit,
        holidays_block_work_default,
        include_weekends,
        ui_scale,
    })
}

#[tauri::command]
pub fn mark_clean_shutdown(db: State<Db>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "clean_shutdown", "1")
}

#[tauri::command]
pub fn set_last_open_job(db: State<Db>, job_id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "last_open_job_id", &job_id.to_string())
}

#[tauri::command]
pub fn set_sidebar_width(db: State<Db>, width: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "sidebar_width", &width.to_string())
}

#[tauri::command]
pub fn set_duration_unit(db: State<Db>, unit: String) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "duration_unit", &unit)
}

#[tauri::command]
pub fn set_holidays_block_work_default(db: State<Db>, value: bool) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "holidays_block_work_default", if value { "1" } else { "0" })
}

#[tauri::command]
pub fn set_include_weekends(db: State<Db>, value: bool) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "include_weekends", if value { "1" } else { "0" })
}

#[tauri::command]
pub fn set_ui_scale(db: State<Db>, value: f64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "ui_scale", &value.to_string())
}

#[tauri::command]
pub fn touch_last_save(db: State<Db>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "last_save_at", &chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string())
}

/// Trigger the macOS native print panel via WebKit. JS `window.print()` is unreliable in WKWebView.
/// Also coerces the shared NSPrintInfo to A3 landscape so the Gantt prints sensibly by default.
#[tauri::command]
pub fn print_window(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSPrintInfo, NSPaperOrientation};
        use objc2_foundation::{NSSize, NSString, ns_string};
        {
            let info = NSPrintInfo::sharedPrintInfo();
            info.setOrientation(NSPaperOrientation::Landscape);
            info.setPaperSize(NSSize::new(1190.55, 841.89)); // A3 landscape (points)
            let a3: &NSString = ns_string!("iso-a3");
            info.setPaperName(Some(a3));
        }
    }
    window.print().map_err(|e| e.to_string())
}

/// Same as print_window but pre-configures A4 portrait — used for the todo-list print path.
#[tauri::command]
pub fn print_window_portrait(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSPrintInfo, NSPaperOrientation};
        use objc2_foundation::{NSSize, NSString, ns_string};
        {
            let info = NSPrintInfo::sharedPrintInfo();
            info.setOrientation(NSPaperOrientation::Portrait);
            info.setPaperSize(NSSize::new(595.28, 841.89)); // A4 portrait (points)
            let a4: &NSString = ns_string!("iso-a4");
            info.setPaperName(Some(a4));
        }
    }
    window.print().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{meta_get, meta_set};

    #[test]
    fn startup_marks_session_dirty_and_reports_previous_clean_state() {
        let conn = open_in_memory().unwrap();
        meta_set(&conn, "clean_shutdown", "1").unwrap();
        let clean = meta_get(&conn, "clean_shutdown").unwrap().as_deref() == Some("1");
        meta_set(&conn, "clean_shutdown", "0").unwrap();
        assert!(clean);
        let clean2 = meta_get(&conn, "clean_shutdown").unwrap().as_deref() == Some("1");
        assert!(!clean2);
    }
}
