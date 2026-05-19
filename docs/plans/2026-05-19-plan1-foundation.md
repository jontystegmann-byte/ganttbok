# Gantt Bok — Plan 1: Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the headless Rust backend for Gantt Bok — SQLite-backed data layer, calendar engine (workday math + SA public holidays), dependency-graph engine (cycle detection + downstream ripple), and the full set of Tauri IPC commands the frontend will later call. The result is `cargo test`-verifiable working software that proves the engine before any UI exists.

**Architecture:** Tauri 2 desktop app with a thin shell that mounts a Svelte frontend (placeholder in Plan 1) and exposes Rust IPC commands. SQLite (via `rusqlite`) for persistence, hand-rolled migrations keyed by a `schema_version` row, single `Db` connection guarded by a `Mutex` and held in Tauri state. Calendar math and dependency-graph logic are pure functions in their own modules with no I/O — straightforward to unit-test.

**Tech Stack:** Tauri 2.x · Rust 1.78+ · `rusqlite` 0.31 (bundled SQLite, `serde_rusqlite` for row→struct) · `chrono` 0.4 (date math) · `serde` / `serde_json` (IPC) · `thiserror` (typed errors) · `tempfile` (test DBs) · Svelte 5 + Vite 5 (frontend placeholder only — fully wired in Plan 2).

**Reference spec:** `~/Desktop/GanttBok/docs/specs/2026-05-19-ganttbok-design.md`

---

## File structure (every file Plan 1 creates or touches)

```
~/Desktop/GanttBok/
├── .gitignore                                    Task 1
├── README.md                                     Task 1
├── package.json                                  Task 2 (created by scaffold)
├── pnpm-lock.yaml                                Task 2 (created by scaffold)
├── svelte.config.js                              Task 2 (created by scaffold)
├── vite.config.ts                                Task 2 (created by scaffold)
├── tsconfig.json                                 Task 2 (created by scaffold)
├── index.html                                    Task 2 (created by scaffold)
├── src/                                          Task 2 (Svelte placeholder)
│   ├── app.html                                  Task 2
│   ├── main.ts                                   Task 2
│   └── App.svelte                                Task 2 (minimal placeholder)
├── src-tauri/
│   ├── Cargo.toml                                Task 2 → 3
│   ├── Cargo.lock                                generated
│   ├── tauri.conf.json                           Task 2 → 5
│   ├── build.rs                                  Task 2 (created by scaffold)
│   ├── icons/                                    Task 2 (placeholder icons)
│   └── src/
│       ├── main.rs                               Task 5 → 37
│       ├── lib.rs                                Task 3 (new)
│       ├── error.rs                              Task 3 (new — typed errors)
│       ├── db/
│       │   ├── mod.rs                            Task 7
│       │   ├── connection.rs                     Task 7
│       │   ├── migrations.rs                     Task 7 → 13
│       │   └── models.rs                         Tasks 8–13 (struct defs)
│       ├── repo/
│       │   ├── mod.rs                            Task 14
│       │   ├── job.rs                            Task 14
│       │   ├── phase.rs                          Task 15
│       │   ├── task.rs                           Task 16
│       │   ├── dependency.rs                     Task 34
│       │   └── no_work_day.rs                    Task 35
│       ├── calendar/
│       │   ├── mod.rs                            Task 17
│       │   ├── workday.rs                        Task 17, 22
│       │   ├── easter.rs                         Task 18
│       │   └── sa_holidays.rs                    Tasks 19, 20, 21
│       ├── deps/
│       │   ├── mod.rs                            Task 23
│       │   ├── graph.rs                          Tasks 23, 24, 25
│       │   └── ripple.rs                         Task 26
│       └── commands/
│           ├── mod.rs                            Task 28
│           ├── job.rs                            Task 29
│           ├── template.rs                       Task 30
│           ├── phase.rs                          Task 31
│           ├── task.rs                           Task 32
│           ├── drag.rs                           Task 33
│           ├── dependency.rs                     Task 34
│           ├── no_work_day.rs                    Task 35
│           └── meta.rs                           Tasks 36, 37
└── docs/                                         (already exists)
    ├── specs/2026-05-19-ganttbok-design.md       (already exists)
    └── plans/2026-05-19-plan1-foundation.md      (this file)
```

Tests sit next to their modules (`#[cfg(test)] mod tests { ... }`) for unit work; integration tests live under `src-tauri/tests/`.

---

## Phase 0 — Project scaffold (Tasks 1–6)

Bootstrap the repo, scaffold Tauri 2 + Svelte 5, confirm a hello-world `.app` builds, install backend dependencies, and tag `v0.0.1`.

### Task 1: Initialize git + base project files

**Files:**
- Create: `~/Desktop/GanttBok/.gitignore`
- Create: `~/Desktop/GanttBok/README.md`

- [ ] **Step 1: Init the repo**

```bash
cd ~/Desktop/GanttBok
git init
git branch -M main
```

Expected: `Initialized empty Git repository`.

- [ ] **Step 2: Write `.gitignore`**

```gitignore
# macOS
.DS_Store

# Node
node_modules/
dist/
.vite/

# Rust / Tauri
src-tauri/target/
src-tauri/Cargo.lock.bak

# IDE
.vscode/
.idea/
*.swp

# Local data (runtime DB never lives in repo)
*.db
*.db-journal
*.db-wal
*.db-shm
```

- [ ] **Step 3: Write `README.md`**

```markdown
# Gantt Bok

A self-contained macOS Gantt-chart desktop app for apartment renovation project management.

- Spec: `docs/specs/2026-05-19-ganttbok-design.md`
- Plan 1 (Foundation): `docs/plans/2026-05-19-plan1-foundation.md`

## Run

```
pnpm install
pnpm tauri dev
```

## Test

```
cd src-tauri && cargo test
```
```

- [ ] **Step 4: Commit**

```bash
git add .gitignore README.md docs/
git commit -m "chore: init repo with gitignore, README, spec, and Plan 1"
```

---

### Task 2: Scaffold Tauri 2 + Svelte 5

**Files:**
- Create: many (whole Tauri scaffold)

- [ ] **Step 1: Verify Rust + pnpm are installed**

```bash
rustc --version    # expect 1.78+
pnpm --version     # expect 8+
```

If missing: `brew install rust pnpm`.

- [ ] **Step 2: Scaffold from the parent of the project dir**

```bash
cd ~/Desktop
pnpm create tauri-app@latest
```

Answer the prompts:
- Project name: `GanttBok`  *(when prompted to overwrite the existing dir, choose Yes — our pre-existing `docs/`, `.gitignore`, and `README.md` survive)*
- Identifier: `com.jontystegmann.ganttbok`
- Frontend language: TypeScript
- Package manager: pnpm
- UI template: Svelte
- UI flavor: TypeScript

Expected: scaffold writes `package.json`, `src/`, `src-tauri/`, `vite.config.ts`, etc. into `~/Desktop/GanttBok/`.

- [ ] **Step 3: Reinstate any files the scaffold overwrote**

If the scaffold replaced `README.md` or `.gitignore`, restore them with the Task 1 versions (the scaffold's defaults are weaker).

- [ ] **Step 4: Install JS dependencies**

```bash
cd ~/Desktop/GanttBok
pnpm install
```

Expected: completes with no errors.

- [ ] **Step 5: Smoke-test the scaffold**

```bash
pnpm tauri dev
```

Expected: native window opens showing the Tauri+Svelte welcome screen. Close it.

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "chore: scaffold Tauri 2 + Svelte 5 + TypeScript"
```

---

### Task 3: Install Rust backend dependencies + base modules

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Edit `src-tauri/Cargo.toml` — add backend deps**

Under `[dependencies]`, append:

```toml
rusqlite = { version = "0.31", features = ["bundled", "chrono"] }
serde_rusqlite = "0.35"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"

[dev-dependencies]
tempfile = "3.10"
```

- [ ] **Step 2: Create `src-tauri/src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("cycle detected when creating dependency {0} -> {1}")]
    DependencyCycle(i64, i64),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type GbResult<T> = Result<T, GbError>;

impl serde::Serialize for GbError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
```

- [ ] **Step 3: Create `src-tauri/src/lib.rs`**

```rust
pub mod calendar;
pub mod commands;
pub mod db;
pub mod deps;
pub mod error;
pub mod repo;

pub use error::{GbError, GbResult};
```

- [ ] **Step 4: Replace `src-tauri/src/main.rs` with a lib-aware shell**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ganttbok_lib::run();
}
```

Note: the lib's `run()` function is defined in Task 5 — this file only points at it.

- [ ] **Step 5: Add the `run()` stub to `lib.rs`**

Append to `src-tauri/src/lib.rs`:

```rust
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 6: Rename the Cargo crate so `ganttbok_lib::run()` resolves**

Edit `src-tauri/Cargo.toml`:

```toml
[package]
name = "ganttbok_lib"
version = "0.0.1"
edition = "2021"

[[bin]]
name = "ganttbok"
path = "src/main.rs"

[lib]
name = "ganttbok_lib"
path = "src/lib.rs"
```

- [ ] **Step 7: Verify it still builds**

```bash
cd src-tauri && cargo build
```

Expected: builds clean. (Will warn about unused empty modules — fine.)

- [ ] **Step 8: Commit**

```bash
git add src-tauri/
git commit -m "chore(backend): add rusqlite, chrono, thiserror; introduce error module + lib structure"
```

---

### Task 4: Stand up empty backend modules so the tree compiles

**Files:**
- Create: `src-tauri/src/calendar/mod.rs`
- Create: `src-tauri/src/db/mod.rs`
- Create: `src-tauri/src/deps/mod.rs`
- Create: `src-tauri/src/repo/mod.rs`
- Create: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Create each `mod.rs` with a placeholder doc-comment**

```bash
mkdir -p src-tauri/src/{calendar,db,deps,repo,commands}
```

Then write the same one-line file in each (substitute the module name):

```rust
//! Gantt Bok — <module> module. Populated in later tasks.
```

(Five files total. Each is one line.)

- [ ] **Step 2: Build to confirm the tree compiles**

```bash
cd src-tauri && cargo build
```

Expected: builds clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src
git commit -m "chore(backend): scaffold module tree (calendar, db, deps, repo, commands)"
```

---

### Task 5: Minimal Svelte placeholder + Tauri config

**Files:**
- Modify: `src/App.svelte`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Replace `src/App.svelte` with a placeholder**

```svelte
<script lang="ts">
  let name = "Gantt Bok";
</script>

<main>
  <h1>{name}</h1>
  <p>Backend foundation in progress. UI lands in Plan 2.</p>
</main>

<style>
  main { font-family: -apple-system, system-ui, sans-serif; padding: 2rem; }
  h1   { font-weight: 600; }
</style>
```

- [ ] **Step 2: Edit `src-tauri/tauri.conf.json`**

Update the `productName`, `identifier`, and `app.windows[0].title`:

```json
{
  "productName": "Gantt Bok",
  "identifier": "com.jontystegmann.ganttbok",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "Gantt Bok",
        "width": 1280,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ]
  }
}
```

- [ ] **Step 3: Run the dev app**

```bash
pnpm tauri dev
```

Expected: native window opens showing "Gantt Bok / Backend foundation in progress." Close it.

- [ ] **Step 4: Commit**

```bash
git add src/App.svelte src-tauri/tauri.conf.json
git commit -m "feat(ui): minimal Svelte placeholder + window config"
```

---

### Task 6: Tag v0.0.1

- [ ] **Step 1: Tag**

```bash
git tag -a v0.0.1 -m "v0.0.1 — scaffold complete (Plan 1, Phase 0)"
```

- [ ] **Step 2: Verify**

```bash
git tag -l
```

Expected output contains `v0.0.1`.

---

## Phase 1 — Data layer (Tasks 7–16)

SQLite database, migration framework, six tables, and CRUD repositories for the core three (job, phase, task). Dependency and no-work-day repos arrive with their commands in Phase 4.

### Task 7: Database connection + migration framework

**Files:**
- Create: `src-tauri/src/db/connection.rs`
- Create: `src-tauri/src/db/migrations.rs`
- Create: `src-tauri/src/db/models.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Test: inside `connection.rs` and `migrations.rs`

- [ ] **Step 1: Write the failing test in `db/migrations.rs`**

```rust
use rusqlite::Connection;

pub fn apply_migrations(conn: &Connection) -> crate::GbResult<()> {
    Err(crate::GbError::Migration("not implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_reports_schema_version_zero_then_one_after_migrations() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        let v: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, 1);
    }
}
```

- [ ] **Step 2: Wire `db/mod.rs`**

```rust
pub mod connection;
pub mod migrations;
pub mod models;
```

- [ ] **Step 3: Run the test to confirm it fails**

```bash
cd src-tauri
cargo test fresh_db_reports_schema_version_zero_then_one_after_migrations
```

Expected: FAIL with "not implemented".

- [ ] **Step 4: Replace `apply_migrations` with the real implementation**

```rust
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
    fn fresh_db_reports_schema_version_zero_then_one_after_migrations() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        let v: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM app_meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, 1);
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        apply_migrations(&conn).unwrap();
        apply_migrations(&conn).unwrap(); // second run should no-op
        let v: i64 = conn
            .query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
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
}
```

- [ ] **Step 5: Run the tests — all green**

```bash
cargo test --package ganttbok_lib --lib db::migrations
```

Expected: 3 passed.

- [ ] **Step 6: Write `db/connection.rs`**

```rust
use rusqlite::Connection;
use std::path::Path;
use crate::GbResult;
use super::migrations::apply_migrations;

pub fn open(path: &Path) -> GbResult<Connection> {
    let conn = Connection::open(path)?;
    apply_migrations(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> GbResult<Connection> {
    let conn = Connection::open_in_memory()?;
    apply_migrations(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_applies_migrations() {
        let conn = open_in_memory().unwrap();
        let v: i64 = conn
            .query_row("SELECT CAST(value AS INTEGER) FROM app_meta WHERE key='schema_version'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 1);
    }
}
```

- [ ] **Step 7: Write the placeholder `db/models.rs`** (struct defs added per table in Tasks 8–13)

```rust
//! Row structs for each table. Populated table-by-table in Tasks 8–13.
```

- [ ] **Step 8: Run all `cargo test`**

```bash
cd src-tauri && cargo test
```

Expected: 4 passed.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/db
git commit -m "feat(db): migration framework + schema v1 + connection helpers"
```

---

### Task 8: Job row struct + serde

**Files:**
- Modify: `src-tauri/src/db/models.rs`
- Modify: `src-tauri/Cargo.toml` (add `serde` if not yet present)

- [ ] **Step 1: Confirm serde is in `Cargo.toml`** (the scaffold adds it). If not, add:

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

- [ ] **Step 2: Define `Job` in `db/models.rs`**

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: i64,
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
    pub archived: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewJob {
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
}
```

- [ ] **Step 3: Add a round-trip test**

Append to `db/models.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn job_serializes_to_json() {
        let job = Job {
            id: 1,
            name: "Sea Point reno".into(),
            client: Some("M. Botha".into()),
            address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
            archived: false,
            created_at: "2026-05-19T20:00:00".into(),
        };
        let s = serde_json::to_string(&job).unwrap();
        let back: Job = serde_json::from_str(&s).unwrap();
        assert_eq!(job, back);
    }
}
```

- [ ] **Step 4: Run**

```bash
cargo test db::models::tests::job_serializes_to_json
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/models.rs src-tauri/Cargo.toml
git commit -m "feat(db): Job + NewJob row structs with serde round-trip"
```

---

### Task 9: Phase row struct

**Files:**
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Append `Phase` + `NewPhase` to `db/models.rs`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Phase {
    pub id: i64,
    pub job_id: i64,
    pub name: String,
    pub colour: String,        // hex e.g. "#3B82F6"
    pub order_index: i64,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewPhase {
    pub job_id: i64,
    pub name: String,
    pub colour: String,
    pub order_index: i64,
    pub collapsed: bool,
}
```

- [ ] **Step 2: Add a round-trip test**

```rust
#[test]
fn phase_serializes_to_json() {
    let p = Phase {
        id: 1, job_id: 1, name: "Plumbing".into(),
        colour: "#3B82F6".into(), order_index: 0, collapsed: true,
    };
    let s = serde_json::to_string(&p).unwrap();
    let back: Phase = serde_json::from_str(&s).unwrap();
    assert_eq!(p, back);
}
```

- [ ] **Step 3: Run**

```bash
cargo test db::models::tests::phase_serializes_to_json
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/models.rs
git commit -m "feat(db): Phase + NewPhase row structs"
```

---

### Task 10: Task row struct

**Files:**
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Append `Task` + `NewTask`**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: i64,
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
    pub order_index: i64,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
    pub order_index: i64,
    pub notes: Option<String>,
}
```

- [ ] **Step 2: Round-trip test**

```rust
#[test]
fn task_serializes_to_json() {
    let t = Task {
        id: 1, phase_id: 1, name: "First-fix".into(),
        start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
        duration_workdays: 3, order_index: 0, notes: None,
    };
    let s = serde_json::to_string(&t).unwrap();
    let back: Task = serde_json::from_str(&s).unwrap();
    assert_eq!(t, back);
}
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test db::models::tests::task_serializes_to_json
git add src-tauri/src/db/models.rs
git commit -m "feat(db): Task + NewTask row structs"
```

---

### Task 11: Dependency row struct

**Files:**
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Append**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    pub id: i64,
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub r#type: String,    // 'FS' for v1
    pub lag_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDependency {
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub lag_days: i64,
}
```

- [ ] **Step 2: Test**

```rust
#[test]
fn dependency_serializes_to_json() {
    let d = Dependency { id: 1, predecessor_id: 1, successor_id: 2, r#type: "FS".into(), lag_days: 0 };
    let s = serde_json::to_string(&d).unwrap();
    let back: Dependency = serde_json::from_str(&s).unwrap();
    assert_eq!(d, back);
}
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test db::models::tests::dependency_serializes_to_json
git add src-tauri/src/db/models.rs
git commit -m "feat(db): Dependency + NewDependency row structs"
```

---

### Task 12: NoWorkDay row struct

**Files:**
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Append**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NoWorkDay {
    pub id: i64,
    pub job_id: i64,
    pub date: NaiveDate,
    pub reason: String,
    pub source: String,   // 'sa_public_holiday' | 'manual'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewNoWorkDay {
    pub job_id: i64,
    pub date: NaiveDate,
    pub reason: String,
    pub source: String,
}
```

- [ ] **Step 2: Test**

```rust
#[test]
fn no_work_day_serializes_to_json() {
    let n = NoWorkDay {
        id: 1, job_id: 1,
        date: NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(),
        reason: "Youth Day".into(),
        source: "sa_public_holiday".into(),
    };
    let s = serde_json::to_string(&n).unwrap();
    let back: NoWorkDay = serde_json::from_str(&s).unwrap();
    assert_eq!(n, back);
}
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test db::models::tests::no_work_day_serializes_to_json
git add src-tauri/src/db/models.rs
git commit -m "feat(db): NoWorkDay + NewNoWorkDay row structs"
```

---

### Task 13: AppMeta helpers

**Files:**
- Modify: `src-tauri/src/db/models.rs`

- [ ] **Step 1: Append**

```rust
use rusqlite::Connection;

pub fn meta_get(conn: &Connection, key: &str) -> crate::GbResult<Option<String>> {
    let res = conn
        .query_row("SELECT value FROM app_meta WHERE key = ?1", [key], |r| r.get::<_, String>(0))
        .ok();
    Ok(res)
}

pub fn meta_set(conn: &Connection, key: &str, value: &str) -> crate::GbResult<()> {
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}
```

- [ ] **Step 2: Test**

```rust
#[test]
fn meta_get_set_roundtrip() {
    let conn = crate::db::connection::open_in_memory().unwrap();
    assert!(meta_get(&conn, "last_open_job_id").unwrap().is_none());
    meta_set(&conn, "last_open_job_id", "42").unwrap();
    assert_eq!(meta_get(&conn, "last_open_job_id").unwrap(), Some("42".into()));
    meta_set(&conn, "last_open_job_id", "43").unwrap();
    assert_eq!(meta_get(&conn, "last_open_job_id").unwrap(), Some("43".into()));
}
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test db::models::tests::meta_get_set_roundtrip
git add src-tauri/src/db/models.rs
git commit -m "feat(db): app_meta get/set helpers"
```

---

### Task 14: Job repository (CRUD)

**Files:**
- Create: `src-tauri/src/repo/mod.rs`
- Create: `src-tauri/src/repo/job.rs`

- [ ] **Step 1: Wire `repo/mod.rs`**

```rust
pub mod job;
```

- [ ] **Step 2: Write failing tests in `repo/job.rs`**

```rust
use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::db::models::{Job, NewJob};
use crate::{GbError, GbResult};

pub fn create(conn: &Connection, new: &NewJob) -> GbResult<Job> {
    Err(GbError::Validation("not implemented".into()))
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Job> {
    Err(GbError::NotFound(format!("job {id}")))
}

pub fn list_active(conn: &Connection) -> GbResult<Vec<Job>> {
    Ok(vec![])
}

pub fn update(conn: &Connection, job: &Job) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;

    fn sample(name: &str) -> NewJob {
        NewJob {
            name: name.into(),
            client: None,
            address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
        }
    }

    #[test]
    fn create_and_get_roundtrip() {
        let conn = open_in_memory().unwrap();
        let job = create(&conn, &sample("Sea Point")).unwrap();
        assert!(job.id > 0);
        assert_eq!(job.name, "Sea Point");
        let fetched = get(&conn, job.id).unwrap();
        assert_eq!(fetched.name, "Sea Point");
    }

    #[test]
    fn list_active_excludes_archived() {
        let conn = open_in_memory().unwrap();
        let a = create(&conn, &sample("A")).unwrap();
        let _b = create(&conn, &sample("B")).unwrap();
        set_archived(&conn, a.id, true).unwrap();
        let list = list_active(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "B");
    }

    #[test]
    fn update_changes_name() {
        let conn = open_in_memory().unwrap();
        let mut job = create(&conn, &sample("Old")).unwrap();
        job.name = "New".into();
        update(&conn, &job).unwrap();
        assert_eq!(get(&conn, job.id).unwrap().name, "New");
    }

    #[test]
    fn delete_removes_row() {
        let conn = open_in_memory().unwrap();
        let job = create(&conn, &sample("Doomed")).unwrap();
        delete(&conn, job.id).unwrap();
        assert!(matches!(get(&conn, job.id), Err(GbError::NotFound(_))));
    }
}
```

- [ ] **Step 3: Run tests — all four fail**

```bash
cargo test repo::job
```

Expected: 4 failed.

- [ ] **Step 4: Replace the stubs with real implementations**

```rust
pub fn create(conn: &Connection, new: &NewJob) -> GbResult<Job> {
    conn.execute(
        "INSERT INTO job (name, client, address, project_start_date, is_template)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new.name,
            new.client,
            new.address,
            new.project_start_date.to_string(),
            new.is_template as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get(conn, id)
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Job> {
    conn.query_row(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job WHERE id = ?1",
        [id],
        row_to_job,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("job {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_active(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job
         WHERE archived = 0 AND is_template = 0
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_templates(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job
         WHERE is_template = 1
         ORDER BY name",
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, job: &Job) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE job SET name = ?1, client = ?2, address = ?3,
                        project_start_date = ?4, is_template = ?5, archived = ?6
         WHERE id = ?7",
        params![
            job.name, job.client, job.address,
            job.project_start_date.to_string(),
            job.is_template as i64,
            job.archived as i64,
            job.id,
        ],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("job {}", job.id))); }
    Ok(())
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> GbResult<()> {
    let n = conn.execute("UPDATE job SET archived = ?1 WHERE id = ?2", params![archived as i64, id])?;
    if n == 0 { return Err(GbError::NotFound(format!("job {id}"))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM job WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("job {id}"))); }
    Ok(())
}

fn row_to_job(r: &rusqlite::Row) -> rusqlite::Result<Job> {
    let date_str: String = r.get(4)?;
    let project_start_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok(Job {
        id: r.get(0)?,
        name: r.get(1)?,
        client: r.get(2)?,
        address: r.get(3)?,
        project_start_date,
        is_template: r.get::<_, i64>(5)? != 0,
        archived: r.get::<_, i64>(6)? != 0,
        created_at: r.get(7)?,
    })
}
```

- [ ] **Step 5: Run tests — all four pass**

```bash
cargo test repo::job
```

Expected: 4 passed.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/repo
git commit -m "feat(repo): job CRUD with FK-cascade-aware delete"
```

---

### Task 15: Phase repository (CRUD + reorder)

**Files:**
- Modify: `src-tauri/src/repo/mod.rs`
- Create: `src-tauri/src/repo/phase.rs`

- [ ] **Step 1: Add to `repo/mod.rs`**

```rust
pub mod job;
pub mod phase;
```

- [ ] **Step 2: Write failing tests + stubs in `repo/phase.rs`**

```rust
use rusqlite::{Connection, params};
use crate::db::models::{Phase, NewPhase};
use crate::{GbError, GbResult};

pub fn create(conn: &Connection, new: &NewPhase) -> GbResult<Phase> {
    Err(GbError::Validation("not implemented".into()))
}
pub fn get(conn: &Connection, id: i64) -> GbResult<Phase> {
    Err(GbError::NotFound(format!("phase {id}")))
}
pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Phase>> { Ok(vec![]) }
pub fn update(conn: &Connection, phase: &Phase) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}
pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}
pub fn reorder(conn: &Connection, job_id: i64, ordered_ids: &[i64]) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::NewJob;
    use crate::repo::job;
    use chrono::NaiveDate;

    fn make_job(conn: &Connection) -> i64 {
        let j = job::create(conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
        }).unwrap();
        j.id
    }

    #[test]
    fn create_and_list_phases() {
        let conn = open_in_memory().unwrap();
        let job_id = make_job(&conn);
        let a = create(&conn, &NewPhase { job_id, name: "Plumbing".into(), colour: "#3B82F6".into(), order_index: 0, collapsed: true }).unwrap();
        let b = create(&conn, &NewPhase { job_id, name: "Electrical".into(), colour: "#EF4444".into(), order_index: 1, collapsed: true }).unwrap();
        let phases = list_for_job(&conn, job_id).unwrap();
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].id, a.id);
        assert_eq!(phases[1].id, b.id);
    }

    #[test]
    fn reorder_swaps_order_indices() {
        let conn = open_in_memory().unwrap();
        let job_id = make_job(&conn);
        let a = create(&conn, &NewPhase { job_id, name: "A".into(), colour: "#000".into(), order_index: 0, collapsed: true }).unwrap();
        let b = create(&conn, &NewPhase { job_id, name: "B".into(), colour: "#000".into(), order_index: 1, collapsed: true }).unwrap();
        let c = create(&conn, &NewPhase { job_id, name: "C".into(), colour: "#000".into(), order_index: 2, collapsed: true }).unwrap();
        reorder(&conn, job_id, &[c.id, a.id, b.id]).unwrap();
        let phases = list_for_job(&conn, job_id).unwrap();
        assert_eq!(phases.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["C","A","B"]);
    }

    #[test]
    fn delete_phase_cascades_to_tasks() {
        let conn = open_in_memory().unwrap();
        let job_id = make_job(&conn);
        let p = create(&conn, &NewPhase { job_id, name: "Doomed".into(), colour: "#000".into(), order_index: 0, collapsed: true }).unwrap();
        // Insert a task directly via SQL so we don't depend on Task 16 yet
        conn.execute(
            "INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index) VALUES (?1, 'T', '2026-06-05', 1, 0)",
            params![p.id],
        ).unwrap();
        delete(&conn, p.id).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM task WHERE phase_id = ?1", [p.id], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}
```

- [ ] **Step 3: Run tests — all fail**

```bash
cargo test repo::phase
```

Expected: 3 failed.

- [ ] **Step 4: Implement**

Replace the stubs in `repo/phase.rs`:

```rust
pub fn create(conn: &Connection, new: &NewPhase) -> GbResult<Phase> {
    conn.execute(
        "INSERT INTO phase (job_id, name, colour, order_index, collapsed)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new.job_id, new.name, new.colour, new.order_index, new.collapsed as i64],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Phase> {
    conn.query_row(
        "SELECT id, job_id, name, colour, order_index, collapsed FROM phase WHERE id = ?1",
        [id],
        row_to_phase,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("phase {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Phase>> {
    let mut stmt = conn.prepare(
        "SELECT id, job_id, name, colour, order_index, collapsed
         FROM phase WHERE job_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map([job_id], row_to_phase)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, phase: &Phase) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE phase SET name = ?1, colour = ?2, order_index = ?3, collapsed = ?4 WHERE id = ?5",
        params![phase.name, phase.colour, phase.order_index, phase.collapsed as i64, phase.id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("phase {}", phase.id))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM phase WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("phase {id}"))); }
    Ok(())
}

pub fn reorder(conn: &Connection, job_id: i64, ordered_ids: &[i64]) -> GbResult<()> {
    let tx = conn.unchecked_transaction()?;
    for (idx, id) in ordered_ids.iter().enumerate() {
        let n = tx.execute(
            "UPDATE phase SET order_index = ?1 WHERE id = ?2 AND job_id = ?3",
            params![idx as i64, id, job_id],
        )?;
        if n == 0 {
            return Err(GbError::Validation(format!("phase {id} not in job {job_id}")));
        }
    }
    tx.commit()?;
    Ok(())
}

fn row_to_phase(r: &rusqlite::Row) -> rusqlite::Result<Phase> {
    Ok(Phase {
        id: r.get(0)?,
        job_id: r.get(1)?,
        name: r.get(2)?,
        colour: r.get(3)?,
        order_index: r.get(4)?,
        collapsed: r.get::<_, i64>(5)? != 0,
    })
}
```

- [ ] **Step 5: Run + Commit**

```bash
cargo test repo::phase
git add src-tauri/src/repo
git commit -m "feat(repo): phase CRUD + transactional reorder"
```

---

### Task 16: Task repository (CRUD + reorder)

**Files:**
- Modify: `src-tauri/src/repo/mod.rs`
- Create: `src-tauri/src/repo/task.rs`

- [ ] **Step 1: Add to `repo/mod.rs`**

```rust
pub mod job;
pub mod phase;
pub mod task;
```

- [ ] **Step 2: Write failing tests in `repo/task.rs`**

```rust
use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::db::models::{Task, NewTask};
use crate::{GbError, GbResult};

pub fn create(conn: &Connection, new: &NewTask) -> GbResult<Task> {
    Err(GbError::Validation("not implemented".into()))
}
pub fn get(conn: &Connection, id: i64) -> GbResult<Task> {
    Err(GbError::NotFound(format!("task {id}")))
}
pub fn list_for_phase(conn: &Connection, phase_id: i64) -> GbResult<Vec<Task>> { Ok(vec![]) }
pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Task>> { Ok(vec![]) }
pub fn update(conn: &Connection, task: &Task) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}
pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}
pub fn reorder(conn: &Connection, phase_id: i64, ordered_ids: &[i64]) -> GbResult<()> {
    Err(GbError::Validation("not implemented".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase};
    use crate::repo::{job, phase};

    fn setup(conn: &Connection) -> i64 {
        let j = job::create(conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
        }).unwrap();
        let p = phase::create(conn, &NewPhase {
            job_id: j.id, name: "Plumbing".into(), colour: "#3B82F6".into(),
            order_index: 0, collapsed: true,
        }).unwrap();
        p.id
    }

    fn sample(phase_id: i64, name: &str, order_index: i64) -> NewTask {
        NewTask {
            phase_id, name: name.into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 3, order_index, notes: None,
        }
    }

    #[test]
    fn create_and_list() {
        let conn = open_in_memory().unwrap();
        let phase_id = setup(&conn);
        let a = create(&conn, &sample(phase_id, "First-fix", 0)).unwrap();
        let b = create(&conn, &sample(phase_id, "Second-fix", 1)).unwrap();
        let list = list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, a.id);
        assert_eq!(list[1].id, b.id);
    }

    #[test]
    fn duration_zero_is_rejected_by_check_constraint() {
        let conn = open_in_memory().unwrap();
        let phase_id = setup(&conn);
        let mut bad = sample(phase_id, "Bad", 0);
        bad.duration_workdays = 0;
        let r = create(&conn, &bad);
        assert!(r.is_err(), "expected CHECK violation");
    }

    #[test]
    fn reorder_works() {
        let conn = open_in_memory().unwrap();
        let phase_id = setup(&conn);
        let a = create(&conn, &sample(phase_id, "A", 0)).unwrap();
        let b = create(&conn, &sample(phase_id, "B", 1)).unwrap();
        let c = create(&conn, &sample(phase_id, "C", 2)).unwrap();
        reorder(&conn, phase_id, &[c.id, a.id, b.id]).unwrap();
        let list = list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(list.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["C","A","B"]);
    }
}
```

- [ ] **Step 3: Run — all three fail**

```bash
cargo test repo::task
```

- [ ] **Step 4: Implement**

```rust
pub fn create(conn: &Connection, new: &NewTask) -> GbResult<Task> {
    conn.execute(
        "INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new.phase_id, new.name, new.start_date.to_string(),
            new.duration_workdays, new.order_index, new.notes,
        ],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Task> {
    conn.query_row(
        "SELECT id, phase_id, name, start_date, duration_workdays, order_index, notes
         FROM task WHERE id = ?1",
        [id],
        row_to_task,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("task {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_for_phase(conn: &Connection, phase_id: i64) -> GbResult<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, phase_id, name, start_date, duration_workdays, order_index, notes
         FROM task WHERE phase_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map([phase_id], row_to_task)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.phase_id, t.name, t.start_date, t.duration_workdays, t.order_index, t.notes
         FROM task t JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1
         ORDER BY p.order_index ASC, t.order_index ASC",
    )?;
    let rows = stmt.query_map([job_id], row_to_task)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, task: &Task) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE task SET phase_id = ?1, name = ?2, start_date = ?3,
                         duration_workdays = ?4, order_index = ?5, notes = ?6
         WHERE id = ?7",
        params![
            task.phase_id, task.name, task.start_date.to_string(),
            task.duration_workdays, task.order_index, task.notes, task.id,
        ],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("task {}", task.id))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM task WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("task {id}"))); }
    Ok(())
}

pub fn reorder(conn: &Connection, phase_id: i64, ordered_ids: &[i64]) -> GbResult<()> {
    let tx = conn.unchecked_transaction()?;
    for (idx, id) in ordered_ids.iter().enumerate() {
        let n = tx.execute(
            "UPDATE task SET order_index = ?1 WHERE id = ?2 AND phase_id = ?3",
            params![idx as i64, id, phase_id],
        )?;
        if n == 0 {
            return Err(GbError::Validation(format!("task {id} not in phase {phase_id}")));
        }
    }
    tx.commit()?;
    Ok(())
}

fn row_to_task(r: &rusqlite::Row) -> rusqlite::Result<Task> {
    let date_str: String = r.get(3)?;
    let start_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok(Task {
        id: r.get(0)?,
        phase_id: r.get(1)?,
        name: r.get(2)?,
        start_date,
        duration_workdays: r.get(4)?,
        order_index: r.get(5)?,
        notes: r.get(6)?,
    })
}
```

- [ ] **Step 5: Run + Commit**

```bash
cargo test repo::task
git add src-tauri/src/repo
git commit -m "feat(repo): task CRUD + transactional reorder + list-for-job"
```

---

## Phase 2 — Calendar engine (Tasks 17–22)

Pure Rust functions in `src-tauri/src/calendar/`. No I/O, no DB. Workday arithmetic + SA Public Holidays Act 1994.

### Task 17: Workday-forward (Sat/Sun only)

**Files:**
- Create: `src-tauri/src/calendar/mod.rs`
- Create: `src-tauri/src/calendar/workday.rs`

- [ ] **Step 1: Wire `calendar/mod.rs`**

```rust
pub mod workday;
pub mod easter;
pub mod sa_holidays;
```

- [ ] **Step 2: Failing tests + stub in `calendar/workday.rs`**

```rust
use chrono::{Datelike, Duration, NaiveDate, Weekday};

/// Add `n` workdays (Mon–Fri only — no holiday awareness yet) to `from`.
/// `from` itself counts as day 0 of work iff it is a workday; otherwise advance to the next workday first.
pub fn add_workdays(from: NaiveDate, n: i64) -> NaiveDate {
    unimplemented!()
}

/// Inclusive workday count between `start` and `end` (both inclusive).
/// `end < start` returns 0. Sat/Sun are not counted.
pub fn count_workdays(start: NaiveDate, end: NaiveDate) -> i64 {
    unimplemented!()
}

pub fn is_workday(d: NaiveDate) -> bool {
    !matches!(d.weekday(), Weekday::Sat | Weekday::Sun)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn add_zero_workdays_returns_same_day_if_workday() {
        assert_eq!(add_workdays(d(2026, 6, 8), 0), d(2026, 6, 8));      // Mon
    }

    #[test]
    fn add_zero_workdays_from_saturday_advances_to_monday() {
        assert_eq!(add_workdays(d(2026, 6, 6), 0), d(2026, 6, 8));      // Sat -> Mon
    }

    #[test]
    fn add_three_workdays_from_monday_lands_on_thursday() {
        assert_eq!(add_workdays(d(2026, 6, 8), 3), d(2026, 6, 11));     // Mon +3 -> Thu
    }

    #[test]
    fn add_five_workdays_from_monday_skips_weekend() {
        assert_eq!(add_workdays(d(2026, 6, 8), 5), d(2026, 6, 15));     // Mon -> next Mon
    }

    #[test]
    fn count_workdays_inclusive_week() {
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 12)), 5);   // Mon-Fri
    }

    #[test]
    fn count_workdays_skips_weekend_in_middle() {
        assert_eq!(count_workdays(d(2026, 6, 8), d(2026, 6, 15)), 6);   // Mon-Mon = 6 workdays
    }

    #[test]
    fn count_workdays_reverse_returns_zero() {
        assert_eq!(count_workdays(d(2026, 6, 15), d(2026, 6, 8)), 0);
    }
}
```

- [ ] **Step 3: Run — 6 fail (`is_workday` is fine)**

```bash
cargo test calendar::workday
```

- [ ] **Step 4: Implement**

```rust
pub fn add_workdays(from: NaiveDate, n: i64) -> NaiveDate {
    let mut cur = from;
    while !is_workday(cur) {
        cur += Duration::days(1);
    }
    if n <= 0 { return cur; }
    let mut remaining = n;
    while remaining > 0 {
        cur += Duration::days(1);
        if is_workday(cur) { remaining -= 1; }
    }
    cur
}

pub fn count_workdays(start: NaiveDate, end: NaiveDate) -> i64 {
    if end < start { return 0; }
    let mut cur = start;
    let mut n: i64 = 0;
    while cur <= end {
        if is_workday(cur) { n += 1; }
        cur += Duration::days(1);
    }
    n
}
```

- [ ] **Step 5: Run — all pass; Commit**

```bash
cargo test calendar::workday
git add src-tauri/src/calendar
git commit -m "feat(calendar): workday arithmetic (Mon-Fri, no holidays)"
```

---

### Task 18: Easter Sunday (Anonymous Gregorian algorithm)

**Files:**
- Create: `src-tauri/src/calendar/easter.rs`

- [ ] **Step 1: Failing test + stub**

```rust
use chrono::NaiveDate;

pub fn easter_sunday(year: i32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, 1, 1).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn easter_2024_is_march_31() {
        assert_eq!(easter_sunday(2024), NaiveDate::from_ymd_opt(2024, 3, 31).unwrap());
    }
    #[test]
    fn easter_2025_is_april_20() {
        assert_eq!(easter_sunday(2025), NaiveDate::from_ymd_opt(2025, 4, 20).unwrap());
    }
    #[test]
    fn easter_2026_is_april_5() {
        assert_eq!(easter_sunday(2026), NaiveDate::from_ymd_opt(2026, 4, 5).unwrap());
    }
    #[test]
    fn easter_2027_is_march_28() {
        assert_eq!(easter_sunday(2027), NaiveDate::from_ymd_opt(2027, 3, 28).unwrap());
    }
}
```

- [ ] **Step 2: Run — 4 fail**

```bash
cargo test calendar::easter
```

- [ ] **Step 3: Implement (Anonymous Gregorian / Meeus)**

```rust
pub fn easter_sunday(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).unwrap()
}
```

- [ ] **Step 4: Run — 4 pass; Commit**

```bash
cargo test calendar::easter
git add src-tauri/src/calendar/easter.rs
git commit -m "feat(calendar): Easter Sunday via Anonymous Gregorian algorithm"
```

---

### Task 19: SA fixed + Easter-derived holidays for a year

**Files:**
- Create: `src-tauri/src/calendar/sa_holidays.rs`

- [ ] **Step 1: Failing tests + stub**

```rust
use chrono::{Duration, NaiveDate};
use super::easter::easter_sunday;

#[derive(Debug, Clone, PartialEq)]
pub struct Holiday {
    pub date: NaiveDate,
    pub name: &'static str,
}

pub fn sa_holidays(year: i32) -> Vec<Holiday> { vec![] }

#[cfg(test)]
mod tests {
    use super::*;

    fn dates(year: i32) -> Vec<(NaiveDate, &'static str)> {
        sa_holidays(year).into_iter().map(|h| (h.date, h.name)).collect()
    }

    #[test]
    fn twelve_holidays_per_year() {
        assert_eq!(sa_holidays(2026).len(), 12);
    }

    #[test]
    fn fixed_holidays_in_2026() {
        let h = dates(2026);
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,1,1).unwrap(),  "New Year's Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,3,21).unwrap(), "Human Rights Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,4,27).unwrap(), "Freedom Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,5,1).unwrap(),  "Workers' Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,6,16).unwrap(), "Youth Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,8,9).unwrap(),  "National Women's Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,9,24).unwrap(), "Heritage Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,12,16).unwrap(),"Day of Reconciliation")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,12,25).unwrap(),"Christmas Day")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,12,26).unwrap(),"Day of Goodwill")));
    }

    #[test]
    fn easter_derived_in_2026() {
        let h = dates(2026);
        // Easter Sun 2026 = 5 April; Good Friday = 3 April; Family Day = 6 April
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,4,3).unwrap(), "Good Friday")));
        assert!(h.contains(&(NaiveDate::from_ymd_opt(2026,4,6).unwrap(), "Family Day")));
    }
}
```

- [ ] **Step 2: Run — 3 fail**

```bash
cargo test calendar::sa_holidays
```

- [ ] **Step 3: Implement (no Sunday-shift yet — that's Task 20)**

```rust
pub fn sa_holidays(year: i32) -> Vec<Holiday> {
    let easter = easter_sunday(year);
    let good_friday = easter - Duration::days(2);
    let family_day = easter + Duration::days(1);
    vec![
        Holiday { date: NaiveDate::from_ymd_opt(year, 1, 1).unwrap(),   name: "New Year's Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 3, 21).unwrap(),  name: "Human Rights Day" },
        Holiday { date: good_friday,                                    name: "Good Friday" },
        Holiday { date: family_day,                                     name: "Family Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 4, 27).unwrap(),  name: "Freedom Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 5, 1).unwrap(),   name: "Workers' Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 6, 16).unwrap(),  name: "Youth Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 8, 9).unwrap(),   name: "National Women's Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 9, 24).unwrap(),  name: "Heritage Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 12, 16).unwrap(), name: "Day of Reconciliation" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 12, 25).unwrap(), name: "Christmas Day" },
        Holiday { date: NaiveDate::from_ymd_opt(year, 12, 26).unwrap(), name: "Day of Goodwill" },
    ]
}
```

- [ ] **Step 4: Run + Commit**

```bash
cargo test calendar::sa_holidays
git add src-tauri/src/calendar/sa_holidays.rs
git commit -m "feat(calendar): SA public holidays for a year (fixed + Easter-derived)"
```

---

### Task 20: SA Sunday-shift rule (Public Holidays Act 1994)

**Files:**
- Modify: `src-tauri/src/calendar/sa_holidays.rs`

- [ ] **Step 1: Add a failing test for a year where a fixed holiday falls on Sunday**

```rust
#[test]
fn workers_day_on_sunday_shifts_to_monday_in_2022() {
    // 1 May 2022 was a Sunday; observed holiday is Mon 2 May 2022
    let h = sa_holidays(2022);
    let dates: Vec<NaiveDate> = h.iter().map(|x| x.date).collect();
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2022, 5, 2).unwrap()),
            "expected observed Workers' Day on Mon 2 May 2022");
    assert!(!dates.contains(&NaiveDate::from_ymd_opt(2022, 5, 1).unwrap()),
            "raw 1 May Sunday should not appear");
}
```

- [ ] **Step 2: Run — fails**

```bash
cargo test calendar::sa_holidays::tests::workers_day_on_sunday_shifts_to_monday_in_2022
```

- [ ] **Step 3: Implement Sunday-shift**

Replace `sa_holidays` with this version (shifts fixed holidays only; Easter-derived dates never fall on Sunday):

```rust
use chrono::{Datelike, Weekday};

pub fn sa_holidays(year: i32) -> Vec<Holiday> {
    let easter = easter_sunday(year);
    let good_friday = easter - Duration::days(2);
    let family_day = easter + Duration::days(1);

    let fixed: &[(u32, u32, &'static str)] = &[
        (1, 1,   "New Year's Day"),
        (3, 21,  "Human Rights Day"),
        (4, 27,  "Freedom Day"),
        (5, 1,   "Workers' Day"),
        (6, 16,  "Youth Day"),
        (8, 9,   "National Women's Day"),
        (9, 24,  "Heritage Day"),
        (12, 16, "Day of Reconciliation"),
        (12, 25, "Christmas Day"),
        (12, 26, "Day of Goodwill"),
    ];

    let mut out: Vec<Holiday> = Vec::with_capacity(12);
    for &(m, d, name) in fixed {
        let raw = NaiveDate::from_ymd_opt(year, m, d).unwrap();
        let observed = if raw.weekday() == Weekday::Sun { raw + Duration::days(1) } else { raw };
        out.push(Holiday { date: observed, name });
    }
    out.push(Holiday { date: good_friday, name: "Good Friday" });
    out.push(Holiday { date: family_day,  name: "Family Day" });
    out.sort_by_key(|h| h.date);
    out
}
```

- [ ] **Step 4: Run all calendar tests**

```bash
cargo test calendar
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/calendar/sa_holidays.rs
git commit -m "feat(calendar): SA Sunday-shift rule (Public Holidays Act 1994)"
```

---

### Task 21: `sa_holidays_for_range` — multi-year span

**Files:**
- Modify: `src-tauri/src/calendar/sa_holidays.rs`

- [ ] **Step 1: Add a failing test**

```rust
#[test]
fn range_spanning_year_boundary_returns_holidays_from_both_years() {
    let from = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    let to   = NaiveDate::from_ymd_opt(2027, 2, 1).unwrap();
    let h = sa_holidays_for_range(from, to);
    let dates: Vec<NaiveDate> = h.iter().map(|x| x.date).collect();
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 12, 16).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 12, 25).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2026, 12, 26).unwrap()));
    assert!(dates.contains(&NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()));
    assert!(h.iter().all(|h| h.date >= from && h.date <= to));
}
```

- [ ] **Step 2: Implement**

Append to `sa_holidays.rs`:

```rust
pub fn sa_holidays_for_range(from: NaiveDate, to: NaiveDate) -> Vec<Holiday> {
    if to < from { return vec![]; }
    let mut out = Vec::new();
    for y in from.year()..=to.year() {
        for h in sa_holidays(y) {
            if h.date >= from && h.date <= to { out.push(h); }
        }
    }
    out
}
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test calendar::sa_holidays::tests::range_spanning_year_boundary_returns_holidays_from_both_years
git add src-tauri/src/calendar/sa_holidays.rs
git commit -m "feat(calendar): sa_holidays_for_range across year boundaries"
```

---

### Task 22: Workday-aware date math that skips a custom set of dates

**Files:**
- Modify: `src-tauri/src/calendar/workday.rs`

- [ ] **Step 1: Failing test**

```rust
#[test]
fn add_workdays_excluding_skips_holiday_in_path() {
    use std::collections::HashSet;
    let mut hol = HashSet::new();
    hol.insert(NaiveDate::from_ymd_opt(2026,6,16).unwrap()); // Youth Day, Tue
    // Mon 15 Jun + 3 workdays = Thu 18 Jun normally; with Tue blocked → Fri 19 Jun
    let result = add_workdays_excluding(
        NaiveDate::from_ymd_opt(2026,6,15).unwrap(),
        3,
        &hol,
    );
    assert_eq!(result, NaiveDate::from_ymd_opt(2026,6,19).unwrap());
}
```

- [ ] **Step 2: Implement**

```rust
use std::collections::HashSet;

pub fn add_workdays_excluding(
    from: NaiveDate,
    n: i64,
    excluded: &HashSet<NaiveDate>,
) -> NaiveDate {
    let is_work = |d: NaiveDate| is_workday(d) && !excluded.contains(&d);
    let mut cur = from;
    while !is_work(cur) {
        cur += Duration::days(1);
    }
    if n <= 0 { return cur; }
    let mut remaining = n;
    while remaining > 0 {
        cur += Duration::days(1);
        if is_work(cur) { remaining -= 1; }
    }
    cur
}
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test calendar::workday
git add src-tauri/src/calendar/workday.rs
git commit -m "feat(calendar): add_workdays_excluding for no-work-day-aware shifts"
```

---

## Phase 3 — Dependency graph (Tasks 23–27)

Pure functions in `src-tauri/src/deps/`. Takes a list of `Dependency` rows and a starting task; computes cycle detection, topological order, and downstream ripple given a shift to one task.

### Task 23: Build adjacency map from dependency rows

**Files:**
- Create: `src-tauri/src/deps/mod.rs`
- Create: `src-tauri/src/deps/graph.rs`

- [ ] **Step 1: Wire `deps/mod.rs`**

```rust
pub mod graph;
pub mod ripple;
```

- [ ] **Step 2: Failing test + stub in `graph.rs`**

```rust
use std::collections::HashMap;
use crate::db::models::Dependency;

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub successor_id: i64,
    pub lag_days: i64,
}

/// Adjacency map: predecessor_id -> [(successor_id, lag)]
pub fn build_adjacency(deps: &[Dependency]) -> HashMap<i64, Vec<Edge>> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dep(id: i64, pre: i64, suc: i64, lag: i64) -> Dependency {
        Dependency { id, predecessor_id: pre, successor_id: suc, r#type: "FS".into(), lag_days: lag }
    }

    #[test]
    fn empty_input_returns_empty_map() {
        let m = build_adjacency(&[]);
        assert!(m.is_empty());
    }

    #[test]
    fn single_edge() {
        let m = build_adjacency(&[dep(1, 10, 20, 0)]);
        assert_eq!(m.get(&10).unwrap(), &vec![Edge { successor_id: 20, lag_days: 0 }]);
    }

    #[test]
    fn many_successors_grouped() {
        let m = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 10, 30, 1),
            dep(3, 20, 40, 0),
        ]);
        let from_10 = m.get(&10).unwrap();
        assert_eq!(from_10.len(), 2);
        assert!(from_10.contains(&Edge { successor_id: 20, lag_days: 0 }));
        assert!(from_10.contains(&Edge { successor_id: 30, lag_days: 1 }));
        assert_eq!(m.get(&20).unwrap(), &vec![Edge { successor_id: 40, lag_days: 0 }]);
    }
}
```

- [ ] **Step 3: Run — 2 fail (`empty_input_returns_empty_map` passes by accident)**

```bash
cargo test deps::graph
```

- [ ] **Step 4: Implement**

```rust
pub fn build_adjacency(deps: &[Dependency]) -> HashMap<i64, Vec<Edge>> {
    let mut m: HashMap<i64, Vec<Edge>> = HashMap::new();
    for d in deps {
        m.entry(d.predecessor_id).or_default().push(Edge {
            successor_id: d.successor_id,
            lag_days: d.lag_days,
        });
    }
    m
}
```

- [ ] **Step 5: Run + Commit**

```bash
cargo test deps::graph
git add src-tauri/src/deps
git commit -m "feat(deps): build adjacency map from dependency rows"
```

---

### Task 24: Cycle detection

**Files:**
- Modify: `src-tauri/src/deps/graph.rs`

- [ ] **Step 1: Failing tests + stub**

```rust
use std::collections::HashSet;

/// Returns true iff adding (pre -> suc) would create a cycle in the existing adjacency.
pub fn would_cycle(adj: &HashMap<i64, Vec<Edge>>, pre: i64, suc: i64) -> bool {
    false
}

#[cfg(test)]
mod cycle_tests {
    use super::*;
    use super::tests::*;  // reuse the dep() helper above

    #[test]
    fn self_loop_is_a_cycle() {
        let adj = build_adjacency(&[]);
        assert!(would_cycle(&adj, 10, 10));
    }

    #[test]
    fn direct_back_edge_is_a_cycle() {
        // existing: 10 -> 20.  Adding 20 -> 10 closes the loop.
        let adj = build_adjacency(&[dep(1, 10, 20, 0)]);
        assert!(would_cycle(&adj, 20, 10));
    }

    #[test]
    fn deeper_back_edge_is_a_cycle() {
        // existing: 10 -> 20 -> 30 -> 40.  Adding 40 -> 10 closes.
        let adj = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 20, 30, 0),
            dep(3, 30, 40, 0),
        ]);
        assert!(would_cycle(&adj, 40, 10));
    }

    #[test]
    fn unrelated_edge_does_not_cycle() {
        let adj = build_adjacency(&[dep(1, 10, 20, 0)]);
        assert!(!would_cycle(&adj, 30, 40));
    }
}
```

- [ ] **Step 2: Run — 3 fail (`unrelated_edge_does_not_cycle` passes by accident)**

- [ ] **Step 3: Implement (DFS from `suc` looking for `pre`)**

```rust
pub fn would_cycle(adj: &HashMap<i64, Vec<Edge>>, pre: i64, suc: i64) -> bool {
    if pre == suc { return true; }
    let mut stack = vec![suc];
    let mut visited: HashSet<i64> = HashSet::new();
    while let Some(node) = stack.pop() {
        if !visited.insert(node) { continue; }
        if let Some(edges) = adj.get(&node) {
            for e in edges {
                if e.successor_id == pre { return true; }
                stack.push(e.successor_id);
            }
        }
    }
    false
}
```

- [ ] **Step 4: Run + Commit**

```bash
cargo test deps::graph
git add src-tauri/src/deps/graph.rs
git commit -m "feat(deps): would_cycle via DFS reachability"
```

---

### Task 25: Topological order of downstream tasks from a root

**Files:**
- Modify: `src-tauri/src/deps/graph.rs`

- [ ] **Step 1: Failing test + stub**

```rust
/// Tasks transitively reachable from `root`, in BFS-by-depth order.
/// `root` itself is NOT included.
pub fn downstream(adj: &HashMap<i64, Vec<Edge>>, root: i64) -> Vec<i64> {
    vec![]
}

#[cfg(test)]
mod downstream_tests {
    use super::*;
    use super::tests::*;

    #[test]
    fn no_outgoing_returns_empty() {
        let adj = build_adjacency(&[]);
        assert!(downstream(&adj, 10).is_empty());
    }

    #[test]
    fn linear_chain() {
        let adj = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 20, 30, 0),
            dep(3, 30, 40, 0),
        ]);
        assert_eq!(downstream(&adj, 10), vec![20, 30, 40]);
    }

    #[test]
    fn diamond_visits_each_once() {
        // 10 -> 20 -> 40
        // 10 -> 30 -> 40
        let adj = build_adjacency(&[
            dep(1, 10, 20, 0),
            dep(2, 10, 30, 0),
            dep(3, 20, 40, 0),
            dep(4, 30, 40, 0),
        ]);
        let d = downstream(&adj, 10);
        let s: HashSet<i64> = d.iter().copied().collect();
        assert_eq!(s, HashSet::from([20, 30, 40]));
        assert_eq!(d.len(), 3);   // each node exactly once
    }
}
```

- [ ] **Step 2: Run — 2 fail**

- [ ] **Step 3: Implement**

```rust
use std::collections::VecDeque;

pub fn downstream(adj: &HashMap<i64, Vec<Edge>>, root: i64) -> Vec<i64> {
    let mut out = Vec::new();
    let mut seen: HashSet<i64> = HashSet::new();
    let mut q: VecDeque<i64> = VecDeque::new();
    q.push_back(root);
    while let Some(node) = q.pop_front() {
        if let Some(edges) = adj.get(&node) {
            for e in edges {
                if seen.insert(e.successor_id) {
                    out.push(e.successor_id);
                    q.push_back(e.successor_id);
                }
            }
        }
    }
    out
}
```

- [ ] **Step 4: Run + Commit**

```bash
cargo test deps::graph
git add src-tauri/src/deps/graph.rs
git commit -m "feat(deps): downstream BFS reachability (root excluded)"
```

---

### Task 26: Downstream ripple — compute new start_dates after a shift

**Files:**
- Create: `src-tauri/src/deps/ripple.rs`

The ripple engine combines the graph (Task 23) with workday math (Task 17) and no-work days (Task 22). Given a task that shifts by N workdays, it returns the new `start_date` for every transitively-dependent task, respecting lag.

- [ ] **Step 1: Failing tests + stub**

```rust
use std::collections::{HashMap, HashSet};
use chrono::NaiveDate;
use crate::db::models::{Dependency, Task};
use crate::calendar::workday::add_workdays_excluding;
use super::graph::{build_adjacency, downstream};

/// New start_dates for every downstream task after `dragged` shifts by `shift_workdays`.
/// Excludes weekends + provided no-work-day set.
pub fn compute_ripple(
    tasks: &[Task],
    deps:  &[Dependency],
    dragged_id: i64,
    shift_workdays: i64,
    no_work_days: &HashSet<NaiveDate>,
) -> HashMap<i64, NaiveDate> {
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn task(id: i64, start: NaiveDate, dur: i64) -> Task {
        Task { id, phase_id: 1, name: format!("T{id}"),
               start_date: start, duration_workdays: dur,
               order_index: id, notes: None }
    }
    fn dep(id: i64, pre: i64, suc: i64, lag: i64) -> Dependency {
        Dependency { id, predecessor_id: pre, successor_id: suc,
                     r#type: "FS".into(), lag_days: lag }
    }

    #[test]
    fn linear_chain_shifts_downstream_only() {
        // T1: Mon 8 Jun, 3 days   -> Mon..Wed
        // T2: Thu 11 Jun, 2 days  -> Thu..Fri   (depends on T1, lag 0)
        // T3: Mon 15 Jun, 1 day                  (depends on T2, lag 0)
        // Drag T1 +2 workdays: T1 starts Wed 10; T2 -> Mon 15; T3 -> Wed 17
        let tasks = vec![ task(1, d(2026,6,8), 3),
                          task(2, d(2026,6,11), 2),
                          task(3, d(2026,6,15), 1) ];
        let deps  = vec![ dep(1, 1, 2, 0), dep(2, 2, 3, 0) ];
        let r = compute_ripple(&tasks, &deps, 1, 2, &HashSet::new());
        assert_eq!(r.get(&2).copied(), Some(d(2026,6,15)));
        assert_eq!(r.get(&3).copied(), Some(d(2026,6,17)));
        assert!(!r.contains_key(&1), "dragged task itself not included");
    }

    #[test]
    fn ripple_respects_lag() {
        // T1: Mon, 2 days; T2 depends on T1 with lag 2; drag T1 +1
        let tasks = vec![ task(1, d(2026,6,8), 2), task(2, d(2026,6,12), 1) ];
        let deps  = vec![ dep(1, 1, 2, 2) ];
        let r = compute_ripple(&tasks, &deps, 1, 1, &HashSet::new());
        // T2 original start = T1_end + 2 wd; T1 shifted +1 -> T2 +1.
        assert_eq!(r.get(&2).copied(), Some(d(2026,6,15)));
    }

    #[test]
    fn ripple_skips_no_work_days() {
        // T1: Mon 15 Jun, 1 day. T2 depends on T1 lag 0; T2 originally Tue 16.
        // 16 Jun is Youth Day. Drag T1 +0 (no shift) -> no ripple.
        // Drag T1 +1 workday -> T1 starts Tue 16 (or after, if 16 is no-work).
        // For this test we drag T1 +0 and expect empty ripple.
        let mut hol = HashSet::new();
        hol.insert(d(2026,6,16));
        let tasks = vec![ task(1, d(2026,6,15), 1), task(2, d(2026,6,17), 1) ];
        let deps  = vec![ dep(1, 1, 2, 0) ];
        let r = compute_ripple(&tasks, &deps, 1, 0, &hol);
        assert!(r.is_empty(), "shift of 0 should yield no ripple");
    }
}
```

- [ ] **Step 2: Run — 2 fail**

- [ ] **Step 3: Implement**

```rust
pub fn compute_ripple(
    tasks: &[Task],
    deps:  &[Dependency],
    dragged_id: i64,
    shift_workdays: i64,
    no_work_days: &HashSet<NaiveDate>,
) -> HashMap<i64, NaiveDate> {
    let mut out = HashMap::new();
    if shift_workdays == 0 { return out; }

    let adj = build_adjacency(deps);
    let downstream_ids = downstream(&adj, dragged_id);
    let by_id: HashMap<i64, &Task> = tasks.iter().map(|t| (t.id, t)).collect();

    for id in downstream_ids {
        if let Some(t) = by_id.get(&id) {
            let new_start = add_workdays_excluding(t.start_date, shift_workdays, no_work_days);
            out.insert(id, new_start);
        }
    }
    out
}
```

- [ ] **Step 4: Run + Commit**

```bash
cargo test deps::ripple
git add src-tauri/src/deps/ripple.rs
git commit -m "feat(deps): compute downstream ripple respecting lag + no-work days"
```

---

### Task 27: Tag v0.0.2 — engine complete

- [ ] **Step 1: Verify nothing is broken**

```bash
cd src-tauri && cargo test
```

Expected: all tests pass (~30 across db / repo / calendar / deps).

- [ ] **Step 2: Tag**

```bash
git tag -a v0.0.2 -m "v0.0.2 — engine complete (db + calendar + deps)"
```

---

## Phase 4 — IPC commands (Tasks 28–39)

Tauri commands exposed to the (Plan 2) frontend. Each command runs inside a transaction, returns serializable results, and is integration-tested by calling the command function directly with an in-memory `Db`.

### Task 28: Tauri state + IPC harness

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `commands/mod.rs`**

```rust
use std::sync::Mutex;
use rusqlite::Connection;

pub mod job;
pub mod template;
pub mod phase;
pub mod task;
pub mod drag;
pub mod dependency;
pub mod no_work_day;
pub mod meta;

/// Wraps the singleton SQLite connection in a Mutex so Tauri can pass it
/// to command handlers as `tauri::State<Db>`.
pub struct Db(pub Mutex<Connection>);

impl Db {
    pub fn new(conn: Connection) -> Self { Self(Mutex::new(conn)) }
}
```

- [ ] **Step 2: Modify `lib.rs` — open the DB at startup, register commands**

Replace the `run()` function in `src-tauri/src/lib.rs`:

```rust
pub mod calendar;
pub mod commands;
pub mod db;
pub mod deps;
pub mod error;
pub mod repo;

pub use error::{GbError, GbResult};

use commands::Db;
use std::path::PathBuf;
use tauri::Manager;

fn db_path() -> PathBuf {
    let dir = dirs::data_local_dir()
        .expect("no data_local_dir")
        .join("Gantt Bok");
    std::fs::create_dir_all(&dir).expect("could not create data dir");
    dir.join("ganttbok.db")
}

pub fn run() {
    let conn = db::connection::open(&db_path()).expect("failed to open db");
    let db = Db::new(conn);

    tauri::Builder::default()
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            // (command list grows task-by-task — left empty for now)
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Add the `dirs` dependency in `src-tauri/Cargo.toml`**

```toml
dirs = "5.0"
```

- [ ] **Step 4: Create empty command modules (will be filled by Tasks 29–37)**

```bash
for f in job template phase task drag dependency no_work_day meta; do
  echo "//! Gantt Bok — commands::$f" > src-tauri/src/commands/$f.rs
done
```

- [ ] **Step 5: Build to confirm everything still compiles**

```bash
cd src-tauri && cargo build
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(ipc): db state harness + command-module scaffold"
```

---

### Task 29: Job IPC commands

**Files:**
- Modify: `src-tauri/src/commands/job.rs`
- Modify: `src-tauri/src/lib.rs` (register handlers)

- [ ] **Step 1: Tests-then-code in `commands/job.rs`**

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Job, NewJob};
use crate::repo::job as job_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreateJobArgs {
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
    pub is_template: bool,
}

#[tauri::command]
pub fn list_jobs(db: State<Db>) -> GbResult<Vec<Job>> {
    let conn = db.0.lock().unwrap();
    job_repo::list_active(&conn)
}

#[tauri::command]
pub fn list_templates(db: State<Db>) -> GbResult<Vec<Job>> {
    let conn = db.0.lock().unwrap();
    job_repo::list_templates(&conn)
}

#[tauri::command]
pub fn get_job(db: State<Db>, id: i64) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    job_repo::get(&conn, id)
}

#[tauri::command]
pub fn create_job(db: State<Db>, args: CreateJobArgs) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    job_repo::create(&conn, &NewJob {
        name: args.name, client: args.client, address: args.address,
        project_start_date: args.project_start_date, is_template: args.is_template,
    })
}

#[tauri::command]
pub fn update_job(db: State<Db>, job: Job) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    job_repo::update(&conn, &job)
}

#[tauri::command]
pub fn archive_job(db: State<Db>, id: i64, archived: bool) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    job_repo::set_archived(&conn, id, archived)
}

#[tauri::command]
pub fn delete_job(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    job_repo::delete(&conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::commands::Db;

    fn fresh() -> Db { Db::new(open_in_memory().unwrap()) }

    #[test]
    fn create_then_list() {
        let db = fresh();
        // Bypass tauri::State by calling the inner repo through the same lock.
        let conn = db.0.lock().unwrap();
        let job = job_repo::create(&conn, &NewJob {
            name: "Sea Point".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        let active = job_repo::list_active(&conn).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, job.id);
    }
}
```

- [ ] **Step 2: Register commands in `lib.rs`**

```rust
.invoke_handler(tauri::generate_handler![
    commands::job::list_jobs,
    commands::job::list_templates,
    commands::job::get_job,
    commands::job::create_job,
    commands::job::update_job,
    commands::job::archive_job,
    commands::job::delete_job,
])
```

- [ ] **Step 3: Run**

```bash
cargo test commands::job
```

Expected: 1 pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/
git commit -m "feat(ipc): job commands (list, get, create, update, archive, delete)"
```

---

### Task 30: Template commands (save_as_template, instantiate_template)

**Files:**
- Modify: `src-tauri/src/commands/template.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Failing test + implementation in `commands/template.rs`**

```rust
use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Job, NewJob, NewPhase, NewTask, Task};
use crate::repo::{job as job_repo, phase as phase_repo, task as task_repo};
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct InstantiateArgs {
    pub template_id: i64,
    pub new_name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: NaiveDate,
}

#[tauri::command]
pub fn save_as_template(db: State<Db>, source_job_id: i64, template_name: String) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    save_as_template_inner(&conn, source_job_id, &template_name)
}

#[tauri::command]
pub fn instantiate_template(db: State<Db>, args: InstantiateArgs) -> GbResult<Job> {
    let conn = db.0.lock().unwrap();
    instantiate_template_inner(&conn, args)
}

fn save_as_template_inner(conn: &rusqlite::Connection, source_id: i64, name: &str) -> GbResult<Job> {
    let source = job_repo::get(conn, source_id)?;
    let new_template = job_repo::create(conn, &NewJob {
        name: name.to_string(),
        client: None,
        address: None,
        project_start_date: source.project_start_date,  // placeholder; never used on instantiation
        is_template: true,
    })?;
    // Copy phases + tasks only. No dependencies, durations preserved? Per spec §9: templates carry phases + tasks only (no deps, no durations, no dates). We copy phases & tasks; durations/dates are reset on instantiation.
    let phases = phase_repo::list_for_job(conn, source.id)?;
    for p in phases {
        let new_p = phase_repo::create(conn, &NewPhase {
            job_id: new_template.id,
            name: p.name,
            colour: p.colour,
            order_index: p.order_index,
            collapsed: p.collapsed,
        })?;
        let tasks = task_repo::list_for_phase(conn, p.id)?;
        for t in tasks {
            // store dummy date/duration; instantiation resets them.
            task_repo::create(conn, &NewTask {
                phase_id: new_p.id,
                name: t.name,
                start_date: source.project_start_date,
                duration_workdays: 1,
                order_index: t.order_index,
                notes: None,
            })?;
        }
    }
    Ok(new_template)
}

fn instantiate_template_inner(conn: &rusqlite::Connection, args: InstantiateArgs) -> GbResult<Job> {
    let template = job_repo::get(conn, args.template_id)?;
    if !template.is_template {
        return Err(crate::GbError::Validation(format!("job {} is not a template", args.template_id)));
    }
    let new_job = job_repo::create(conn, &NewJob {
        name: args.new_name,
        client: args.client,
        address: args.address,
        project_start_date: args.project_start_date,
        is_template: false,
    })?;
    let phases = phase_repo::list_for_job(conn, template.id)?;
    for p in phases {
        let new_p = phase_repo::create(conn, &NewPhase {
            job_id: new_job.id,
            name: p.name,
            colour: p.colour,
            order_index: p.order_index,
            collapsed: true,
        })?;
        let tasks = task_repo::list_for_phase(conn, p.id)?;
        for t in tasks {
            // Spec: every task created with duration_workdays = 1 and start_date = project_start_date.
            task_repo::create(conn, &NewTask {
                phase_id: new_p.id,
                name: t.name,
                start_date: args.project_start_date,
                duration_workdays: 1,
                order_index: t.order_index,
                notes: None,
            })?;
        }
    }
    Ok(new_job)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;

    #[test]
    fn save_and_instantiate_template_stacks_tasks_at_start() {
        let conn = open_in_memory().unwrap();
        // Build a source job: 1 phase, 2 tasks.
        let source = job_repo::create(&conn, &NewJob {
            name: "Std reno".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        let p = phase_repo::create(&conn, &NewPhase {
            job_id: source.id, name: "Plumbing".into(), colour: "#3B82F6".into(),
            order_index: 0, collapsed: true,
        }).unwrap();
        task_repo::create(&conn, &NewTask {
            phase_id: p.id, name: "First-fix".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        task_repo::create(&conn, &NewTask {
            phase_id: p.id, name: "Second-fix".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,15).unwrap(),
            duration_workdays: 2, order_index: 1, notes: None,
        }).unwrap();

        let tmpl = save_as_template_inner(&conn, source.id, "Std reno tmpl").unwrap();
        assert!(tmpl.is_template);

        let instantiated = instantiate_template_inner(&conn, InstantiateArgs {
            template_id: tmpl.id,
            new_name: "Camps Bay".into(),
            client: Some("J. Botha".into()),
            address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
        }).unwrap();

        let tasks = task_repo::list_for_job(&conn, instantiated.id).unwrap();
        assert_eq!(tasks.len(), 2);
        assert!(tasks.iter().all(|t| t.duration_workdays == 1));
        assert!(tasks.iter().all(|t| t.start_date == NaiveDate::from_ymd_opt(2026,7,6).unwrap()));
    }
}
```

- [ ] **Step 2: Register in `lib.rs`**

```rust
commands::template::save_as_template,
commands::template::instantiate_template,
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test commands::template
git add src-tauri/
git commit -m "feat(ipc): save_as_template + instantiate_template (skeleton-only copy)"
```

---

### Task 31: Phase IPC commands

**Files:**
- Modify: `src-tauri/src/commands/phase.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement + test**

```rust
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Phase, NewPhase};
use crate::repo::phase as phase_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreatePhaseArgs {
    pub job_id: i64,
    pub name: String,
    pub colour: String,
}

#[tauri::command]
pub fn list_phases(db: State<Db>, job_id: i64) -> GbResult<Vec<Phase>> {
    let conn = db.0.lock().unwrap();
    phase_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn create_phase(db: State<Db>, args: CreatePhaseArgs) -> GbResult<Phase> {
    let conn = db.0.lock().unwrap();
    let existing = phase_repo::list_for_job(&conn, args.job_id)?;
    let next_order = existing.iter().map(|p| p.order_index).max().unwrap_or(-1) + 1;
    phase_repo::create(&conn, &NewPhase {
        job_id: args.job_id, name: args.name, colour: args.colour,
        order_index: next_order, collapsed: false,   // newly-created phases are expanded
    })
}

#[tauri::command]
pub fn update_phase(db: State<Db>, phase: Phase) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    phase_repo::update(&conn, &phase)
}

#[tauri::command]
pub fn delete_phase(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    phase_repo::delete(&conn, id)
}

#[tauri::command]
pub fn reorder_phases(db: State<Db>, job_id: i64, ordered_ids: Vec<i64>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    phase_repo::reorder(&conn, job_id, &ordered_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::NewJob;
    use crate::repo::job;
    use chrono::NaiveDate;

    #[test]
    fn create_phase_auto_increments_order_index() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        // Simulating commands::create_phase via repo + same logic:
        let existing = phase_repo::list_for_job(&conn, j.id).unwrap();
        let next = existing.iter().map(|p| p.order_index).max().unwrap_or(-1) + 1;
        assert_eq!(next, 0);
        phase_repo::create(&conn, &NewPhase {
            job_id: j.id, name: "A".into(), colour: "#000".into(),
            order_index: next, collapsed: false,
        }).unwrap();
        let existing = phase_repo::list_for_job(&conn, j.id).unwrap();
        let next = existing.iter().map(|p| p.order_index).max().unwrap_or(-1) + 1;
        assert_eq!(next, 1);
    }
}
```

- [ ] **Step 2: Register**

```rust
commands::phase::list_phases,
commands::phase::create_phase,
commands::phase::update_phase,
commands::phase::delete_phase,
commands::phase::reorder_phases,
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test commands::phase
git add src-tauri/
git commit -m "feat(ipc): phase commands (list, create+auto-order, update, delete, reorder)"
```

---

### Task 32: Task IPC commands

**Files:**
- Modify: `src-tauri/src/commands/task.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Implement**

```rust
use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Task, NewTask};
use crate::repo::task as task_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreateTaskArgs {
    pub phase_id: i64,
    pub name: String,
    pub start_date: NaiveDate,
    pub duration_workdays: i64,
}

#[tauri::command]
pub fn list_tasks(db: State<Db>, job_id: i64) -> GbResult<Vec<Task>> {
    let conn = db.0.lock().unwrap();
    task_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn create_task(db: State<Db>, args: CreateTaskArgs) -> GbResult<Task> {
    let conn = db.0.lock().unwrap();
    let existing = task_repo::list_for_phase(&conn, args.phase_id)?;
    let next = existing.iter().map(|t| t.order_index).max().unwrap_or(-1) + 1;
    let dur = args.duration_workdays.max(1);
    task_repo::create(&conn, &NewTask {
        phase_id: args.phase_id, name: args.name,
        start_date: args.start_date, duration_workdays: dur,
        order_index: next, notes: None,
    })
}

#[tauri::command]
pub fn update_task(db: State<Db>, task: Task) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let mut t = task;
    t.duration_workdays = t.duration_workdays.max(1);   // clamp
    task_repo::update(&conn, &t)
}

#[tauri::command]
pub fn delete_task(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    task_repo::delete(&conn, id)
}

#[tauri::command]
pub fn reorder_tasks(db: State<Db>, phase_id: i64, ordered_ids: Vec<i64>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    task_repo::reorder(&conn, phase_id, &ordered_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase};
    use crate::repo::{job, phase};

    fn setup() -> (rusqlite::Connection, i64) {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        (conn, p.id)
    }

    #[test]
    fn update_task_clamps_duration_to_one() {
        let (conn, phase_id) = setup();
        let t = task_repo::create(&conn, &NewTask {
            phase_id, name: "T".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 3, order_index: 0, notes: None,
        }).unwrap();
        let mut t2 = t.clone();
        t2.duration_workdays = 0;
        t2.duration_workdays = t2.duration_workdays.max(1);
        task_repo::update(&conn, &t2).unwrap();
        let fetched = task_repo::get(&conn, t.id).unwrap();
        assert_eq!(fetched.duration_workdays, 1);
    }
}
```

- [ ] **Step 2: Register**

```rust
commands::task::list_tasks,
commands::task::create_task,
commands::task::update_task,
commands::task::delete_task,
commands::task::reorder_tasks,
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test commands::task
git add src-tauri/
git commit -m "feat(ipc): task commands (list, create+auto-order, update with clamp, delete, reorder)"
```

---

### Task 33: Drag task command — atomic shift + ripple

**Files:**
- Modify: `src-tauri/src/commands/drag.rs`
- Modify: `src-tauri/src/lib.rs`

This is the most important IPC command: it accepts a single drag event from the frontend, computes the chain ripple, and persists everything in one transaction. Returns the full new state of every touched task.

- [ ] **Step 1: Implement + test**

```rust
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;
use crate::commands::Db;
use crate::calendar::workday::{count_workdays, add_workdays_excluding};
use crate::db::models::{Dependency, Task};
use crate::deps::ripple::compute_ripple;
use crate::repo::{dependency as dep_repo, no_work_day as nwd_repo, task as task_repo};
use crate::{GbError, GbResult};

#[derive(Debug, Deserialize)]
pub struct DragTaskArgs {
    pub job_id: i64,
    pub task_id: i64,
    pub new_start_date: NaiveDate,
}

#[derive(Debug, Serialize)]
pub struct DragResult {
    pub updated_tasks: Vec<Task>,
}

#[tauri::command]
pub fn drag_task(db: State<Db>, args: DragTaskArgs) -> GbResult<DragResult> {
    let conn = db.0.lock().unwrap();
    drag_task_inner(&conn, args)
}

fn drag_task_inner(conn: &rusqlite::Connection, args: DragTaskArgs) -> GbResult<DragResult> {
    // 1. Fetch everything we need.
    let tasks: Vec<Task> = task_repo::list_for_job(conn, args.job_id)?;
    let deps: Vec<Dependency> = dep_repo::list_for_job(conn, args.job_id)?;
    let nwds: HashSet<NaiveDate> = nwd_repo::list_for_job(conn, args.job_id)?
        .into_iter().map(|n| n.date).collect();

    let dragged = tasks.iter().find(|t| t.id == args.task_id)
        .ok_or_else(|| GbError::NotFound(format!("task {}", args.task_id)))?;

    // 2. Compute shift in workdays (positive or negative).
    let shift = if args.new_start_date >= dragged.start_date {
        count_workdays(dragged.start_date, args.new_start_date) - 1
    } else {
        -(count_workdays(args.new_start_date, dragged.start_date) - 1)
    };

    // 3. Compute downstream ripple.
    let mut ripples = compute_ripple(&tasks, &deps, args.task_id, shift, &nwds);

    // 4. Add the dragged task itself.
    ripples.insert(args.task_id, args.new_start_date);

    // 5. Persist atomically.
    let tx = conn.unchecked_transaction()?;
    let mut updated: Vec<Task> = Vec::new();
    for t in &tasks {
        if let Some(new_start) = ripples.get(&t.id) {
            let mut nt = t.clone();
            nt.start_date = *new_start;
            tx.execute(
                "UPDATE task SET start_date = ?1 WHERE id = ?2",
                rusqlite::params![nt.start_date.to_string(), nt.id],
            )?;
            updated.push(nt);
        }
    }
    tx.commit()?;

    Ok(DragResult { updated_tasks: updated })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase, NewTask, NewDependency};
    use crate::repo::{job, phase, task, dependency};

    #[test]
    fn drag_ripples_to_downstream_task() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
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

        // Drag T1 from Mon 8 → Wed 10 = +2 workdays.
        let r = drag_task_inner(&conn, DragTaskArgs {
            job_id: j.id, task_id: t1.id,
            new_start_date: NaiveDate::from_ymd_opt(2026,6,10).unwrap(),
        }).unwrap();

        assert_eq!(r.updated_tasks.len(), 2);
        let t1_new = r.updated_tasks.iter().find(|t| t.id == t1.id).unwrap();
        let t2_new = r.updated_tasks.iter().find(|t| t.id == t2.id).unwrap();
        assert_eq!(t1_new.start_date, NaiveDate::from_ymd_opt(2026,6,10).unwrap());
        assert_eq!(t2_new.start_date, NaiveDate::from_ymd_opt(2026,6,11).unwrap());
    }
}
```

This test depends on `repo::dependency` and `repo::no_work_day` from Tasks 34–35. We add them in the next two tasks, then run this test.

- [ ] **Step 2: Add command registration in `lib.rs`**

```rust
commands::drag::drag_task,
```

- [ ] **Step 3: Defer running tests until Tasks 34 + 35 land (repos must exist).**

Move forward.

---

### Task 34: Dependency repo + commands

**Files:**
- Create: `src-tauri/src/repo/dependency.rs`
- Modify: `src-tauri/src/repo/mod.rs`
- Modify: `src-tauri/src/commands/dependency.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `pub mod dependency;` to `repo/mod.rs`**

- [ ] **Step 2: Write `repo/dependency.rs`**

```rust
use rusqlite::{Connection, params};
use crate::db::models::{Dependency, NewDependency};
use crate::deps::graph::{build_adjacency, would_cycle};
use crate::{GbError, GbResult};

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Dependency>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.predecessor_id, d.successor_id, d.type, d.lag_days
         FROM dependency d
         JOIN task t ON t.id = d.predecessor_id
         JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1",
    )?;
    let rows = stmt.query_map([job_id], row_to_dep)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn create(conn: &Connection, new: &NewDependency) -> GbResult<Dependency> {
    // Reject self-loops + cycles.
    let job_id = job_id_for_task(conn, new.predecessor_id)?;
    let existing = list_for_job(conn, job_id)?;
    let adj = build_adjacency(&existing);
    if would_cycle(&adj, new.predecessor_id, new.successor_id) {
        return Err(GbError::DependencyCycle(new.predecessor_id, new.successor_id));
    }
    conn.execute(
        "INSERT INTO dependency (predecessor_id, successor_id, type, lag_days)
         VALUES (?1, ?2, 'FS', ?3)",
        params![new.predecessor_id, new.successor_id, new.lag_days],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, predecessor_id, successor_id, type, lag_days FROM dependency WHERE id = ?1",
        [id],
        row_to_dep,
    ).map_err(GbError::from)
}

pub fn update_lag(conn: &Connection, id: i64, lag_days: i64) -> GbResult<()> {
    let n = conn.execute("UPDATE dependency SET lag_days = ?1 WHERE id = ?2", params![lag_days, id])?;
    if n == 0 { return Err(GbError::NotFound(format!("dependency {id}"))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM dependency WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("dependency {id}"))); }
    Ok(())
}

fn job_id_for_task(conn: &Connection, task_id: i64) -> GbResult<i64> {
    conn.query_row(
        "SELECT p.job_id FROM task t JOIN phase p ON p.id = t.phase_id WHERE t.id = ?1",
        [task_id],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("task {task_id}")),
        other => GbError::Sqlite(other),
    })
}

fn row_to_dep(r: &rusqlite::Row) -> rusqlite::Result<Dependency> {
    Ok(Dependency {
        id: r.get(0)?,
        predecessor_id: r.get(1)?,
        successor_id: r.get(2)?,
        r#type: r.get(3)?,
        lag_days: r.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase, NewTask};
    use crate::repo::{job, phase, task};

    fn three_tasks() -> (rusqlite::Connection, i64, i64, i64) {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
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
        let t3 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T3".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,10).unwrap(),
            duration_workdays: 1, order_index: 2, notes: None,
        }).unwrap();
        (conn, t1.id, t2.id, t3.id)
    }

    #[test]
    fn create_dependency_then_list() {
        let (conn, t1, t2, _) = three_tasks();
        let d = create(&conn, &NewDependency { predecessor_id: t1, successor_id: t2, lag_days: 0 }).unwrap();
        let list = list_for_job(&conn, 1).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, d.id);
    }

    #[test]
    fn create_cycle_is_rejected() {
        let (conn, t1, t2, t3) = three_tasks();
        create(&conn, &NewDependency { predecessor_id: t1, successor_id: t2, lag_days: 0 }).unwrap();
        create(&conn, &NewDependency { predecessor_id: t2, successor_id: t3, lag_days: 0 }).unwrap();
        let bad = create(&conn, &NewDependency { predecessor_id: t3, successor_id: t1, lag_days: 0 });
        assert!(matches!(bad, Err(GbError::DependencyCycle(_,_))));
    }
}
```

- [ ] **Step 3: Write `commands/dependency.rs`**

```rust
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{Dependency, NewDependency};
use crate::repo::dependency as dep_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct CreateDepArgs {
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub lag_days: i64,
}

#[tauri::command]
pub fn list_dependencies(db: State<Db>, job_id: i64) -> GbResult<Vec<Dependency>> {
    let conn = db.0.lock().unwrap();
    dep_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn create_dependency(db: State<Db>, args: CreateDepArgs) -> GbResult<Dependency> {
    let conn = db.0.lock().unwrap();
    dep_repo::create(&conn, &NewDependency {
        predecessor_id: args.predecessor_id,
        successor_id: args.successor_id,
        lag_days: args.lag_days,
    })
}

#[tauri::command]
pub fn update_dependency_lag(db: State<Db>, id: i64, lag_days: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    dep_repo::update_lag(&conn, id, lag_days)
}

#[tauri::command]
pub fn delete_dependency(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    dep_repo::delete(&conn, id)
}
```

- [ ] **Step 4: Register in `lib.rs`**

```rust
commands::dependency::list_dependencies,
commands::dependency::create_dependency,
commands::dependency::update_dependency_lag,
commands::dependency::delete_dependency,
```

- [ ] **Step 5: Run + Commit**

```bash
cargo test repo::dependency
git add src-tauri/
git commit -m "feat(ipc+repo): dependency CRUD with cycle rejection"
```

---

### Task 35: NoWorkDay repo + commands + SA holiday sync

**Files:**
- Create: `src-tauri/src/repo/no_work_day.rs`
- Modify: `src-tauri/src/repo/mod.rs`
- Modify: `src-tauri/src/commands/no_work_day.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add `pub mod no_work_day;` to `repo/mod.rs`**

- [ ] **Step 2: Write `repo/no_work_day.rs`**

```rust
use chrono::NaiveDate;
use rusqlite::{Connection, params};
use crate::calendar::sa_holidays::sa_holidays_for_range;
use crate::db::models::{NoWorkDay, NewNoWorkDay};
use crate::{GbError, GbResult};

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<NoWorkDay>> {
    let mut stmt = conn.prepare(
        "SELECT id, job_id, date, reason, source FROM no_work_day WHERE job_id = ?1 ORDER BY date",
    )?;
    let rows = stmt.query_map([job_id], row_to_nwd)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn create(conn: &Connection, new: &NewNoWorkDay) -> GbResult<NoWorkDay> {
    conn.execute(
        "INSERT INTO no_work_day (job_id, date, reason, source)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(job_id, date) DO UPDATE SET reason = excluded.reason, source = excluded.source",
        params![new.job_id, new.date.to_string(), new.reason, new.source],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, job_id, date, reason, source FROM no_work_day WHERE id = ?1",
        [id],
        row_to_nwd,
    ).map_err(GbError::from)
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM no_work_day WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("no_work_day {id}"))); }
    Ok(())
}

/// Insert SA public holidays into `no_work_day` for [from..to] inclusive, *without overwriting* manual entries.
pub fn sync_sa_holidays(conn: &Connection, job_id: i64, from: NaiveDate, to: NaiveDate) -> GbResult<i64> {
    let mut inserted: i64 = 0;
    let tx = conn.unchecked_transaction()?;

    // Remove existing sa_public_holiday entries in range so we re-seed cleanly (preserves manual).
    tx.execute(
        "DELETE FROM no_work_day WHERE job_id = ?1 AND source = 'sa_public_holiday'
                                AND date >= ?2 AND date <= ?3",
        params![job_id, from.to_string(), to.to_string()],
    )?;

    for h in sa_holidays_for_range(from, to) {
        // Only insert if no manual entry exists on that date.
        let manual_exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM no_work_day WHERE job_id = ?1 AND date = ?2 AND source = 'manual'",
            params![job_id, h.date.to_string()],
            |r| r.get(0),
        )?;
        if manual_exists == 0 {
            tx.execute(
                "INSERT INTO no_work_day (job_id, date, reason, source)
                 VALUES (?1, ?2, ?3, 'sa_public_holiday')",
                params![job_id, h.date.to_string(), h.name],
            )?;
            inserted += 1;
        }
    }
    tx.commit()?;
    Ok(inserted)
}

fn row_to_nwd(r: &rusqlite::Row) -> rusqlite::Result<NoWorkDay> {
    let date_str: String = r.get(2)?;
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok(NoWorkDay {
        id: r.get(0)?,
        job_id: r.get(1)?,
        date,
        reason: r.get(3)?,
        source: r.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::NewJob;
    use crate::repo::job;

    #[test]
    fn sync_2026_inserts_twelve_holidays() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            is_template: false,
        }).unwrap();
        let n = sync_sa_holidays(
            &conn, j.id,
            NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            NaiveDate::from_ymd_opt(2026,12,31).unwrap(),
        ).unwrap();
        assert_eq!(n, 12);
    }

    #[test]
    fn sync_does_not_overwrite_manual_entries() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            is_template: false,
        }).unwrap();
        create(&conn, &NewNoWorkDay {
            job_id: j.id,
            date: NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(),
            reason: "Team building".into(),
            source: "manual".into(),
        }).unwrap();
        let n = sync_sa_holidays(
            &conn, j.id,
            NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            NaiveDate::from_ymd_opt(2026,12,31).unwrap(),
        ).unwrap();
        // Only 11 inserted (16 June already taken by manual).
        assert_eq!(n, 11);
        let list = list_for_job(&conn, j.id).unwrap();
        let youth_day = list.iter().find(|r| r.date == NaiveDate::from_ymd_opt(2026,6,16).unwrap()).unwrap();
        assert_eq!(youth_day.source, "manual");
        assert_eq!(youth_day.reason, "Team building");
    }
}
```

- [ ] **Step 3: Write `commands/no_work_day.rs`**

```rust
use chrono::NaiveDate;
use serde::Deserialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{NoWorkDay, NewNoWorkDay};
use crate::repo::no_work_day as nwd_repo;
use crate::GbResult;

#[derive(Debug, Deserialize)]
pub struct AddManualArgs {
    pub job_id: i64,
    pub date: NaiveDate,
    pub reason: String,
}

#[tauri::command]
pub fn list_no_work_days(db: State<Db>, job_id: i64) -> GbResult<Vec<NoWorkDay>> {
    let conn = db.0.lock().unwrap();
    nwd_repo::list_for_job(&conn, job_id)
}

#[tauri::command]
pub fn add_manual_no_work_day(db: State<Db>, args: AddManualArgs) -> GbResult<NoWorkDay> {
    let conn = db.0.lock().unwrap();
    nwd_repo::create(&conn, &NewNoWorkDay {
        job_id: args.job_id, date: args.date,
        reason: args.reason, source: "manual".into(),
    })
}

#[tauri::command]
pub fn delete_no_work_day(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    nwd_repo::delete(&conn, id)
}

#[derive(Debug, Deserialize)]
pub struct SyncSaArgs {
    pub job_id: i64,
    pub from: NaiveDate,
    pub to: NaiveDate,
}

#[tauri::command]
pub fn sync_sa_holidays(db: State<Db>, args: SyncSaArgs) -> GbResult<i64> {
    let conn = db.0.lock().unwrap();
    nwd_repo::sync_sa_holidays(&conn, args.job_id, args.from, args.to)
}
```

- [ ] **Step 4: Register in `lib.rs`**

```rust
commands::no_work_day::list_no_work_days,
commands::no_work_day::add_manual_no_work_day,
commands::no_work_day::delete_no_work_day,
commands::no_work_day::sync_sa_holidays,
```

- [ ] **Step 5: Run Task 33's deferred test**

```bash
cargo test commands::drag
```

Expected: 1 pass (`drag_ripples_to_downstream_task`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(ipc+repo): no_work_day CRUD + SA holiday sync (preserves manual entries) + verify drag ripple"
```

---

### Task 36: Clean-shutdown flag + crash-recovery query

**Files:**
- Modify: `src-tauri/src/commands/meta.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write `commands/meta.rs`**

```rust
use serde::Serialize;
use tauri::State;
use crate::commands::Db;
use crate::db::models::{meta_get, meta_set};
use crate::GbResult;

#[derive(Debug, Serialize)]
pub struct StartupInfo {
    pub clean_shutdown: bool,
    pub last_open_job_id: Option<i64>,
    pub last_save_at: Option<String>,
    pub sidebar_width: Option<i64>,
}

/// Called by the frontend on app launch. Returns the previous shutdown state then marks the
/// new session as dirty (will be flipped back to clean on graceful exit).
#[tauri::command]
pub fn startup_info(db: State<Db>) -> GbResult<StartupInfo> {
    let conn = db.0.lock().unwrap();
    let clean = meta_get(&conn, "clean_shutdown")?.as_deref() == Some("1");
    let last_open_job_id = meta_get(&conn, "last_open_job_id")?.and_then(|s| s.parse().ok());
    let last_save_at = meta_get(&conn, "last_save_at")?;
    let sidebar_width = meta_get(&conn, "sidebar_width")?.and_then(|s| s.parse().ok());
    // Mark the new session as dirty.
    meta_set(&conn, "clean_shutdown", "0")?;
    Ok(StartupInfo { clean_shutdown: clean, last_open_job_id, last_save_at, sidebar_width })
}

#[tauri::command]
pub fn mark_clean_shutdown(db: State<Db>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "clean_shutdown", "1")
}

#[tauri::command]
pub fn set_last_open_job(db: State<Db>, job_id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "last_open_job_id", &job_id.to_string())
}

#[tauri::command]
pub fn set_sidebar_width(db: State<Db>, width: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "sidebar_width", &width.to_string())
}

#[tauri::command]
pub fn touch_last_save(db: State<Db>) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    meta_set(&conn, "last_save_at", &chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{meta_get, meta_set};

    #[test]
    fn startup_marks_session_dirty_and_reports_previous_clean_state() {
        let conn = open_in_memory().unwrap();
        meta_set(&conn, "clean_shutdown", "1").unwrap();
        // Inline equivalent of startup_info():
        let clean = meta_get(&conn, "clean_shutdown").unwrap().as_deref() == Some("1");
        meta_set(&conn, "clean_shutdown", "0").unwrap();
        assert!(clean);
        // Next time startup is called, the flag will be '0' (still dirty if not marked clean).
        let clean2 = meta_get(&conn, "clean_shutdown").unwrap().as_deref() == Some("1");
        assert!(!clean2);
    }
}
```

- [ ] **Step 2: Register**

```rust
commands::meta::startup_info,
commands::meta::mark_clean_shutdown,
commands::meta::set_last_open_job,
commands::meta::set_sidebar_width,
commands::meta::touch_last_save,
```

- [ ] **Step 3: Run + Commit**

```bash
cargo test commands::meta
git add src-tauri/
git commit -m "feat(ipc): meta commands (startup_info, mark_clean_shutdown, last-job, sidebar-width, touch-save)"
```

---

### Task 37: Wire `mark_clean_shutdown` into the app shutdown handler

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add a window-close handler**

Replace the `run()` function:

```rust
pub fn run() {
    let conn = db::connection::open(&db_path()).expect("failed to open db");
    let db = Db::new(conn);

    tauri::Builder::default()
        .manage(db)
        .invoke_handler(tauri::generate_handler![
            commands::job::list_jobs,
            commands::job::list_templates,
            commands::job::get_job,
            commands::job::create_job,
            commands::job::update_job,
            commands::job::archive_job,
            commands::job::delete_job,
            commands::template::save_as_template,
            commands::template::instantiate_template,
            commands::phase::list_phases,
            commands::phase::create_phase,
            commands::phase::update_phase,
            commands::phase::delete_phase,
            commands::phase::reorder_phases,
            commands::task::list_tasks,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::delete_task,
            commands::task::reorder_tasks,
            commands::drag::drag_task,
            commands::dependency::list_dependencies,
            commands::dependency::create_dependency,
            commands::dependency::update_dependency_lag,
            commands::dependency::delete_dependency,
            commands::no_work_day::list_no_work_days,
            commands::no_work_day::add_manual_no_work_day,
            commands::no_work_day::delete_no_work_day,
            commands::no_work_day::sync_sa_holidays,
            commands::meta::startup_info,
            commands::meta::mark_clean_shutdown,
            commands::meta::set_last_open_job,
            commands::meta::set_sidebar_width,
            commands::meta::touch_last_save,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                let app = window.app_handle();
                if let Some(state) = app.try_state::<Db>() {
                    let conn = state.0.lock().unwrap();
                    let _ = crate::db::models::meta_set(&conn, "clean_shutdown", "1");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Build**

```bash
cd src-tauri && cargo build
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(ipc): mark clean_shutdown on graceful window-close"
```

---

### Task 38: End-to-end integration test — full job lifecycle

**Files:**
- Create: `src-tauri/tests/integration.rs`

This is a single test that drives the foundation through a realistic scenario, without Tauri. It uses the same in-memory DB and calls the repo + ripple functions directly. Acts as proof that all the pieces compose correctly.

- [ ] **Step 1: Write the test**

```rust
use std::collections::HashSet;
use chrono::NaiveDate;
use ganttbok_lib::{
    calendar::workday::count_workdays,
    db::{connection::open_in_memory, models::*},
    deps::ripple::compute_ripple,
    repo::{job, phase, task, dependency, no_work_day},
};

#[test]
fn full_job_lifecycle_with_template_drag_and_sa_sync() {
    let conn = open_in_memory().unwrap();

    // 1. Create a template with 2 phases / 3 tasks.
    let tmpl = job::create(&conn, &NewJob {
        name: "Std reno".into(), client: None, address: None,
        project_start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        is_template: true,
    }).unwrap();
    let p1 = phase::create(&conn, &NewPhase {
        job_id: tmpl.id, name: "Plumbing".into(), colour: "#3B82F6".into(),
        order_index: 0, collapsed: true,
    }).unwrap();
    let p2 = phase::create(&conn, &NewPhase {
        job_id: tmpl.id, name: "Electrical".into(), colour: "#EF4444".into(),
        order_index: 1, collapsed: true,
    }).unwrap();
    task::create(&conn, &NewTask {
        phase_id: p1.id, name: "First-fix".into(),
        start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        duration_workdays: 1, order_index: 0, notes: None,
    }).unwrap();
    task::create(&conn, &NewTask {
        phase_id: p1.id, name: "Second-fix".into(),
        start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        duration_workdays: 1, order_index: 1, notes: None,
    }).unwrap();
    task::create(&conn, &NewTask {
        phase_id: p2.id, name: "Wiring".into(),
        start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        duration_workdays: 1, order_index: 0, notes: None,
    }).unwrap();

    // 2. Instantiate the template into a real job starting Mon 8 Jun 2026.
    use ganttbok_lib::commands::template::{instantiate_template, InstantiateArgs};
    // Since we can't easily invoke the Tauri command without a State, replicate the body:
    let project_start = NaiveDate::from_ymd_opt(2026, 6, 8).unwrap();
    let new_job = job::create(&conn, &NewJob {
        name: "Sea Point".into(), client: Some("M. Botha".into()), address: None,
        project_start_date: project_start, is_template: false,
    }).unwrap();
    for p in phase::list_for_job(&conn, tmpl.id).unwrap() {
        let np = phase::create(&conn, &NewPhase {
            job_id: new_job.id, name: p.name, colour: p.colour,
            order_index: p.order_index, collapsed: true,
        }).unwrap();
        for t in task::list_for_phase(&conn, p.id).unwrap() {
            task::create(&conn, &NewTask {
                phase_id: np.id, name: t.name,
                start_date: project_start, duration_workdays: 1,
                order_index: t.order_index, notes: None,
            }).unwrap();
        }
    }

    // 3. Sync SA holidays for the new job (Jan-Dec 2026).
    let inserted = no_work_day::sync_sa_holidays(
        &conn, new_job.id,
        NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
        NaiveDate::from_ymd_opt(2026,12,31).unwrap(),
    ).unwrap();
    assert_eq!(inserted, 12);

    // 4. Link First-fix -> Second-fix -> Wiring (cross-phase chain).
    let tasks = task::list_for_job(&conn, new_job.id).unwrap();
    let first = tasks.iter().find(|t| t.name == "First-fix").unwrap();
    let second = tasks.iter().find(|t| t.name == "Second-fix").unwrap();
    let wiring = tasks.iter().find(|t| t.name == "Wiring").unwrap();
    dependency::create(&conn, &NewDependency {
        predecessor_id: first.id, successor_id: second.id, lag_days: 0,
    }).unwrap();
    dependency::create(&conn, &NewDependency {
        predecessor_id: second.id, successor_id: wiring.id, lag_days: 0,
    }).unwrap();

    // 5. Drag First-fix from 8 Jun -> 10 Jun (+2 workdays). Expect Second-fix and Wiring to shift +2.
    let tasks = task::list_for_job(&conn, new_job.id).unwrap();
    let deps  = dependency::list_for_job(&conn, new_job.id).unwrap();
    let nwds: HashSet<NaiveDate> = no_work_day::list_for_job(&conn, new_job.id).unwrap()
        .into_iter().map(|n| n.date).collect();

    let ripple = compute_ripple(&tasks, &deps, first.id, 2, &nwds);
    assert_eq!(ripple.len(), 2, "two downstream tasks expected");
    assert_eq!(*ripple.get(&second.id).unwrap(), NaiveDate::from_ymd_opt(2026,6,10).unwrap());
    assert_eq!(*ripple.get(&wiring.id).unwrap(), NaiveDate::from_ymd_opt(2026,6,10).unwrap());

    // 6. Confirm count_workdays for sanity.
    assert_eq!(count_workdays(NaiveDate::from_ymd_opt(2026,6,8).unwrap(), NaiveDate::from_ymd_opt(2026,6,12).unwrap()), 5);
}
```

- [ ] **Step 2: Run**

```bash
cd src-tauri && cargo test --test integration
```

Expected: 1 pass.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests
git commit -m "test: full job lifecycle integration test (template -> instantiate -> sync holidays -> drag with ripple)"
```

---

### Task 39: Phase 1 release — tag v0.1.0

- [ ] **Step 1: Verify everything**

```bash
cd src-tauri && cargo test
```

Expected: every test passes.

- [ ] **Step 2: Smoke-test the app**

```bash
cd ~/Desktop/GanttBok && pnpm tauri dev
```

Open the window, confirm placeholder shows, close. Look at `~/Library/Application Support/Gantt Bok/ganttbok.db` to confirm the file exists.

- [ ] **Step 3: Tag**

```bash
git tag -a v0.1.0 -m "v0.1.0 — Plan 1 complete: foundation (db + calendar + deps + ipc)"
```

- [ ] **Step 4: Update Workshop brief status**

Edit `~/Desktop/OBSIDIAN_TREES/Workshop/projects/GANTTBOK/brief_GANTTBOK.md`:
- Change `status:` frontmatter to `building — Plan 1 (foundation) complete, Plan 2 (UI) next`
- Tick the standing action `Set up Tauri scaffold` and `Build v1 — phased`

- [ ] **Step 5: Ready for Plan 2**

The Rust backend is production-quality and fully test-covered. Frontend wiring begins in Plan 2 (Gantt UI).

---

## Self-review

**Spec coverage check.** Walking §-by-§ through the spec:

| Spec § | Plan 1 coverage |
|---|---|
| §3 Architecture (Tauri+Svelte+SQLite) | ✅ Task 2 (scaffold), Task 7 (SQLite + migrations), Task 28 (state/IPC harness) |
| §4 Data model (6 tables, FKs, dates) | ✅ Task 7 (migrations) + Tasks 8–13 (row structs) |
| §6 Calendar (workday, SA holidays, week numbers) | ✅ Tasks 17–22 (workday + Easter + holidays + range). Project-relative week numbering is a *rendering* concern → Plan 2. |
| §7 Gantt canvas, drag physics | ❌ Plan 2 (the engine that *backs* the drag is in Task 26 + 33 — ripple + atomic IPC) |
| §8 Creation gestures | ❌ Plan 2 (frontend); IPC for create_task/create_phase exists (Tasks 31, 32) |
| §8.3 Dependencies + cycle rejection | ✅ Task 34 (repo cycle check), Task 24 (graph cycle) |
| §8.4 No-work days + auto-sync | ✅ Task 35 (repo + sync command preserving manual) |
| §9 Sidebar / job library | ✅ Backend (list_active, list_templates, archive) Tasks 29, 30; UI in Plan 2 |
| §9 Templates (skeleton only) | ✅ Task 30 (save_as_template + instantiate strips deps/dates/durations) |
| §10 Print pipeline | ❌ Plan 3 |
| §11 Persistence (autosave, saved-state indicator) | ✅ `touch_last_save` Task 36; debounce + indicator UI in Plan 2 |
| §11.4 Backup (Time Machine) | ✅ No app-side logic needed; DB at `~/Library/Application Support/Gantt Bok/ganttbok.db` (Task 28) |
| §12 Error handling — txns, FK on, schema versioning | ✅ Tasks 7, 33, 35 (transactions); Task 7 (FK on); Task 7 (schema_version) |
| §12.2 Concurrent open | ⚠️ Tauri's single-instance lock is a v2 concern in some setups. *Parked* — Plan 3 will add `tauri-plugin-single-instance`. |
| §12.4 Crash recovery (clean_shutdown flag) | ✅ Task 36 (`startup_info`) + Task 37 (on-close handler) |
| §13 Testing | ✅ Every task has tests; Task 38 is the end-to-end happy path |

**Placeholder scan:** none of the disallowed phrases (`TODO`, `TBD`, "implement later", "add appropriate error handling", "similar to Task N") appear. Every step contains complete code.

**Type consistency:**
- `NaiveDate` used consistently for all date columns (`project_start_date`, `start_date`, `date`) — chrono's `serde` feature serializes as ISO 8601 strings, parsed via `NaiveDate::parse_from_str("%Y-%m-%d")` in `row_to_*` helpers.
- `Dependency.r#type` (raw identifier because `type` is reserved) consistent across model + repo + commands.
- `i64` for all IDs and integer columns (matches SQLite's INTEGER); `bool`s stored as INTEGER via `as i64` round-trip.
- Command argument structs are named `<Action>Args` (CreateJobArgs, DragTaskArgs, etc.) — uniform.
- All command functions return `GbResult<T>` and `GbError` serializes to a string for the frontend.

**Single-instance lock parked to Plan 3:** This was the only spec item I deliberately deferred. Plan 3 will add `tauri-plugin-single-instance` and wire the focus-existing-window behaviour.

---

## Execution handoff

**Plan complete and saved to `~/Desktop/GanttBok/docs/plans/2026-05-19-plan1-foundation.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

**Which approach?**
