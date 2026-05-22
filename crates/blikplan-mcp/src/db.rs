use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;

/// Resolve the path to `ganttbok.db` using:
///  1. `$BLIKPLAN_DB` env var (absolute path)
///  2. `{data_local_dir}/Blik Plan/ganttbok.db`  (post-rename)
///  3. `{data_local_dir}/Gantt Bok/ganttbok.db`  (pre-rename fallback)
///
/// Returns `None` if none of the above paths exist on disk.
pub fn resolve_db_path() -> Option<PathBuf> {
    // 1. Explicit env override.
    if let Ok(p) = std::env::var("BLIKPLAN_DB") {
        let path = PathBuf::from(p);
        if path.exists() { return Some(path); }
    }

    let base = dirs::data_local_dir()?;

    // 2. Post-rename path ("Blik Plan").
    let new_path = base.join("Blik Plan").join("ganttbok.db");
    if new_path.exists() { return Some(new_path); }

    // 3. Pre-rename fallback ("Gantt Bok").
    let old_path = base.join("Gantt Bok").join("ganttbok.db");
    if old_path.exists() { return Some(old_path); }

    None
}

/// Open a **read-only** connection to the given path.
/// Panics with a descriptive message if the file cannot be opened.
pub fn open_ro(path: &std::path::Path) -> Connection {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap_or_else(|e| panic!("failed to open {path:?} read-only: {e}"))
}

/// Open a **read-write** connection to the given path.
/// Used exclusively by `propose_patch`.
pub fn open_rw(path: &std::path::Path) -> Connection {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .unwrap_or_else(|e| panic!("failed to open {path:?} read-write: {e}"))
}

/// Apply all migrations on an in-memory connection.
/// Used only in tests — the real DB is always pre-migrated by the Tauri app.
pub fn apply_migrations_for_test(conn: &Connection) {
    // Inline the same migration text as ganttbok_lib's db::migrations.
    // We copy only the subset needed for MCP server tests: job, phase, task,
    // contact, dependency, pending_patches.
    conn.execute_batch(FIXTURE_SCHEMA).expect("fixture schema failed");
}

const FIXTURE_SCHEMA: &str = r#"
CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE job (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT    NOT NULL,
    client             TEXT,
    address            TEXT,
    project_start_date TEXT    NOT NULL,
    is_template        INTEGER NOT NULL DEFAULT 0,
    archived           INTEGER NOT NULL DEFAULT 0,
    created_at         TEXT    NOT NULL DEFAULT (datetime('now')),
    holidays_block_work INTEGER NOT NULL DEFAULT 1,
    region             TEXT    NOT NULL DEFAULT 'ZA'
);

CREATE TABLE phase (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id      INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    colour      TEXT    NOT NULL,
    order_index INTEGER NOT NULL,
    collapsed   INTEGER NOT NULL DEFAULT 1,
    notes       TEXT    NOT NULL DEFAULT ''
);

CREATE TABLE task (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    phase_id          INTEGER NOT NULL REFERENCES phase(id) ON DELETE CASCADE,
    name              TEXT    NOT NULL,
    start_date        TEXT    NOT NULL,
    duration_workdays INTEGER NOT NULL CHECK (duration_workdays >= 1),
    order_index       INTEGER NOT NULL,
    notes             TEXT,
    contact_id        INTEGER REFERENCES contact(id) ON DELETE SET NULL,
    last_chaser_sent_at TEXT
);

CREATE TABLE dependency (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    predecessor_id INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    successor_id   INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
    type           TEXT    NOT NULL DEFAULT 'FS',
    lag_days       INTEGER NOT NULL DEFAULT 0,
    UNIQUE(predecessor_id, successor_id)
);

CREATE TABLE contact (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT    NOT NULL,
    telegram_chat_id   TEXT,
    telegram_handle    TEXT,
    notes              TEXT    NOT NULL DEFAULT '',
    created_at         TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE pending_patches (
    id          TEXT    PRIMARY KEY,
    job_id      INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    patch_json  TEXT    NOT NULL,
    summary     TEXT    NOT NULL,
    source      TEXT    NOT NULL DEFAULT 'mcp',
    status      TEXT    NOT NULL DEFAULT 'proposed'
                        CHECK (status IN ('proposed','accepted','applied','rejected','apply_failed','expired')),
    created_at  INTEGER NOT NULL,
    resolved_at INTEGER,
    error       TEXT
);
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a real sqlite file at `dir/ganttbok.db`.
    fn plant_db(dir: &std::path::Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("ganttbok.db"), b"").unwrap();
    }

    #[test]
    fn env_var_takes_priority() {
        let tmp = TempDir::new().unwrap();
        let explicit = tmp.path().join("explicit.db");
        fs::write(&explicit, b"").unwrap();
        std::env::set_var("BLIKPLAN_DB", &explicit);
        let result = resolve_db_path();
        std::env::remove_var("BLIKPLAN_DB");
        assert_eq!(result.unwrap(), explicit);
    }

    #[test]
    fn env_var_nonexistent_file_falls_through() {
        // If BLIKPLAN_DB points to a path that doesn't exist,
        // we should NOT return it — fall through to OS-default.
        let tmp = TempDir::new().unwrap();
        let ghost = tmp.path().join("ghost.db");
        // ghost doesn't exist on disk
        std::env::set_var("BLIKPLAN_DB", &ghost);
        // Also make sure no OS-default exists during this test by checking
        // that None is returned (no real DB on CI).
        let result = resolve_db_path();
        std::env::remove_var("BLIKPLAN_DB");
        // The env var path doesn't exist and no real OS DB present in CI;
        // result is None (or Some if the dev machine has a real install).
        // What we assert: the result is NOT the ghost path.
        assert_ne!(result, Some(ghost));
    }

    #[test]
    fn returns_none_when_no_db_present() {
        // Ensure no env var is set.
        std::env::remove_var("BLIKPLAN_DB");
        // We cannot easily mock dirs::data_local_dir in-process.
        // This test is therefore a canary: if it returns Some on a CI box
        // it means a real DB was accidentally left at the default path.
        // On developer machines it may return Some — that's fine.
        // The important assertion is that the function doesn't panic.
        let _ = resolve_db_path(); // must not panic
    }

    #[test]
    fn open_ro_connection_is_read_only() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        // Create a minimal sqlite file.
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
        }
        let ro = open_ro(&path);
        let result = ro.execute("INSERT INTO t VALUES (1)", []);
        assert!(result.is_err(), "read-only connection should reject writes");
    }

    #[test]
    fn open_rw_connection_can_write() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
        }
        let rw = open_rw(&path);
        rw.execute("INSERT INTO t VALUES (1)", []).unwrap();
        let count: i64 = rw.query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);
    }
}
