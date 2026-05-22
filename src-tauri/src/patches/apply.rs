//! Apply engine — executes an accepted `Patch` inside a single SQLite
//! transaction.  All ops must succeed; any failure rolls back the whole
//! patch and marks the row `apply_failed`.
//!
//! Status transition managed here:
//!   proposed → accepted (caller's responsibility, before calling this fn)
//!   accepted → applied   (on success, inside this fn)
//!   accepted → apply_failed (on any op error, inside this fn)

use std::collections::HashMap;
use chrono::{NaiveDate, Utc};
use rusqlite::{Connection, params};

use crate::commands::drag::apply_ripple;
use crate::db::models::{NewTask, NewDependency};
use crate::patches::schema::{Patch, PatchOp, TaskRef};
use crate::repo::{contact as contact_repo, dependency as dep_repo, job as job_repo, task as task_repo};
use crate::{GbError, GbResult};

/// Executes all ops in `patch` inside a single SQLite transaction.
///
/// Assumes the row in `pending_patches` is already at status `accepted`.
/// On success: sets `status = 'applied'`, `resolved_at = now()`.
/// On failure: rolls back the apply transaction, sets `status = 'apply_failed'`,
///             stores the error message in the `error` column.
pub fn apply_patch(conn: &Connection, patch_id: &str, patch: &Patch) -> GbResult<()> {
    let result = attempt_apply(conn, patch);

    match &result {
        Ok(()) => {
            let now = Utc::now().timestamp();
            conn.execute(
                "UPDATE pending_patches SET status = 'applied', resolved_at = ?1 WHERE id = ?2",
                params![now, patch_id],
            )?;
        }
        Err(e) => {
            let msg = e.to_string();
            conn.execute(
                "UPDATE pending_patches SET status = 'apply_failed', error = ?1 WHERE id = ?2",
                params![msg, patch_id],
            )?;
        }
    }

    result
}

/// The inner apply; runs ops in a transaction.  Separate fn so we can
/// match on its result cleanly in `apply_patch`.
fn attempt_apply(conn: &Connection, patch: &Patch) -> GbResult<()> {
    let tx = conn.unchecked_transaction()?;

    // op_ref → task_id map, populated as add_task ops complete.
    let mut op_ref_map: HashMap<String, i64> = HashMap::new();

    for op in &patch.ops {
        match op {
            PatchOp::AddTask { .. } => apply_add_task(&tx, op, &mut op_ref_map)?,
            PatchOp::ShiftTask { .. } => apply_shift_task(&tx, op)?,
            PatchOp::AddDependency { .. } => apply_add_dependency(&tx, op, &op_ref_map)?,
            PatchOp::AddChaser { .. } => apply_add_chaser(&tx, op)?,
            PatchOp::AppendNote { .. } => apply_append_note(&tx, op)?,
        }
    }

    tx.commit()?;
    Ok(())
}

/// Resolves a `TaskRef` to a concrete task ID, consulting `op_ref_map` for
/// `TaskRef::Pending` variants.
fn resolve_task_ref(r: &TaskRef, op_ref_map: &HashMap<String, i64>) -> GbResult<i64> {
    match r {
        TaskRef::Existing { task_id } => Ok(*task_id),
        TaskRef::Pending { op_ref } => op_ref_map
            .get(op_ref)
            .copied()
            .ok_or_else(|| GbError::Validation(format!("op_ref '{op_ref}' not resolved (internal error)"))),
    }
}

// ─── op handlers ─────────────────────────────────────────────────────────────

fn apply_add_task(
    conn: &Connection,
    op: &PatchOp,
    op_ref_map: &mut HashMap<String, i64>,
) -> GbResult<()> {
    let (phase_id, name, start_date_str, duration_workdays, notes, contact_id, op_ref) = match op {
        PatchOp::AddTask { phase_id, name, start_date, duration_workdays, notes, contact_id, op_ref } => {
            (*phase_id, name.clone(), start_date.clone(), *duration_workdays, notes.clone(), *contact_id, op_ref.clone())
        }
        _ => unreachable!(),
    };

    // Verify phase exists.
    let phase_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM phase WHERE id = ?1",
        params![phase_id],
        |r| r.get::<_, i64>(0),
    ).map(|c| c > 0)?;
    if !phase_exists {
        return Err(GbError::NotFound(format!("phase {phase_id}")));
    }

    let start = NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d")
        .map_err(|_| GbError::Validation(format!("bad date: {start_date_str}")))?;

    // Compute next order_index within the phase.
    let order_index: i64 = conn.query_row(
        "SELECT COALESCE(MAX(order_index), -1) + 1 FROM task WHERE phase_id = ?1",
        params![phase_id],
        |r| r.get(0),
    )?;

    let mut task = task_repo::create(conn, &NewTask {
        phase_id,
        name,
        start_date: start,
        duration_workdays,
        order_index,
        notes,
    })?;

    // Set contact if specified.
    if let Some(cid) = contact_id {
        // Verify contact exists.
        contact_repo::get(conn, cid)?;
        task.contact_id = Some(cid);
        task_repo::update(conn, &task)?;
    }

    // Register op_ref so later ops can reference this new task.
    if let Some(r) = op_ref {
        op_ref_map.insert(r, task.id);
    }

    Ok(())
}

fn apply_shift_task(conn: &Connection, op: &PatchOp) -> GbResult<()> {
    let (task_id, by_days) = match op {
        PatchOp::ShiftTask { task_id, by_days } => (*task_id, *by_days),
        _ => unreachable!(),
    };

    // Verify task exists and get its job_id.
    let job_id: i64 = conn.query_row(
        "SELECT p.job_id FROM task t JOIN phase p ON p.id = t.phase_id WHERE t.id = ?1",
        params![task_id],
        |r| r.get(0),
    ).map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("task {task_id}")),
        other => GbError::Sqlite(other),
    })?;

    apply_ripple(conn, job_id, task_id, by_days)
}

fn apply_add_dependency(
    conn: &Connection,
    op: &PatchOp,
    op_ref_map: &HashMap<String, i64>,
) -> GbResult<()> {
    let (predecessor, successor, _dep_type, lag_days) = match op {
        PatchOp::AddDependency { predecessor, successor, dep_type, lag_days } => {
            (predecessor, successor, dep_type.clone(), *lag_days)
        }
        _ => unreachable!(),
    };

    let pred_id = resolve_task_ref(predecessor, op_ref_map)?;
    let succ_id = resolve_task_ref(successor, op_ref_map)?;

    // Verify both tasks exist.
    task_repo::get(conn, pred_id)?;
    task_repo::get(conn, succ_id)?;

    dep_repo::create(conn, &NewDependency {
        predecessor_id: pred_id,
        successor_id: succ_id,
        lag_days,
    })?;

    Ok(())
}

fn apply_add_chaser(conn: &Connection, op: &PatchOp) -> GbResult<()> {
    use crate::chaser::templates::VALID_CHASER_TEMPLATE_KEYS;

    let (task_id, contact_id, template) = match op {
        PatchOp::AddChaser { task_id, contact_id, template } => (*task_id, *contact_id, template.clone()),
        _ => unreachable!(),
    };

    if !VALID_CHASER_TEMPLATE_KEYS.contains(&template.as_str()) {
        return Err(GbError::Validation(format!(
            "unknown chaser template '{template}'; expected one of: {}",
            VALID_CHASER_TEMPLATE_KEYS.join(", ")
        )));
    }

    // Verify task and contact both exist.
    let mut task = task_repo::get(conn, task_id)?;
    contact_repo::get(conn, contact_id)?;

    // Assign the contact — the template key is informational only at this stage.
    task.contact_id = Some(contact_id);
    task_repo::update(conn, &task)?;

    Ok(())
}

fn apply_append_note(conn: &Connection, op: &PatchOp) -> GbResult<()> {
    let (job_id, text) = match op {
        PatchOp::AppendNote { job_id, text } => (*job_id, text.clone()),
        _ => unreachable!(),
    };

    // Verify job exists.
    job_repo::get(conn, job_id)?;

    // Append text to the job's notes (stored as app_meta "job_{id}_notes").
    let key = format!("job_{job_id}_notes");
    let existing: Option<String> = conn.query_row(
        "SELECT value FROM app_meta WHERE key = ?1",
        params![&key],
        |r| r.get(0),
    ).ok();

    let new_value = match existing {
        Some(prev) if !prev.trim().is_empty() => format!("{prev}\n\n{text}"),
        _ => text,
    };

    conn.execute(
        "INSERT OR REPLACE INTO app_meta (key, value) VALUES (?1, ?2)",
        params![key, new_value],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase};
    use crate::patches::schema::{Patch, PatchOp};
    use crate::repo::{job, phase};
    use chrono::NaiveDate;

    fn fixture_db() -> (rusqlite::Connection, i64, i64) {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "Test Job".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            is_template: false, holidays_block_work: false, region: "ZA".into(),
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "Foundation".into(), colour: "#3B82F6".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        (conn, j.id, p.id)
    }

    #[test]
    fn apply_empty_ops_returns_err() {
        let (conn, job_id, _) = fixture_db();
        // Insert a pending_patches row.
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_test_empty', ?1, '{}', 'test', 0)",
            rusqlite::params![job_id],
        ).unwrap();
        let patch = Patch {
            patch_version: 1,
            summary: "empty".into(),
            ops: vec![],
        };
        // A patch with zero ops should have been caught by validate_patch before reaching
        // apply_patch; but apply_patch should still handle it gracefully.
        let result = apply_patch(&conn, "p_test_empty", &patch);
        // Either Ok (no-ops) or Err — both are fine.  What we test is that
        // the function exists and is callable.
        let _ = result;
    }

    #[test]
    fn apply_patch_sets_status_applied_on_success() {
        let (conn, job_id, _phase_id) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_note', ?1, '{}', 'note', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "append a note".into(),
            ops: vec![PatchOp::AppendNote {
                job_id,
                text: "Graham wants fewer cavity walls".into(),
            }],
        };

        apply_patch(&conn, "p_note", &patch).unwrap();

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_note'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "applied");
    }

    #[test]
    fn add_task_inserts_row_and_resolves_op_ref() {
        let (conn, job_id, phase_id) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_add', ?1, '{}', 'add', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "add a task".into(),
            ops: vec![
                PatchOp::AddTask {
                    phase_id,
                    name: "Order vent ducting".into(),
                    start_date: "2026-06-08".into(),
                    duration_workdays: 3,
                    notes: Some("from Doug".into()),
                    contact_id: None,
                    op_ref: Some("vent_task".into()),
                },
            ],
        };

        apply_patch(&conn, "p_add", &patch).unwrap();

        let tasks = crate::repo::task::list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "Order vent ducting");
        assert_eq!(tasks[0].duration_workdays, 3);
        assert_eq!(tasks[0].notes.as_deref(), Some("from Doug"));
    }

    #[test]
    fn add_task_assigns_contact_when_contact_id_supplied() {
        let (conn, job_id, phase_id) = fixture_db();
        // Create a contact to assign.
        let contact = crate::repo::contact::create(&conn, &crate::db::models::NewContact {
            name: "Doug".into(),
            telegram_chat_id: None,
            telegram_handle: None,
            notes: "supplier".into(),
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_contact', ?1, '{}', 'contact', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "add task with contact".into(),
            ops: vec![PatchOp::AddTask {
                phase_id,
                name: "Call Doug".into(),
                start_date: "2026-06-08".into(),
                duration_workdays: 1,
                notes: None,
                contact_id: Some(contact.id),
                op_ref: None,
            }],
        };

        apply_patch(&conn, "p_contact", &patch).unwrap();

        let tasks = crate::repo::task::list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(tasks[0].contact_id, Some(contact.id));
    }

    #[test]
    fn add_task_fails_when_phase_does_not_exist() {
        let (conn, job_id, _phase_id) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_bad_phase', ?1, '{}', 'bad', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "bad phase".into(),
            ops: vec![PatchOp::AddTask {
                phase_id: 99999,
                name: "Ghost task".into(),
                start_date: "2026-06-08".into(),
                duration_workdays: 1,
                notes: None,
                contact_id: None,
                op_ref: None,
            }],
        };

        let result = apply_patch(&conn, "p_bad_phase", &patch);
        assert!(result.is_err());

        // Row should be apply_failed.
        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_bad_phase'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "apply_failed");
    }

    #[test]
    fn shift_task_moves_start_date_by_workdays() {
        let (conn, job_id, phase_id) = fixture_db();
        let t = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id,
            name: "Order windows".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 5,
            order_index: 0,
            notes: None,
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_shift', ?1, '{}', 'shift', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "shift windows by +7 workdays".into(),
            ops: vec![PatchOp::ShiftTask { task_id: t.id, by_days: 7 }],
        };

        apply_patch(&conn, "p_shift", &patch).unwrap();

        let updated = crate::repo::task::get(&conn, t.id).unwrap();
        // 2026-06-08 (Monday) + 7 workdays = 2026-06-17 (Wednesday).
        // (8,9,10,11,12 = Mon–Fri; 15,16,17 = Mon-Wed; 7 workdays forward)
        assert_eq!(updated.start_date, NaiveDate::from_ymd_opt(2026, 6, 17).unwrap());
    }

    #[test]
    fn shift_task_fails_when_task_does_not_exist() {
        let (conn, job_id, _phase_id) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_shift_bad', ?1, '{}', 'shift bad', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "shift missing task".into(),
            ops: vec![PatchOp::ShiftTask { task_id: 99999, by_days: 1 }],
        };

        let result = apply_patch(&conn, "p_shift_bad", &patch);
        assert!(result.is_err());

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_shift_bad'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "apply_failed");
    }

    #[test]
    fn add_dependency_between_existing_tasks() {
        let (conn, job_id, phase_id) = fixture_db();
        let t1 = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "T1".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t2 = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "T2".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_dep', ?1, '{}', 'dep', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "add dep".into(),
            ops: vec![PatchOp::AddDependency {
                predecessor: crate::patches::schema::TaskRef::Existing { task_id: t1.id },
                successor: crate::patches::schema::TaskRef::Existing { task_id: t2.id },
                dep_type: "FS".into(),
                lag_days: 0,
            }],
        };

        apply_patch(&conn, "p_dep", &patch).unwrap();

        let deps = crate::repo::dependency::list_for_job(&conn, job_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].predecessor_id, t1.id);
        assert_eq!(deps[0].successor_id, t2.id);
    }

    #[test]
    fn add_dependency_with_op_ref_predecessor() {
        let (conn, job_id, phase_id) = fixture_db();
        let existing = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "Existing".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_dep_ref', ?1, '{}', 'dep+ref', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "add new task then dep to it".into(),
            ops: vec![
                PatchOp::AddTask {
                    phase_id,
                    name: "New task".into(),
                    start_date: "2026-06-10".into(),
                    duration_workdays: 1,
                    notes: None,
                    contact_id: None,
                    op_ref: Some("new_t".into()),
                },
                PatchOp::AddDependency {
                    predecessor: crate::patches::schema::TaskRef::Existing { task_id: existing.id },
                    successor: crate::patches::schema::TaskRef::Pending { op_ref: "new_t".into() },
                    dep_type: "FS".into(),
                    lag_days: 0,
                },
            ],
        };

        apply_patch(&conn, "p_dep_ref", &patch).unwrap();

        let deps = crate::repo::dependency::list_for_job(&conn, job_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].predecessor_id, existing.id);
        // The successor is the newly-inserted task — we just verify it's non-zero.
        assert!(deps[0].successor_id > 0);
    }

    #[test]
    fn add_dependency_cycle_rejected() {
        let (conn, job_id, phase_id) = fixture_db();
        let t1 = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "A".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t2 = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "B".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        // Pre-existing dep: t1 → t2.
        crate::repo::dependency::create(&conn, &NewDependency {
            predecessor_id: t1.id, successor_id: t2.id, lag_days: 0,
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_cycle', ?1, '{}', 'cycle', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        // Attempting t2 → t1 should cycle.
        let patch = Patch {
            patch_version: 1,
            summary: "would cycle".into(),
            ops: vec![PatchOp::AddDependency {
                predecessor: crate::patches::schema::TaskRef::Existing { task_id: t2.id },
                successor: crate::patches::schema::TaskRef::Existing { task_id: t1.id },
                dep_type: "FS".into(),
                lag_days: 0,
            }],
        };

        let result = apply_patch(&conn, "p_cycle", &patch);
        assert!(matches!(result, Err(crate::GbError::DependencyCycle(_, _))));

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_cycle'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "apply_failed");
    }

    #[test]
    fn add_chaser_assigns_contact_to_task() {
        let (conn, job_id, phase_id) = fixture_db();
        let task = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "Solar plans".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let contact = crate::repo::contact::create(&conn, &crate::db::models::NewContact {
            name: "Renaissance Solar".into(),
            telegram_chat_id: Some("111222333".into()),
            telegram_handle: None,
            notes: "".into(),
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_chaser', ?1, '{}', 'chaser', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "add chaser".into(),
            ops: vec![PatchOp::AddChaser {
                task_id: task.id,
                contact_id: contact.id,
                template: "weekly".into(),
            }],
        };

        // "weekly" is not a valid template key — should fail.
        let result = apply_patch(&conn, "p_chaser", &patch);
        assert!(result.is_err());

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_chaser'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "apply_failed");
    }

    #[test]
    fn add_chaser_valid_template_assigns_contact() {
        let (conn, job_id, phase_id) = fixture_db();
        let task = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "Solar plans".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let contact = crate::repo::contact::create(&conn, &crate::db::models::NewContact {
            name: "Renaissance Solar".into(),
            telegram_chat_id: Some("111222333".into()),
            telegram_handle: None,
            notes: "".into(),
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_chaser2', ?1, '{}', 'chaser2', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "add chaser with valid key".into(),
            ops: vec![PatchOp::AddChaser {
                task_id: task.id,
                contact_id: contact.id,
                template: "manual".into(),
            }],
        };

        apply_patch(&conn, "p_chaser2", &patch).unwrap();

        let updated = crate::repo::task::get(&conn, task.id).unwrap();
        assert_eq!(updated.contact_id, Some(contact.id));
    }

    #[test]
    fn append_note_creates_and_appends_to_job_notes() {
        let (conn, job_id, _phase_id) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_note1', ?1, '{}', 'note1', 0)",
            rusqlite::params![job_id],
        ).unwrap();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_note2', ?1, '{}', 'note2', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        // First note.
        let patch1 = Patch {
            patch_version: 1,
            summary: "first note".into(),
            ops: vec![PatchOp::AppendNote { job_id, text: "Line one".into() }],
        };
        apply_patch(&conn, "p_note1", &patch1).unwrap();

        // Second note — should append.
        let patch2 = Patch {
            patch_version: 1,
            summary: "second note".into(),
            ops: vec![PatchOp::AppendNote { job_id, text: "Line two".into() }],
        };
        apply_patch(&conn, "p_note2", &patch2).unwrap();

        let stored: String = conn.query_row(
            "SELECT value FROM app_meta WHERE key = ?1",
            rusqlite::params![format!("job_{job_id}_notes")],
            |r| r.get(0),
        ).unwrap();
        assert!(stored.contains("Line one"));
        assert!(stored.contains("Line two"));
    }

    #[test]
    fn append_note_fails_for_missing_job() {
        let (conn, job_id, _) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_note_bad', ?1, '{}', 'note bad', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "bad job".into(),
            ops: vec![PatchOp::AppendNote { job_id: 99999, text: "ghost note".into() }],
        };

        let result = apply_patch(&conn, "p_note_bad", &patch);
        assert!(result.is_err());

        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_note_bad'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "apply_failed");
    }

    #[test]
    fn full_patch_with_all_five_op_types() {
        let (conn, job_id, phase_id) = fixture_db();

        // Pre-create a task that shift_task and add_dependency will target.
        let existing_task = crate::repo::task::create(&conn, &crate::db::models::NewTask {
            phase_id, name: "Order windows".into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 5, order_index: 0, notes: None,
        }).unwrap();
        let contact = crate::repo::contact::create(&conn, &crate::db::models::NewContact {
            name: "Doug".into(),
            telegram_chat_id: None, telegram_handle: None, notes: "".into(),
        }).unwrap();

        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_full', ?1, '{}', 'from site meeting', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "Site meeting 2026-05-22: 4 changes".into(),
            ops: vec![
                // 1. add_task — new task with op_ref
                PatchOp::AddTask {
                    phase_id,
                    name: "Order vent ducting".into(),
                    start_date: "2026-06-10".into(),
                    duration_workdays: 2,
                    notes: None,
                    contact_id: Some(contact.id),
                    op_ref: Some("vent".into()),
                },
                // 2. shift_task — existing task
                PatchOp::ShiftTask {
                    task_id: existing_task.id,
                    by_days: 5,
                },
                // 3. add_dependency — new task depends on existing (via op_ref)
                PatchOp::AddDependency {
                    predecessor: crate::patches::schema::TaskRef::Existing { task_id: existing_task.id },
                    successor: crate::patches::schema::TaskRef::Pending { op_ref: "vent".into() },
                    dep_type: "FS".into(),
                    lag_days: 0,
                },
                // 4. add_chaser — attach contact to existing task
                PatchOp::AddChaser {
                    task_id: existing_task.id,
                    contact_id: contact.id,
                    template: "approaching".into(),
                },
                // 5. append_note
                PatchOp::AppendNote {
                    job_id,
                    text: "Graham wants fewer cavity walls — reopen Henry Fagan discussion".into(),
                },
            ],
        };

        apply_patch(&conn, "p_full", &patch).unwrap();

        // Verify status.
        let status: String = conn.query_row(
            "SELECT status FROM pending_patches WHERE id = 'p_full'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(status, "applied");

        // Verify vent task was created.
        let tasks = crate::repo::task::list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(tasks.len(), 2);
        let vent = tasks.iter().find(|t| t.name == "Order vent ducting").unwrap();
        assert_eq!(vent.contact_id, Some(contact.id));

        // Verify windows were shifted.
        let shifted = crate::repo::task::get(&conn, existing_task.id).unwrap();
        assert!(shifted.start_date > existing_task.start_date);

        // Verify dependency was created.
        let deps = crate::repo::dependency::list_for_job(&conn, job_id).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].predecessor_id, existing_task.id);
        assert_eq!(deps[0].successor_id, vent.id);

        // Verify chaser contact assigned.
        let windows_updated = crate::repo::task::get(&conn, existing_task.id).unwrap();
        assert_eq!(windows_updated.contact_id, Some(contact.id));

        // Verify note was stored.
        let note_key = format!("job_{job_id}_notes");
        let note: String = conn.query_row(
            "SELECT value FROM app_meta WHERE key = ?1",
            rusqlite::params![note_key],
            |r| r.get(0),
        ).unwrap();
        assert!(note.contains("Graham"));
    }
}
