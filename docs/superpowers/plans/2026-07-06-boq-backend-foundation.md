# BoQ Backend Foundation — Implementation Plan (Plan 1 of 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the data layer for the Bill of Quantities feature — a job-scoped `boq_item` table, a per-job budget, the Rust repo/command surface, and a read-only MCP tool — with no frontend yet.

**Architecture:** Mirror the existing entity stack exactly (migration string → `models.rs` struct + enum → `repo/<entity>.rs` SQL → thin `commands/<entity>.rs` Tauri handlers registered in `lib.rs`). The Procurement enum copies the `TaskStatus` pattern. Content updates never touch procurement/delivery state (the "structural writes never revert status" guard from `task.rs`). The MCP `list_boq` read tool copies the `list_contacts` pattern.

**Tech Stack:** Rust, `rusqlite` (SQLite), `serde`, `chrono`, Tauri commands, `rmcp` (MCP), `schemars`.

**Spec:** `docs/superpowers/specs/2026-07-06-boq-page-design.md` (§3, §4, §6).

**Branch:** `feat/boq-page` (already created; the spec commit is its first commit).

---

## File structure

| File | Responsibility | Action |
|---|---|---|
| `src-tauri/src/db/migrations.rs` | v10 migration: `boq_item` table + `job.budget` | Modify |
| `src-tauri/src/db/models.rs` | `Procurement` enum, `BoqItem` struct | Modify |
| `src-tauri/src/repo/boq.rs` | All BoQ SQL (CRUD, set_procurement, budget) | Create |
| `src-tauri/src/repo/mod.rs` | declare `pub mod boq;` | Modify |
| `src-tauri/src/commands/boq.rs` | thin Tauri command handlers + payloads | Create |
| `src-tauri/src/commands/mod.rs` | declare `pub mod boq;` | Modify |
| `src-tauri/src/lib.rs` | register the new commands in `invoke_handler!` | Modify |
| `crates/blikplan-mcp/src/tools/read.rs` | `BoqItemSummary`, `ListBoqParams`, `query_list_boq` | Modify |
| `crates/blikplan-mcp/src/server.rs` | register the `list_boq` tool | Modify |
| `crates/blikplan-mcp/src/db.rs` | add `boq_item` + `job.budget` to `FIXTURE_SCHEMA_FOR_TEST` | Modify |

Money is stored as SQLite `REAL` → Rust `Option<f64>`. **Cost is never stored** — it is `qty × rate`, computed by the frontend (Plan 2/3) and written as a live formula on export (Plan 3).

---

## Task 1: Migration v10 — `boq_item` table + `job.budget`

**Files:**
- Modify: `src-tauri/src/db/migrations.rs` (append to `MIGRATIONS`, add tests)

- [ ] **Step 1: Write the failing tests**

Add these two tests inside the existing `mod tests { ... }` block in `src-tauri/src/db/migrations.rs` (after `new_job_defaults_to_auto_shift_enabled`):

```rust
    #[test]
    fn boq_item_table_has_expected_columns() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        let cols: Vec<String> = conn
            .prepare("PRAGMA table_info(boq_item)")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        for expected in &[
            "id", "job_id", "order_index", "item", "qty", "unit", "rate", "trade",
            "full_spec", "w_mm", "d_mm", "h_mm", "dia_mm", "supplier", "location",
            "procurement", "delivered_date", "lead_weeks", "invoice_no",
            "tut_ref_no", "organisation", "created_at",
        ] {
            assert!(
                cols.iter().any(|c| c == expected),
                "missing column {expected}; got {cols:?}"
            );
        }
    }

    #[test]
    fn boq_item_defaults_to_not_ordered_and_cascades() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();
        let job_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO boq_item (job_id, order_index, item) VALUES (?1, 0, 'Heat pump')",
            params![job_id],
        ).unwrap();

        let proc: String = conn.query_row(
            "SELECT procurement FROM boq_item WHERE job_id = ?1",
            params![job_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(proc, "not_ordered");

        // job.budget column exists and defaults to NULL
        let budget: Option<f64> = conn.query_row(
            "SELECT budget FROM job WHERE id = ?1", params![job_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(budget, None);

        // deleting the job cascades to its boq_items
        conn.execute("DELETE FROM job WHERE id = ?1", params![job_id]).unwrap();
        let remaining: i64 = conn.query_row(
            "SELECT COUNT(*) FROM boq_item", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(remaining, 0);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test --lib db::migrations::tests::boq_item -- --nocapture`
Expected: FAIL — compile error / `no such table: boq_item`.

- [ ] **Step 3: Append the v10 migration**

In `src-tauri/src/db/migrations.rs`, add this as the last element of the `MIGRATIONS` array (immediately after the v9 block, before the closing `];`):

```rust
    // v10 — Bill of Quantities: per-job line items + per-job budget.
    // boq_item is job-scoped like phase/no_work_day (cascade on job delete).
    // Money columns are REAL rand; `cost` is NOT stored (computed qty*rate).
    // `procurement` is the single status lifecycle; `update` must never touch it.
    r#"
    ALTER TABLE job ADD COLUMN budget REAL;

    CREATE TABLE boq_item (
        id             INTEGER PRIMARY KEY AUTOINCREMENT,
        job_id         INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
        order_index    INTEGER NOT NULL,
        item           TEXT    NOT NULL DEFAULT '',
        qty            REAL,
        unit           TEXT,
        rate           REAL,
        trade          TEXT,
        full_spec      TEXT,
        w_mm           REAL,
        d_mm           REAL,
        h_mm           REAL,
        dia_mm         REAL,
        supplier       TEXT,
        location       TEXT,
        procurement    TEXT    NOT NULL DEFAULT 'not_ordered'
                               CHECK (procurement IN ('not_ordered','quoted','ordered','delivered')),
        delivered_date TEXT,
        lead_weeks     REAL,
        invoice_no     TEXT,
        tut_ref_no     TEXT,
        organisation   TEXT,
        created_at     TEXT    NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX idx_boq_item_job ON boq_item(job_id, order_index);
    "#,
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test --lib db::migrations::tests -- --nocapture`
Expected: PASS — all migration tests green (including the two new ones).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/migrations.rs
git commit -m "feat(boq): v10 migration — boq_item table + job.budget"
```

---

## Task 2: `Procurement` enum + `BoqItem` struct

**Files:**
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `src-tauri/src/db/models.rs`:

```rust
    #[test]
    fn procurement_db_str_roundtrip() {
        for p in [
            Procurement::NotOrdered, Procurement::Quoted,
            Procurement::Ordered, Procurement::Delivered,
        ] {
            assert_eq!(Procurement::from_db_str(p.as_db_str()).unwrap(), p);
        }
        assert!(Procurement::from_db_str("bogus").is_err());
        assert_eq!(Procurement::default(), Procurement::NotOrdered);
    }

    #[test]
    fn boq_item_serializes_to_json() {
        let b = BoqItem {
            id: 1, job_id: 1, order_index: 0, item: "Heat pump".into(),
            qty: Some(1.0), unit: Some("item".into()), rate: Some(49444.25),
            trade: Some("HVAC".into()), full_spec: None,
            w_mm: None, d_mm: None, h_mm: None, dia_mm: None,
            supplier: Some("Hydrofire".into()), location: Some("Whole house".into()),
            procurement: Procurement::Ordered, delivered_date: None,
            lead_weeks: Some(4.0), invoice_no: None, tut_ref_no: None,
            organisation: Some("HydroFire".into()),
            created_at: "2026-07-06T10:00:00".into(),
        };
        let s = serde_json::to_string(&b).unwrap();
        let back: BoqItem = serde_json::from_str(&s).unwrap();
        assert_eq!(b, back);
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cd src-tauri && cargo test --lib db::models::tests::procurement db::models::tests::boq_item`
Expected: FAIL — `cannot find type Procurement`/`BoqItem`.

- [ ] **Step 3: Add the enum and structs**

In `src-tauri/src/db/models.rs`, after the `TaskStatus` impl/Default block (before `struct Job`), add:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Procurement {
    NotOrdered,
    Quoted,
    Ordered,
    Delivered,
}

impl Procurement {
    pub fn as_db_str(&self) -> &'static str {
        match self {
            Procurement::NotOrdered => "not_ordered",
            Procurement::Quoted     => "quoted",
            Procurement::Ordered    => "ordered",
            Procurement::Delivered  => "delivered",
        }
    }

    pub fn from_db_str(s: &str) -> Result<Self, String> {
        match s {
            "not_ordered" => Ok(Procurement::NotOrdered),
            "quoted"      => Ok(Procurement::Quoted),
            "ordered"     => Ok(Procurement::Ordered),
            "delivered"   => Ok(Procurement::Delivered),
            other         => Err(format!("unknown procurement status: {other}")),
        }
    }
}

impl Default for Procurement {
    fn default() -> Self { Procurement::NotOrdered }
}
```

Then, after the `Contact`/`NewContact` structs, add the `BoqItem` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoqItem {
    pub id: i64,
    pub job_id: i64,
    pub order_index: i64,
    pub item: String,
    pub qty: Option<f64>,
    pub unit: Option<String>,
    pub rate: Option<f64>,
    pub trade: Option<String>,
    pub full_spec: Option<String>,
    pub w_mm: Option<f64>,
    pub d_mm: Option<f64>,
    pub h_mm: Option<f64>,
    pub dia_mm: Option<f64>,
    pub supplier: Option<String>,
    pub location: Option<String>,
    #[serde(default)]
    pub procurement: Procurement,
    pub delivered_date: Option<String>,
    pub lead_weeks: Option<f64>,
    pub invoice_no: Option<String>,
    pub tut_ref_no: Option<String>,
    pub organisation: Option<String>,
    pub created_at: String,
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test --lib db::models::tests`
Expected: PASS — all model tests green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/models.rs
git commit -m "feat(boq): Procurement enum + BoqItem model"
```

---

## Task 3: `repo/boq.rs` — all BoQ SQL

**Files:**
- Create: `src-tauri/src/repo/boq.rs`
- Modify: `src-tauri/src/repo/mod.rs`

- [ ] **Step 1: Declare the module**

In `src-tauri/src/repo/mod.rs`, add at the end:

```rust
pub mod boq;
```

- [ ] **Step 2: Write the failing tests + the repo file skeleton**

Create `src-tauri/src/repo/boq.rs` with the full implementation AND its tests below. (The implementation and tests are written together here because the repo functions are the unit under test.)

```rust
use rusqlite::{Connection, params};
use crate::db::models::{BoqItem, Procurement};
use crate::{GbError, GbResult};

const SELECT_COLS: &str = "id, job_id, order_index, item, qty, unit, rate, trade, \
    full_spec, w_mm, d_mm, h_mm, dia_mm, supplier, location, procurement, \
    delivered_date, lead_weeks, invoice_no, tut_ref_no, organisation, created_at";

/// Append a blank line item to a job. order_index = current max + 1.
pub fn create(conn: &Connection, job_id: i64) -> GbResult<BoqItem> {
    let next_index: i64 = conn.query_row(
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM boq_item WHERE job_id = ?1",
        [job_id], |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO boq_item (job_id, order_index, item) VALUES (?1, ?2, '')",
        params![job_id, next_index],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> GbResult<BoqItem> {
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM boq_item WHERE id = ?1"),
        [id],
        row_to_boq_item,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("boq_item {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_by_job(conn: &Connection, job_id: i64) -> GbResult<Vec<BoqItem>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM boq_item WHERE job_id = ?1 ORDER BY order_index ASC"),
    )?;
    let rows = stmt.query_map([job_id], row_to_boq_item)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

/// Update CONTENT fields only. Deliberately does NOT write `procurement` or
/// `delivered_date` — those are owned by `set_procurement`, mirroring the
/// task.rs guard so grid edits never clobber procurement state.
pub fn update(conn: &Connection, b: &BoqItem) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE boq_item SET
            item = ?1, qty = ?2, unit = ?3, rate = ?4, trade = ?5, full_spec = ?6,
            w_mm = ?7, d_mm = ?8, h_mm = ?9, dia_mm = ?10, supplier = ?11,
            location = ?12, lead_weeks = ?13, invoice_no = ?14, tut_ref_no = ?15,
            organisation = ?16
         WHERE id = ?17",
        params![
            b.item, b.qty, b.unit, b.rate, b.trade, b.full_spec,
            b.w_mm, b.d_mm, b.h_mm, b.dia_mm, b.supplier,
            b.location, b.lead_weeks, b.invoice_no, b.tut_ref_no,
            b.organisation, b.id,
        ],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {}", b.id))); }
    Ok(())
}

/// The ONLY writer of procurement/delivered_date.
/// When status == Delivered, `delivered_date` is stored; otherwise it is cleared.
pub fn set_procurement(
    conn: &Connection,
    id: i64,
    status: Procurement,
    delivered_date: Option<&str>,
) -> GbResult<()> {
    let stored_date = if status == Procurement::Delivered { delivered_date } else { None };
    let n = conn.execute(
        "UPDATE boq_item SET procurement = ?1, delivered_date = ?2 WHERE id = ?3",
        params![status.as_db_str(), stored_date, id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {id}"))); }
    Ok(())
}

pub fn reorder(conn: &Connection, id: i64, order_index: i64) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE boq_item SET order_index = ?1 WHERE id = ?2",
        params![order_index, id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {id}"))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM boq_item WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {id}"))); }
    Ok(())
}

pub fn set_job_budget(conn: &Connection, job_id: i64, budget: Option<f64>) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE job SET budget = ?1 WHERE id = ?2",
        params![budget, job_id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("job {job_id}"))); }
    Ok(())
}

pub fn get_job_budget(conn: &Connection, job_id: i64) -> GbResult<Option<f64>> {
    conn.query_row("SELECT budget FROM job WHERE id = ?1", [job_id], |r| r.get(0))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("job {job_id}")),
            other => GbError::Sqlite(other),
        })
}

fn row_to_boq_item(r: &rusqlite::Row) -> rusqlite::Result<BoqItem> {
    let proc_str: String = r.get(15)?;
    Ok(BoqItem {
        id: r.get(0)?,
        job_id: r.get(1)?,
        order_index: r.get(2)?,
        item: r.get(3)?,
        qty: r.get(4)?,
        unit: r.get(5)?,
        rate: r.get(6)?,
        trade: r.get(7)?,
        full_spec: r.get(8)?,
        w_mm: r.get(9)?,
        d_mm: r.get(10)?,
        h_mm: r.get(11)?,
        dia_mm: r.get(12)?,
        supplier: r.get(13)?,
        location: r.get(14)?,
        procurement: Procurement::from_db_str(&proc_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(15, "procurement".into(), rusqlite::types::Type::Text))?,
        delivered_date: r.get(16)?,
        lead_weeks: r.get(17)?,
        invoice_no: r.get(18)?,
        tut_ref_no: r.get(19)?,
        organisation: r.get(20)?,
        created_at: r.get(21)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Procurement;

    fn seed_job(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn create_appends_and_increments_order_index() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let a = create(&conn, job).unwrap();
        let b = create(&conn, job).unwrap();
        assert_eq!(a.order_index, 0);
        assert_eq!(b.order_index, 1);
        assert_eq!(a.procurement, Procurement::NotOrdered);
        assert_eq!(list_by_job(&conn, job).unwrap().len(), 2);
    }

    #[test]
    fn update_changes_content_but_preserves_procurement() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let mut item = create(&conn, job).unwrap();

        // Move it to Ordered via the dedicated setter.
        set_procurement(&conn, item.id, Procurement::Ordered, None).unwrap();

        // Now a content edit that (maliciously) carries a different procurement value.
        item.item = "Heat pump".into();
        item.rate = Some(49444.25);
        item.procurement = Procurement::NotOrdered; // must be ignored by update()
        update(&conn, &item).unwrap();

        let fetched = get(&conn, item.id).unwrap();
        assert_eq!(fetched.item, "Heat pump");
        assert_eq!(fetched.rate, Some(49444.25));
        assert_eq!(fetched.procurement, Procurement::Ordered, "update must not clobber procurement");
    }

    #[test]
    fn set_procurement_sets_and_clears_delivered_date() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let item = create(&conn, job).unwrap();

        set_procurement(&conn, item.id, Procurement::Delivered, Some("2026-07-06")).unwrap();
        assert_eq!(get(&conn, item.id).unwrap().delivered_date, Some("2026-07-06".into()));

        // Moving back off Delivered clears the date.
        set_procurement(&conn, item.id, Procurement::Ordered, None).unwrap();
        assert_eq!(get(&conn, item.id).unwrap().delivered_date, None);
    }

    #[test]
    fn budget_set_and_get_roundtrip() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        assert_eq!(get_job_budget(&conn, job).unwrap(), None);
        set_job_budget(&conn, job, Some(2_000_000.0)).unwrap();
        assert_eq!(get_job_budget(&conn, job).unwrap(), Some(2_000_000.0));
    }

    #[test]
    fn delete_removes_row() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let item = create(&conn, job).unwrap();
        delete(&conn, item.id).unwrap();
        assert!(get(&conn, item.id).is_err());
    }
}
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `cd src-tauri && cargo test --lib repo::boq::tests -- --nocapture`
Expected: PASS — all five repo tests green.

> If `crate::db::connection::open_in_memory()` does not exist or does not apply migrations, replace it in the tests with:
> ```rust
> let conn = Connection::open_in_memory().unwrap();
> crate::db::migrations::apply_migrations(&conn).unwrap();
> ```
> (Check `src-tauri/src/db/connection.rs` — `models.rs` tests already use `crate::db::connection::open_in_memory()`, so it should exist and pre-migrate.)

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/repo/boq.rs src-tauri/src/repo/mod.rs
git commit -m "feat(boq): repo layer — CRUD, set_procurement guard, budget"
```

---

## Task 4: `commands/boq.rs` — Tauri command handlers

**Files:**
- Create: `src-tauri/src/commands/boq.rs`
- Modify: `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`

Commands are thin pass-throughs (the existing codebase does not unit-test command handlers — the repo layer carries the test coverage). Verification here is a clean compile + all tests still green.

- [ ] **Step 1: Declare the module**

In `src-tauri/src/commands/mod.rs`, add to the module list (after `pub mod claude;`):

```rust
pub mod boq;
```

- [ ] **Step 2: Write the command handlers**

Create `src-tauri/src/commands/boq.rs`:

```rust
use chrono::Local;
use serde::Deserialize;
use tauri::State;

use crate::commands::Db;
use crate::db::models::{BoqItem, Procurement};
use crate::repo::boq as boq_repo;
use crate::{GbError, GbResult};

#[tauri::command]
pub fn list_boq_items(db: State<Db>, job_id: i64) -> GbResult<Vec<BoqItem>> {
    let conn = db.0.lock().unwrap();
    boq_repo::list_by_job(&conn, job_id)
}

#[tauri::command]
pub fn create_boq_item(db: State<Db>, job_id: i64) -> GbResult<BoqItem> {
    let conn = db.0.lock().unwrap();
    boq_repo::create(&conn, job_id)
}

/// Content update. Never changes procurement/delivered_date (repo guard).
#[tauri::command]
pub fn update_boq_item(db: State<Db>, args: BoqItem) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::update(&conn, &args)
}

#[derive(Debug, Deserialize)]
pub struct SetProcurementArgs {
    pub id: i64,
    /// "not_ordered" | "quoted" | "ordered" | "delivered"
    pub procurement: String,
    /// ISO date; only used when procurement == "delivered".
    /// If omitted while delivering, today's date is used.
    pub delivered_date: Option<String>,
}

#[tauri::command]
pub fn set_boq_procurement(db: State<Db>, args: SetProcurementArgs) -> GbResult<()> {
    let status = Procurement::from_db_str(&args.procurement)
        .map_err(GbError::Validation)?;
    let today = Local::now().naive_local().date().format("%Y-%m-%d").to_string();
    let delivered_date: Option<String> = if status == Procurement::Delivered {
        Some(args.delivered_date.unwrap_or(today))
    } else {
        None
    };
    let conn = db.0.lock().unwrap();
    boq_repo::set_procurement(&conn, args.id, status, delivered_date.as_deref())
}

#[derive(Debug, Deserialize)]
pub struct ReorderArgs {
    pub id: i64,
    pub order_index: i64,
}

#[tauri::command]
pub fn reorder_boq_item(db: State<Db>, args: ReorderArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::reorder(&conn, args.id, args.order_index)
}

#[tauri::command]
pub fn delete_boq_item(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::delete(&conn, id)
}

#[derive(Debug, Deserialize)]
pub struct SetBudgetArgs {
    pub job_id: i64,
    pub budget: Option<f64>,
}

#[tauri::command]
pub fn set_job_budget(db: State<Db>, args: SetBudgetArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    boq_repo::set_job_budget(&conn, args.job_id, args.budget)
}

#[tauri::command]
pub fn get_job_budget(db: State<Db>, job_id: i64) -> GbResult<Option<f64>> {
    let conn = db.0.lock().unwrap();
    boq_repo::get_job_budget(&conn, job_id)
}
```

- [ ] **Step 3: Register the commands**

In `src-tauri/src/lib.rs`, inside `tauri::generate_handler![ ... ]`, add after the `commands::claude::disconnect_from_claude,` line:

```rust
            commands::boq::list_boq_items,
            commands::boq::create_boq_item,
            commands::boq::update_boq_item,
            commands::boq::set_boq_procurement,
            commands::boq::reorder_boq_item,
            commands::boq::delete_boq_item,
            commands::boq::set_job_budget,
            commands::boq::get_job_budget,
```

- [ ] **Step 4: Verify it compiles and all tests pass**

Run: `cd src-tauri && cargo build && cargo test --lib`
Expected: build succeeds; all existing + new tests green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/boq.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(boq): Tauri command handlers + registration"
```

---

## Task 5: MCP `list_boq` read tool

**Files:**
- Modify: `crates/blikplan-mcp/src/db.rs` (fixture schema)
- Modify: `crates/blikplan-mcp/src/tools/read.rs` (summary type, params, query fn, test)
- Modify: `crates/blikplan-mcp/src/server.rs` (register tool)

- [ ] **Step 1: Extend the test fixture schema**

In `crates/blikplan-mcp/src/db.rs`, inside `FIXTURE_SCHEMA_FOR_TEST`, (a) add `budget REAL` to the `job` table definition (append it as the last column before the closing `)` — remember to add a comma after the current last column `region TEXT NOT NULL DEFAULT 'ZA'`), and (b) add the `boq_item` table at the end of the schema string (before the closing `"#`):

```sql

CREATE TABLE boq_item (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id         INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    order_index    INTEGER NOT NULL,
    item           TEXT    NOT NULL DEFAULT '',
    qty            REAL,
    unit           TEXT,
    rate           REAL,
    trade          TEXT,
    full_spec      TEXT,
    w_mm           REAL,
    d_mm           REAL,
    h_mm           REAL,
    dia_mm         REAL,
    supplier       TEXT,
    location       TEXT,
    procurement    TEXT    NOT NULL DEFAULT 'not_ordered'
                           CHECK (procurement IN ('not_ordered','quoted','ordered','delivered')),
    delivered_date TEXT,
    lead_weeks     REAL,
    invoice_no     TEXT,
    tut_ref_no     TEXT,
    organisation   TEXT,
    created_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);
```

- [ ] **Step 2: Write the failing test + the query code**

In `crates/blikplan-mcp/src/tools/read.rs`:

(a) Add the output summary type, after the `DepSummary` struct:

```rust
#[derive(Debug, Serialize)]
pub struct BoqItemSummary {
    pub id: i64,
    pub item: String,
    pub qty: Option<f64>,
    pub rate: Option<f64>,
    /// qty * rate when both present, else null. Convenience so callers don't recompute.
    pub cost: Option<f64>,
    pub trade: Option<String>,
    pub supplier: Option<String>,
    pub location: Option<String>,
    pub procurement: String,
}
```

(b) Add the params struct, after `GetTaskParams`:

```rust
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListBoqParams {
    /// Job id whose BoQ line items to list.
    pub job_id: i64,
}
```

(c) Add the query function, after `query_list_contacts`:

```rust
pub fn query_list_boq(conn: &Connection, job_id: i64) -> Result<Vec<BoqItemSummary>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, item, qty, rate, trade, supplier, location, procurement
         FROM boq_item WHERE job_id = ?1 ORDER BY order_index ASC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([job_id], |r| {
        let qty: Option<f64> = r.get(2)?;
        let rate: Option<f64> = r.get(3)?;
        Ok(BoqItemSummary {
            id: r.get(0)?,
            item: r.get(1)?,
            qty,
            rate,
            cost: match (qty, rate) { (Some(q), Some(rt)) => Some(q * rt), _ => None },
            trade: r.get(4)?,
            supplier: r.get(5)?,
            location: r.get(6)?,
            procurement: r.get(7)?,
        })
    }).map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}
```

(d) Add a test. If `read.rs` has no `#[cfg(test)] mod tests`, add one at the end of the file; otherwise add the test into the existing module:

```rust
#[cfg(test)]
mod boq_tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn query_list_boq_returns_items_with_cost() {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::apply_migrations_for_test(&conn);
        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();
        let job_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO boq_item (job_id, order_index, item, qty, rate, trade, procurement)
             VALUES (?1, 0, 'Heat pump', 1, 49444.25, 'HVAC', 'ordered')",
            rusqlite::params![job_id],
        ).unwrap();

        let out = query_list_boq(&conn, job_id).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].item, "Heat pump");
        assert_eq!(out[0].cost, Some(49444.25));
        assert_eq!(out[0].procurement, "ordered");
    }
}
```

- [ ] **Step 3: Run the test to verify it fails, then passes after Step 4 wiring**

Run: `cd crates/blikplan-mcp && cargo test query_list_boq -- --nocapture`
Expected first run: PASS is fine once (a)–(d) compile; if it fails to compile because `apply_migrations_for_test`/`db` module path differs, adjust the `crate::db::` path to match how other tests in this crate reference it (grep: `rg apply_migrations_for_test crates/blikplan-mcp`).

- [ ] **Step 4: Register the MCP tool**

In `crates/blikplan-mcp/src/server.rs`:

(a) extend the import from `crate::tools::read` to include the new items:

```rust
use crate::tools::read::{
    GetJobParams, ListTasksParams, GetTaskParams, SearchParams, TodayParams, ListBoqParams,
    query_list_jobs, query_get_job,
    query_list_tasks, query_get_task, query_list_contacts,
    query_search, query_today, query_list_boq,
};
```

(b) add the tool method inside the `#[tool_router] impl BlikPlanServer` block, after `today`:

```rust
    #[tool(description = "List Bill of Quantities line items for a job. Returns id, item, qty, rate, cost, trade, supplier, location, procurement (not_ordered|quoted|ordered|delivered).")]
    async fn list_boq(&self, Parameters(p): Parameters<ListBoqParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_list_boq(&conn, p.job_id) {
            Ok(items) => serde_json::to_string_pretty(&items).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }
```

- [ ] **Step 5: Verify the whole MCP crate builds and tests pass**

Run: `cd crates/blikplan-mcp && cargo build && cargo test`
Expected: build succeeds; all tests (existing + `query_list_boq_returns_items_with_cost`) green.

- [ ] **Step 6: Commit**

```bash
git add crates/blikplan-mcp/src/db.rs crates/blikplan-mcp/src/tools/read.rs crates/blikplan-mcp/src/server.rs
git commit -m "feat(boq): MCP list_boq read tool + fixture schema"
```

---

## Task 6: Full workspace verification

- [ ] **Step 1: Build and test the entire workspace**

Run: `cd ~/Desktop/GanttBok && cargo test --workspace && cargo build --workspace`
Expected: all crates compile; all tests green.

- [ ] **Step 2: Confirm the MCP sidecar still cross-builds (optional, if toolchain present)**

Run: `npm run build:mcp-sidecar`
Expected: sidecar binaries rebuild without error. (If the cross-compile toolchain isn't installed locally, skip — it runs in the release pipeline.)

- [ ] **Step 3: Commit any binary artifacts if the sidecar was rebuilt**

```bash
git add src-tauri/binaries/blikplan-mcp-* 2>/dev/null || true
git commit -m "chore(boq): rebuild MCP sidecar with list_boq" || echo "nothing to commit"
```

---

## Done criteria

- `boq_item` table + `job.budget` exist (migration v10) and cascade-delete with the job.
- `Procurement` enum + `BoqItem` model round-trip through JSON.
- Repo layer: create/get/list/update/set_procurement/reorder/delete/budget, with `update` proven not to clobber procurement.
- Eight Tauri commands registered and compiling.
- MCP `list_boq` tool returns items (with computed cost) and its fixture test passes.
- `cargo test --workspace` fully green.

**Next:** Plan 2 — the BoQ view & grid (frontend: view switch, hand-rolled grid, inline edit, procurement control + delivery approval, wired to these commands).
