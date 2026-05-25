use rusqlite::{Connection, params};
use crate::{GbError, GbResult};

const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE app_meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE job (
        id                 INTEGER PRIMARY KEY AUTOINCREMENT,
        name               TEXT    NOT NULL,
        client             TEXT,
        address            TEXT,
        project_start_date TEXT    NOT NULL,
        is_template        INTEGER NOT NULL DEFAULT 0,
        archived           INTEGER NOT NULL DEFAULT 0,
        created_at         TEXT    NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE phase (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id      INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
        name        TEXT    NOT NULL,
        colour      TEXT    NOT NULL,
        order_index INTEGER NOT NULL,
        collapsed   INTEGER NOT NULL DEFAULT 1
    );
    CREATE INDEX idx_phase_job ON phase(job_id, order_index);

    CREATE TABLE task (
        id                INTEGER PRIMARY KEY AUTOINCREMENT,
        phase_id          INTEGER NOT NULL REFERENCES phase(id) ON DELETE CASCADE,
        name              TEXT    NOT NULL,
        start_date        TEXT    NOT NULL,
        duration_workdays INTEGER NOT NULL CHECK (duration_workdays >= 1),
        order_index       INTEGER NOT NULL,
        notes             TEXT
    );
    CREATE INDEX idx_task_phase ON task(phase_id, order_index);

    CREATE TABLE dependency (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        predecessor_id INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
        successor_id   INTEGER NOT NULL REFERENCES task(id) ON DELETE CASCADE,
        type           TEXT    NOT NULL DEFAULT 'FS',
        lag_days       INTEGER NOT NULL DEFAULT 0,
        UNIQUE(predecessor_id, successor_id)
    );
    CREATE INDEX idx_dep_pred ON dependency(predecessor_id);
    CREATE INDEX idx_dep_succ ON dependency(successor_id);

    CREATE TABLE no_work_day (
        id      INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id  INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
        date    TEXT    NOT NULL,
        reason  TEXT    NOT NULL,
        source  TEXT    NOT NULL CHECK (source IN ('sa_public_holiday','manual')),
        UNIQUE(job_id, date)
    );
    "#,
    // v2 — per-job holiday-split toggle. 1 = SA holidays split bars (current default).
    "ALTER TABLE job ADD COLUMN holidays_block_work INTEGER NOT NULL DEFAULT 1;",
    // v3 — per-phase free-text notes (drives the todo-list side panel).
    "ALTER TABLE phase ADD COLUMN notes TEXT NOT NULL DEFAULT '';",
    // v4 — per-job region (drives which set of public holidays to sync).
    "ALTER TABLE job ADD COLUMN region TEXT NOT NULL DEFAULT 'ZA';",
    // v5 — broaden no_work_day.source CHECK to include all 5 regions.
    // SQLite can't ALTER a CHECK constraint; rename + recreate + copy.
    r#"
    ALTER TABLE no_work_day RENAME TO no_work_day_old;

    CREATE TABLE no_work_day (
        id      INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id  INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
        date    TEXT    NOT NULL,
        reason  TEXT    NOT NULL,
        source  TEXT    NOT NULL CHECK (source IN (
            'za_holiday', 'us_holiday', 'gb_holiday', 'in_holiday', 'cn_holiday',
            'sa_public_holiday', -- legacy alias from v1
            'manual'
        )),
        UNIQUE(job_id, date)
    );

    INSERT INTO no_work_day (id, job_id, date, reason, source)
        SELECT id, job_id, date, reason,
            CASE WHEN source = 'sa_public_holiday' THEN 'za_holiday' ELSE source END
        FROM no_work_day_old;

    DROP TABLE no_work_day_old;
    "#,
    // v6 — Chaser feature: contacts + per-task contact + per-task last-chaser-sent timestamp.
    r#"
    CREATE TABLE contact (
        id                  INTEGER PRIMARY KEY AUTOINCREMENT,
        name                TEXT    NOT NULL,
        telegram_chat_id    TEXT,
        telegram_handle     TEXT,
        notes               TEXT    NOT NULL DEFAULT '',
        created_at          TEXT    NOT NULL DEFAULT (datetime('now'))
    );

    ALTER TABLE task ADD COLUMN contact_id INTEGER REFERENCES contact(id) ON DELETE SET NULL;
    ALTER TABLE task ADD COLUMN last_chaser_sent_at TEXT;
    "#,
    // v7 — pending_patches queue for proposals coming from external sources (MCP, webhooks).
    // Status lifecycle: proposed → accepted → applied  (or proposed → rejected/expired,
    //                                                   or accepted → apply_failed).
    r#"
    CREATE TABLE pending_patches (
        id            TEXT    PRIMARY KEY,
        job_id        INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
        patch_json    TEXT    NOT NULL,
        summary       TEXT    NOT NULL,
        source        TEXT    NOT NULL DEFAULT 'mcp',
        status        TEXT    NOT NULL DEFAULT 'proposed'
                              CHECK (status IN ('proposed','accepted','applied','rejected','apply_failed','expired')),
        created_at    INTEGER NOT NULL,
        resolved_at   INTEGER,
        error         TEXT
    );
    CREATE INDEX idx_pending_patches_status_created
        ON pending_patches(status, created_at);
    "#,
    // v8 — interactive task status (v1.7).
    // Adds:
    //   task.status            — 'not_started' | 'on_track' | 'done' | 'late' (default 'on_track')
    //   task.completion_date   — nullable YYYY-MM-DD string, set when status = 'done'
    //   job.auto_shift_dependents — 1 by default; if 0, ripple is skipped for this job
    r#"
    ALTER TABLE task ADD COLUMN status TEXT NOT NULL DEFAULT 'on_track'
        CHECK (status IN ('not_started','on_track','done','late'));
    ALTER TABLE task ADD COLUMN completion_date TEXT;
    ALTER TABLE job  ADD COLUMN auto_shift_dependents INTEGER NOT NULL DEFAULT 1;
    "#,
];

pub fn apply_migrations(conn: &Connection) -> GbResult<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    let current: i64 = conn
        .query_row(
            "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key = 'schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let target = MIGRATIONS.len() as i64;
    if current > target {
        return Err(GbError::Migration(format!(
            "db schema_version {current} is ahead of binary's {target}; aborting"
        )));
    }
    if current == target {
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('schema_version', ?1)",
            params![(i + 1) as i64],
        )?;
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_reports_latest_schema_version_after_migrations() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        let v: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        apply_migrations(&conn).unwrap(); // second run should no-op
        let v: i64 = conn
            .query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, MIGRATIONS.len() as i64);
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        let bad = conn.execute(
            "INSERT INTO phase (job_id, name, colour, order_index) VALUES (999, 'X', '#000', 0)",
            [],
        );
        assert!(bad.is_err(), "expected FK violation");
    }

    #[test]
    fn pending_patches_table_exists_with_expected_columns() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(pending_patches)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        for expected in &[
            "id", "job_id", "patch_json", "summary", "source",
            "status", "created_at", "resolved_at", "error",
        ] {
            assert!(
                cols.iter().any(|c| c == expected),
                "missing column {expected}; got {cols:?}"
            );
        }
    }

    #[test]
    fn pending_patches_default_status_is_proposed() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        // Need a job to satisfy FK.
        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();
        let job_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p1', ?1, '{}', 's', 0)",
            params![job_id],
        ).unwrap();

        let status: String = conn
            .query_row(
                "SELECT status FROM pending_patches WHERE id = 'p1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "proposed");
    }

    #[test]
    fn task_table_has_status_and_completion_date_columns() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(task)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        for expected in &["status", "completion_date"] {
            assert!(
                cols.iter().any(|c| c == expected),
                "missing column {expected}; got {cols:?}"
            );
        }
    }

    #[test]
    fn job_table_has_auto_shift_dependents_column() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(job)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert!(
            cols.iter().any(|c| c == "auto_shift_dependents"),
            "missing column auto_shift_dependents; got {cols:?}"
        );
    }

    #[test]
    fn new_task_defaults_to_on_track_status() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();
        let job_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO phase (job_id, name, colour, order_index, collapsed)
             VALUES (?1, 'p', '#3B82F6', 0, 0)",
            params![job_id],
        ).unwrap();
        let phase_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index)
             VALUES (?1, 'tk', '2026-01-02', 1, 0)",
            params![phase_id],
        ).unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM task WHERE phase_id = ?1",
            params![phase_id],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "on_track");
    }

    #[test]
    fn new_job_defaults_to_auto_shift_enabled() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();

        let auto: i64 = conn.query_row(
            "SELECT auto_shift_dependents FROM job WHERE name = 't'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(auto, 1);
    }
}
