# Blik Plan ↔ Claude Connector — Plan 3: Inbox Panel + Apply Engine

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the apply engine (executes accepted patches inside a single SQLite transaction) and the Inbox panel (Svelte component that lists proposed patches, renders diffs, and lets the user accept or reject them).

**Architecture:** A new Rust module `src-tauri/src/patches/apply.rs` iterates over `PatchOp` variants inside an `unchecked_transaction`, resolves `op_ref` handles to freshly-inserted row IDs as it goes, and delegates to existing `repo::*` functions and an extracted `apply_ripple` helper. Tauri commands in `src-tauri/src/commands/patches.rs` expose list/get/accept/reject/clear-resolved/expire-stale over IPC. The Svelte side adds inbox state to `store.svelte.ts`, IPC bindings to `ipc.ts`, an `InboxPanel.svelte` component with a per-op diff renderer, and a badge button in `BottomToolbar.svelte`. A 5-second poll loop fetches proposed patches while the window is open.

**Tech Stack:** Rust (rusqlite, chrono, serde_json, existing `repo::*`), TypeScript/Svelte 5 (`$state`, `$derived`, `onMount`/`onDestroy`).

**Spec reference:** `docs/specs/2026-05-22-blikplan-claude-connector-design.md`

---

## Pre-reading required before implementing

Before Task 1, read:
- `src-tauri/src/patches/schema.rs` — `Patch`, `PatchOp`, `TaskRef`
- `src-tauri/src/patches/validate.rs` — `validate_patch`
- `src-tauri/src/commands/drag.rs` — `drag_task_inner` (contains `compute_ripple` call we will extract)
- `src-tauri/src/repo/task.rs`, `src-tauri/src/repo/dependency.rs` — `create`, `get`, `update`
- `src-tauri/src/db/models.rs` — `NewTask`, `Task`, `NewDependency`
- `src-tauri/src/chaser/templates.rs` — valid template keys
- `src-tauri/src/lib.rs` — `run()` / `invoke_handler` registration point
- `src/lib/store.svelte.ts` — `bootstrap()` for poll placement
- `src/lib/components/BottomToolbar.svelte` — button strip pattern

---

## Architectural decisions locked in this plan

**A. Transaction model.** Apply uses `conn.unchecked_transaction()` (same as `drag_task_inner` and `task::reorder`). Op handlers receive `&rusqlite::Connection`; the transaction is committed at the top of `apply_patch` after all ops succeed, or left to drop (auto-rollback) on any `?` error.

**B. `add_task` `order_index`.** Computed as `SELECT COALESCE(MAX(order_index), -1) + 1 FROM task WHERE phase_id = ?` inside `apply_add_task`. Then `task::create` is called; if `contact_id` is present, a subsequent `task::update` sets it.

**C. `add_chaser` template validation.** Valid keys are `"manual"`, `"approaching"`, `"overdue"` (see `src-tauri/src/chaser/templates.rs`). `apply_add_chaser` validates the key is in that set, then sets `task.contact_id` (the only writable field for a chaser at this stage). The template key is stored nowhere — it is validated and dropped. A future plan can add a per-task template column if required.

**D. `shift_task.by_days` unit.** Workdays (consistent with `drag_task_inner` which also measures shifts in workdays). The ripple engine is the same one that `drag_task_inner` uses — extracted into `apply_ripple` in Task 3.

**E. Status transitions.** Three separate writes, two transactions:
1. `UPDATE pending_patches SET status='accepted' WHERE id=?` — before starting the apply transaction.
2. Apply transaction runs all ops; on success, `UPDATE pending_patches SET status='applied', resolved_at=? WHERE id=?`.
3. On any error from the apply transaction, `UPDATE pending_patches SET status='apply_failed', error=? WHERE id=?`.

There is a deliberate window where status is `accepted` but apply has not yet completed. That is fine — if the app crashes at that point the row stays `accepted` and the Inbox will show it with no button state change. A future plan can add a startup sweep that resets orphaned `accepted` rows to `proposed`.

**F. Op-ref resolver.** `HashMap<String, i64>` of `op_ref → inserted_task_id`, populated as `AddTask` ops complete. Passed by mutable reference to each handler.

**G. Poll interval.** 5 seconds. Stored as `const INBOX_POLL_MS: number = 5000`. Started in `store.bootstrap()` alongside the existing chaser interval. Cleared by the `onDestroy` in `InboxPanel.svelte`; the interval ID is kept in a store field.

**H. Diff renderer.** Pure TypeScript function `renderPatchOp(op: PatchOp, phases: Phase[], tasks: Task[], contacts: Contact[]): string` in `src/lib/inbox-diff.ts`. Returns a one-line human-readable string per op. Name lookups use the in-memory store slices passed as arguments.

---

## File Structure

**Files this plan creates:**

- `src-tauri/src/patches/apply.rs` — apply engine: transaction wrapper + all op handlers
- `src-tauri/src/commands/patches.rs` — Tauri IPC commands for inbox management
- `src/lib/inbox-diff.ts` — pure TS diff renderer (no framework dependency)
- `src/lib/components/InboxPanel.svelte` — Inbox panel component

**Files this plan modifies:**

- `src-tauri/src/patches/mod.rs` — expose `apply` module
- `src-tauri/src/commands/drag.rs` — extract `apply_ripple` helper (pub, tx-safe)
- `src-tauri/src/commands/mod.rs` — register `patches` submodule
- `src-tauri/src/lib.rs` — register 6 new Tauri commands + call `expire_stale_patches` in `run()`
- `src/lib/ipc.ts` — 6 new IPC bindings
- `src/lib/store.svelte.ts` — inbox state + poll loop
- `src/lib/components/BottomToolbar.svelte` — Inbox button + badge

---

## Task 1: Extract `apply_ripple` from `drag_task_inner`

This is a prerequisite for the apply engine. `drag_task_inner` currently opens its own transaction. We extract the pure logic into a `pub fn apply_ripple` that operates on a bare `&Connection` (can be called inside an existing transaction or standalone), then update `drag_task_inner` to call it.

**Files:**
- Modify: `src-tauri/src/commands/drag.rs`

- [ ] **Step 1: Write a failing test for `apply_ripple`**

Open `src-tauri/src/commands/drag.rs` and append a new test to the existing `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn apply_ripple_shifts_task_and_downstream() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false, holidays_block_work: true, region: "ZA".into(),
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        let t1 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T1".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t2 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T2".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        dependency::create(&conn, &NewDependency {
            predecessor_id: t1.id, successor_id: t2.id, lag_days: 0,
        }).unwrap();

        // Apply a +2 workday shift to t1 using apply_ripple (not drag_task_inner).
        apply_ripple(&conn, j.id, t1.id, 2).unwrap();

        let t1_updated = task::get(&conn, t1.id).unwrap();
        let t2_updated = task::get(&conn, t2.id).unwrap();
        assert_eq!(t1_updated.start_date, NaiveDate::from_ymd_opt(2026,6,10).unwrap());
        assert_eq!(t2_updated.start_date, NaiveDate::from_ymd_opt(2026,6,11).unwrap());
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml drag::tests::apply_ripple_shifts_task_and_downstream`
Expected: FAIL — `apply_ripple` is not defined.

- [ ] **Step 3: Extract `apply_ripple` as a pub helper**

In `src-tauri/src/commands/drag.rs`, insert the following new function **after** the `drag_task_inner` function and before the `#[cfg(test)]` block. Then update `drag_task_inner` to call it:

```rust
/// Applies a workday shift to `task_id` and ripples the change through
/// all downstream tasks in the same job. Safe to call inside an existing
/// `unchecked_transaction` because it does not open a new one — callers
/// are responsible for their own transaction boundary.
///
/// `by_days` is signed workdays (positive = later, negative = earlier).
/// Internally mirrors the logic that `drag_task_inner` performs but
/// accepts a pre-computed shift rather than a new absolute date.
pub fn apply_ripple(
    conn: &rusqlite::Connection,
    job_id: i64,
    task_id: i64,
    by_days: i64,
) -> GbResult<()> {
    use crate::calendar::workday::add_workdays;
    use crate::deps::ripple::compute_ripple;
    use std::collections::HashSet;
    use chrono::NaiveDate;

    let tasks = task_repo::list_for_job(conn, job_id)?;
    let deps = dep_repo::list_for_job(conn, job_id)?;
    let job = job_repo::get(conn, job_id)?;
    let nwds: HashSet<NaiveDate> = nwd_repo::list_for_job(conn, job_id)?
        .into_iter()
        .filter(|n| {
            job.holidays_block_work
                || (!n.source.ends_with("_holiday") && n.source != "sa_public_holiday")
        })
        .map(|n| n.date)
        .collect();

    let dragged = tasks
        .iter()
        .find(|t| t.id == task_id)
        .ok_or_else(|| GbError::NotFound(format!("task {task_id}")))?;

    let new_start = add_workdays(dragged.start_date, by_days, &nwds);

    let mut ripples = compute_ripple(&tasks, &deps, task_id, by_days, &nwds);
    ripples.insert(task_id, new_start);

    for t in &tasks {
        if let Some(new_date) = ripples.get(&t.id) {
            conn.execute(
                "UPDATE task SET start_date = ?1 WHERE id = ?2",
                rusqlite::params![new_date.to_string(), t.id],
            )?;
        }
    }

    Ok(())
}
```

Now update `drag_task_inner` to delegate to `apply_ripple` for its core logic. Replace the body of `drag_task_inner` (everything after the `let dragged = ...` line up to and including the `tx.commit()?;` and `Ok(DragResult { ... })`) with:

```rust
fn drag_task_inner(conn: &rusqlite::Connection, args: DragTaskArgs) -> GbResult<DragResult> {
    let tasks: Vec<Task> = task_repo::list_for_job(conn, args.job_id)?;
    let deps = dep_repo::list_for_job(conn, args.job_id)?;
    let job = job_repo::get(conn, args.job_id)?;
    use std::collections::HashSet;
    use chrono::NaiveDate;
    use crate::calendar::workday::count_workdays;
    let nwds: HashSet<NaiveDate> = nwd_repo::list_for_job(conn, args.job_id)?
        .into_iter()
        .filter(|n| {
            job.holidays_block_work
                || (!n.source.ends_with("_holiday") && n.source != "sa_public_holiday")
        })
        .map(|n| n.date)
        .collect();

    let dragged = tasks
        .iter()
        .find(|t| t.id == args.task_id)
        .ok_or_else(|| GbError::NotFound(format!("task {}", args.task_id)))?;

    let shift = if args.new_start_date >= dragged.start_date {
        count_workdays(dragged.start_date, args.new_start_date) - 1
    } else {
        -(count_workdays(args.new_start_date, dragged.start_date) - 1)
    };

    let tx = conn.unchecked_transaction()?;
    apply_ripple(&tx, args.job_id, args.task_id, shift)?;
    tx.commit()?;

    let updated = task_repo::list_for_job(conn, args.job_id)?
        .into_iter()
        .filter(|t| {
            tasks.iter().find(|old| old.id == t.id).map(|old| old.start_date) != Some(t.start_date)
        })
        .collect();

    Ok(DragResult { updated_tasks: updated })
}
```

Note: `apply_ripple` accepts `&rusqlite::Connection` and `rusqlite::Transaction` both satisfy that (via `Deref<Target=Connection>`), so passing `&tx` compiles cleanly.

- [ ] **Step 4: Check that the existing drag test still compiles**

Run: `cargo test --manifest-path src-tauri/Cargo.toml drag`
Expected: both `drag_ripples_to_downstream_task` and `apply_ripple_shifts_task_and_downstream` PASS.

Note: the `drag_ripples_to_downstream_task` test uses `DragResult.updated_tasks` — confirm the count is still 2. If `list_for_job` iteration order differs, adjust the assertion to use `.len()` rather than index order (it already does this with `.find`).

- [ ] **Step 5: Verify `add_workdays` exists in `calendar::workday`**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: zero errors. If `add_workdays` is not exported, check `src-tauri/src/calendar/workday.rs` — it may be named `advance_workdays` or similar. Use whatever name is present in that file and update `apply_ripple` accordingly.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/drag.rs
git commit -m "refactor(drag): extract apply_ripple as a tx-safe pub helper

Splits drag_task_inner into a thin transaction wrapper + apply_ripple,
so the Plan 3 apply engine can call apply_ripple inside its own
outer transaction without nesting conflicts."
```

---

## Task 2: Apply engine skeleton + op-ref resolver + transaction wrapper

**Files:**
- Create: `src-tauri/src/patches/apply.rs`
- Modify: `src-tauri/src/patches/mod.rs`

- [ ] **Step 1: Write a failing test for the apply skeleton**

Create `src-tauri/src/patches/apply.rs` with ONLY the test module first:

```rust
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
        let (conn, job_id, phase_id) = fixture_db();
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
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply`
Expected: FAIL — `apply_patch` not defined, compile errors.

- [ ] **Step 3: Implement the skeleton**

Prepend this to `src-tauri/src/patches/apply.rs` (above the `#[cfg(test)]` block):

```rust
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
use crate::db::models::{NewTask, NewDependency, Task};
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

// ─── op handlers (stubs — filled in Tasks 2–6) ───────────────────────────────

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
```

- [ ] **Step 4: Register `apply` in `src-tauri/src/patches/mod.rs`**

Open `src-tauri/src/patches/mod.rs` and add:

```rust
pub mod apply;

pub use apply::apply_patch;
```

The full file should now read:

```rust
//! Shared patch schema used by external clients (the MCP server)
//! and the in-app Inbox apply engine. See
//! `docs/specs/2026-05-22-blikplan-claude-connector-design.md`.

pub mod apply;
pub mod schema;
pub mod validate;

pub use apply::apply_patch;
pub use schema::{Patch, PatchOp, PATCH_VERSION};
pub use validate::{validate_patch, ValidationError};
```

- [ ] **Step 5: Add `VALID_CHASER_TEMPLATE_KEYS` to `src-tauri/src/chaser/templates.rs`**

Open `src-tauri/src/chaser/templates.rs` and add after the three `DEFAULT_*` constants:

```rust
/// The set of valid chaser template keys accepted by `apply_add_chaser`.
/// Matches the hard-coded keys handled in `commands::chaser::send_chaser`.
pub const VALID_CHASER_TEMPLATE_KEYS: &[&str] = &["manual", "approaching", "overdue"];
```

- [ ] **Step 6: Run the skeleton tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply`
Expected:
- `apply_empty_ops_returns_err` — PASS
- `apply_patch_sets_status_applied_on_success` — PASS

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: zero errors (warnings about unused imports are acceptable at this stage).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/patches/apply.rs src-tauri/src/patches/mod.rs src-tauri/src/chaser/templates.rs
git commit -m "feat(patches): add apply engine skeleton with op-ref resolver and tx wrapper

All five op handlers are in place; Tasks 3–6 add per-handler
unit tests. VALID_CHASER_TEMPLATE_KEYS added to templates module."
```

---

## Task 3: `apply_add_task` op handler tests

**Files:**
- Modify: `src-tauri/src/patches/apply.rs`

- [ ] **Step 1: Write failing tests for `apply_add_task`**

Append these tests to the `#[cfg(test)] mod tests` block in `src-tauri/src/patches/apply.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply`
Expected: all 5 tests in `patches::apply::tests` PASS.

Note: the handler code was already written in Task 2's skeleton. These tests exist to verify correctness of the `add_task` handler specifically, including the `contact_id` two-step and the phase-existence check.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/patches/apply.rs
git commit -m "test(patches): apply_add_task handler tests (phase check, contact assignment, op_ref)"
```

---

## Task 4: `apply_shift_task` op handler tests

**Files:**
- Modify: `src-tauri/src/patches/apply.rs`

- [ ] **Step 1: Write failing tests for `apply_shift_task`**

Append these tests to the `#[cfg(test)] mod tests` block in `src-tauri/src/patches/apply.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply`
Expected: all 7 tests PASS. The shift test verifies date arithmetic; if `add_workdays` behaviour differs from the expected date in the comment, adjust the expected date to match actual workday calculation for that locale (ZA, no holidays).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/patches/apply.rs
git commit -m "test(patches): apply_shift_task handler tests (workday shift, missing task)"
```

---

## Task 5: `apply_add_dependency` op handler tests

**Files:**
- Modify: `src-tauri/src/patches/apply.rs`

- [ ] **Step 1: Write failing tests**

Append these tests to the `#[cfg(test)] mod tests` block in `src-tauri/src/patches/apply.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply`
Expected: all 10 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/patches/apply.rs
git commit -m "test(patches): apply_add_dependency handler tests (existing tasks, op_ref, cycle guard)"
```

---

## Task 6: `apply_add_chaser` and `apply_append_note` op handler tests

**Files:**
- Modify: `src-tauri/src/patches/apply.rs`

- [ ] **Step 1: Write failing tests**

Append these tests to the `#[cfg(test)] mod tests` block in `src-tauri/src/patches/apply.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply`
Expected: all 14 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/patches/apply.rs
git commit -m "test(patches): apply_add_chaser and apply_append_note handler tests

Chaser: validates template key in {manual,approaching,overdue};
assigns contact_id to task. AppendNote: writes to app_meta; appends
on second call."
```

---

## Task 7: Full apply integration test (all 5 op types in one patch)

**Files:**
- Modify: `src-tauri/src/patches/apply.rs`

- [ ] **Step 1: Write the integration test**

Append this test to the `#[cfg(test)] mod tests` block in `src-tauri/src/patches/apply.rs`:

```rust
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
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply::tests::full_patch_with_all_five_op_types`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/patches/apply.rs
git commit -m "test(patches): full integration test — all 5 op types in one patch"
```

---

## Task 8: Rollback test — one bad op rolls back all previous ops

**Files:**
- Modify: `src-tauri/src/patches/apply.rs`

- [ ] **Step 1: Write the rollback test**

Append this test to the `#[cfg(test)] mod tests` block in `src-tauri/src/patches/apply.rs`:

```rust
    #[test]
    fn failed_op_rolls_back_all_preceding_ops() {
        let (conn, job_id, phase_id) = fixture_db();
        conn.execute(
            "INSERT INTO pending_patches (id, job_id, patch_json, summary, created_at)
             VALUES ('p_rollback', ?1, '{}', 'should rollback', 0)",
            rusqlite::params![job_id],
        ).unwrap();

        let patch = Patch {
            patch_version: 1,
            summary: "first op ok, second op bad".into(),
            ops: vec![
                // Op 1: valid — adds a new task.
                PatchOp::AddTask {
                    phase_id,
                    name: "Should be rolled back".into(),
                    start_date: "2026-06-08".into(),
                    duration_workdays: 1,
                    notes: None,
                    contact_id: None,
                    op_ref: None,
                },
                // Op 2: invalid — references a phase that does not exist.
                PatchOp::AddTask {
                    phase_id: 99999,
                    name: "Bad phase task".into(),
                    start_date: "2026-06-08".into(),
                    duration_workdays: 1,
                    notes: None,
                    contact_id: None,
                    op_ref: None,
                },
            ],
        };

        let result = apply_patch(&conn, "p_rollback", &patch);
        assert!(result.is_err());

        // Op 1 must have been rolled back — zero tasks in the phase.
        let tasks = crate::repo::task::list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(tasks.len(), 0, "rollback failed — task from op 1 survived");

        // Row must be apply_failed.
        let (status, error): (String, Option<String>) = conn.query_row(
            "SELECT status, error FROM pending_patches WHERE id = 'p_rollback'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(status, "apply_failed");
        assert!(error.is_some(), "error column should be populated");
        assert!(error.unwrap().contains("phase 99999"),
            "error message should mention the missing phase");
    }
```

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::apply::tests::failed_op_rolls_back_all_preceding_ops`
Expected: PASS. This is the critical safety property of the apply engine.

- [ ] **Step 3: Run the full patches test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches`
Expected: all tests across `patches::schema`, `patches::validate`, and `patches::apply` PASS. Count of `patches::apply::tests` tests at this point: 16.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/patches/apply.rs
git commit -m "test(patches): rollback test — bad op in a multi-op patch rolls back all changes"
```

---

## Task 9: Tauri commands — list, get, accept, reject

**Files:**
- Create: `src-tauri/src/commands/patches.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing tests for the list/get/accept/reject commands**

Create `src-tauri/src/commands/patches.rs` with ONLY the test module first:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::patches`
Expected: FAIL — `list_pending_patches_inner`, `get_pending_patch_inner`, etc. not defined.

- [ ] **Step 3: Implement the commands**

Prepend this to `src-tauri/src/commands/patches.rs` (above the `#[cfg(test)]` block):

```rust
//! Tauri IPC commands for the Inbox panel.
//!
//! Each command has a matching `*_inner` function that takes a bare
//! `&Connection` so it can be unit-tested without Tauri's `State` wrapper.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
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
    use chrono::Utc;

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
```

- [ ] **Step 4: Register the module in `src-tauri/src/commands/mod.rs`**

Open `src-tauri/src/commands/mod.rs` and add `pub mod patches;` to the module list:

```rust
pub mod chaser;
pub mod dependency;
pub mod drag;
pub mod job;
pub mod meta;
pub mod no_work_day;
pub mod patches;
pub mod phase;
pub mod sync;
pub mod task;
pub mod template;
```

- [ ] **Step 5: Register commands in `src-tauri/src/lib.rs`**

Open `src-tauri/src/lib.rs` and add the four new commands to the `tauri::generate_handler!` macro (after the existing chaser commands):

```rust
            commands::patches::list_pending_patches,
            commands::patches::get_pending_patch,
            commands::patches::accept_patch,
            commands::patches::reject_patch,
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::patches`
Expected: all 4 tests PASS.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: zero errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/patches.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add list/get/accept/reject Tauri commands for the Inbox

accept_patch handles the full proposed→accepted→applied lifecycle.
All four commands have testable inner helpers."
```

---

## Task 10: Tauri commands — `clear_resolved` and `expire_stale_patches`

**Files:**
- Modify: `src-tauri/src/commands/patches.rs`

- [ ] **Step 1: Write failing tests**

Append these tests to the `#[cfg(test)] mod tests` block in `src-tauri/src/commands/patches.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::patches`
Expected: FAIL — `clear_resolved_patches_inner` and `expire_stale_patches_inner` not defined.

- [ ] **Step 3: Implement `clear_resolved` and `expire_stale`**

Append these functions to `src-tauri/src/commands/patches.rs` (before the `#[cfg(test)]` block):

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::patches`
Expected: all 6 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/patches.rs
git commit -m "feat(commands): add clear_resolved_patches and expire_stale_patches commands

clear_resolved: sweeps rejected/expired/apply_failed rows plus applied
rows older than 7 days.
expire_stale: sets status=expired for proposed rows older than 30 days."
```

---

## Task 11: Wire `expire_stale_patches` into app startup

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/patches.rs` (register new commands in handler)

- [ ] **Step 1: Register the two new commands in `lib.rs`**

Open `src-tauri/src/lib.rs`. In the `tauri::generate_handler!` macro, add after `reject_patch`:

```rust
            commands::patches::clear_resolved_patches,
            commands::patches::expire_stale_patches,
```

- [ ] **Step 2: Call `expire_stale_patches` on startup**

In `src-tauri/src/lib.rs`, the `run()` function opens the connection and creates `Db`. Add the expiry sweep immediately after `open()` succeeds and before `Db::new(conn)`:

```rust
pub fn run() {
    let conn = db::connection::open(&db_path()).expect("failed to open db");

    // Sweep any proposed patches older than 30 days to 'expired'.
    // This runs synchronously before the window opens, so the Inbox
    // never shows stale rows.
    if let Err(e) = commands::patches::expire_stale_patches_inner(&conn) {
        eprintln!("warn: expire_stale_patches on startup failed: {e}");
    }

    let db = Db::new(conn);
    // ... rest of run() unchanged
```

- [ ] **Step 3: Verify build**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: zero errors.

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: ALL tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(startup): call expire_stale_patches on app launch

Marks proposed patches older than 30 days as expired before the
window opens, so the Inbox never surfaces stale proposals."
```

---

## Task 12: Svelte — store state + IPC bindings

**Files:**
- Modify: `src/lib/ipc.ts`
- Modify: `src/lib/store.svelte.ts`
- Modify: `src/lib/types.ts` (update import to include `PendingPatch`)

- [ ] **Step 1: Add IPC bindings to `src/lib/ipc.ts`**

Open `src/lib/ipc.ts`. Add this import at the top with the existing type imports:

```typescript
import type { PendingPatch } from './types';
```

Then append a new section at the end of the file:

```typescript
// Inbox / Patches
export const listPendingPatches = (statusFilter?: string) =>
  invoke<PendingPatch[]>('list_pending_patches', { statusFilter: statusFilter ?? null });

export const getPendingPatch = (id: string) =>
  invoke<PendingPatch>('get_pending_patch', { id });

export const acceptPatch = (id: string) =>
  invoke<void>('accept_patch', { id });

export const rejectPatch = (id: string) =>
  invoke<void>('reject_patch', { id });

export const clearResolvedPatches = () =>
  invoke<number>('clear_resolved_patches');

export const expireStalePatches = () =>
  invoke<number>('expire_stale_patches');
```

- [ ] **Step 2: Add inbox state to `store.svelte.ts`**

Open `src/lib/store.svelte.ts`.

Add `PendingPatch` to the import line at the top:

```typescript
import type { Job, Phase, Task, Dependency, NoWorkDay, Contact, PendingPatch } from './types';
```

In the `class Store { ... }` body, add these fields after the `contacts` field (around line 58):

```typescript
  // Inbox — proposed patches from MCP / external sources
  inboxPatches    = $state<PendingPatch[]>([]);
  inboxOpen       = $state<boolean>(false);
  private inboxPollTimer: number | null = null;
```

Add these methods to the `Store` class (after `runChaserCheck`):

```typescript
  /** Poll interval for the Inbox. 5 seconds while the window is open. */
  static readonly INBOX_POLL_MS = 5000;

  async refreshInbox(): Promise<void> {
    try {
      this.inboxPatches = await ipc.listPendingPatches('proposed');
    } catch (e) {
      console.warn('inbox refresh failed', e);
    }
  }

  startInboxPoll(): void {
    if (this.inboxPollTimer !== null) return;
    this.inboxPollTimer = window.setInterval(
      () => this.refreshInbox(),
      Store.INBOX_POLL_MS,
    );
  }

  stopInboxPoll(): void {
    if (this.inboxPollTimer !== null) {
      clearInterval(this.inboxPollTimer);
      this.inboxPollTimer = null;
    }
  }

  async acceptInboxPatch(id: string): Promise<void> {
    await ipc.acceptPatch(id);
    await this.refreshInbox();
    // Re-load the current job to reflect the applied changes.
    if (this.currentJob) {
      await this.openJob(this.currentJob.id);
    }
  }

  async rejectInboxPatch(id: string): Promise<void> {
    await ipc.rejectPatch(id);
    await this.refreshInbox();
  }

  async clearResolvedPatches(): Promise<void> {
    await ipc.clearResolvedPatches();
    await this.refreshInbox();
  }
```

In the `bootstrap()` method, add inbox initialisation after the `runChaserCheck` block (around line 335):

```typescript
    // Inbox: initial fetch + 5-second poll + refresh on focus.
    await this.refreshInbox();
    this.startInboxPoll();
    window.addEventListener('focus', () => this.refreshInbox());
```

- [ ] **Step 3: Verify type-check**

Run: `cd ~/Desktop/GanttBok && npm run check`
Expected: zero errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/ipc.ts src/lib/store.svelte.ts
git commit -m "feat(store): add inbox state, poll loop, and accept/reject/clear actions

Polls pending_patches every 5s; refreshes on focus; re-opens current
job after accept to reload rippled task positions."
```

---

## Task 13: Svelte — `inbox-diff.ts` diff renderer

**Files:**
- Create: `src/lib/inbox-diff.ts`

The diff renderer is a pure function — no Svelte or store imports — so it can be unit-tested in isolation via Vitest.

- [ ] **Step 1: Write failing tests for the diff renderer**

Create `src/lib/__tests__/inbox-diff.test.ts` (check the existing test directory path first with `ls src/lib/__tests__/`):

```typescript
import { describe, it, expect } from 'vitest';
import { renderPatchOp } from '../inbox-diff';
import type { PatchOp, Phase, Task, Contact } from '../types';

const phases: Phase[] = [
  { id: 1, job_id: 1, name: 'Foundation', colour: '#3B82F6', order_index: 0, collapsed: false, notes: '' },
];
const tasks: Task[] = [
  { id: 10, phase_id: 1, name: 'Order windows', start_date: '2026-06-08',
    duration_workdays: 5, order_index: 0, notes: null, contact_id: null, last_chaser_sent_at: null },
];
const contacts: Contact[] = [
  { id: 100, name: 'Doug Supplies', telegram_chat_id: null, telegram_handle: null,
    notes: '', created_at: '2026-05-01' },
];

describe('renderPatchOp', () => {
  it('renders add_task', () => {
    const op: PatchOp = {
      op: 'add_task', phase_id: 1, name: 'Order vent ducting',
      start_date: '2026-06-10', duration_workdays: 3,
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order vent ducting');
    expect(line).toContain('Foundation');
    expect(line).toContain('2026-06-10');
  });

  it('renders shift_task with positive delta', () => {
    const op: PatchOp = { op: 'shift_task', task_id: 10, by_days: 7 };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
    expect(line).toContain('+7');
  });

  it('renders shift_task with negative delta', () => {
    const op: PatchOp = { op: 'shift_task', task_id: 10, by_days: -3 };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
    expect(line).toContain('-3');
  });

  it('renders add_dependency with known task names', () => {
    const op: PatchOp = {
      op: 'add_dependency',
      predecessor: { task_id: 10 },
      successor: { task_id: 10 },
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
  });

  it('renders add_chaser with contact name', () => {
    const op: PatchOp = {
      op: 'add_chaser', task_id: 10, contact_id: 100, template: 'manual',
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
    expect(line).toContain('Doug Supplies');
  });

  it('renders append_note', () => {
    const op: PatchOp = { op: 'append_note', job_id: 1, text: 'Graham wants fewer cavity walls' };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Graham wants fewer cavity walls');
  });

  it('falls back gracefully for unknown task_id', () => {
    const op: PatchOp = { op: 'shift_task', task_id: 9999, by_days: 1 };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('task #9999');
  });

  it('falls back gracefully for unknown phase_id', () => {
    const op: PatchOp = {
      op: 'add_task', phase_id: 9999, name: 'Mystery', start_date: '2026-06-10', duration_workdays: 1,
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Mystery');
    expect(line).toContain('phase #9999');
  });
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd ~/Desktop/GanttBok && npx vitest run src/lib/__tests__/inbox-diff.test.ts`
Expected: FAIL — `renderPatchOp` not found.

If `npx vitest` is not available, check `package.json` for the test command and use it instead.

- [ ] **Step 3: Implement the diff renderer**

Create `src/lib/inbox-diff.ts`:

```typescript
/**
 * inbox-diff.ts
 *
 * Pure function that converts a single PatchOp into a human-readable
 * one-line diff string for display in the Inbox panel.
 * No Svelte, no store, no side effects.
 */
import type { PatchOp, Phase, Task, Contact } from './types';

export function renderPatchOp(
  op: PatchOp,
  phases: Phase[],
  tasks: Task[],
  contacts: Contact[],
): string {
  const taskName = (id: number) =>
    tasks.find((t) => t.id === id)?.name ?? `task #${id}`;

  const phaseName = (id: number) =>
    phases.find((p) => p.id === id)?.name ?? `phase #${id}`;

  const contactName = (id: number) =>
    contacts.find((c) => c.id === id)?.name ?? `contact #${id}`;

  const taskRefLabel = (ref: { task_id: number } | { op_ref: string }): string => {
    if ('task_id' in ref) return taskName(ref.task_id);
    return `(new: ${ref.op_ref})`;
  };

  switch (op.op) {
    case 'add_task': {
      const phase = phaseName(op.phase_id);
      const contact = op.contact_id != null ? ` (assigned: ${contactName(op.contact_id)})` : '';
      return `+ Add "${op.name}" to ${phase}, starts ${op.start_date}, ${op.duration_workdays}d${contact}`;
    }

    case 'shift_task': {
      const sign = op.by_days >= 0 ? '+' : '';
      return `↻ Shift "${taskName(op.task_id)}" by ${sign}${op.by_days} workdays`;
    }

    case 'add_dependency': {
      const pred = taskRefLabel(op.predecessor);
      const succ = taskRefLabel(op.successor);
      const lag = (op.lag_days ?? 0) !== 0 ? ` (lag: ${op.lag_days}d)` : '';
      const type_ = op.dep_type ?? 'FS';
      return `→ Add dependency: "${pred}" ${type_} "${succ}"${lag}`;
    }

    case 'add_chaser': {
      return `🔔 Chaser "${taskName(op.task_id)}" → ${contactName(op.contact_id)} (${op.template})`;
    }

    case 'append_note': {
      const preview = op.text.length > 80 ? op.text.slice(0, 77) + '…' : op.text;
      return `📝 Note: "${preview}"`;
    }

    default: {
      // Exhaustiveness guard — TypeScript will warn if a new op is added.
      const _: never = op;
      return `Unknown op`;
    }
  }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd ~/Desktop/GanttBok && npx vitest run src/lib/__tests__/inbox-diff.test.ts`
Expected: all 8 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/inbox-diff.ts src/lib/__tests__/inbox-diff.test.ts
git commit -m "feat(inbox): add diff renderer for patch ops

Pure TS function renderPatchOp returns a one-line human-readable
description per op. Tested with Vitest; falls back gracefully for
unknown task/phase/contact IDs."
```

---

## Task 14: Svelte — `InboxPanel.svelte` component

**Files:**
- Create: `src/lib/components/InboxPanel.svelte`

- [ ] **Step 1: Create the component**

Create `src/lib/components/InboxPanel.svelte`:

```svelte
<script lang="ts">
  import { onDestroy } from 'svelte';
  import { store } from '../store.svelte';
  import { renderPatchOp } from '../inbox-diff';

  // Stop polling when this panel is destroyed (e.g. if it's conditionally mounted).
  // The poll is started in store.bootstrap(), so we only stop it here if explicitly needed.
  // In this implementation the panel is always mounted alongside BottomToolbar, so we
  // rely on the store's stopInboxPoll() only when the user explicitly closes/reopens.
  onDestroy(() => {
    store.stopInboxPoll();
  });

  let acceptingId = $state<string | null>(null);
  let rejectingId = $state<string | null>(null);
  let clearing    = $state(false);
  let actionError = $state<string | null>(null);

  async function accept(id: string) {
    acceptingId = id;
    actionError = null;
    try {
      await store.acceptInboxPatch(id);
    } catch (e) {
      actionError = `Accept failed: ${e}`;
    } finally {
      acceptingId = null;
    }
  }

  async function reject(id: string) {
    rejectingId = id;
    actionError = null;
    try {
      await store.rejectInboxPatch(id);
    } catch (e) {
      actionError = `Reject failed: ${e}`;
    } finally {
      rejectingId = null;
    }
  }

  async function clearResolved() {
    clearing = true;
    actionError = null;
    try {
      await store.clearResolvedPatches();
    } catch (e) {
      actionError = `Clear failed: ${e}`;
    } finally {
      clearing = false;
    }
  }

  function formatDate(ts: number): string {
    return new Date(ts * 1000).toLocaleDateString('en-GB', {
      day: '2-digit', month: 'short', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  }
</script>

<aside class="inbox-panel">
  <header class="inbox-header">
    <h2>Inbox</h2>
    <button class="close-btn" onclick={() => (store.inboxOpen = false)} aria-label="Close inbox">×</button>
  </header>

  {#if actionError}
    <div class="action-error">{actionError}</div>
  {/if}

  {#if store.inboxPatches.length === 0}
    <div class="empty-state">
      <p>No proposals pending.</p>
      <p class="hint">Connect Claude in Settings → Integrations to start sending patches here.</p>
    </div>
  {:else}
    <div class="patch-list">
      {#each store.inboxPatches as patch (patch.id)}
        <article class="patch-card">
          <header class="patch-header">
            <span class="patch-summary">{patch.summary}</span>
            <span class="patch-meta">{formatDate(patch.created_at)}</span>
          </header>

          <ul class="op-list">
            {#each patch.patch.ops as op}
              <li class="op-line">
                {renderPatchOp(op, store.phases, store.tasks, store.contacts)}
              </li>
            {/each}
          </ul>

          <footer class="patch-actions">
            <button
              class="accept-btn"
              disabled={acceptingId === patch.id || rejectingId === patch.id}
              onclick={() => accept(patch.id)}
            >
              {acceptingId === patch.id ? 'Applying…' : 'Accept'}
            </button>
            <button
              class="reject-btn"
              disabled={acceptingId === patch.id || rejectingId === patch.id}
              onclick={() => reject(patch.id)}
            >
              {rejectingId === patch.id ? 'Rejecting…' : 'Reject'}
            </button>
          </footer>
        </article>
      {/each}
    </div>
  {/if}

  <footer class="inbox-footer">
    <button class="clear-btn" disabled={clearing} onclick={clearResolved}>
      {clearing ? 'Clearing…' : 'Clear resolved'}
    </button>
  </footer>
</aside>

<style>
  .inbox-panel {
    position: fixed;
    top: 0; right: 0; bottom: 0;
    width: 380px;
    background: var(--c-panel);
    border-left: 1px solid var(--c-border);
    z-index: 40;
    display: flex;
    flex-direction: column;
    box-shadow: -6px 0 18px rgba(0, 0, 0, 0.08);
  }

  .inbox-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--sp-3) var(--sp-4);
    border-bottom: 1px solid var(--c-border);
    flex-shrink: 0;
  }

  .inbox-header h2 {
    margin: 0;
    font-size: var(--font-size-base);
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 22px;
    cursor: pointer;
    color: var(--c-text-muted);
    line-height: 1;
    padding: 0 var(--sp-1);
  }

  .close-btn:hover { color: var(--c-text); }

  .action-error {
    background: #FEE2E2;
    color: #C8121E;
    font-size: var(--font-size-xs);
    padding: var(--sp-2) var(--sp-4);
    border-bottom: 1px solid #FCA5A5;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--sp-6) var(--sp-4);
    text-align: center;
    color: var(--c-text-muted);
  }

  .empty-state p { margin: 0 0 var(--sp-2); }

  .hint {
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
  }

  .patch-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--sp-3) var(--sp-4);
    display: flex;
    flex-direction: column;
    gap: var(--sp-3);
  }

  .patch-card {
    border: 1px solid var(--c-border);
    border-radius: 6px;
    background: var(--c-bg);
    overflow: hidden;
  }

  .patch-header {
    display: flex;
    flex-direction: column;
    gap: var(--sp-1);
    padding: var(--sp-2) var(--sp-3);
    background: var(--c-panel);
    border-bottom: 1px solid var(--c-border);
  }

  .patch-summary {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--c-text);
  }

  .patch-meta {
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    font-family: var(--font-mono);
  }

  .op-list {
    list-style: none;
    margin: 0;
    padding: var(--sp-2) var(--sp-3);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .op-line {
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    line-height: 1.4;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .patch-actions {
    display: flex;
    gap: var(--sp-2);
    padding: var(--sp-2) var(--sp-3);
    border-top: 1px solid var(--c-border);
  }

  .accept-btn {
    flex: 1;
    background: var(--c-accent);
    color: white;
    border: none;
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-family: inherit;
  }

  .accept-btn:hover:not(:disabled) { filter: brightness(1.1); }
  .accept-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .reject-btn {
    background: var(--c-bg);
    color: var(--c-text-muted);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer;
    font-size: var(--font-size-sm);
    font-family: inherit;
  }

  .reject-btn:hover:not(:disabled) {
    background: #FEE2E2;
    color: #C8121E;
    border-color: #FCA5A5;
  }

  .reject-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .inbox-footer {
    padding: var(--sp-3) var(--sp-4);
    border-top: 1px solid var(--c-border);
    flex-shrink: 0;
  }

  .clear-btn {
    width: 100%;
    background: var(--c-bg);
    border: 1px solid var(--c-border);
    border-radius: 4px;
    padding: var(--sp-2) var(--sp-3);
    cursor: pointer;
    font-size: var(--font-size-xs);
    color: var(--c-text-muted);
    font-family: inherit;
  }

  .clear-btn:hover:not(:disabled) {
    background: var(--c-accent-fade);
    color: var(--c-accent);
  }

  .clear-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 2: Verify type-check**

Run: `cd ~/Desktop/GanttBok && npm run check`
Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/InboxPanel.svelte
git commit -m "feat(inbox): add InboxPanel.svelte component

Lists proposed patches; renders per-op diff using renderPatchOp;
Accept/Reject buttons per patch; Clear resolved button in footer;
empty state with hint to connect Claude."
```

---

## Task 15: Svelte — integrate Inbox into `BottomToolbar.svelte` with badge + poll

**Files:**
- Modify: `src/lib/components/BottomToolbar.svelte`

- [ ] **Step 1: Add InboxPanel import and badge button**

Open `src/lib/components/BottomToolbar.svelte`.

In the `<script>` section, add the import near the top with the other component imports:

```typescript
  import InboxPanel from './InboxPanel.svelte';
```

The `store` import already exists (`import { store } from '../store.svelte';`). No change needed there.

In the `<div class="bottom-toolbar">` block, add the Inbox button **after** the Contacts button (after the `<button onclick={() => (store.showContactsPage = true)} ...>` button):

```svelte
  <button
    class="icon-btn inbox-btn"
    class:has-proposals={store.inboxPatches.length > 0}
    onclick={() => (store.inboxOpen = !store.inboxOpen)}
    title="Inbox — {store.inboxPatches.length} pending proposal{store.inboxPatches.length === 1 ? '' : 's'}"
    aria-label="Open Inbox"
  >
    <!-- Envelope icon -->
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
      <polyline points="22,6 12,13 2,6"/>
    </svg>
    {#if store.inboxPatches.length > 0}
      <span class="badge">{store.inboxPatches.length}</span>
    {/if}
  </button>
```

At the **bottom** of the template (after the closing `{/if}` of the Notes panel section but before `<style>`), add the conditional render of the Inbox panel:

```svelte
{#if store.inboxOpen}
  <InboxPanel />
{/if}
```

In the `<style>` block, append:

```css
  /* ============ Inbox badge button ============ */
  .inbox-btn {
    position: relative;
  }
  .inbox-btn.has-proposals {
    color: var(--c-accent);
  }
  .badge {
    position: absolute;
    top: -2px;
    right: -4px;
    background: var(--c-accent);
    color: white;
    border-radius: 50%;
    width: 14px;
    height: 14px;
    font-size: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-mono);
    font-weight: 700;
    line-height: 1;
  }
```

- [ ] **Step 2: Verify type-check**

Run: `cd ~/Desktop/GanttBok && npm run check`
Expected: zero TypeScript errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/BottomToolbar.svelte
git commit -m "feat(inbox): integrate InboxPanel into BottomToolbar with badge

Envelope icon button in toolbar; badge shows count of proposed
patches; clicking opens/closes InboxPanel slide-over."
```

---

## Task 16: Final verification

- [ ] **Step 1: Run the full Rust test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: ALL tests pass. New tests added across this plan:
- `drag::tests::apply_ripple_shifts_task_and_downstream` (Task 1)
- `patches::apply::tests` — 16 tests (Tasks 2–8)
- `commands::patches::tests` — 6 tests (Tasks 9–10)

- [ ] **Step 2: Run the TypeScript type-check**

Run: `cd ~/Desktop/GanttBok && npm run check`
Expected: zero errors.

- [ ] **Step 3: Run the Vitest tests**

Run: `cd ~/Desktop/GanttBok && npx vitest run`
Expected: all tests pass, including the 8 tests in `inbox-diff.test.ts`.

- [ ] **Step 4: Confirm git log shape**

Run: `git log --oneline -20`
Expected (newest first):

```
feat(inbox): integrate InboxPanel into BottomToolbar with badge
feat(inbox): add InboxPanel.svelte component
feat(inbox): add diff renderer for patch ops
feat(store): add inbox state, poll loop, and accept/reject/clear actions
feat(startup): call expire_stale_patches on app launch
feat(commands): add clear_resolved_patches and expire_stale_patches commands
feat(commands): add list/get/accept/reject Tauri commands for the Inbox
test(patches): rollback test — bad op in a multi-op patch rolls back all changes
test(patches): full integration test — all 5 op types in one patch
test(patches): apply_add_chaser and apply_append_note handler tests
test(patches): apply_add_dependency handler tests (existing tasks, op_ref, cycle guard)
test(patches): apply_shift_task handler tests (workday shift, missing task)
test(patches): apply_add_task handler tests (phase check, contact assignment, op_ref)
feat(patches): add apply engine skeleton with op-ref resolver and tx wrapper
refactor(drag): extract apply_ripple as a tx-safe pub helper
```

- [ ] **Step 5: Smoke-test checklist (manual)**

Before declaring complete, verify manually:
1. Build the app: `cd ~/Desktop/GanttBok && npm run tauri dev`
2. The bottom toolbar shows an envelope icon button.
3. Clicking the envelope icon opens the Inbox panel (empty state visible).
4. Insert a test row directly into `ganttbok.db` (`INSERT INTO pending_patches ...`) — within 5 seconds the badge count updates.
5. Close and reopen the app — the expiry sweep runs (check no warnings in the console).

- [ ] **Step 6: Final commit (nothing left staged)**

If any files were accidentally modified during the verification run, stage and commit them. Otherwise confirm with:

```bash
git status
```

Expected: `nothing to commit, working tree clean`.

---

## Out of scope for Plan 3

- MCP server (`blikplan-mcp` binary, Plan 2) — Plan 3 only consumes the `pending_patches` table that the MCP server populates.
- "Connect to Claude (beta)" Settings panel — Plan 4.
- Per-task template column (add_chaser currently validates key and assigns contact; storing the chosen key for later re-send is a future enhancement).
- Reset of orphaned `accepted` rows on startup (the window between `accepted` and `applied/apply_failed` is narrow; a startup sweep is a nice-to-have for Plan 4).
- `AppendTaskNote` op variant — not in the spec for v1.
- npm wrapper package (`@blikplan/mcp`) — Plan 2.
- Phase-level note ops — not in spec for v1.

---

## Risks logged for Plan 4

1. **`apply_ripple` uses `add_workdays` from `src-tauri/src/calendar/workday.rs`** — verify this function exists by name before running Task 1. If the function is named differently (e.g. `advance_workdays`, `offset_workdays`), update `apply_ripple` accordingly. The Task 1 Step 5 check will catch this.

2. **`PendingPatch.patch` is typed as `Patch` in `types.ts` (parsed JSON), but `PendingPatchRow` in Rust serialises the entire struct including the nested `patch: Patch` field.** Tauri's JSON serialisation will produce the correct shape, but if the front-end receives `patch_json` as a raw string instead of a parsed object, the `InboxPanel` will need to `JSON.parse` it. The `PendingPatchRow` Rust struct already deserialises `patch_json` into a `Patch` before returning — so the IPC wire format should be correct. Verify this in the smoke test.

3. **`onDestroy` in `InboxPanel` calls `store.stopInboxPoll()`.** Because the panel is conditionally rendered (`{#if store.inboxOpen}`), mounting/unmounting the panel will start/stop polling. When the panel is closed, polling stops. The `window.addEventListener('focus', …)` listener added in `bootstrap()` continues to refresh on focus even when the panel is closed — this is intentional. If background polling is wanted regardless of panel open state, move `startInboxPoll()` to `bootstrap()` only and remove the `onDestroy` stop. Current design is conservative (poll only when panel is open). Plan 4 can revisit.

4. **`accept_patch_inner` re-reads `patch_json` from the DB after writing `status='accepted'`** — there is a brief race window if a second agent/process modifies the row between those two writes. For a single-user local app this is acceptable, but document it.

5. **Badge count in `BottomToolbar` shows only `proposed` patches** (from `store.inboxPatches` which is filtered to `proposed`). If the user wants to see `apply_failed` rows (for diagnosis), they must open the panel. A future plan could add a second filter chip in the Inbox panel to show failed rows.
