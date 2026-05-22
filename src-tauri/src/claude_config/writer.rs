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
