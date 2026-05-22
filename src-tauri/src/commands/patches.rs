//! Tauri IPC commands for the Inbox panel.
//!
//! Each command has a matching `*_inner` function that takes a bare
//! `&Connection` so it can be unit-tested without Tauri's `State` wrapper.

use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::State;

use crate::commands::Db;
use crate::patches::{apply_patch, schema::Patch};
use crate::{GbError, GbResult};

/// The shape returned over IPC to the Svelte front-end.
/// Mirrors `PendingPatch` in `src/lib/types.ts`.
#[derive(Debug, Clone, Serialize)]
pub struct PendingPatchRow {
    pub id: String,
    pub job_id: i64,
    pub patch: Patch,          // deserialised from patch_json
    pub summary: String,
    pub source: String,
    pub status: String,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
    pub error: Option<String>,
}

// ─── Inner helpers (testable without Tauri State) ─────────────────────────────

pub fn list_pending_patches_inner(
    conn: &Connection,
    status_filter: Option<String>,
) -> GbResult<Vec<PendingPatchRow>> {
    let sql = match &status_filter {
        Some(_) => "SELECT id, job_id, patch_json, summary, source, status, \
                           created_at, resolved_at, error \
                    FROM pending_patches WHERE status = ?1 ORDER BY created_at DESC",
        None    => "SELECT id, job_id, patch_json, summary, source, status, \
                           created_at, resolved_at, error \
                    FROM pending_patches ORDER BY created_at DESC",
    };

    let mut stmt = conn.prepare(sql)?;

    let rows = if let Some(ref s) = status_filter {
        stmt.query_map(params![s], row_to_pending_patch)?
    } else {
        stmt.query_map([], row_to_pending_patch)?
    };

    let mut out = Vec::new();
    for r in rows { out.push(r??); }
    Ok(out)
}

pub fn get_pending_patch_inner(conn: &Connection, id: String) -> GbResult<PendingPatchRow> {
    conn.query_row(
        "SELECT id, job_id, patch_json, summary, source, status, \
                created_at, resolved_at, error \
         FROM pending_patches WHERE id = ?1",
        params![id],
        row_to_pending_patch,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("patch {id}")),
        other => GbError::Sqlite(other),
    })?
}

pub fn reject_patch_inner(conn: &Connection, id: String) -> GbResult<()> {
    use chrono::Utc;
    let now = Utc::now().timestamp();
    let n = conn.execute(
        "UPDATE pending_patches SET status = 'rejected', resolved_at = ?1 WHERE id = ?2 AND status = 'proposed'",
        params![now, id],
    )?;
    if n == 0 {
        return Err(GbError::Validation(format!(
            "patch {id} not found or not in 'proposed' state"
        )));
    }
    Ok(())
}

pub fn accept_patch_inner(conn: &Connection, id: String) -> GbResult<()> {
    // Step 1: Transition proposed → accepted.
    let n = conn.execute(
        "UPDATE pending_patches SET status = 'accepted' WHERE id = ?1 AND status = 'proposed'",
        params![id],
    )?;
    if n == 0 {
        return Err(GbError::Validation(format!(
            "patch {id} not found or not in 'proposed' state"
        )));
    }

    // Step 2: Load and parse the patch document.
    let (patch_json,): (String,) = conn.query_row(
        "SELECT patch_json FROM pending_patches WHERE id = ?1",
        params![id],
        |r| Ok((r.get(0)?,)),
    )?;
    let patch: Patch = serde_json::from_str(&patch_json)?;

    // Step 3: Apply (transitions accepted → applied or apply_failed internally).
    apply_patch(conn, &id, &patch)
}

fn row_to_pending_patch(
    r: &rusqlite::Row,
) -> rusqlite::Result<Result<PendingPatchRow, GbError>> {
    let patch_json: String = r.get(2)?;
    let patch: Patch = match serde_json::from_str(&patch_json) {
        Ok(p) => p,
        Err(e) => return Ok(Err(GbError::Serde(e))),
    };
    Ok(Ok(PendingPatchRow {
        id: r.get(0)?,
        job_id: r.get(1)?,
        patch,
        summary: r.get(3)?,
        source: r.get(4)?,
        status: r.get(5)?,
        created_at: r.get(6)?,
        resolved_at: r.get(7)?,
        error: r.get(8)?,
    }))
}

// ─── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_pending_patches(
    db: State<Db>,
    status_filter: Option<String>,
) -> GbResult<Vec<PendingPatchRow>> {
    let conn = db.0.lock().unwrap();
    list_pending_patches_inner(&conn, status_filter)
}

#[tauri::command]
pub fn get_pending_patch(db: State<Db>, id: String) -> GbResult<PendingPatchRow> {
    let conn = db.0.lock().unwrap();
    get_pending_patch_inner(&conn, id)
}

#[tauri::command]
pub fn accept_patch(db: State<Db>, id: String) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    accept_patch_inner(&conn, id)
}

#[tauri::command]
pub fn reject_patch(db: State<Db>, id: String) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    reject_patch_inner(&conn, id)
}

pub fn clear_resolved_patches_inner(conn: &Connection) -> GbResult<u32> {
    use chrono::Utc;
    // "Applied older than 7 days" + "rejected/expired/apply_failed at any age".
    let cutoff = Utc::now().timestamp() - 7 * 24 * 3600;
    let n = conn.execute(
        "DELETE FROM pending_patches WHERE
             (status = 'applied' AND resolved_at < ?1)
          OR status IN ('rejected', 'expired', 'apply_failed')",
        params![cutoff],
    )?;
    Ok(n as u32)
}

pub fn expire_stale_patches_inner(conn: &Connection) -> GbResult<u32> {
    use chrono::Utc;
    let cutoff = Utc::now().timestamp() - 30 * 24 * 3600;
    let n = conn.execute(
        "UPDATE pending_patches SET status = 'expired'
         WHERE status = 'proposed' AND created_at < ?1",
        params![cutoff],
    )?;
    Ok(n as u32)
}

#[tauri::command]
pub fn clear_resolved_patches(db: State<Db>) -> GbResult<u32> {
    let conn = db.0.lock().unwrap();
    clear_resolved_patches_inner(&conn)
}

#[tauri::command]
pub fn expire_stale_patches(db: State<Db>) -> GbResult<u32> {
    let conn = db.0.lock().unwrap();
    expire_stale_patches_inner(&conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase};
    use crate::repo::{job, phase};
    use chrono::NaiveDate;

    fn fixture(conn: &rusqlite::Connection) -> (i64, i64) {
        let j = job::create(conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,1).unwrap(),
            is_template: false, holidays_block_work: false, region: "ZA".into(),
        }).unwrap();
        let p = phase::create(conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#3B82F6".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        (j.id, p.id)
    }

    fn insert_patch(conn: &rusqlite::Connection, id: &str, job_id: i64, status: &str) {
        use crate::patches::schema::{Patch, PatchOp};
        let patch = Patch {
            patch_version: 1,
            summary: "test summary".into(),
            ops: vec![PatchOp::AppendNote { job_id, text: "hi".into() }],
        };
        let json = serde_json::to_string(&patch).unwrap();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, status, created_at)
             VALUES (?1, ?2, ?3, 'test summary', ?4, 0)",
            rusqlite::params![id, job_id, json, status],
        ).unwrap();
    }

    #[test]
    fn list_pending_patches_filters_by_status() {
        let conn = open_in_memory().unwrap();
        let (job_id, _) = fixture(&conn);
        insert_patch(&conn, "p1", job_id, "proposed");
        insert_patch(&conn, "p2", job_id, "applied");
        insert_patch(&conn, "p3", job_id, "proposed");

        let proposed = list_pending_patches_inner(&conn, Some("proposed".into())).unwrap();
        assert_eq!(proposed.len(), 2);
        assert!(proposed.iter().all(|p| p.status == "proposed"));

        let all = list_pending_patches_inner(&conn, None).unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn get_pending_patch_returns_row() {
        let conn = open_in_memory().unwrap();
        let (job_id, _) = fixture(&conn);
        insert_patch(&conn, "p_get", job_id, "proposed");

        let pp = get_pending_patch_inner(&conn, "p_get".into()).unwrap();
        assert_eq!(pp.id, "p_get");
        assert_eq!(pp.job_id, job_id);
        assert_eq!(pp.status, "proposed");
    }

    #[test]
    fn reject_patch_sets_status_rejected() {
        let conn = open_in_memory().unwrap();
        let (job_id, _) = fixture(&conn);
        insert_patch(&conn, "p_rej", job_id, "proposed");

        reject_patch_inner(&conn, "p_rej".into()).unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_rej'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "rejected");
    }

    #[test]
    fn clear_resolved_removes_old_resolved_rows() {
        let conn = open_in_memory().unwrap();
        let (job_id, _) = fixture(&conn);

        // Insert rows in various terminal states with old resolved_at.
        let old_ts = 0i64; // epoch — definitely older than 7 days
        for (id, status) in &[
            ("r1", "applied"),
            ("r2", "rejected"),
            ("r3", "apply_failed"),
            ("r4", "expired"),
        ] {
            conn.execute(
                "INSERT INTO pending_patches (id, job_id, patch_json, summary, status, created_at, resolved_at)
                 VALUES (?1, ?2, '{}', 's', ?3, 0, ?4)",
                rusqlite::params![id, job_id, status, old_ts],
            ).unwrap();
        }
        // One proposed row — must NOT be cleared.
        insert_patch(&conn, "keep", job_id, "proposed");

        let count = clear_resolved_patches_inner(&conn).unwrap();
        assert_eq!(count, 4);

        let remaining: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pending_patches", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(remaining, 1); // only "keep"
    }

    #[test]
    fn expire_stale_marks_old_proposed_rows_expired() {
        let conn = open_in_memory().unwrap();
        let (job_id, _) = fixture(&conn);

        // Old row: created 31 days ago.
        let old_ts = chrono::Utc::now().timestamp() - 31 * 24 * 3600;
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, status, created_at)
             VALUES ('old', ?1, '{}', 'old', 'proposed', ?2)",
            rusqlite::params![job_id, old_ts],
        ).unwrap();

        // Recent row: 1 day old.
        let new_ts = chrono::Utc::now().timestamp() - 1 * 24 * 3600;
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, status, created_at)
             VALUES ('new', ?1, '{}', 'new', 'proposed', ?2)",
            rusqlite::params![job_id, new_ts],
        ).unwrap();

        let count = expire_stale_patches_inner(&conn).unwrap();
        assert_eq!(count, 1);

        let old_status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'old'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(old_status, "expired");

        let new_status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'new'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(new_status, "proposed");
    }

    #[test]
    fn accept_patch_applies_and_sets_status_applied() {
        let conn = open_in_memory().unwrap();
        let (job_id, _) = fixture(&conn);
        insert_patch(&conn, "p_acc", job_id, "proposed");

        accept_patch_inner(&conn, "p_acc".into()).unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_acc'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "applied");
    }
}
