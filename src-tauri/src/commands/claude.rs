//! Tauri commands wiring the Settings → Integrations → Connect to Claude UI
//! to the `claude_config` module.

use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;

use crate::claude_config::{
    claude_code_config_path, claude_desktop_config_path,
    merge_blikplan_entry, remove_blikplan_entry,
    ClaudeSurface, WriteError,
};
use crate::{GbError, GbResult};

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDetection {
    pub surface: ClaudeSurface,
    pub display_name: String,
    pub config_path: String,
    /// True if the config file exists. We don't require it to exist — connect
    /// can create it — but the UI uses this to label the surface as "detected".
    pub config_exists: bool,
    /// True if our `blikplan` entry is currently present in the file.
    pub blikplan_connected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClaudeDetectionResult {
    pub surfaces: Vec<ClaudeDetection>,
}

fn db_path() -> GbResult<PathBuf> {
    // Reuse the same DB-location convention as the rest of the app
    // (defined in `crate::db_path`). Do NOT duplicate that logic here.
    Ok(crate::db_path())
}

fn bundled_mcp_bin_path(_app: &tauri::AppHandle) -> GbResult<PathBuf> {
    // Tauri externalBin sidecars sit next to the main executable: on macOS that's
    // .app/Contents/MacOS/blikplan-mcp (NOT Contents/Resources/). Resolve relative
    // to the running exe so this works whether the user installed via DMG, ran
    // via `cargo run`, etc.
    let exe = std::env::current_exe()
        .map_err(|e| GbError::Validation(format!("could not find current exe: {e}")))?;
    let dir = exe
        .parent()
        .ok_or_else(|| GbError::Validation("current exe has no parent dir".into()))?;
    Ok(dir.join("blikplan-mcp"))
}

fn detect_one(
    surface: ClaudeSurface,
    path_result: Result<PathBuf, impl std::fmt::Display>,
) -> ClaudeDetection {
    match path_result {
        Ok(path) => {
            let exists = path.exists();
            let blikplan_connected = exists
                && std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
                    .map(|v| v["mcpServers"]["blikplan"].is_object())
                    .unwrap_or(false);
            ClaudeDetection {
                surface,
                display_name: surface.display_name().to_string(),
                config_path: path.to_string_lossy().into_owned(),
                config_exists: exists,
                blikplan_connected,
            }
        }
        Err(_) => ClaudeDetection {
            surface,
            display_name: surface.display_name().to_string(),
            config_path: String::new(),
            config_exists: false,
            blikplan_connected: false,
        },
    }
}

#[tauri::command]
pub fn detect_claude_surfaces() -> GbResult<ClaudeDetectionResult> {
    let code = detect_one(ClaudeSurface::Code, claude_code_config_path());
    let desktop = detect_one(ClaudeSurface::Desktop, claude_desktop_config_path());
    Ok(ClaudeDetectionResult {
        surfaces: vec![code, desktop],
    })
}

#[tauri::command]
pub fn connect_to_claude(
    app: tauri::AppHandle,
    surfaces: Vec<ClaudeSurface>,
) -> GbResult<ClaudeDetectionResult> {
    let bin = bundled_mcp_bin_path(&app)?;
    let db = db_path()?;
    for surface in &surfaces {
        let path = match surface {
            ClaudeSurface::Code => claude_code_config_path(),
            ClaudeSurface::Desktop => claude_desktop_config_path(),
        }
        .map_err(|e| GbError::Validation(format!("path resolution: {e}")))?;
        merge_blikplan_entry(&path, &bin, &db)
            .map_err(|e: WriteError| GbError::Validation(format!("write {path:?}: {e}")))?;
    }
    detect_claude_surfaces()
}

#[tauri::command]
pub fn disconnect_from_claude(
    surfaces: Vec<ClaudeSurface>,
) -> GbResult<ClaudeDetectionResult> {
    for surface in &surfaces {
        let path = match surface {
            ClaudeSurface::Code => claude_code_config_path(),
            ClaudeSurface::Desktop => claude_desktop_config_path(),
        }
        .map_err(|e| GbError::Validation(format!("path resolution: {e}")))?;
        remove_blikplan_entry(&path)
            .map_err(|e: WriteError| GbError::Validation(format!("write {path:?}: {e}")))?;
    }
    detect_claude_surfaces()
}
