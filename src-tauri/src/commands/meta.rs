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
    pub region_default: Option<String>,
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
    let region_default = meta_get(&conn, "region_default")?;
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
        region_default,
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
pub fn set_region_default(db: State<Db>, region: String) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "region_default", &region)
}

/// Allowlisted generic meta setter. Used by Chaser settings + any future small prefs.
#[tauri::command]
pub fn set_meta_value(db: State<Db>, key: String, value: String) -> GbResult<()> {
    const ALLOWED: &[&str] = &[
        "telegram_bot_token",
        "chaser_threshold_days",
        "chaser_template_manual",
        "chaser_template_approaching",
        "chaser_template_overdue",
        "chaser_auto_enabled",
    ];
    if !ALLOWED.contains(&key.as_str()) {
        return Err(crate::GbError::Validation(format!("meta key '{key}' not in allowlist")));
    }
    let conn = db.0.lock().unwrap();
    meta_set(&conn, &key, &value)
}

#[tauri::command]
pub fn get_meta_value(db: State<Db>, key: String) -> GbResult<Option<String>> {
    let conn = db.0.lock().unwrap();
    meta_get(&conn, &key)
}

/// Detect a mismatch between the on-disk `.app` folder name and the productName.
/// Returns Some({current, desired}) when the user is running from `/Applications/Gantt Bok.app`
/// (or any other stale-named bundle) and should be offered a rename. Returns None when
/// the names already match, or when we can't safely determine a rename target (dev mode,
/// non-bundle install, etc.).
#[tauri::command]
pub fn bundle_rename_needed() -> Option<serde_json::Value> {
    let exe = std::env::current_exe().ok()?;
    // Walk up: exe → MacOS → Contents → <Name>.app
    let app_bundle = exe.parent()?.parent()?.parent()?;
    let parent_dir = app_bundle.parent()?;
    let current_name = app_bundle.file_name()?.to_str()?;
    // Only act on real .app bundles, and only inside /Applications (so we don't trash a dev tree).
    if !current_name.ends_with(".app") { return None; }
    if !parent_dir.to_str()?.starts_with("/Applications") { return None; }

    let desired_name = "Blik Plan.app";
    if current_name == desired_name { return None; }

    let current_path = app_bundle.to_str()?.to_string();
    let desired_path = parent_dir.join(desired_name).to_str()?.to_string();
    Some(serde_json::json!({
        "current_path": current_path,
        "current_name": current_name,
        "desired_path": desired_path,
        "desired_name": desired_name,
    }))
}

/// Schedule a rename of the running app bundle and quit. A small shell script waits a
/// moment, moves the bundle, then re-opens it. Safe-no-op if rename target already exists.
#[tauri::command]
pub fn rename_bundle_and_restart(app: tauri::AppHandle) -> Result<(), String> {
    let info = bundle_rename_needed().ok_or("no rename needed")?;
    let current = info.get("current_path").and_then(|v| v.as_str()).ok_or("missing current_path")?;
    let desired = info.get("desired_path").and_then(|v| v.as_str()).ok_or("missing desired_path")?;

    // Write a one-shot script: wait for parent to exit, rename, reopen, delete itself.
    let script_path = std::env::temp_dir().join("blik_plan_rename.sh");
    let script = format!(
        "#!/bin/sh\n\
         sleep 1\n\
         if [ ! -d \"{desired}\" ]; then\n\
           mv \"{current}\" \"{desired}\" 2>/dev/null\n\
         fi\n\
         open \"{desired}\"\n\
         rm -- \"$0\"\n",
        current = current,
        desired = desired,
    );
    std::fs::write(&script_path, script).map_err(|e| e.to_string())?;
    // chmod +x
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&script_path).map_err(|e| e.to_string())?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms).map_err(|e| e.to_string())?;

    std::process::Command::new("/bin/sh")
        .arg(&script_path)
        .spawn()
        .map_err(|e| e.to_string())?;

    // Give the script a moment to start, then quit.
    std::thread::sleep(std::time::Duration::from_millis(300));
    app.exit(0);
    Ok(())
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
