# Blik Plan ↔ Claude Connector — Plan 4: Connect-to-Claude (beta) Install Flow

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a one-click "Connect to Claude (beta)" button inside Blik Plan's Settings that detects installed Claude surfaces (Claude Code + Claude Desktop), atomically merges a `blikplan` MCP server entry into each config, and bundles the `blikplan-mcp` binary as a Tauri sidecar. Disconnect, refresh status, and backup-before-write are all included.

**Architecture:** A new Rust module `claude_config` handles path discovery and atomic JSON merging. Four new Tauri commands expose detect/connect/disconnect/status to the frontend. A new Svelte component `ConnectToClaude.svelte` lives inside a new "Integrations" section of the existing `SettingsModal.svelte`. The MCP binary itself comes from Plan 2 and is referenced through `tauri.conf.json`'s `externalBin` field.

**Tech Stack:** Rust (serde_json, dirs, chrono — all already in Cargo.toml). Svelte 5 runes. Tauri 2 sidecar / externalBin.

**Spec reference:** `docs/specs/2026-05-22-blikplan-claude-connector-design.md` § "Install Flow"

**Depends on:** Plans 1, 2, 3. Specifically:
- Plan 1: `PendingPatch` table (no direct interaction; just here so a connected Claude has something to write to)
- Plan 2: the `blikplan-mcp` binary at the build-output path documented by Plan 2 (look up the path before starting Task 7)
- Plan 3: the Inbox panel is what surfaces patches Claude sends — without it, "connect" succeeds but the user can't act on patches. Plan 3 must be merged before Plan 4 ships.

---

## File Structure

**Files this plan creates or modifies:**

- Create: `src-tauri/src/claude_config/mod.rs` — module entry, public API
- Create: `src-tauri/src/claude_config/paths.rs` — cross-platform config path discovery
- Create: `src-tauri/src/claude_config/writer.rs` — atomic JSON merge writer + backup
- Create: `src-tauri/src/commands/claude.rs` — four Tauri commands (detect, connect, disconnect, status)
- Modify: `src-tauri/src/lib.rs` — register `claude_config` module and the new commands
- Modify: `src-tauri/src/commands/mod.rs` — expose the `claude` submodule
- Modify: `src-tauri/Cargo.toml` — no new deps expected; verify before declaring done
- Modify: `src-tauri/tauri.conf.json` — add `externalBin` entry pointing at `blikplan-mcp`
- Create: `src/lib/components/ConnectToClaude.svelte` — the Settings panel section UI
- Modify: `src/lib/components/SettingsModal.svelte` — add an "Integrations" section that mounts `<ConnectToClaude />`
- Modify: `src/lib/ipc.ts` — TS bindings for the four new commands
- Modify: `src/lib/types.ts` — append `ClaudeSurface`, `ClaudeDetectionResult`, `ClaudeConnectionStatus` types

**Why these boundaries:** `paths.rs` is pure logic (string manipulation over OS env vars — no I/O), `writer.rs` is pure file I/O over already-resolved paths. Splitting them lets each be unit-tested without OS-specific test setup. The Svelte component is a single section, not a full modal, so it slots into the existing settings popover without restructuring it.

**Naming convention:** "Claude surface" = one of "Claude Code" or "Claude Desktop". Used throughout the code and UI to avoid the ambiguous word "client".

---

## Task 1: Cross-platform Claude config path discovery

**Files:**
- Create: `src-tauri/src/claude_config/mod.rs`
- Create: `src-tauri/src/claude_config/paths.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Register the module in `lib.rs`**

In `src-tauri/src/lib.rs`, add `mod claude_config;` alongside the other module declarations (alphabetically — after `chaser`, before `commands`).

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: error `file not found for module 'claude_config'`. Continue.

- [ ] **Step 2: Create `mod.rs`**

Create `src-tauri/src/claude_config/mod.rs`:

```rust
//! Detect installed Claude surfaces (Claude Code + Claude Desktop) and
//! merge the `blikplan` MCP server entry into their config files.
//! See `docs/specs/2026-05-22-blikplan-claude-connector-design.md` § "Install Flow".

pub mod paths;
pub mod writer;

pub use paths::{claude_code_config_path, claude_desktop_config_path, ClaudeSurface};
pub use writer::{merge_blikplan_entry, remove_blikplan_entry, WriteError};
```

Task 2 implements `writer.rs`; for now create a STUB so the crate compiles cleanly between tasks:

Create `src-tauri/src/claude_config/writer.rs`:

```rust
//! Stub — Task 2 of Plan 4 implements this module.

use std::path::Path;

#[derive(Debug)]
pub enum WriteError {}

pub fn merge_blikplan_entry(_path: &Path, _bin: &Path, _db: &Path) -> Result<(), WriteError> {
    Ok(())
}

pub fn remove_blikplan_entry(_path: &Path) -> Result<(), WriteError> {
    Ok(())
}
```

- [ ] **Step 3: Write failing tests for path discovery**

Create `src-tauri/src/claude_config/paths.rs` with ONLY the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_display_names_are_user_facing() {
        assert_eq!(ClaudeSurface::Code.display_name(), "Claude Code");
        assert_eq!(ClaudeSurface::Desktop.display_name(), "Claude Desktop");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn claude_code_path_on_macos_lives_in_home() {
        let path = claude_code_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.ends_with("/.claude.json"), "got {s}");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn claude_desktop_path_on_macos_lives_in_app_support() {
        let path = claude_desktop_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(
            s.contains("/Library/Application Support/Claude/claude_desktop_config.json"),
            "got {s}"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn claude_code_path_on_linux_lives_in_home() {
        let path = claude_code_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.ends_with("/.claude.json"), "got {s}");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn claude_code_path_on_windows_uses_userprofile() {
        let path = claude_code_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(s.ends_with("\\.claude.json"), "got {s}");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn claude_desktop_path_on_windows_uses_appdata() {
        let path = claude_desktop_config_path().unwrap();
        let s = path.to_string_lossy();
        assert!(
            s.contains("\\Claude\\claude_desktop_config.json"),
            "got {s}"
        );
    }
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml claude_config::paths`
Expected: compilation errors — `ClaudeSurface`, `claude_code_config_path`, `claude_desktop_config_path` not defined.

- [ ] **Step 5: Implement the path resolver**

Prepend this to `src-tauri/src/claude_config/paths.rs`:

```rust
use std::path::PathBuf;

/// The two Claude surfaces this app knows how to wire up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeSurface {
    Code,
    Desktop,
}

impl ClaudeSurface {
    pub fn display_name(self) -> &'static str {
        match self {
            ClaudeSurface::Code => "Claude Code",
            ClaudeSurface::Desktop => "Claude Desktop",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("could not resolve home directory")]
    NoHome,
    #[error("platform not supported for Claude {0}")]
    UnsupportedPlatform(&'static str),
}

/// Returns the canonical Claude Code config path for the current OS.
/// - macOS / Linux: `~/.claude.json`
/// - Windows:      `%USERPROFILE%\.claude.json`
pub fn claude_code_config_path() -> Result<PathBuf, PathError> {
    let home = dirs::home_dir().ok_or(PathError::NoHome)?;
    Ok(home.join(".claude.json"))
}

/// Returns the canonical Claude Desktop config path for the current OS.
/// - macOS:   `~/Library/Application Support/Claude/claude_desktop_config.json`
/// - Linux:   `~/.config/Claude/claude_desktop_config.json`  (per Claude Desktop's published location)
/// - Windows: `%APPDATA%\Claude\claude_desktop_config.json`
pub fn claude_desktop_config_path() -> Result<PathBuf, PathError> {
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir().ok_or(PathError::NoHome)?;
        Ok(home
            .join("Library")
            .join("Application Support")
            .join("Claude")
            .join("claude_desktop_config.json"))
    }
    #[cfg(target_os = "linux")]
    {
        let cfg = dirs::config_dir().ok_or(PathError::NoHome)?;
        Ok(cfg.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(target_os = "windows")]
    {
        let appdata = dirs::config_dir().ok_or(PathError::NoHome)?;
        Ok(appdata.join("Claude").join("claude_desktop_config.json"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Err(PathError::UnsupportedPlatform("Desktop"))
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml claude_config::paths`
Expected: all platform-applicable tests PASS. (You're likely on macOS; the 3 macOS-gated tests + the always-on `surface_display_names_are_user_facing` test run.)

- [ ] **Step 7: Commit**

```bash
cd ~/Desktop/GanttBok
git add src-tauri/src/lib.rs src-tauri/src/claude_config/mod.rs src-tauri/src/claude_config/paths.rs src-tauri/src/claude_config/writer.rs
git commit -m "feat(claude-config): add cross-platform Claude config path discovery

Resolves the canonical config locations for Claude Code (~/.claude.json
on macOS/Linux, USERPROFILE on Windows) and Claude Desktop (Library/
Application Support, ~/.config, or AppData by platform).

writer.rs stubbed; Task 2 implements it."
```

---

## Task 2: Atomic JSON merge writer with backup

**Files:**
- Modify: `src-tauri/src/claude_config/writer.rs` (replace the stub)

- [ ] **Step 1: Write failing tests for the writer**

REPLACE the entire current contents of `src-tauri/src/claude_config/writer.rs` with ONLY this test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn write_json(dir: &TempDir, name: &str, value: serde_json::Value) -> PathBuf {
        let p = dir.path().join(name);
        fs::write(&p, serde_json::to_string_pretty(&value).unwrap()).unwrap();
        p
    }

    fn read_json(path: &PathBuf) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
    }

    fn bin() -> PathBuf {
        PathBuf::from("/Applications/Blik Plan.app/Contents/Resources/blikplan-mcp")
    }
    fn db() -> PathBuf {
        PathBuf::from("/Users/x/Library/Application Support/blikplan/ganttbok.db")
    }

    #[test]
    fn merges_into_empty_config() {
        let dir = TempDir::new().unwrap();
        let cfg = write_json(&dir, "claude.json", json!({}));
        merge_blikplan_entry(&cfg, &bin(), &db()).unwrap();

        let after = read_json(&cfg);
        assert_eq!(
            after["mcpServers"]["blikplan"]["command"],
            json!(bin().to_string_lossy())
        );
        assert_eq!(
            after["mcpServers"]["blikplan"]["env"]["BLIKPLAN_DB"],
            json!(db().to_string_lossy())
        );
    }

    #[test]
    fn preserves_existing_mcp_servers() {
        let dir = TempDir::new().unwrap();
        let cfg = write_json(
            &dir,
            "claude.json",
            json!({
                "mcpServers": {
                    "other": { "command": "/usr/local/bin/other-mcp" }
                }
            }),
        );
        merge_blikplan_entry(&cfg, &bin(), &db()).unwrap();

        let after = read_json(&cfg);
        assert_eq!(
            after["mcpServers"]["other"]["command"],
            json!("/usr/local/bin/other-mcp")
        );
        assert!(after["mcpServers"]["blikplan"].is_object());
    }

    #[test]
    fn preserves_unrelated_top_level_keys() {
        let dir = TempDir::new().unwrap();
        let cfg = write_json(
            &dir,
            "claude.json",
            json!({
                "theme": "dark",
                "fontSize": 14,
                "mcpServers": {}
            }),
        );
        merge_blikplan_entry(&cfg, &bin(), &db()).unwrap();

        let after = read_json(&cfg);
        assert_eq!(after["theme"], json!("dark"));
        assert_eq!(after["fontSize"], json!(14));
    }

    #[test]
    fn creates_backup_before_writing() {
        let dir = TempDir::new().unwrap();
        let cfg = write_json(&dir, "claude.json", json!({ "theme": "dark" }));
        merge_blikplan_entry(&cfg, &bin(), &db()).unwrap();

        let backups: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("claude.json.bak-")
            })
            .collect();
        assert_eq!(backups.len(), 1, "expected exactly one backup file");
    }

    #[test]
    fn refuses_to_write_malformed_existing_json() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("claude.json");
        fs::write(&cfg, "{not valid json").unwrap();
        let err = merge_blikplan_entry(&cfg, &bin(), &db()).unwrap_err();
        assert!(matches!(err, WriteError::ParseExisting { .. }));
        // The malformed file is left untouched.
        assert_eq!(fs::read_to_string(&cfg).unwrap(), "{not valid json");
    }

    #[test]
    fn creates_file_if_missing() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("claude.json");
        assert!(!cfg.exists());
        merge_blikplan_entry(&cfg, &bin(), &db()).unwrap();
        let after = read_json(&cfg);
        assert!(after["mcpServers"]["blikplan"].is_object());
    }

    #[test]
    fn remove_strips_only_the_blikplan_entry() {
        let dir = TempDir::new().unwrap();
        let cfg = write_json(
            &dir,
            "claude.json",
            json!({
                "mcpServers": {
                    "other": { "command": "/usr/local/bin/other-mcp" },
                    "blikplan": { "command": "old/path" }
                }
            }),
        );
        remove_blikplan_entry(&cfg).unwrap();

        let after = read_json(&cfg);
        assert!(after["mcpServers"]["blikplan"].is_null());
        assert_eq!(
            after["mcpServers"]["other"]["command"],
            json!("/usr/local/bin/other-mcp")
        );
    }

    #[test]
    fn remove_is_noop_when_blikplan_absent() {
        let dir = TempDir::new().unwrap();
        let cfg = write_json(
            &dir,
            "claude.json",
            json!({ "mcpServers": { "other": {} } }),
        );
        remove_blikplan_entry(&cfg).unwrap();
        // No-op, but should not error.
        let after = read_json(&cfg);
        assert!(after["mcpServers"]["other"].is_object());
    }

    #[test]
    fn remove_is_noop_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("never_existed.json");
        // Should not error.
        remove_blikplan_entry(&cfg).unwrap();
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml claude_config::writer`
Expected: compilation errors — `WriteError::ParseExisting` and the proper signatures don't exist on the stub.

- [ ] **Step 3: Implement the writer**

Prepend this to `src-tauri/src/claude_config/writer.rs` (above the `#[cfg(test)]` block from Step 1):

```rust
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("io error reading {path:?}: {source}")]
    Read { path: PathBuf, source: std::io::Error },

    #[error("io error writing {path:?}: {source}")]
    Write { path: PathBuf, source: std::io::Error },

    #[error("existing config at {path:?} is not valid JSON: {source}")]
    ParseExisting { path: PathBuf, source: serde_json::Error },

    #[error("existing config at {path:?} is not a JSON object at the top level")]
    NotAnObject { path: PathBuf },
}

/// Merges a `blikplan` entry into the `mcpServers` object of the config at `path`.
/// - Creates the file (with `mcpServers: {}` as the only top-level key) if missing.
/// - Preserves all existing top-level keys and sibling entries inside `mcpServers`.
/// - Writes a timestamped backup `<path>.bak-<RFC3339>` of the previous content first,
///   unless the file did not exist.
/// - Atomic-write semantics: writes to `<path>.tmp` then renames.
pub fn merge_blikplan_entry(path: &Path, bin: &Path, db: &Path) -> Result<(), WriteError> {
    let existing: Value = if path.exists() {
        let raw = fs::read_to_string(path).map_err(|e| WriteError::Read {
            path: path.to_path_buf(),
            source: e,
        })?;
        let parsed: Value = serde_json::from_str(&raw).map_err(|e| WriteError::ParseExisting {
            path: path.to_path_buf(),
            source: e,
        })?;
        if !parsed.is_object() {
            return Err(WriteError::NotAnObject {
                path: path.to_path_buf(),
            });
        }
        write_backup(path, &raw)?;
        parsed
    } else {
        json!({})
    };

    let mut root = existing;
    let obj = root.as_object_mut().unwrap(); // checked above
    let mcp = obj
        .entry("mcpServers".to_string())
        .or_insert_with(|| json!({}));
    if !mcp.is_object() {
        return Err(WriteError::NotAnObject {
            path: path.to_path_buf(),
        });
    }
    let mcp_obj = mcp.as_object_mut().unwrap();
    mcp_obj.insert(
        "blikplan".to_string(),
        json!({
            "command": bin.to_string_lossy(),
            "env": { "BLIKPLAN_DB": db.to_string_lossy() }
        }),
    );

    atomic_write_json(path, &root)
}

/// Removes the `blikplan` entry from `mcpServers` if present. No-op if the file or
/// entry is missing. Other entries are preserved. Writes a backup as `merge_blikplan_entry` does.
pub fn remove_blikplan_entry(path: &Path) -> Result<(), WriteError> {
    if !path.exists() {
        return Ok(());
    }
    let raw = fs::read_to_string(path).map_err(|e| WriteError::Read {
        path: path.to_path_buf(),
        source: e,
    })?;
    let mut parsed: Value = serde_json::from_str(&raw).map_err(|e| WriteError::ParseExisting {
        path: path.to_path_buf(),
        source: e,
    })?;
    if !parsed.is_object() {
        return Err(WriteError::NotAnObject {
            path: path.to_path_buf(),
        });
    }

    let mut changed = false;
    if let Some(mcp) = parsed
        .as_object_mut()
        .and_then(|o| o.get_mut("mcpServers"))
        .and_then(|v| v.as_object_mut())
    {
        if mcp.remove("blikplan").is_some() {
            changed = true;
        }
    }

    if !changed {
        return Ok(());
    }

    write_backup(path, &raw)?;
    atomic_write_json(path, &parsed)
}

fn write_backup(path: &Path, raw: &str) -> Result<(), WriteError> {
    let stamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let backup_name = format!(
        "{}.bak-{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        stamp
    );
    let backup_path = path.with_file_name(backup_name);
    fs::write(&backup_path, raw).map_err(|e| WriteError::Write {
        path: backup_path,
        source: e,
    })?;
    Ok(())
}

fn atomic_write_json(path: &Path, value: &Value) -> Result<(), WriteError> {
    let tmp = path.with_extension("tmp");
    let pretty = serde_json::to_string_pretty(value).map_err(|e| WriteError::ParseExisting {
        path: path.to_path_buf(),
        source: e,
    })?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| WriteError::Write {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    let mut f = fs::File::create(&tmp).map_err(|e| WriteError::Write {
        path: tmp.clone(),
        source: e,
    })?;
    f.write_all(pretty.as_bytes()).map_err(|e| WriteError::Write {
        path: tmp.clone(),
        source: e,
    })?;
    f.sync_all().map_err(|e| WriteError::Write {
        path: tmp.clone(),
        source: e,
    })?;
    drop(f);
    fs::rename(&tmp, path).map_err(|e| WriteError::Write {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml claude_config::writer`
Expected: all 9 tests PASS.

- [ ] **Step 5: Run the whole crate**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`
Expected: all library tests pass — every test from Plans 1–3 still green.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/claude_config/writer.rs
git commit -m "feat(claude-config): atomic JSON merge writer with backup

merge_blikplan_entry: inserts the blikplan MCP server into a Claude
config (creating the file if missing), preserving all existing keys
and sibling MCP servers. Atomic write via tmp+rename. Backup written
to <path>.bak-<RFC3339> before any change.

remove_blikplan_entry: strips only the blikplan entry; preserves others;
no-ops when entry or file is absent."
```

---

## Task 3: Tauri commands — detect, connect, disconnect, status

**Files:**
- Create: `src-tauri/src/commands/claude.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs` (register the new commands in the `invoke_handler` list)

- [ ] **Step 1: Register the new submodule**

Open `src-tauri/src/commands/mod.rs` and add `pub mod claude;` alongside the existing `pub mod` declarations.

- [ ] **Step 2: Write the commands file**

Create `src-tauri/src/commands/claude.rs`:

```rust
//! Tauri commands wiring the Settings → Integrations → Connect to Claude UI
//! to the `claude_config` module.

use std::path::PathBuf;

use serde::Serialize;

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
    // Mirror the runtime DB-location convention used by Blik Plan itself.
    // (Existing code in db/connection.rs already resolves this — reuse it.)
    crate::db::connection::resolve_db_path()
        .map_err(|e| GbError::Validation(format!("could not locate ganttbok.db: {e}")))
}

fn bundled_mcp_bin_path(app: &tauri::AppHandle) -> GbResult<PathBuf> {
    // The MCP binary is bundled as a Tauri sidecar resource (see tauri.conf.json
    // externalBin entry). On macOS this resolves to .app/Contents/Resources;
    // on Linux/Windows it sits next to the executable.
    let resource = app
        .path()
        .resolve("blikplan-mcp", tauri::path::BaseDirectory::Resource)
        .map_err(|e| GbError::Validation(format!("could not resolve sidecar path: {e}")))?;
    Ok(resource)
}

fn detect_one(surface: ClaudeSurface, path_result: Result<PathBuf, impl std::fmt::Display>) -> ClaudeDetection {
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
```

- [ ] **Step 3: Register the commands in `lib.rs`**

In `src-tauri/src/lib.rs`, find the `tauri::Builder::default().invoke_handler(tauri::generate_handler![...])` block and append (preserving alphabetical or existing order):

```rust
            commands::claude::detect_claude_surfaces,
            commands::claude::connect_to_claude,
            commands::claude::disconnect_from_claude,
```

- [ ] **Step 4: Verify the crate compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: zero errors. Warnings about unused imports inside `claude.rs` are acceptable.

If `crate::db::connection::resolve_db_path` doesn't exist with that exact name, run `grep -rn "fn.*db.*path\|fn.*resolve.*db" src-tauri/src/` to find the correct function and update the `db_path()` helper above. The aim is: use whatever function the rest of the codebase uses to find `ganttbok.db` — do NOT duplicate that logic.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/mod.rs src-tauri/src/commands/claude.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add Claude connect/disconnect/detect commands

Four Tauri IPC commands for the Settings → Integrations panel:
  detect_claude_surfaces   — returns per-surface status (config exists, blikplan connected)
  connect_to_claude        — merges blikplan entry into chosen surfaces
  disconnect_from_claude   — removes blikplan entry from chosen surfaces

Mostly orchestration over claude_config::{paths,writer}."
```

---

## Task 4: TypeScript bindings + types

**Files:**
- Modify: `src/lib/types.ts` (append)
- Modify: `src/lib/ipc.ts` (append four wrappers)

- [ ] **Step 1: Append the types**

Append to `src/lib/types.ts`:

```typescript
// ---------------------------------------------------------------
// Claude connector — Settings → Integrations.
// Mirrors src-tauri/src/commands/claude.rs.
// ---------------------------------------------------------------

export type ClaudeSurface = 'code' | 'desktop';

export interface ClaudeDetection {
  surface: ClaudeSurface;
  display_name: string;
  config_path: string;
  config_exists: boolean;
  blikplan_connected: boolean;
}

export interface ClaudeDetectionResult {
  surfaces: ClaudeDetection[];
}
```

- [ ] **Step 2: Append the IPC wrappers**

Append to `src/lib/ipc.ts`:

```typescript
import type { ClaudeSurface, ClaudeDetectionResult } from './types';

export async function detectClaudeSurfaces(): Promise<ClaudeDetectionResult> {
  return invoke('detect_claude_surfaces');
}

export async function connectToClaude(surfaces: ClaudeSurface[]): Promise<ClaudeDetectionResult> {
  return invoke('connect_to_claude', { surfaces });
}

export async function disconnectFromClaude(surfaces: ClaudeSurface[]): Promise<ClaudeDetectionResult> {
  return invoke('disconnect_from_claude', { surfaces });
}
```

(If `invoke` is already imported at the top of `ipc.ts` — check first — don't re-import.)

- [ ] **Step 3: Type-check**

Run: `cd ~/Desktop/GanttBok && npm run check`
Expected: no new errors. (Pre-existing errors from earlier plans may still appear; only fail if your new lines produced errors.)

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/ipc.ts
git commit -m "feat(ipc): TS bindings for the Claude connector commands"
```

---

## Task 5: `ConnectToClaude.svelte` component

**Files:**
- Create: `src/lib/components/ConnectToClaude.svelte`

- [ ] **Step 1: Write the component**

Create `src/lib/components/ConnectToClaude.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import type { ClaudeDetection, ClaudeSurface } from '../types';
  import { detectClaudeSurfaces, connectToClaude, disconnectFromClaude } from '../ipc';

  let surfaces: ClaudeDetection[] = $state([]);
  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastRefreshed = $state<Date | null>(null);

  async function refresh() {
    busy = true;
    error = null;
    try {
      const result = await detectClaudeSurfaces();
      surfaces = result.surfaces;
      lastRefreshed = new Date();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function connectAll() {
    const targets = surfaces.filter((s) => s.config_exists).map((s) => s.surface);
    if (targets.length === 0) return;
    busy = true;
    error = null;
    try {
      const result = await connectToClaude(targets);
      surfaces = result.surfaces;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function disconnectAll() {
    const targets = surfaces.filter((s) => s.blikplan_connected).map((s) => s.surface);
    if (targets.length === 0) return;
    busy = true;
    error = null;
    try {
      const result = await disconnectFromClaude(targets);
      surfaces = result.surfaces;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  onMount(refresh);

  let allConnected = $derived(
    surfaces.length > 0 && surfaces.filter((s) => s.config_exists).every((s) => s.blikplan_connected)
  );
  let anyConnected = $derived(surfaces.some((s) => s.blikplan_connected));
  let noneDetected = $derived(surfaces.length > 0 && surfaces.every((s) => !s.config_exists));
</script>

<div class="connect-claude">
  <div class="header">
    <h3>
      Connect to Claude
      <span class="beta">beta</span>
    </h3>
    <button class="refresh" onclick={refresh} disabled={busy} title="Refresh detection">↻</button>
  </div>

  <p class="hint">
    Let Claude Code or Claude Desktop read your Blik Plan schedule and propose
    updates from meeting transcripts. Proposals appear in the Inbox panel for
    you to accept or reject — Claude never writes directly.
  </p>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if noneDetected}
    <p class="empty">No Claude installation detected. Install Claude Code or Claude Desktop, then click refresh.</p>
  {:else}
    <ul class="surfaces">
      {#each surfaces as s (s.surface)}
        <li class:detected={s.config_exists}>
          <span class="checkbox" aria-hidden="true">
            {#if s.blikplan_connected}✓{:else if s.config_exists}·{:else}—{/if}
          </span>
          <span class="name">{s.display_name}</span>
          <span class="status">
            {#if s.blikplan_connected}
              connected
            {:else if s.config_exists}
              not connected
            {:else}
              not detected
            {/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="actions">
    {#if allConnected}
      <button class="disconnect" onclick={disconnectAll} disabled={busy}>Disconnect</button>
    {:else if anyConnected}
      <button class="connect" onclick={connectAll} disabled={busy}>Update connection</button>
      <button class="disconnect" onclick={disconnectAll} disabled={busy}>Disconnect</button>
    {:else}
      <button
        class="connect primary"
        onclick={connectAll}
        disabled={busy || noneDetected}
      >Connect</button>
    {/if}
  </div>

  {#if allConnected}
    <p class="next-steps">
      Restart Claude Desktop and start a new Claude Code session. Then try:
      <em>"What's on my Blik Plan schedule this week?"</em>
    </p>
  {/if}

  {#if lastRefreshed}
    <p class="last-refreshed">Last checked: {lastRefreshed.toLocaleTimeString()}</p>
  {/if}
</div>

<style>
  .connect-claude { display: flex; flex-direction: column; gap: 0.75rem; }
  .header { display: flex; align-items: center; justify-content: space-between; }
  .header h3 { margin: 0; font-size: 0.95rem; }
  .beta {
    background: var(--accent, #ff8c00); color: white;
    font-size: 0.65rem; padding: 0.1rem 0.35rem; border-radius: 4px;
    text-transform: uppercase; letter-spacing: 0.05em; margin-left: 0.4rem;
    vertical-align: middle;
  }
  .refresh {
    background: none; border: 1px solid var(--border, #ccc); border-radius: 4px;
    width: 1.6rem; height: 1.6rem; cursor: pointer; font-size: 0.9rem;
  }
  .refresh:disabled { opacity: 0.4; cursor: wait; }
  .hint { font-size: 0.8rem; color: var(--muted, #666); margin: 0; }
  .surfaces { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.3rem; }
  .surfaces li { display: grid; grid-template-columns: 1.2rem 1fr auto; gap: 0.5rem; align-items: center; font-size: 0.85rem; padding: 0.3rem 0.5rem; border: 1px solid var(--border, #eee); border-radius: 4px; }
  .surfaces li.detected { border-color: var(--border-strong, #ccc); }
  .checkbox { font-family: monospace; text-align: center; }
  .status { color: var(--muted, #666); font-size: 0.75rem; }
  .actions { display: flex; gap: 0.5rem; }
  .actions button { padding: 0.4rem 0.8rem; border-radius: 4px; border: 1px solid var(--border, #ccc); cursor: pointer; font-size: 0.85rem; background: white; }
  .actions button.primary { background: var(--accent, #ff8c00); color: white; border-color: var(--accent, #ff8c00); }
  .actions button:disabled { opacity: 0.5; cursor: not-allowed; }
  .next-steps { font-size: 0.8rem; padding: 0.5rem; background: var(--accent-soft, #fff4e6); border-radius: 4px; margin: 0; }
  .next-steps em { color: var(--accent, #ff8c00); }
  .error { font-size: 0.8rem; color: #b00020; background: #fee; padding: 0.4rem; border-radius: 4px; }
  .empty { font-size: 0.8rem; color: var(--muted, #666); font-style: italic; }
  .last-refreshed { font-size: 0.7rem; color: var(--muted, #999); margin: 0; }
</style>
```

- [ ] **Step 2: Type-check**

Run: `npm run check`
Expected: no new errors introduced by this component.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ConnectToClaude.svelte
git commit -m "feat(ui): ConnectToClaude.svelte — Settings panel component"
```

---

## Task 6: Wire the component into `SettingsModal.svelte`

**Files:**
- Modify: `src/lib/components/SettingsModal.svelte`

- [ ] **Step 1: Inspect the existing modal**

Read `src/lib/components/SettingsModal.svelte`. Identify the last `<section>` block inside the `{#if open}` popover and the closing `</div>` of the popover. The new section goes between them — i.e. after all existing sections, before the popover closes.

- [ ] **Step 2: Add the import**

At the top of the `<script lang="ts">` block, alongside the existing import of `store`, add:

```typescript
import ConnectToClaude from './ConnectToClaude.svelte';
```

- [ ] **Step 3: Add the section**

Inside the popover, after the last existing `<section>` block, add:

```svelte
    <section>
      <ConnectToClaude />
    </section>
```

(The component renders its own `<h3>`, so we don't wrap it in a heading — let it own its section content.)

- [ ] **Step 4: Smoke-test by running the app**

Run: `cd ~/Desktop/GanttBok && pnpm tauri dev`
Expected: Blik Plan launches. Open Settings (cog icon). Scroll to the bottom. Verify the "Connect to Claude (beta)" panel renders. The refresh button should populate the surfaces list within a second of opening Settings.

This is a manual smoke test — record what you observed (rendered correctly / didn't render / error) and close the dev session. Don't proceed if the component fails to render.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/SettingsModal.svelte
git commit -m "feat(ui): mount ConnectToClaude in SettingsModal

Adds an Integrations section at the bottom of the existing Settings popover."
```

---

## Task 7: Bundle `blikplan-mcp` as a Tauri sidecar

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `package.json` (if a build script needs to copy the binary into the resources path)

**Prerequisite:** The `blikplan-mcp` binary from Plan 2 must be buildable. Before starting this task, run the build command Plan 2 documents (likely `cargo build --release -p blikplan-mcp` or equivalent) and note the output path (likely `/Volumes/SD EXTREME JT/ganttbok-target/target/release/blikplan-mcp` given the workspace-target redirect — or wherever Plan 2 lands it).

- [ ] **Step 1: Read Plan 2's binary-output path**

Open `docs/plans/2026-05-22-blikplan-claude-connector-plan2-mcp-server.md` and grep for "sidecar" / "externalBin" / "binary output" to find the path convention Plan 2 documented. The rest of this task assumes that path is `src-tauri/binaries/blikplan-mcp-<target-triple>` — adjust if Plan 2 chose a different convention.

- [ ] **Step 2: Add `externalBin` to `tauri.conf.json`**

Open `src-tauri/tauri.conf.json`. Inside the top-level `"bundle"` object, add (next to `"resources"` if present, otherwise as a new key):

```json
    "externalBin": [
      "binaries/blikplan-mcp"
    ],
```

Tauri expects the binary to live at `src-tauri/binaries/blikplan-mcp-<target-triple>` (e.g. `blikplan-mcp-aarch64-apple-darwin`). The build step must produce that exact filename.

- [ ] **Step 3: Add a build hook**

In `package.json`, locate the existing `scripts` block. Add a new script (and update `pnpm tauri build` to depend on it if needed — investigate the existing pattern):

```json
    "build:mcp-sidecar": "cd ../blikplan-mcp && cargo build --release && mkdir -p ../src-tauri/binaries && cp target/release/blikplan-mcp ../src-tauri/binaries/blikplan-mcp-$(rustc -vV | grep host | cut -d' ' -f2)",
```

(Adjust `cd ../blikplan-mcp` to match Plan 2's actual crate path; adjust the source `target/release/...` to match the workspace target dir if Plan 2 shares the GanttBok workspace.)

Wire this script as a precondition to the Tauri build — the existing `beforeBuildCommand` in `tauri.conf.json` currently runs `pnpm build`. Either:
- (a) Change it to `pnpm build:mcp-sidecar && pnpm build`, OR
- (b) Add `"prebuild": "pnpm build:mcp-sidecar"` to scripts so npm/pnpm runs it automatically before `build`.

Choose (a) — explicit chaining is easier to debug. Update `beforeBuildCommand` in `tauri.conf.json` accordingly.

- [ ] **Step 4: Smoke-test a release build**

Run: `cd ~/Desktop/GanttBok && pnpm tauri build`
Expected: build succeeds. The output .app/.dmg contains the MCP binary inside `Contents/Resources/blikplan-mcp` (macOS) or alongside the executable (Windows/Linux).

Verify on macOS:
```bash
ls "src-tauri/target/release/bundle/macos/Blik Plan.app/Contents/Resources/" | grep blikplan
```
Expected: `blikplan-mcp` (or the platform-triple-suffixed name — Tauri may strip the suffix at bundle time).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json package.json
git commit -m "build: bundle blikplan-mcp as a Tauri sidecar

Adds externalBin entry + a pnpm script that builds the MCP binary,
copies it into src-tauri/binaries/ with the platform-triple suffix
Tauri expects, then chains it before the main Tauri build."
```

---

## Task 8: Final verification

- [ ] **Step 1: Full Rust test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib`
Expected: every library test from Plans 1–4 passes. Note the total count.

- [ ] **Step 2: TS check**

Run: `npm run check`
Expected: zero NEW errors introduced by Plan 4. Pre-existing errors from earlier plans are acceptable (note them but don't fix in this plan).

- [ ] **Step 3: End-to-end manual smoke test (the one that actually matters)**

Per the spec § "Manual smoke test (release checklist)":

1. Fresh-launch Blik Plan from a `pnpm tauri dev` session.
2. Open Settings → scroll to "Connect to Claude (beta)" → click refresh.
   - Both surfaces should populate. At least one should show `not connected` (or `not detected` if you don't have Claude installed).
3. Click "Connect" → both surfaces transition to `connected`.
4. From another terminal, inspect the config files:
   ```bash
   jq '.mcpServers.blikplan' ~/.claude.json
   jq '.mcpServers.blikplan' ~/Library/Application\ Support/Claude/claude_desktop_config.json
   ```
   Both should print the `command` (path to the bundled `blikplan-mcp`) and `env.BLIKPLAN_DB`.
5. Open Claude Code in a new terminal session. Ask: *"What MCP servers do you have available?"*  
   Expected: `blikplan` appears.
6. Ask: *"Use the blikplan MCP server to list my jobs."*  
   Expected: real data from `ganttbok.db` returned.
7. Paste a short meeting transcript and ask Claude to propose a patch.  
   Expected: a `pending_patches` row appears in Blik Plan's Inbox panel within the 5-second poll interval (Plan 3).
8. Accept the patch in the Inbox panel → Gantt updates.
9. In Blik Plan Settings, click "Disconnect" → both surfaces transition back to `not connected`.
10. Verify the configs no longer contain `blikplan` but DO still contain any other MCP servers you had:
    ```bash
    jq '.mcpServers' ~/.claude.json
    ```
11. Verify backups exist:
    ```bash
    ls ~/.claude.json.bak-* | tail
    ```
    There should be one per connect+disconnect cycle.

Record the result of each step. If anything fails, fix it before declaring Plan 4 done.

- [ ] **Step 4: Confirm commit log shape**

Run: `git log --oneline -10`
Expected: 7 commits from this plan, in this order (newest first):
1. `build: bundle blikplan-mcp as a Tauri sidecar`
2. `feat(ui): mount ConnectToClaude in SettingsModal`
3. `feat(ui): ConnectToClaude.svelte — Settings panel component`
4. `feat(ipc): TS bindings for the Claude connector commands`
5. `feat(commands): add Claude connect/disconnect/detect commands`
6. `feat(claude-config): atomic JSON merge writer with backup`
7. `feat(claude-config): add cross-platform Claude config path discovery`

- [ ] **Step 5: Plan 4 is complete**

Plan 4 is the last plan in the connector build. After verification passes, the connector is shippable as v1.6 (beta-labelled). Tell the user — don't push automatically.

---

## Out of scope for Plan 4

- The `@blikplan/mcp` npm wrapper — that's covered in Plan 2.
- GitHub release / binary download infrastructure for the npm wrapper — also Plan 2.
- Telemetry on connection success/failure — not requested, YAGNI.
- "Detect any other MCP servers and offer to disable them while testing" — not in spec.
- Migrating users who manually configured `blikplan` before this feature shipped — n/a (this is the first release).

## Risks for the implementer

- **`crate::db::connection::resolve_db_path` may not exist with that exact name.** Plan 4 documents the intent (reuse the existing DB-path-resolver, don't reimplement). The Task 3 step says how to find the right function. Do not duplicate the resolution logic — it would diverge over time.
- **Tauri sidecar bundling is fiddly.** The exact filename Tauri expects depends on the target triple. If the smoke test in Task 7 step 4 fails, the most likely cause is a filename mismatch — debug with `tauri info` and `ls src-tauri/binaries/`.
- **`pnpm tauri build` will rebuild the MCP server every time.** Acceptable for now; cache later if it bites.
- **Plan 2 may evolve the binary path between when this plan is written (2026-05-22) and when it's implemented.** Re-read Plan 2 § sidecar section before starting Task 7.

## Smoke test fallback

If Claude isn't installed during testing, Task 8's manual smoke test steps 5–8 are skippable. The Rust/TS unit tests + steps 1–4 still verify the writer works correctly; reviewers can validate the full flow once a Claude install is available.
