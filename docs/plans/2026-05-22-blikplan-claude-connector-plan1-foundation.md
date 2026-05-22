# Blik Plan ↔ Claude Connector — Plan 1: Foundation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the `pending_patches` table and the shared JSON patch schema (Rust + TypeScript) so the MCP server (Plan 2) and the Inbox panel (Plan 3) can be built against a stable foundation. No user-visible behaviour change.

**Architecture:** A new SQLite migration adds `pending_patches`. A new Rust module `src-tauri/src/patches/` defines the `Patch` document and `PatchOp` enum with serde derives, plus a structural validator. Matching TypeScript types are added to `src/lib/types.ts`. Nothing is wired into commands, IPC, or UI in this plan — that's Plans 2/3.

**Tech Stack:** Rust (rusqlite, serde, serde_json, chrono), TypeScript (existing types module). No new crates required.

**Spec reference:** `docs/specs/2026-05-22-blikplan-claude-connector-design.md`

---

## File Structure

**Files this plan creates or modifies:**

- Create: `src-tauri/src/patches/mod.rs` — module entry; re-exports schema + validator
- Create: `src-tauri/src/patches/schema.rs` — `Patch` and `PatchOp` types with serde derives
- Create: `src-tauri/src/patches/validate.rs` — structural validator (returns typed errors)
- Modify: `src-tauri/src/lib.rs` — register the new `patches` module
- Modify: `src-tauri/src/db/migrations.rs:109` (end of `MIGRATIONS` array) — append v7 migration
- Modify: `src/lib/types.ts` — append `Patch`, `PatchOp`, `PendingPatch`, `PatchStatus` types

`GbError` already has a `Validation(String)` variant; Plans 2/3 convert `ValidationError` to that at the IPC boundary. No new error variant needed in Plan 1.

**Why these boundaries:** `schema.rs` holds pure data shapes (no behaviour, no I/O), `validate.rs` is pure logic over those shapes (no DB), so both can be unit-tested in isolation with zero fixtures. The migration is the only thing that touches the DB.

**ID convention (locked in this plan, used by all later plans):** All references to existing rows use integer DB IDs (`job_id: i64`, `task_id: i64`, etc.). For ops that *create* rows that later ops in the same patch may reference, the creation op carries an optional `op_ref: String` (e.g. `"new_vent_task"`). The apply engine in Plan 3 resolves `op_ref` strings to freshly-inserted IDs.

---

## Task 1: Add the `pending_patches` migration

**Files:**
- Modify: `src-tauri/src/db/migrations.rs:109`

- [ ] **Step 1: Write a failing test for the new table**

Append this test to the existing `#[cfg(test)] mod tests` block in `src-tauri/src/db/migrations.rs`:

```rust
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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pending_patches`
Expected: both tests FAIL with errors mentioning `no such table: pending_patches`.

- [ ] **Step 3: Append the v7 migration**

In `src-tauri/src/db/migrations.rs`, find the closing `];` of the `MIGRATIONS` array (currently around line 109, after the v6 chaser migration). Add this entry immediately before the `];`:

```rust
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
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml pending_patches`
Expected: both tests PASS. Also re-run the whole `db::migrations::tests` module to make sure existing tests still pass:

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::migrations`
Expected: all tests in the module PASS, including `fresh_db_reports_latest_schema_version_after_migrations` (which now expects version 7).

- [ ] **Step 5: Commit**

```bash
cd ~/Desktop/GanttBok
git add src-tauri/src/db/migrations.rs
git commit -m "feat(db): add pending_patches table (v7 migration)

Foundation for the Blik Plan ↔ Claude connector. Patches proposed by
external sources (MCP server, future webhooks) queue here until the
user accepts or rejects them inside the Inbox panel."
```

---

## Task 2: Define the Rust patch schema

**Files:**
- Create: `src-tauri/src/patches/mod.rs`
- Create: `src-tauri/src/patches/schema.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Register the module in `lib.rs`**

Open `src-tauri/src/lib.rs` and add the new module declaration alongside the existing ones (`mod calendar; mod chaser; mod commands; mod db; mod deps; mod repo;` — add at the matching depth, alphabetically):

```rust
mod patches;
```

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: error `file not found for module 'patches'`. Continue.

- [ ] **Step 2: Create `mod.rs`**

Create `src-tauri/src/patches/mod.rs` with:

```rust
//! Shared patch schema used by external clients (the MCP server)
//! and the in-app Inbox apply engine. See
//! `docs/specs/2026-05-22-blikplan-claude-connector-design.md`.

pub mod schema;
pub mod validate;

pub use schema::{Patch, PatchOp, PATCH_VERSION};
pub use validate::{validate_patch, ValidationError};
```

- [ ] **Step 3: Write failing tests for the schema**

Create `src-tauri/src/patches/schema.rs` with ONLY the test module first (we want the failing-test step to come before the implementation):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserialises_add_task_op() {
        let raw = json!({
            "op": "add_task",
            "phase_id": 42,
            "name": "Order vent ducting",
            "start_date": "2026-06-03",
            "duration_workdays": 3,
            "op_ref": "new_vent_task"
        });
        let op: PatchOp = serde_json::from_value(raw).unwrap();
        match op {
            PatchOp::AddTask { phase_id, name, op_ref, .. } => {
                assert_eq!(phase_id, 42);
                assert_eq!(name, "Order vent ducting");
                assert_eq!(op_ref.as_deref(), Some("new_vent_task"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn deserialises_shift_task_op() {
        let raw = json!({ "op": "shift_task", "task_id": 7, "by_days": -2 });
        let op: PatchOp = serde_json::from_value(raw).unwrap();
        assert!(matches!(op, PatchOp::ShiftTask { task_id: 7, by_days: -2 }));
    }

    #[test]
    fn deserialises_full_patch() {
        let raw = json!({
            "patch_version": 1,
            "summary": "Two changes from the meeting",
            "ops": [
                { "op": "append_note", "job_id": 1, "text": "hello" },
                { "op": "shift_task", "task_id": 7, "by_days": 1 }
            ]
        });
        let p: Patch = serde_json::from_value(raw).unwrap();
        assert_eq!(p.patch_version, 1);
        assert_eq!(p.ops.len(), 2);
        assert_eq!(p.summary, "Two changes from the meeting");
    }

    #[test]
    fn rejects_unknown_op() {
        let raw = json!({ "op": "delete_universe", "scope": "all" });
        let r: Result<PatchOp, _> = serde_json::from_value(raw);
        assert!(r.is_err());
    }

    #[test]
    fn rejects_unknown_patch_version() {
        let raw = json!({ "patch_version": 99, "summary": "x", "ops": [] });
        // Deserialisation succeeds (we accept the field as a number);
        // validate_patch in the next module will reject. Document that here:
        let p: Patch = serde_json::from_value(raw).unwrap();
        assert_eq!(p.patch_version, 99);
    }
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::schema`
Expected: compilation errors — `Patch` and `PatchOp` not defined.

- [ ] **Step 5: Implement the schema**

Prepend this to `src-tauri/src/patches/schema.rs` (above the existing `#[cfg(test)]` block):

```rust
use serde::{Deserialize, Serialize};

/// The patch document version. The current MCP server and the Inbox apply
/// engine both target this version. Mismatched versions are rejected by
/// `validate::validate_patch`.
pub const PATCH_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub patch_version: u32,
    pub summary: String,
    pub ops: Vec<PatchOp>,
}

/// All operations that may appear inside a patch. Each variant maps to
/// an existing Tauri command in `commands/*` — Plan 3's apply engine
/// dispatches accordingly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOp {
    AddTask {
        phase_id: i64,
        name: String,
        start_date: String,       // ISO 8601 YYYY-MM-DD
        duration_workdays: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contact_id: Option<i64>,
        /// Optional local handle so later ops in the same patch can
        /// reference this not-yet-created task.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        op_ref: Option<String>,
    },
    ShiftTask {
        task_id: i64,
        by_days: i64,
    },
    AddDependency {
        /// Either a real task id (`{ "task_id": 7 }`) or an `op_ref`
        /// from an earlier `add_task` in the same patch
        /// (`{ "op_ref": "new_vent_task" }`).
        predecessor: TaskRef,
        successor: TaskRef,
        #[serde(default = "default_dep_type")]
        dep_type: String,        // "FS", "SS", "FF", "SF"
        #[serde(default)]
        lag_days: i64,
    },
    AddChaser {
        task_id: i64,
        contact_id: i64,
        /// One of the user's three editable chaser templates.
        /// Plan 3 enforces that the value is one of the configured template keys.
        template: String,
    },
    AppendNote {
        /// Targets the job-level notes field. Phase-level / task-level
        /// note edits get separate ops if/when we need them.
        job_id: i64,
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskRef {
    Existing { task_id: i64 },
    Pending { op_ref: String },
}

fn default_dep_type() -> String {
    "FS".to_string()
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::schema`
Expected: all 5 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/patches/mod.rs src-tauri/src/patches/schema.rs
git commit -m "feat(patches): add shared Patch + PatchOp schema

Adds the v1 patch document format used by the MCP server (Plan 2)
and the Inbox apply engine (Plan 3). Pure data; no behaviour."
```

---

## Task 3: Implement the patch validator

**Files:**
- Create: `src-tauri/src/patches/validate.rs`

- [ ] **Step 1: Write failing tests for the validator**

Create `src-tauri/src/patches/validate.rs` with ONLY the test module first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::patches::schema::{Patch, PatchOp, TaskRef};

    fn ok_patch() -> Patch {
        Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AppendNote {
                job_id: 1,
                text: "hi".into(),
            }],
        }
    }

    #[test]
    fn accepts_well_formed_patch() {
        assert!(validate_patch(&ok_patch()).is_ok());
    }

    #[test]
    fn rejects_unknown_patch_version() {
        let mut p = ok_patch();
        p.patch_version = 99;
        let err = validate_patch(&p).unwrap_err();
        assert!(matches!(err, ValidationError::UnsupportedVersion(99)));
    }

    #[test]
    fn rejects_empty_ops() {
        let mut p = ok_patch();
        p.ops.clear();
        assert!(matches!(validate_patch(&p).unwrap_err(), ValidationError::EmptyOps));
    }

    #[test]
    fn rejects_empty_summary() {
        let mut p = ok_patch();
        p.summary = "".into();
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::EmptySummary
        ));
    }

    #[test]
    fn rejects_bad_date_in_add_task() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddTask {
                phase_id: 1,
                name: "n".into(),
                start_date: "not-a-date".into(),
                duration_workdays: 1,
                notes: None,
                contact_id: None,
                op_ref: None,
            }],
        };
        let err = validate_patch(&p).unwrap_err();
        assert!(matches!(err, ValidationError::BadDate { .. }));
    }

    #[test]
    fn rejects_non_positive_duration() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddTask {
                phase_id: 1,
                name: "n".into(),
                start_date: "2026-06-03".into(),
                duration_workdays: 0,
                notes: None,
                contact_id: None,
                op_ref: None,
            }],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::NonPositiveDuration { .. }
        ));
    }

    #[test]
    fn rejects_unknown_dep_type() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddDependency {
                predecessor: TaskRef::Existing { task_id: 1 },
                successor: TaskRef::Existing { task_id: 2 },
                dep_type: "XX".into(),
                lag_days: 0,
            }],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::BadDepType { .. }
        ));
    }

    #[test]
    fn rejects_dangling_op_ref() {
        // successor references "ghost" but no add_task in this patch declares op_ref="ghost"
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![PatchOp::AddDependency {
                predecessor: TaskRef::Existing { task_id: 1 },
                successor: TaskRef::Pending { op_ref: "ghost".into() },
                dep_type: "FS".into(),
                lag_days: 0,
            }],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::DanglingOpRef { .. }
        ));
    }

    #[test]
    fn accepts_valid_op_ref_chain() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![
                PatchOp::AddTask {
                    phase_id: 1,
                    name: "a".into(),
                    start_date: "2026-06-03".into(),
                    duration_workdays: 1,
                    notes: None,
                    contact_id: None,
                    op_ref: Some("a".into()),
                },
                PatchOp::AddDependency {
                    predecessor: TaskRef::Pending { op_ref: "a".into() },
                    successor: TaskRef::Existing { task_id: 99 },
                    dep_type: "FS".into(),
                    lag_days: 0,
                },
            ],
        };
        assert!(validate_patch(&p).is_ok());
    }

    #[test]
    fn rejects_duplicate_op_ref() {
        let p = Patch {
            patch_version: 1,
            summary: "x".into(),
            ops: vec![
                PatchOp::AddTask {
                    phase_id: 1, name: "a".into(),
                    start_date: "2026-06-03".into(), duration_workdays: 1,
                    notes: None, contact_id: None,
                    op_ref: Some("dup".into()),
                },
                PatchOp::AddTask {
                    phase_id: 1, name: "b".into(),
                    start_date: "2026-06-03".into(), duration_workdays: 1,
                    notes: None, contact_id: None,
                    op_ref: Some("dup".into()),
                },
            ],
        };
        assert!(matches!(
            validate_patch(&p).unwrap_err(),
            ValidationError::DuplicateOpRef { .. }
        ));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::validate`
Expected: compilation errors — `validate_patch` and `ValidationError` not defined.

- [ ] **Step 3: Implement the validator**

Prepend this to `src-tauri/src/patches/validate.rs`:

```rust
use chrono::NaiveDate;
use thiserror::Error;
use std::collections::HashSet;

use crate::patches::schema::{Patch, PatchOp, TaskRef, PATCH_VERSION};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("unsupported patch_version: got {0}, this binary supports {}", PATCH_VERSION)]
    UnsupportedVersion(u32),

    #[error("patch.ops must not be empty")]
    EmptyOps,

    #[error("patch.summary must not be empty")]
    EmptySummary,

    #[error("op #{op_index}: bad date {value:?} (expected YYYY-MM-DD)")]
    BadDate { op_index: usize, value: String },

    #[error("op #{op_index}: duration_workdays must be >= 1, got {got}")]
    NonPositiveDuration { op_index: usize, got: i64 },

    #[error("op #{op_index}: unknown dep_type {value:?} (expected FS/SS/FF/SF)")]
    BadDepType { op_index: usize, value: String },

    #[error("op #{op_index}: op_ref {value:?} does not match any add_task in this patch")]
    DanglingOpRef { op_index: usize, value: String },

    #[error("op_ref {value:?} declared more than once in the same patch")]
    DuplicateOpRef { value: String },

    #[error("op #{op_index}: empty name")]
    EmptyName { op_index: usize },

    #[error("op #{op_index}: empty text")]
    EmptyText { op_index: usize },
}

const VALID_DEP_TYPES: &[&str] = &["FS", "SS", "FF", "SF"];

/// Validates a patch's *structural* soundness — shape, value ranges,
/// op_ref consistency. Referential integrity against the database
/// (does task 42 exist?) is checked at apply-time in Plan 3, not here,
/// because the MCP server may run against a snapshot the user has
/// since edited.
pub fn validate_patch(p: &Patch) -> Result<(), ValidationError> {
    if p.patch_version != PATCH_VERSION {
        return Err(ValidationError::UnsupportedVersion(p.patch_version));
    }
    if p.summary.trim().is_empty() {
        return Err(ValidationError::EmptySummary);
    }
    if p.ops.is_empty() {
        return Err(ValidationError::EmptyOps);
    }

    // First pass: collect all declared op_refs, checking for duplicates.
    let mut declared: HashSet<String> = HashSet::new();
    for op in &p.ops {
        if let PatchOp::AddTask { op_ref: Some(r), .. } = op {
            if !declared.insert(r.clone()) {
                return Err(ValidationError::DuplicateOpRef { value: r.clone() });
            }
        }
    }

    // Second pass: per-op structural checks.
    for (i, op) in p.ops.iter().enumerate() {
        match op {
            PatchOp::AddTask {
                name, start_date, duration_workdays, ..
            } => {
                if name.trim().is_empty() {
                    return Err(ValidationError::EmptyName { op_index: i });
                }
                parse_date(i, start_date)?;
                if *duration_workdays < 1 {
                    return Err(ValidationError::NonPositiveDuration {
                        op_index: i,
                        got: *duration_workdays,
                    });
                }
            }
            PatchOp::ShiftTask { .. } => { /* nothing structural to check */ }
            PatchOp::AddDependency {
                predecessor, successor, dep_type, ..
            } => {
                if !VALID_DEP_TYPES.contains(&dep_type.as_str()) {
                    return Err(ValidationError::BadDepType {
                        op_index: i,
                        value: dep_type.clone(),
                    });
                }
                check_taskref(i, predecessor, &declared)?;
                check_taskref(i, successor, &declared)?;
            }
            PatchOp::AddChaser { .. } => { /* template-name check happens at apply-time */ }
            PatchOp::AppendNote { text, .. } => {
                if text.trim().is_empty() {
                    return Err(ValidationError::EmptyText { op_index: i });
                }
            }
        }
    }

    Ok(())
}

fn parse_date(op_index: usize, s: &str) -> Result<NaiveDate, ValidationError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| ValidationError::BadDate {
        op_index,
        value: s.to_string(),
    })
}

fn check_taskref(
    op_index: usize,
    r: &TaskRef,
    declared: &HashSet<String>,
) -> Result<(), ValidationError> {
    match r {
        TaskRef::Existing { .. } => Ok(()),
        TaskRef::Pending { op_ref } => {
            if declared.contains(op_ref) {
                Ok(())
            } else {
                Err(ValidationError::DanglingOpRef {
                    op_index,
                    value: op_ref.clone(),
                })
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml patches::validate`
Expected: all 10 tests PASS.

- [ ] **Step 5: Verify the whole crate still builds cleanly**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: ALL tests across the crate pass. Zero new warnings except possibly "unused" on `ValidationError` (the variant is referenced by tests but not yet by app code — that's expected; Plans 2 and 3 will use it).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/patches/validate.rs
git commit -m "feat(patches): add structural validator with typed errors

Checks patch_version, op shape, date format, duration range,
dependency type, op_ref consistency. Referential checks against
the live DB are deferred to apply-time (Plan 3)."
```

---

## Task 4: Mirror the schema in TypeScript

**Files:**
- Modify: `src/lib/types.ts`

- [ ] **Step 1: Append the new types**

Open `src/lib/types.ts` and append at the end of the file:

```typescript
// ---------------------------------------------------------------
// Patch schema — shared with the MCP server and the Inbox panel.
// Source of truth: src-tauri/src/patches/schema.rs (PATCH_VERSION = 1).
// Keep these two definitions in sync; any change here needs a
// matching change there.
// ---------------------------------------------------------------

export const PATCH_VERSION = 1;

export type TaskRef =
  | { task_id: number }
  | { op_ref: string };

export type PatchOp =
  | {
      op: 'add_task';
      phase_id: number;
      name: string;
      start_date: string;          // YYYY-MM-DD
      duration_workdays: number;
      notes?: string;
      contact_id?: number;
      op_ref?: string;
    }
  | {
      op: 'shift_task';
      task_id: number;
      by_days: number;
    }
  | {
      op: 'add_dependency';
      predecessor: TaskRef;
      successor: TaskRef;
      dep_type?: 'FS' | 'SS' | 'FF' | 'SF';   // default FS
      lag_days?: number;
    }
  | {
      op: 'add_chaser';
      task_id: number;
      contact_id: number;
      template: string;
    }
  | {
      op: 'append_note';
      job_id: number;
      text: string;
    };

export interface Patch {
  patch_version: number;
  summary: string;
  ops: PatchOp[];
}

export type PatchStatus =
  | 'proposed'
  | 'accepted'
  | 'applied'
  | 'rejected'
  | 'apply_failed'
  | 'expired';

export interface PendingPatch {
  id: string;
  job_id: number;
  patch: Patch;            // parsed from patch_json at the IPC boundary
  summary: string;
  source: string;          // 'mcp' for v1
  status: PatchStatus;
  created_at: number;      // unix seconds
  resolved_at: number | null;
  error: string | null;
}
```

- [ ] **Step 2: Verify the project still type-checks**

Run: `cd ~/Desktop/GanttBok && npm run check`
Expected: zero TypeScript errors. (If the script name differs, use whatever the existing convention is — check `package.json`.)

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.ts
git commit -m "feat(types): add Patch + PendingPatch TS types

Mirrors src-tauri/src/patches/schema.rs. Used by the Inbox panel
in Plan 3 to render diffs and by Plan 4's MCP config writer."
```

---

## Task 5: Final verification

- [ ] **Step 1: Run the entire Rust test suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: ALL tests pass. New tests added in this plan:
- `db::migrations::tests::pending_patches_table_exists_with_expected_columns`
- `db::migrations::tests::pending_patches_default_status_is_proposed`
- 5 tests in `patches::schema::tests`
- 10 tests in `patches::validate::tests`

- [ ] **Step 2: Run the entire TS check**

Run: `npm run check`
Expected: zero errors.

- [ ] **Step 3: Confirm git log shape**

Run: `git log --oneline -6`
Expected: five commits from this plan, in this order (newest first):
1. `feat(types): add Patch + PendingPatch TS types`
2. `feat(patches): add structural validator with typed errors`
3. `feat(patches): add shared Patch + PatchOp schema`
4. `feat(db): add pending_patches table (v7 migration)`
5. (previous commit, before this plan)

- [ ] **Step 4: Push (optional, defer to user)**

This plan touches `main`. JT's convention: stay local until the whole feature lands, then push as one branch. **Do not push automatically.** Tell the user the plan is complete and ask whether to push or keep local.

---

## Out of scope for Plan 1 (handled in later plans)

- MCP server implementation → Plan 2
- Inbox panel UI → Plan 3
- Apply engine (executing accepted patches) → Plan 3
- Connect-to-Claude install flow → Plan 4
- Any Tauri command exposure of patches over IPC → Plan 3
- `expired` auto-sweep / "Clear resolved" button → Plan 3
- JSON Schema file emission (for external validators) → Plan 2 if needed

## Risks logged for next plans

- The `add_chaser` op carries `template: String`. Plan 3 must enforce that the value matches one of the user's three configured chaser templates from v1.4 — that lookup table didn't exist when this plan was written. If v1.4 ships first, lift the validation into `validate.rs`; otherwise leave it as an apply-time check.
- The `AppendNote` op currently only targets job-level notes. If Plan 3's UI mockups want task-level note edits, add `AppendTaskNote { task_id, text }` as a sibling variant — pure additive, won't bump `PATCH_VERSION`.
