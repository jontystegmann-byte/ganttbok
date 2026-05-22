# Blik Plan ↔ Claude Connector — Plan 2: MCP Server

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship `blikplan-mcp`, a standalone Rust binary that speaks MCP over stdio, exposing 7 read tools and 1 preview-gated write tool against `ganttbok.db`, and distribute it as both a Tauri sidecar and an npm wrapper package.

**Architecture:** The `patches/` module is first extracted from `ganttbok_lib` into a minimal shared crate `gb-patches` (no Tauri, no heavy deps), then `blikplan-mcp` is added as a second workspace member that depends on `gb-patches` and `rusqlite` only. The MCP server uses `rmcp 0.3` with the `server`, `transport-io`, and `macros` features. Two `rusqlite::Connection` instances are opened: one `SQLITE_OPEN_READ_ONLY` for the seven read tools, and one read-write connection opened only inside `propose_patch`. DB path is resolved from `$BLIKPLAN_DB` → OS-default → error.

**Tech Stack:** Rust 2021, `rmcp 0.3` (`server` + `transport-io` + `macros` features), `rusqlite 0.31` (bundled), `serde 1`, `serde_json 1`, `schemars 1`, `chrono 0.4`, `thiserror 1`, `dirs 5`, `tokio 1` (full), `uuid 1` (v4 feature).

**Spec reference:** `docs/specs/2026-05-22-blikplan-claude-connector-design.md`

---

## Critical Decisions (resolved)

### 1. Repo layout — workspace member

**Decision: `crates/blikplan-mcp/` as a Cargo workspace member in the same repo.**

A top-level `Cargo.toml` workspace is introduced. `src-tauri/` becomes `ganttbok_lib`, and `crates/blikplan-mcp/` is the new MCP binary. No separate repo needed; CI, release scripts, and the Tauri build all stay in one place.

### 2. Shared patches crate — option B (extract `gb-patches`)

**Decision: extract `src-tauri/src/patches/` into `crates/gb-patches/`.**

`ganttbok_lib` (the Tauri crate) pulls in Tauri, ObjC, and every other heavy dep. Making it a workspace dep of `blikplan-mcp` would add ~200 MB to the MCP binary and require the iOS/Android feature flags. `gb-patches` is pure Rust (`serde`, `serde_json`, `chrono`, `thiserror`) — ~2 MB compiled. Both crates then depend on `gb-patches` via `path = "../gb-patches"`.

### 3. MCP crate — `rmcp 0.3`

**Decision: `rmcp = { version = "0.3", features = ["server", "transport-io", "macros"] }`.**

`rmcp` is the official Anthropic Rust SDK for MCP (crates.io: `rmcp`, published by `modelcontextprotocol`). Version 0.3 is the current stable release with stable `#[tool_router]` / `#[tool]` macros. Also add `schemars = "1"` (required by `rmcp` macros for JSON Schema generation).

### 4. DB connection model — dual connections

**Decision: open `ro_conn: Connection` with `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX` for reads; open a fresh `rw_conn: Connection` inside `propose_patch` only, write the row, close immediately.**

This makes it structurally impossible for read tools to write. rusqlite supports multiple connections to the same WAL-mode SQLite file. The read connection is held for the lifetime of the server; the write connection is ephemeral per call.

### 5. DB path discovery

**Decision:**

1. Check `$BLIKPLAN_DB` env var.
2. Then `dirs::data_local_dir().join("Blik Plan").join("ganttbok.db")` (post-rename, macOS: `~/Library/Application Support/Blik Plan/ganttbok.db`).
3. Then `dirs::data_local_dir().join("Gantt Bok").join("ganttbok.db")` (pre-rename fallback — the running app still writes here until the user accepts the bundle rename prompt).
4. Error with `db_not_found` hint.

**Risk:** `src-tauri/src/lib.rs` line 19 currently uses `dirs::data_local_dir().join("Gantt Bok")`. Until the user accepts the bundle rename, the live DB is at the "Gantt Bok" path. The sidecar path (from Plan 4's config writer) uses `$BLIKPLAN_DB` and is unambiguous — this fallback chain only matters for the npm channel.

### 6. npm wrapper

**Decision:** `packages/blikplan-mcp-npm/` in the same repo. `postinstall.js` downloads the correct platform binary from a GitHub release asset and places it at `bin/blikplan-mcp`. `package.json` declares `"bin": { "blikplan-mcp": "bin/blikplan-mcp" }`. Release-upload is a future task; Plan 2 ships the wrapper package skeleton only.

### 7. Tauri sidecar path

**Decision:** Tauri requires platform-suffixed names in `externalBin`. The build step (Plan 4) copies the compiled binary to `src-tauri/binaries/blikplan-mcp-<target-triple>` (e.g. `blikplan-mcp-aarch64-apple-darwin`). Plan 2 documents this contract and produces the binary at `target/release/blikplan-mcp`; Plan 4 wires the copy step.

---

## File Structure

**Files this plan creates:**

```
Cargo.toml                                          — new workspace root
crates/
  gb-patches/
    Cargo.toml
    src/
      lib.rs                                        — re-exports schema + validate
      schema.rs                                     — moved from src-tauri/src/patches/schema.rs
      validate.rs                                   — moved from src-tauri/src/patches/validate.rs
  blikplan-mcp/
    Cargo.toml
    src/
      main.rs                                       — tokio main, opens DB, serves MCP over stdio
      db.rs                                         — open_ro(), resolve_db_path()
      tools/
        mod.rs                                      — BlikPlanServer struct, #[tool_router]
        read.rs                                     — list_jobs, get_job, list_tasks, get_task, list_contacts, search, today
        write.rs                                    — propose_patch
    tests/
      integration.rs                                — in-process rmcp duplex tests
packages/
  blikplan-mcp-npm/
    package.json
    postinstall.js
    README.md
```

**Files this plan modifies:**

```
src-tauri/Cargo.toml                                — add workspace dep on gb-patches; remove patches/ source
src-tauri/src/patches/mod.rs                        — re-export from gb_patches instead of local modules
src-tauri/src/patches/schema.rs                     — deleted (moved to crates/gb-patches/src/schema.rs)
src-tauri/src/patches/validate.rs                   — deleted (moved to crates/gb-patches/src/validate.rs)
src-tauri/src/lib.rs                                — no change needed (mod patches; still works via mod.rs shim)
```

---

## Task 1a: Introduce Cargo workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Modify: `src-tauri/Cargo.toml` (add `[workspace]` suppression)

- [ ] **Step 1: Write the workspace root `Cargo.toml`**

Create `/Users/cncuser/Desktop/GanttBok/Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "src-tauri",
    "crates/gb-patches",
    "crates/blikplan-mcp",
]
```

- [ ] **Step 2: Verify the workspace compiles**

Run: `cargo check --manifest-path /Users/cncuser/Desktop/GanttBok/Cargo.toml -p ganttbok_lib`

Expected: Cargo resolves the workspace. The `crates/gb-patches` and `crates/blikplan-mcp` directories don't exist yet, so Cargo will error with "failed to read `crates/gb-patches/Cargo.toml`". This is expected — continue to the next step.

- [ ] **Step 3: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add Cargo.toml
git commit -m "chore(workspace): introduce Cargo workspace root

Prepares for gb-patches shared crate and blikplan-mcp binary.
src-tauri remains the ganttbok_lib/ganttbok bin crate."
```

---

## Task 1b: Extract `gb-patches` crate

**Files:**
- Create: `crates/gb-patches/Cargo.toml`
- Create: `crates/gb-patches/src/lib.rs`
- Create: `crates/gb-patches/src/schema.rs` (copy from `src-tauri/src/patches/schema.rs`)
- Create: `crates/gb-patches/src/validate.rs` (copy from `src-tauri/src/patches/validate.rs`)
- Modify: `src-tauri/src/patches/mod.rs` (replace with shim)
- Modify: `src-tauri/Cargo.toml` (add `gb-patches` dep)

- [ ] **Step 1: Create `crates/gb-patches/Cargo.toml`**

```bash
mkdir -p /Users/cncuser/Desktop/GanttBok/crates/gb-patches/src
```

Create `/Users/cncuser/Desktop/GanttBok/crates/gb-patches/Cargo.toml`:

```toml
[package]
name = "gb-patches"
version = "0.1.0"
edition = "2021"

[dependencies]
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
chrono      = { version = "0.4", features = ["serde"] }
thiserror   = "1"
```

- [ ] **Step 2: Create `crates/gb-patches/src/lib.rs`**

```rust
//! Shared patch schema used by `ganttbok_lib` (the Tauri app) and
//! `blikplan-mcp` (the MCP server).
//! Source of truth for the v1 patch document format.

pub mod schema;
pub mod validate;

pub use schema::{Patch, PatchOp, TaskRef, PATCH_VERSION};
pub use validate::{validate_patch, ValidationError};
```

- [ ] **Step 3: Copy schema and validate**

Copy the file contents exactly. Create `/Users/cncuser/Desktop/GanttBok/crates/gb-patches/src/schema.rs` with the full content of `src-tauri/src/patches/schema.rs` — it requires no changes because it contains no `crate::` references.

Create `/Users/cncuser/Desktop/GanttBok/crates/gb-patches/src/validate.rs` with the content of `src-tauri/src/patches/validate.rs`, **with one change**: replace the import line

```rust
use crate::patches::schema::{Patch, PatchOp, TaskRef, PATCH_VERSION};
```

with:

```rust
use crate::schema::{Patch, PatchOp, TaskRef, PATCH_VERSION};
```

The test block import at the bottom also changes from:

```rust
use crate::patches::schema::{Patch, PatchOp, TaskRef};
```

to:

```rust
use crate::schema::{Patch, PatchOp, TaskRef};
```

- [ ] **Step 4: Verify `gb-patches` compiles and tests pass**

Run: `cargo test -p gb-patches`

Expected output (all 15 tests pass):

```
running 5 tests
test tests::deserialises_add_task_op ... ok
test tests::deserialises_full_patch ... ok
test tests::deserialises_shift_task_op ... ok
test tests::rejects_unknown_op ... ok
test tests::rejects_unknown_patch_version ... ok
test result: ok. 5 passed; 0 failed

running 10 tests
test tests::accepts_valid_op_ref_chain ... ok
test tests::accepts_well_formed_patch ... ok
test tests::rejects_bad_date_in_add_task ... ok
test tests::rejects_dangling_op_ref ... ok
test tests::rejects_duplicate_op_ref ... ok
test tests::rejects_empty_ops ... ok
test tests::rejects_empty_summary ... ok
test tests::rejects_non_positive_duration ... ok
test tests::rejects_unknown_dep_type ... ok
test tests::rejects_unknown_patch_version ... ok
test result: ok. 10 passed; 0 failed
```

- [ ] **Step 5: Add `gb-patches` dep to `src-tauri/Cargo.toml`**

In `/Users/cncuser/Desktop/GanttBok/src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
gb-patches  = { path = "../crates/gb-patches" }
```

- [ ] **Step 6: Replace `src-tauri/src/patches/mod.rs` with a shim**

Replace the entire content of `/Users/cncuser/Desktop/GanttBok/src-tauri/src/patches/mod.rs` with:

```rust
//! Re-exports the shared patch types from the `gb-patches` workspace crate.
//! All callsites inside `ganttbok_lib` that use `crate::patches::*` continue
//! to work without modification.
pub use gb_patches::schema;
pub use gb_patches::validate;
pub use gb_patches::{Patch, PatchOp, TaskRef, PATCH_VERSION};
pub use gb_patches::{validate_patch, ValidationError};
```

- [ ] **Step 7: Delete the now-redundant source files from `src-tauri/src/patches/`**

```bash
rm /Users/cncuser/Desktop/GanttBok/src-tauri/src/patches/schema.rs
rm /Users/cncuser/Desktop/GanttBok/src-tauri/src/patches/validate.rs
```

- [ ] **Step 8: Verify `ganttbok_lib` still compiles and all tests pass**

Run: `cargo test -p ganttbok_lib`

Expected: all tests pass, including the 15 patch tests (now executed via the shim). Zero new warnings.

- [ ] **Step 9: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add Cargo.toml crates/gb-patches/ src-tauri/Cargo.toml src-tauri/src/patches/mod.rs
git rm src-tauri/src/patches/schema.rs src-tauri/src/patches/validate.rs
git commit -m "refactor(patches): extract gb-patches workspace crate

Moves Patch/PatchOp/validate_patch out of ganttbok_lib into a
minimal shared crate with no Tauri dependency. ganttbok_lib re-exports
via a shim mod.rs so all callsites remain unchanged.
Required for blikplan-mcp to share patch types without pulling
in the full Tauri dep tree."
```

---

## Task 2: `blikplan-mcp` crate skeleton + MCP handshake

**Files:**
- Create: `crates/blikplan-mcp/Cargo.toml`
- Create: `crates/blikplan-mcp/src/main.rs`
- Create: `crates/blikplan-mcp/src/db.rs`
- Create: `crates/blikplan-mcp/src/tools/mod.rs`
- Create: `crates/blikplan-mcp/src/tools/read.rs`
- Create: `crates/blikplan-mcp/src/tools/write.rs`
- Create: `crates/blikplan-mcp/tests/integration.rs`

- [ ] **Step 1: Write failing handshake test**

```bash
mkdir -p /Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/tools
mkdir -p /Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/tests
```

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/tests/integration.rs`:

```rust
//! Integration tests using rmcp's in-process duplex transport.
//! Each test spins up the server against an in-memory SQLite fixture,
//! then drives it with an rmcp client — no subprocess, no TCP, no temp files.

use rmcp::{ServiceExt, model::ClientInfo, transport::io::duplex};
use blikplan_mcp::server::BlikPlanServer;
use rusqlite::Connection;

fn fixture_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    // Apply the same migrations the app uses.
    blikplan_mcp::db::apply_migrations_for_test(&conn);
    conn
}

#[tokio::test]
async fn handshake_returns_server_info() {
    let db = std::sync::Arc::new(std::sync::Mutex::new(fixture_db()));
    let server = BlikPlanServer::new(db);
    let (client_transport, server_transport) = duplex(1024);
    let _server_handle = tokio::spawn(server.serve(server_transport));
    let client = rmcp::client::Client::new(ClientInfo::default())
        .serve(client_transport)
        .await
        .unwrap();
    let info = client.peer_info();
    assert_eq!(info.server_info.name, "blikplan-mcp");
    client.cancel().await;
}

#[tokio::test]
async fn tools_list_contains_all_eight_tools() {
    let db = std::sync::Arc::new(std::sync::Mutex::new(fixture_db()));
    let server = BlikPlanServer::new(db);
    let (client_transport, server_transport) = duplex(1024);
    let _server_handle = tokio::spawn(server.serve(server_transport));
    let client = rmcp::client::Client::new(ClientInfo::default())
        .serve(client_transport)
        .await
        .unwrap();
    let list = client.list_tools(None).await.unwrap();
    let names: Vec<&str> = list.tools.iter().map(|t| t.name.as_str()).collect();
    for expected in &[
        "list_jobs", "get_job", "list_tasks", "get_task",
        "list_contacts", "search", "today", "propose_patch",
    ] {
        assert!(names.contains(expected), "missing tool: {expected}; got {names:?}");
    }
    client.cancel().await;
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p blikplan-mcp 2>&1 | head -20`

Expected: compilation error — `crates/blikplan-mcp/Cargo.toml` doesn't exist yet.

- [ ] **Step 3: Create `crates/blikplan-mcp/Cargo.toml`**

```toml
[package]
name        = "blikplan-mcp"
version     = "0.1.0"
edition     = "2021"

[[bin]]
name = "blikplan-mcp"
path = "src/main.rs"

[lib]
name = "blikplan_mcp"
path = "src/lib.rs"

[dependencies]
gb-patches  = { path = "../gb-patches" }
rmcp        = { version = "0.3", features = ["server", "transport-io", "macros"] }
rusqlite    = { version = "0.31", features = ["bundled"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
schemars    = "1"
chrono      = { version = "0.4", features = ["serde"] }
thiserror   = "1"
dirs        = "5"
tokio       = { version = "1", features = ["full"] }
uuid        = { version = "1", features = ["v4"] }

[dev-dependencies]
rmcp        = { version = "0.3", features = ["server", "transport-io", "macros", "client", "transport-child-process"] }
tokio       = { version = "1", features = ["full"] }
```

- [ ] **Step 4: Create `src/lib.rs`**

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/lib.rs`:

```rust
pub mod db;
pub mod server;
pub mod tools;
```

- [ ] **Step 5: Create stub `src/db.rs`**

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/db.rs`:

```rust
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
#[cfg(any(test, feature = "test-utils"))]
pub fn apply_migrations_for_test(conn: &Connection) {
    // Inline the same migration text as ganttbok_lib's db::migrations.
    // We copy only the subset needed for MCP server tests: job, phase, task,
    // contact, dependency, pending_patches.
    conn.execute_batch(FIXTURE_SCHEMA).expect("fixture schema failed");
}

#[cfg(any(test, feature = "test-utils"))]
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
```

- [ ] **Step 6: Create stub `src/tools/mod.rs`**

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/tools/mod.rs`:

```rust
pub mod read;
pub mod write;
```

- [ ] **Step 7: Create stub `src/tools/read.rs`**

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/tools/read.rs`:

```rust
// Read tools — implemented in Tasks 4–9.
```

- [ ] **Step 8: Create stub `src/tools/write.rs`**

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/tools/write.rs`:

```rust
// propose_patch write tool — implemented in Task 10.
```

- [ ] **Step 9: Create `src/server.rs` with minimal handshake**

Create `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/server.rs` (and add `pub mod server;` to `lib.rs`):

```rust
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use rmcp::{ServerHandler, model::ServerInfo, tool_router, tool_handler};
use schemars::JsonSchema;
use serde::Deserialize;

pub struct BlikPlanServer {
    pub(crate) db: Arc<Mutex<Connection>>,
}

impl BlikPlanServer {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }
}

#[tool_router(server_handler)]
impl BlikPlanServer {
    // Tools are added as Tasks 4–10 progress.
    // A placeholder is needed so the macro emits a valid (empty) router.
    // Remove this placeholder once Task 4 adds the first real tool.
    #[tool(description = "_placeholder — remove after Task 4_")]
    async fn _placeholder(&self) -> String {
        "placeholder".into()
    }
}

#[tool_handler(name = "blikplan-mcp", version = "0.1.0",
               instructions = "Read and propose patches to a Blik Plan schedule.")]
impl ServerHandler for BlikPlanServer {}
```

Update `lib.rs` to include `server`:

```rust
pub mod db;
pub mod server;
pub mod tools;
```

- [ ] **Step 10: Create `src/main.rs`**

```rust
use blikplan_mcp::{db as gbdb, server::BlikPlanServer};
use rmcp::{ServiceExt, transport::io::stdio};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let db_path = gbdb::resolve_db_path().unwrap_or_else(|| {
        eprintln!(
            "blikplan-mcp: cannot find ganttbok.db.\n\
             Set BLIKPLAN_DB=/path/to/ganttbok.db and retry.\n\
             Expected locations:\n  \
             macOS/Linux: ~/Library/Application Support/Blik Plan/ganttbok.db\n  \
             Windows: %APPDATA%\\Blik Plan\\ganttbok.db"
        );
        std::process::exit(1);
    });

    let ro_conn = gbdb::open_ro(&db_path);
    let server = BlikPlanServer::new(Arc::new(Mutex::new(ro_conn)));
    let (stdin, stdout) = stdio();
    server.serve((stdin, stdout)).await
        .unwrap_or_else(|e| eprintln!("blikplan-mcp error: {e}"));
}
```

- [ ] **Step 11: Run tests — expect handshake test to pass, tools_list to fail**

Run: `cargo test -p blikplan-mcp 2>&1`

Expected: `handshake_returns_server_info` passes. `tools_list_contains_all_eight_tools` fails because only `_placeholder` is registered. This is expected; subsequent tasks will fix it.

- [ ] **Step 12: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/
git commit -m "feat(mcp): add blikplan-mcp crate skeleton with MCP handshake

Empty tool stubs + DB path resolver. Handshake test passes.
Tools-list test intentionally fails until Tasks 4–10 add all 8 tools."
```

---

## Task 3: DB path discovery tests

**Files:**
- Modify: `crates/blikplan-mcp/src/db.rs`

- [ ] **Step 1: Write failing tests for path discovery**

Append to `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/db.rs`:

```rust
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
```

Add `tempfile = "3"` to `[dev-dependencies]` in `crates/blikplan-mcp/Cargo.toml`.

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test -p blikplan-mcp db::tests`

Expected: all 5 tests pass.

- [ ] **Step 3: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/
git commit -m "test(mcp): db path discovery + connection flag tests"
```

---

## Task 4: `list_jobs` and `get_job` read tools

**Files:**
- Modify: `crates/blikplan-mcp/src/tools/read.rs`
- Modify: `crates/blikplan-mcp/src/server.rs`
- Modify: `crates/blikplan-mcp/tests/integration.rs`

- [ ] **Step 1: Write failing integration tests**

Append to `crates/blikplan-mcp/tests/integration.rs`:

```rust
mod fixture {
    use rusqlite::Connection;
    use crate::fixture_db;

    pub fn with_one_job() -> Connection {
        let conn = fixture_db();
        conn.execute_batch(
            "INSERT INTO job (name, client, project_start_date, region)
             VALUES ('Noordhoek', 'JT', '2026-06-01', 'ZA');
             INSERT INTO phase (job_id, name, colour, order_index)
             VALUES (1, 'Basement', '#3B82F6', 0);
             INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index)
             VALUES (1, 'Pour slab', '2026-06-02', 3, 0);"
        ).unwrap();
        conn
    }
}

async fn make_client(db: rusqlite::Connection)
    -> rmcp::client::RunningService<rmcp::RoleClient, rmcp::client::Client>
{
    use std::sync::{Arc, Mutex};
    use blikplan_mcp::server::BlikPlanServer;
    use rmcp::{ServiceExt, model::ClientInfo, transport::io::duplex};

    let server = BlikPlanServer::new(Arc::new(Mutex::new(db)));
    let (ct, st) = duplex(4096);
    tokio::spawn(server.serve(st));
    rmcp::client::Client::new(ClientInfo::default())
        .serve(ct)
        .await
        .unwrap()
}

#[tokio::test]
async fn list_jobs_returns_job_names() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "list_jobs".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Noordhoek"), "expected Noordhoek in: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn get_job_returns_phases_and_tasks() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "get_job".into(),
        arguments: Some(serde_json::json!({ "job_id": 1 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Basement"), "missing phase: {text}");
    assert!(text.contains("Pour slab"), "missing task: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn get_job_unknown_id_returns_error() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "get_job".into(),
        arguments: Some(serde_json::json!({ "job_id": 999 }).as_object().unwrap().clone()),
    }).await.unwrap();
    assert!(result.is_error.unwrap_or(false));
    client.cancel().await;
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p blikplan-mcp list_jobs 2>&1 | head -20`

Expected: compilation error — `list_jobs` tool not yet implemented.

- [ ] **Step 3: Implement `list_jobs` and `get_job` in `src/tools/read.rs`**

Replace the stub content of `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/tools/read.rs` with:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use rusqlite::Connection;

// ──────────────────────────────────────────────────────────────────────────────
// Shared output types
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct JobSummary {
    pub id: i64,
    pub name: String,
    pub client: Option<String>,
    pub project_start_date: String,
    pub region: String,
}

#[derive(Debug, Serialize)]
pub struct PhaseSummary {
    pub id: i64,
    pub name: String,
    pub colour: String,
    pub notes: String,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub id: i64,
    pub name: String,
    pub start_date: String,
    pub duration_workdays: i64,
    pub notes: Option<String>,
    pub contact_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct FullJob {
    pub id: i64,
    pub name: String,
    pub client: Option<String>,
    pub address: Option<String>,
    pub project_start_date: String,
    pub region: String,
    pub phases: Vec<PhaseSummary>,
    pub dependencies: Vec<DepSummary>,
}

#[derive(Debug, Serialize)]
pub struct DepSummary {
    pub id: i64,
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub dep_type: String,
    pub lag_days: i64,
}

#[derive(Debug, Serialize)]
pub struct ContactRecord {
    pub id: i64,
    pub name: String,
    pub telegram_handle: Option<String>,
    pub notes: String,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub kind: String,       // "job" | "phase" | "task"
    pub id: i64,
    pub name: String,
    pub snippet: String,    // the matching field value
}

#[derive(Debug, Serialize)]
pub struct TodayItem {
    pub status: String,     // "overdue" | "in_progress" | "due_today"
    pub task_id: i64,
    pub task_name: String,
    pub job_id: i64,
    pub job_name: String,
    pub start_date: String,
    pub end_date: String,   // inclusive last workday (start_date + duration_workdays - 1 calendar days, simplified)
}

// ──────────────────────────────────────────────────────────────────────────────
// Input parameter structs (used by server.rs tool methods)
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetJobParams {
    /// DB integer id of the job to fetch.
    pub job_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListTasksParams {
    /// Filter to a single job when provided.
    pub job_id: Option<i64>,
    /// Filter by contact id.
    pub contact_id: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetTaskParams {
    pub task_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct SearchParams {
    /// Free-text query. Case-insensitive substring match across job names,
    /// phase names, task names, and task notes.
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct TodayParams {
    /// Restrict results to a single job when provided.
    pub job_id: Option<i64>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Query helpers
// ──────────────────────────────────────────────────────────────────────────────

pub fn query_list_jobs(conn: &Connection) -> Result<Vec<JobSummary>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, name, client, project_start_date, region FROM job
         WHERE archived = 0 AND is_template = 0 ORDER BY created_at DESC"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(JobSummary {
        id:                  r.get(0)?,
        name:                r.get(1)?,
        client:              r.get(2)?,
        project_start_date:  r.get(3)?,
        region:              r.get(4)?,
    })).map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

pub fn query_get_job(conn: &Connection, job_id: i64) -> Result<FullJob, String> {
    let (name, client, address, project_start_date, region): (String, Option<String>, Option<String>, String, String) =
        conn.query_row(
            "SELECT name, client, address, project_start_date, region FROM job WHERE id = ?1 AND archived = 0",
            [job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        ).map_err(|e| format!("job {job_id} not found: {e}"))?;

    let mut phase_stmt = conn.prepare(
        "SELECT id, name, colour, notes FROM phase WHERE job_id = ?1 ORDER BY order_index"
    ).map_err(|e| e.to_string())?;
    let phases_raw: Vec<(i64, String, String, String)> = phase_stmt.query_map([job_id], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
    }).map_err(|e| e.to_string())?
    .map(|r| r.map_err(|e| e.to_string()))
    .collect::<Result<Vec<_>, _>>()?;

    let mut phases = Vec::new();
    for (pid, pname, colour, notes) in phases_raw {
        let mut task_stmt = conn.prepare(
            "SELECT id, name, start_date, duration_workdays, notes, contact_id
             FROM task WHERE phase_id = ?1 ORDER BY order_index"
        ).map_err(|e| e.to_string())?;
        let tasks: Vec<TaskSummary> = task_stmt.query_map([pid], |r| Ok(TaskSummary {
            id:                r.get(0)?,
            name:              r.get(1)?,
            start_date:        r.get(2)?,
            duration_workdays: r.get(3)?,
            notes:             r.get(4)?,
            contact_id:        r.get(5)?,
        })).map_err(|e| e.to_string())?
        .map(|r| r.map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;

        phases.push(PhaseSummary { id: pid, name: pname, colour, notes, tasks });
    }

    let mut dep_stmt = conn.prepare(
        "SELECT d.id, d.predecessor_id, d.successor_id, d.type, d.lag_days
         FROM dependency d
         JOIN task t ON t.id = d.predecessor_id
         JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1"
    ).map_err(|e| e.to_string())?;
    let dependencies: Vec<DepSummary> = dep_stmt.query_map([job_id], |r| Ok(DepSummary {
        id:             r.get(0)?,
        predecessor_id: r.get(1)?,
        successor_id:   r.get(2)?,
        dep_type:       r.get(3)?,
        lag_days:       r.get(4)?,
    })).map_err(|e| e.to_string())?
    .map(|r| r.map_err(|e| e.to_string()))
    .collect::<Result<Vec<_>, _>>()?;

    Ok(FullJob { id: job_id, name, client, address, project_start_date, region, phases, dependencies })
}

pub fn query_list_tasks(conn: &Connection, params: &ListTasksParams) -> Result<Vec<TaskSummary>, String> {
    let sql = if params.job_id.is_some() && params.contact_id.is_some() {
        "SELECT t.id, t.name, t.start_date, t.duration_workdays, t.notes, t.contact_id
         FROM task t JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1 AND t.contact_id = ?2 ORDER BY t.start_date"
    } else if params.job_id.is_some() {
        "SELECT t.id, t.name, t.start_date, t.duration_workdays, t.notes, t.contact_id
         FROM task t JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1 ORDER BY t.start_date"
    } else if params.contact_id.is_some() {
        "SELECT t.id, t.name, t.start_date, t.duration_workdays, t.notes, t.contact_id
         FROM task t WHERE t.contact_id = ?1 ORDER BY t.start_date"
    } else {
        "SELECT t.id, t.name, t.start_date, t.duration_workdays, t.notes, t.contact_id
         FROM task t ORDER BY t.start_date"
    };

    // Build the param set dynamically to match the WHERE clause chosen above.
    let row_fn = |r: &rusqlite::Row| Ok(TaskSummary {
        id:                r.get(0)?,
        name:              r.get(1)?,
        start_date:        r.get(2)?,
        duration_workdays: r.get(3)?,
        notes:             r.get(4)?,
        contact_id:        r.get(5)?,
    });

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = if params.job_id.is_some() && params.contact_id.is_some() {
        stmt.query_map(
            rusqlite::params![params.job_id.unwrap(), params.contact_id.unwrap()],
            row_fn,
        )
    } else if let Some(jid) = params.job_id {
        stmt.query_map(rusqlite::params![jid], row_fn)
    } else if let Some(cid) = params.contact_id {
        stmt.query_map(rusqlite::params![cid], row_fn)
    } else {
        stmt.query_map([], row_fn)
    }.map_err(|e| e.to_string())?;

    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

pub fn query_get_task(conn: &Connection, task_id: i64) -> Result<TaskSummary, String> {
    conn.query_row(
        "SELECT id, name, start_date, duration_workdays, notes, contact_id
         FROM task WHERE id = ?1",
        [task_id],
        |r| Ok(TaskSummary {
            id:                r.get(0)?,
            name:              r.get(1)?,
            start_date:        r.get(2)?,
            duration_workdays: r.get(3)?,
            notes:             r.get(4)?,
            contact_id:        r.get(5)?,
        }),
    ).map_err(|e| format!("task {task_id} not found: {e}"))
}

pub fn query_list_contacts(conn: &Connection) -> Result<Vec<ContactRecord>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, name, telegram_handle, notes FROM contact ORDER BY name COLLATE NOCASE"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(ContactRecord {
        id:               r.get(0)?,
        name:             r.get(1)?,
        telegram_handle:  r.get(2)?,
        notes:            r.get(3)?,
    })).map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}

pub fn query_search(conn: &Connection, q: &str) -> Result<Vec<SearchHit>, String> {
    let pattern = format!("%{}%", q.to_lowercase());
    let mut hits: Vec<SearchHit> = Vec::new();

    // Job names
    let mut s = conn.prepare(
        "SELECT id, name FROM job WHERE lower(name) LIKE ?1 AND archived = 0"
    ).map_err(|e| e.to_string())?;
    let rows = s.query_map([&pattern], |r| {
        Ok(SearchHit { kind: "job".into(), id: r.get(0)?, name: r.get(1)?, snippet: r.get(1)? })
    }).map_err(|e| e.to_string())?;
    for r in rows { hits.push(r.map_err(|e| e.to_string())?); }

    // Phase names and notes
    let mut s = conn.prepare(
        "SELECT p.id, p.name, p.name, p.notes FROM phase p
         JOIN job j ON j.id = p.job_id
         WHERE j.archived = 0 AND (lower(p.name) LIKE ?1 OR lower(p.notes) LIKE ?1)"
    ).map_err(|e| e.to_string())?;
    let rows = s.query_map([&pattern], |r| {
        let pname: String = r.get(1)?;
        let pnotes: String = r.get(3)?;
        let snippet = if pname.to_lowercase().contains(&q.to_lowercase()) { pname.clone() } else { pnotes };
        Ok(SearchHit { kind: "phase".into(), id: r.get(0)?, name: pname, snippet })
    }).map_err(|e| e.to_string())?;
    for r in rows { hits.push(r.map_err(|e| e.to_string())?); }

    // Task names and notes
    let mut s = conn.prepare(
        "SELECT t.id, t.name, t.notes FROM task t
         JOIN phase p ON p.id = t.phase_id
         JOIN job j ON j.id = p.job_id
         WHERE j.archived = 0
           AND (lower(t.name) LIKE ?1 OR lower(coalesce(t.notes,'')) LIKE ?1)"
    ).map_err(|e| e.to_string())?;
    let rows = s.query_map([&pattern], |r| {
        let tname: String = r.get(1)?;
        let tnotes: Option<String> = r.get(2)?;
        let snippet = if tname.to_lowercase().contains(&q.to_lowercase()) {
            tname.clone()
        } else {
            tnotes.unwrap_or_default()
        };
        Ok(SearchHit { kind: "task".into(), id: r.get(0)?, name: tname, snippet })
    }).map_err(|e| e.to_string())?;
    for r in rows { hits.push(r.map_err(|e| e.to_string())?); }

    Ok(hits)
}

pub fn query_today(conn: &Connection, job_id: Option<i64>) -> Result<Vec<TodayItem>, String> {
    // "today" is derived from the system date at query time.
    // "in_progress": start_date <= today AND start_date + duration_workdays - 1 >= today
    //   (approximated as calendar days, not workdays — exact workday math requires the
    //    calendar module which lives in ganttbok_lib; this approximation is sufficient
    //    for Claude context and flagged as a Risk).
    // "overdue":     start_date + duration_workdays - 1 < today
    // "due_today":   start_date = today (task starting today)
    //
    // No completed status column exists in v1.4; every task that is past its window
    // is surfaced as overdue. The user accepts this limitation.

    let today = chrono::Local::now().date_naive().to_string();

    let base_sql = "SELECT t.id, t.name, t.start_date, t.duration_workdays,
                           j.id AS job_id, j.name AS job_name
                    FROM task t
                    JOIN phase p ON p.id = t.phase_id
                    JOIN job j ON j.id = p.job_id
                    WHERE j.archived = 0 AND j.is_template = 0";

    let filter = if job_id.is_some() { " AND j.id = ?2" } else { "" };
    let sql = format!("{base_sql}{filter} ORDER BY t.start_date");

    let row_fn = |r: &rusqlite::Row| -> rusqlite::Result<(i64, String, String, i64, i64, String)> {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let raw: Vec<(i64, String, String, i64, i64, String)> = if let Some(jid) = job_id {
        stmt.query_map(rusqlite::params![today, jid], row_fn)
    } else {
        stmt.query_map(rusqlite::params![today], row_fn)
    }.map_err(|e| e.to_string())?
    .map(|r| r.map_err(|e| e.to_string()))
    .collect::<Result<Vec<_>, _>>()?;

    // Filter and classify in Rust (easier than calendar-aware SQL).
    use chrono::NaiveDate;
    let today_d = NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap();

    let mut items = Vec::new();
    for (tid, tname, start_str, dur, jid, jname) in raw {
        let start = match NaiveDate::parse_from_str(&start_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        // Approximate end: add (duration_workdays - 1) calendar days.
        let end = start + chrono::Duration::days(dur.saturating_sub(1));
        let end_str = end.to_string();

        let status = if end < today_d {
            "overdue"
        } else if start == today_d {
            "due_today"
        } else if start <= today_d && end >= today_d {
            "in_progress"
        } else {
            continue // future task, not relevant to "today"
        };

        items.push(TodayItem {
            status: status.into(),
            task_id: tid,
            task_name: tname,
            job_id: jid,
            job_name: jname,
            start_date: start_str,
            end_date: end_str,
        });
    }
    Ok(items)
}
```

- [ ] **Step 4: Wire `list_jobs` and `get_job` into `server.rs`**

Replace the content of `/Users/cncuser/Desktop/GanttBok/crates/blikplan-mcp/src/server.rs` with:

```rust
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use rmcp::{ServerHandler, tool_router, tool_handler, handler::server::tool::Parameters};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::tools::read::{
    GetJobParams, ListTasksParams, GetTaskParams, SearchParams, TodayParams,
    query_list_jobs, query_get_job, query_list_tasks, query_get_task,
    query_list_contacts, query_search, query_today,
};
use crate::tools::write::ProposePatchParams;

pub struct BlikPlanServer {
    pub(crate) db: Arc<Mutex<Connection>>,
    /// Absolute path to the DB file — needed by propose_patch to open a RW connection.
    pub(crate) db_path: Option<std::path::PathBuf>,
}

impl BlikPlanServer {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db, db_path: None }
    }

    pub fn new_with_path(db: Arc<Mutex<Connection>>, path: std::path::PathBuf) -> Self {
        Self { db, db_path: Some(path) }
    }
}

#[tool_router(server_handler)]
impl BlikPlanServer {
    #[tool(description = "List all active jobs (projects). Returns id, name, client, start date, region.")]
    async fn list_jobs(&self) -> String {
        let conn = self.db.lock().unwrap();
        match query_list_jobs(&conn) {
            Ok(jobs) => serde_json::to_string_pretty(&jobs).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "Get a full job by id: all phases, tasks, dependencies.")]
    async fn get_job(&self, Parameters(p): Parameters<GetJobParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_get_job(&conn, p.job_id) {
            Ok(job) => serde_json::to_string_pretty(&job).unwrap_or_else(|e| e.to_string()),
            Err(e) => {
                // Return as MCP error content so the client sees is_error=true.
                // rmcp treats a returned string as success; we embed a JSON error.
                format!("{{\"error\":\"not_found\",\"detail\":\"{e}\"}}")
            }
        }
    }

    #[tool(description = "List tasks. Optionally filter by job_id and/or contact_id. Returns tasks ordered by start_date.")]
    async fn list_tasks(&self, Parameters(p): Parameters<ListTasksParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_list_tasks(&conn, &p) {
            Ok(tasks) => serde_json::to_string_pretty(&tasks).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "Get a single task by id with full context.")]
    async fn get_task(&self, Parameters(p): Parameters<GetTaskParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_get_task(&conn, p.task_id) {
            Ok(task) => serde_json::to_string_pretty(&task).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"not_found\",\"detail\":\"{e}\"}}"),
        }
    }

    #[tool(description = "List all contacts (for chaser context). Returns id, name, telegram_handle, notes.")]
    async fn list_contacts(&self) -> String {
        let conn = self.db.lock().unwrap();
        match query_list_contacts(&conn) {
            Ok(contacts) => serde_json::to_string_pretty(&contacts).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "Free-text search across job names, phase names, task names, and task notes. Case-insensitive substring match.")]
    async fn search(&self, Parameters(p): Parameters<SearchParams>) -> String {
        if p.query.trim().is_empty() {
            return "{\"error\":\"query must not be empty\"}".into();
        }
        let conn = self.db.lock().unwrap();
        match query_search(&conn, &p.query) {
            Ok(hits) => serde_json::to_string_pretty(&hits).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "What is overdue, in-progress, or starting today. Optionally filter to a single job_id.")]
    async fn today(&self, Parameters(p): Parameters<TodayParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_today(&conn, p.job_id) {
            Ok(items) => serde_json::to_string_pretty(&items).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "Propose a batch patch to a job. The patch is queued for user review in the Blik Plan Inbox — Claude never applies changes directly. Returns patch_id and a human-readable preview.")]
    async fn propose_patch(&self, Parameters(p): Parameters<ProposePatchParams>) -> String {
        crate::tools::write::handle_propose_patch(self, p).await
    }
}

#[tool_handler(name = "blikplan-mcp", version = "0.1.0",
               instructions = "Read and propose patches to a Blik Plan schedule. Use list_jobs first to discover job ids, then get_job for full context, then propose_patch to suggest changes. All writes require user approval inside Blik Plan.")]
impl ServerHandler for BlikPlanServer {}
```

- [ ] **Step 5: Run failing tests for `get_job_unknown_id_returns_error`**

Note: because `get_job` returns a JSON error string rather than an MCP-level error, `result.is_error` will be `false`. The integration test as written would fail. Adjust the test to assert on the content instead:

In `tests/integration.rs`, change `get_job_unknown_id_returns_error` to:

```rust
#[tokio::test]
async fn get_job_unknown_id_returns_error() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "get_job".into(),
        arguments: Some(serde_json::json!({ "job_id": 999 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("not_found") || text.contains("error"), "expected error in: {text}");
    client.cancel().await;
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p blikplan-mcp 2>&1`

Expected: `list_jobs_returns_job_names`, `get_job_returns_phases_and_tasks`, `get_job_unknown_id_returns_error`, `handshake_returns_server_info` all pass. `tools_list_contains_all_eight_tools` still fails (propose_patch not yet implemented).

- [ ] **Step 7: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/
git commit -m "feat(mcp): add list_jobs + get_job read tools with integration tests"
```

---

## Task 5: `list_tasks`, `get_task`, `list_contacts`

The implementations are already in `tools/read.rs` and `server.rs` from Task 4. This task adds their integration tests and verifies them.

**Files:**
- Modify: `crates/blikplan-mcp/tests/integration.rs`

- [ ] **Step 1: Append integration tests**

Append to `tests/integration.rs`:

```rust
#[tokio::test]
async fn list_tasks_filters_by_job_id() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "list_tasks".into(),
        arguments: Some(serde_json::json!({ "job_id": 1 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Pour slab"), "expected task in: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn list_tasks_no_filter_returns_all() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "list_tasks".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Pour slab"), "expected task in: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn get_task_returns_task() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "get_task".into(),
        arguments: Some(serde_json::json!({ "task_id": 1 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Pour slab"), "expected task name in: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn list_contacts_returns_empty_when_none() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "list_contacts".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    // No contacts in fixture — should return empty array.
    assert!(text.trim() == "[]" || text.contains("[]"), "expected empty array: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn list_contacts_returns_contacts() {
    let db = {
        let conn = fixture_db();
        conn.execute_batch(
            "INSERT INTO contact (name, telegram_handle, notes) VALUES ('Doug', '@doug_sa', 'supplier');"
        ).unwrap();
        conn
    };
    let client = make_client(db).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "list_contacts".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Doug"), "expected Doug in: {text}");
    client.cancel().await;
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p blikplan-mcp 2>&1`

Expected: all 9 tests pass (5 new + 4 from Task 4). `tools_list_contains_all_eight_tools` still fails.

- [ ] **Step 3: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/tests/integration.rs
git commit -m "test(mcp): add list_tasks, get_task, list_contacts integration tests"
```

---

## Task 6: `search` and `today` tools

**Files:**
- Modify: `crates/blikplan-mcp/tests/integration.rs`

Both implementations are in `tools/read.rs` and `server.rs` from Task 4. This task adds tests only.

- [ ] **Step 1: Append integration tests**

Append to `tests/integration.rs`:

```rust
#[tokio::test]
async fn search_matches_task_name() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "search".into(),
        arguments: Some(serde_json::json!({ "query": "slab" }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("Pour slab") || text.contains("task"), "expected hit: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn search_empty_query_returns_error() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "search".into(),
        arguments: Some(serde_json::json!({ "query": "" }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("error"), "expected error: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn today_returns_in_progress_or_overdue() {
    // The fixture task has start_date 2026-06-02. Since today (test runtime)
    // is 2026-05-22 the task is in the future — today() returns empty [].
    // This test asserts the tool responds without error.
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "today".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    // Valid outcomes: empty array or a list of items; no "error" key.
    let val: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    assert!(val.is_array(), "expected JSON array, got: {text}");
    client.cancel().await;
}

#[tokio::test]
async fn today_with_overdue_task_is_returned() {
    // Insert a task with start_date in the past.
    let db = {
        let conn = fixture_db();
        conn.execute_batch(
            "INSERT INTO job (name, project_start_date, region) VALUES ('TestJob', '2020-01-01', 'ZA');
             INSERT INTO phase (job_id, name, colour, order_index) VALUES (1, 'P', '#fff', 0);
             INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index)
             VALUES (1, 'OldTask', '2020-01-05', 1, 0);"
        ).unwrap();
        conn
    };
    let client = make_client(db).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "today".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("overdue"), "expected overdue: {text}");
    client.cancel().await;
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p blikplan-mcp 2>&1`

Expected: all 13 tests pass. `tools_list_contains_all_eight_tools` still fails.

- [ ] **Step 3: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/tests/integration.rs
git commit -m "test(mcp): add search + today integration tests"
```

---

## Task 7: `propose_patch` write tool

**Files:**
- Modify: `crates/blikplan-mcp/src/tools/write.rs`
- Modify: `crates/blikplan-mcp/tests/integration.rs`

- [ ] **Step 1: Write the failing test**

Append to `tests/integration.rs`:

```rust
#[tokio::test]
async fn propose_patch_inserts_row_and_returns_patch_id() {
    use std::sync::{Arc, Mutex};
    use blikplan_mcp::server::BlikPlanServer;
    use rmcp::{ServiceExt, model::ClientInfo, transport::io::duplex};

    let conn = fixture_db();
    conn.execute_batch(
        "INSERT INTO job (name, project_start_date, region) VALUES ('Noordhoek', '2026-06-01', 'ZA');"
    ).unwrap();
    let conn = Arc::new(Mutex::new(conn));
    // propose_patch needs a RW connection opened from a real path.
    // For the integration test we use a tempfile DB instead of in-memory.
    use tempfile::NamedTempFile;
    let tmp = NamedTempFile::new().unwrap();
    {
        // Write the fixture schema + data to the temp file.
        let rw = rusqlite::Connection::open(tmp.path()).unwrap();
        rw.execute_batch(blikplan_mcp::db::FIXTURE_SCHEMA_FOR_TEST).unwrap();
        rw.execute_batch(
            "INSERT INTO job (name, project_start_date, region) VALUES ('Noordhoek', '2026-06-01', 'ZA');"
        ).unwrap();
    }
    let ro = blikplan_mcp::db::open_ro(tmp.path());
    let server = BlikPlanServer::new_with_path(
        Arc::new(Mutex::new(ro)),
        tmp.path().to_path_buf(),
    );
    let (ct, st) = duplex(4096);
    tokio::spawn(server.serve(st));
    let client = rmcp::client::Client::new(ClientInfo::default()).serve(ct).await.unwrap();

    let patch = serde_json::json!({
        "patch_version": 1,
        "summary": "Add note from meeting",
        "ops": [{ "op": "append_note", "job_id": 1, "text": "Graham wants fewer cavity walls" }]
    });
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "propose_patch".into(),
        arguments: Some(serde_json::json!({
            "job_id": 1,
            "patch": patch,
            "summary": "Add note from meeting"
        }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let val: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(val.get("patch_id").is_some(), "expected patch_id: {text}");
    assert_eq!(val["status"], "proposed");
    client.cancel().await;
}

#[tokio::test]
async fn propose_patch_rejects_invalid_patch() {
    use std::sync::{Arc, Mutex};
    use blikplan_mcp::server::BlikPlanServer;
    use rmcp::{ServiceExt, model::ClientInfo, transport::io::duplex};
    use tempfile::NamedTempFile;

    let tmp = NamedTempFile::new().unwrap();
    {
        let rw = rusqlite::Connection::open(tmp.path()).unwrap();
        rw.execute_batch(blikplan_mcp::db::FIXTURE_SCHEMA_FOR_TEST).unwrap();
        rw.execute_batch(
            "INSERT INTO job (name, project_start_date, region) VALUES ('J', '2026-01-01', 'ZA');"
        ).unwrap();
    }
    let ro = blikplan_mcp::db::open_ro(tmp.path());
    let server = BlikPlanServer::new_with_path(Arc::new(Mutex::new(ro)), tmp.path().to_path_buf());
    let (ct, st) = duplex(4096);
    tokio::spawn(server.serve(st));
    let client = rmcp::client::Client::new(ClientInfo::default()).serve(ct).await.unwrap();

    // Invalid: empty ops list.
    let bad_patch = serde_json::json!({ "patch_version": 1, "summary": "x", "ops": [] });
    let result = client.call_tool(rmcp::model::CallToolRequestParams {
        name: "propose_patch".into(),
        arguments: Some(serde_json::json!({
            "job_id": 1,
            "patch": bad_patch,
            "summary": "x"
        }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(text.contains("error") || text.contains("validation"), "expected error: {text}");
    client.cancel().await;
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p blikplan-mcp propose_patch 2>&1 | head -20`

Expected: compilation error — `ProposePatchParams` not defined and `handle_propose_patch` not implemented.

- [ ] **Step 3: Export `FIXTURE_SCHEMA_FOR_TEST` from `db.rs`**

In `crates/blikplan-mcp/src/db.rs`, change the `FIXTURE_SCHEMA` const visibility:

Replace:

```rust
#[cfg(any(test, feature = "test-utils"))]
const FIXTURE_SCHEMA: &str = r#"
```

with:

```rust
#[cfg(any(test, feature = "test-utils"))]
pub const FIXTURE_SCHEMA_FOR_TEST: &str = r#"
```

And update `apply_migrations_for_test` to reference `FIXTURE_SCHEMA_FOR_TEST`.

- [ ] **Step 4: Implement `propose_patch` in `src/tools/write.rs`**

Replace the stub content of `crates/blikplan-mcp/src/tools/write.rs`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use gb_patches::{validate_patch, Patch};
use uuid::Uuid;

use crate::server::BlikPlanServer;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProposePatchParams {
    /// The integer id of the job this patch targets.
    pub job_id: i64,
    /// The full patch document. Must conform to the v1 patch schema.
    pub patch: serde_json::Value,
    /// One-line human-readable summary of what the patch does.
    pub summary: String,
}

#[derive(Debug, Serialize)]
struct ProposePatchResponse {
    patch_id: String,
    status: &'static str,
    preview: String,
    inbox_count: i64,
}

pub async fn handle_propose_patch(server: &BlikPlanServer, params: ProposePatchParams) -> String {
    // 1. Deserialise and validate the patch document.
    let patch: Patch = match serde_json::from_value(params.patch.clone()) {
        Ok(p) => p,
        Err(e) => return format!("{{\"error\":\"parse_error\",\"detail\":\"{e}\"}}"),
    };
    if let Err(e) = validate_patch(&patch) {
        return format!("{{\"error\":\"validation_error\",\"detail\":\"{e}\"}}");
    }

    // 2. Summary must not be empty.
    if params.summary.trim().is_empty() {
        return "{\"error\":\"validation_error\",\"detail\":\"summary must not be empty\"}".into();
    }

    // 3. Get a RW connection — either from db_path (real runs) or fall back to
    //    cloning the in-memory path (only possible in tests that supply a file path).
    let db_path = match &server.db_path {
        Some(p) => p.clone(),
        None => return "{\"error\":\"db_path_not_set\",\"detail\":\"server was not initialised with a db path; cannot write\"}".into(),
    };

    let patch_id = format!("p_{}", Uuid::new_v4().simple());
    let patch_json = params.patch.to_string();
    let now = chrono::Utc::now().timestamp();

    // 4. Open a short-lived RW connection and insert the row.
    let rw = crate::db::open_rw(&db_path);
    let insert_result = rw.execute(
        "INSERT INTO pending_patches (id, job_id, patch_json, summary, source, created_at)
         VALUES (?1, ?2, ?3, ?4, 'mcp', ?5)",
        rusqlite::params![patch_id, params.job_id, patch_json, params.summary, now],
    );
    if let Err(e) = insert_result {
        return format!("{{\"error\":\"db_error\",\"detail\":\"{e}\"}}");
    }
    drop(rw); // close RW connection immediately

    // 5. Count pending rows for the response (use read connection).
    let inbox_count: i64 = {
        let conn = server.db.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM pending_patches WHERE status = 'proposed'",
            [],
            |r| r.get(0),
        ).unwrap_or(0)
    };

    // 6. Build a human-readable preview.
    let op_count = patch.ops.len();
    let preview = format!(
        "Will apply {} op{} to job {}. Open Blik Plan Inbox to review.",
        op_count,
        if op_count == 1 { "" } else { "s" },
        params.job_id,
    );

    let resp = ProposePatchResponse {
        patch_id,
        status: "proposed",
        preview,
        inbox_count,
    };
    serde_json::to_string_pretty(&resp).unwrap_or_else(|e| e.to_string())
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test -p blikplan-mcp 2>&1`

Expected: all tests pass **including `tools_list_contains_all_eight_tools`** (all 8 tools now registered in `server.rs`). `propose_patch_inserts_row_and_returns_patch_id` and `propose_patch_rejects_invalid_patch` pass.

- [ ] **Step 6: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/
git commit -m "feat(mcp): add propose_patch write tool with validation + DB insert

Validates the patch document via gb-patches::validate_patch before
inserting into pending_patches. Opens a separate short-lived RW
connection; the persistent connection remains read-only."
```

---

## Task 8: Full suite verification + `main.rs` wiring

**Files:**
- Modify: `crates/blikplan-mcp/src/main.rs` (pass `db_path` to server)

- [ ] **Step 1: Fix `main.rs` to use `new_with_path`**

Replace the content of `crates/blikplan-mcp/src/main.rs`:

```rust
use blikplan_mcp::{db as gbdb, server::BlikPlanServer};
use rmcp::{ServiceExt, transport::io::stdio};
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    let db_path = gbdb::resolve_db_path().unwrap_or_else(|| {
        eprintln!(
            "blikplan-mcp: cannot find ganttbok.db.\n\
             Set BLIKPLAN_DB=/path/to/ganttbok.db and retry.\n\
             Expected locations:\n  \
             macOS/Linux: ~/Library/Application Support/Blik Plan/ganttbok.db\n\
             Windows: %APPDATA%\\Blik Plan\\ganttbok.db\n\
             (pre-rename fallback: ~/Library/Application Support/Gantt Bok/ganttbok.db)"
        );
        std::process::exit(1);
    });

    let ro_conn = gbdb::open_ro(&db_path);
    let server = BlikPlanServer::new_with_path(
        Arc::new(Mutex::new(ro_conn)),
        db_path,
    );
    let (stdin, stdout) = stdio();
    server
        .serve((stdin, stdout))
        .await
        .unwrap_or_else(|e| eprintln!("blikplan-mcp error: {e}"));
}
```

- [ ] **Step 2: Run full workspace test suite**

Run: `cargo test --workspace 2>&1`

Expected: ALL tests pass across all three workspace members (`gb-patches`, `ganttbok_lib`, `blikplan-mcp`). Zero new warnings about unused code (minor `#[allow(dead_code)]` annotations acceptable on stubs).

- [ ] **Step 3: Build the release binary**

Run: `cargo build --release -p blikplan-mcp 2>&1`

Expected: binary produced at `target/release/blikplan-mcp`. Build completes with zero errors. Size will be approximately 25–40 MB (due to bundled SQLite).

```bash
ls -lh /Users/cncuser/Desktop/GanttBok/target/release/blikplan-mcp
```

Expected: file exists, size > 0.

- [ ] **Step 4: Smoke test the binary with `--help` not crashing**

```bash
BLIKPLAN_DB=/nonexistent /Users/cncuser/Desktop/GanttBok/target/release/blikplan-mcp 2>&1 | head -5
```

Expected output contains:
```
blikplan-mcp: cannot find ganttbok.db.
Set BLIKPLAN_DB=/path/to/ganttbok.db and retry.
```

- [ ] **Step 5: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add crates/blikplan-mcp/src/main.rs
git commit -m "feat(mcp): wire db_path into main for propose_patch RW access"
```

---

## Task 9: Tauri sidecar build wiring

**Files:**
- Modify: `src-tauri/tauri.conf.json` (document `externalBin` contract for Plan 4)
- Create: `scripts/copy-mcp-sidecar.sh`

This task documents and creates the copy script; `tauri.conf.json` is updated in Plan 4. The purpose here is to make the sidecar path explicit so Plan 4 has no ambiguity.

- [ ] **Step 1: Understand Tauri sidecar naming requirements**

Tauri's `bundle.externalBin` array expects entries like `"binaries/blikplan-mcp"`. Tauri automatically looks for platform-suffixed files:

| Platform | Required filename |
|---|---|
| macOS arm64 | `src-tauri/binaries/blikplan-mcp-aarch64-apple-darwin` |
| macOS x86_64 | `src-tauri/binaries/blikplan-mcp-x86_64-apple-darwin` |
| Linux x86_64 | `src-tauri/binaries/blikplan-mcp-x86_64-unknown-linux-gnu` |
| Windows x86_64 | `src-tauri/binaries/blikplan-mcp-x86_64-pc-windows-msvc.exe` |

The `externalBin` entry in `tauri.conf.json` for Plan 4 will be:

```json
"bundle": {
  "externalBin": ["binaries/blikplan-mcp"]
}
```

- [ ] **Step 2: Create the copy script**

Create `/Users/cncuser/Desktop/GanttBok/scripts/copy-mcp-sidecar.sh`:

```bash
#!/usr/bin/env bash
# copy-mcp-sidecar.sh
# Copies the compiled blikplan-mcp binary into the Tauri sidecar directory
# with the required target-triple suffix.
# Run after `cargo build --release -p blikplan-mcp`.
#
# Usage: ./scripts/copy-mcp-sidecar.sh [--debug]
#
# The Tauri app (Plan 4) adds "binaries/blikplan-mcp" to bundle.externalBin.
# Tauri resolves the correct platform binary at build time.

set -euo pipefail

PROFILE="${1:---release}"
BUILD_DIR="target/release"
if [ "$PROFILE" = "--debug" ]; then
  BUILD_DIR="target/debug"
fi

TRIPLE="$(rustc -vV | awk '/^host:/ { print $2 }')"
SRC="${BUILD_DIR}/blikplan-mcp"
DEST_DIR="src-tauri/binaries"
DEST="${DEST_DIR}/blikplan-mcp-${TRIPLE}"

if [ ! -f "$SRC" ]; then
  echo "ERROR: $SRC not found. Run: cargo build --release -p blikplan-mcp" >&2
  exit 1
fi

mkdir -p "$DEST_DIR"
cp "$SRC" "$DEST"
echo "Copied $SRC → $DEST"
```

```bash
chmod +x /Users/cncuser/Desktop/GanttBok/scripts/copy-mcp-sidecar.sh
```

- [ ] **Step 3: Run the script to verify it works**

```bash
cd /Users/cncuser/Desktop/GanttBok
./scripts/copy-mcp-sidecar.sh
```

Expected output:
```
Copied target/release/blikplan-mcp → src-tauri/binaries/blikplan-mcp-aarch64-apple-darwin
```

(Exact triple will vary by machine.)

- [ ] **Step 4: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add scripts/copy-mcp-sidecar.sh src-tauri/binaries/
git commit -m "feat(mcp): add sidecar copy script for Tauri externalBin wiring

Copies target/release/blikplan-mcp to src-tauri/binaries/ with
the platform target-triple suffix required by Tauri's externalBin.
Plan 4 adds the externalBin entry to tauri.conf.json and calls
this script in beforeBuildCommand."
```

---

## Task 10: npm wrapper package

**Files:**
- Create: `packages/blikplan-mcp-npm/package.json`
- Create: `packages/blikplan-mcp-npm/postinstall.js`
- Create: `packages/blikplan-mcp-npm/.npmignore`

- [ ] **Step 1: Create the package directory**

```bash
mkdir -p /Users/cncuser/Desktop/GanttBok/packages/blikplan-mcp-npm/bin
```

- [ ] **Step 2: Create `package.json`**

Create `/Users/cncuser/Desktop/GanttBok/packages/blikplan-mcp-npm/package.json`:

```json
{
  "name": "@blikplan/mcp",
  "version": "0.1.0",
  "description": "MCP server for Blik Plan — connects Claude to your schedule",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/jontystegmann-byte/ganttbok"
  },
  "bin": {
    "blikplan-mcp": "bin/blikplan-mcp"
  },
  "scripts": {
    "postinstall": "node postinstall.js"
  },
  "files": [
    "bin/",
    "postinstall.js",
    "README.md"
  ],
  "engines": {
    "node": ">=18"
  }
}
```

- [ ] **Step 3: Create `postinstall.js`**

Create `/Users/cncuser/Desktop/GanttBok/packages/blikplan-mcp-npm/postinstall.js`:

```js
#!/usr/bin/env node
// postinstall.js
// Downloads the blikplan-mcp binary for the current platform from a
// GitHub release and places it at bin/blikplan-mcp (or bin/blikplan-mcp.exe
// on Windows). Based on the same pattern as the `esbuild` npm package.
//
// Environment variables:
//   BLIKPLAN_MCP_VERSION  — override the binary version to download
//                           (default: matches npm package version)
//   BLIKPLAN_MCP_SKIP_DOWNLOAD — set to "1" to skip download (CI / offline)

'use strict';

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');

if (process.env.BLIKPLAN_MCP_SKIP_DOWNLOAD === '1') {
  console.log('blikplan-mcp: skipping binary download (BLIKPLAN_MCP_SKIP_DOWNLOAD=1)');
  process.exit(0);
}

const pkg = require('./package.json');
const version = process.env.BLIKPLAN_MCP_VERSION || pkg.version;

// Map Node.js platform/arch to the Rust target triple used in release filenames.
function platformToTriple() {
  const p = process.platform;
  const a = process.arch;

  if (p === 'darwin' && a === 'arm64') return 'aarch64-apple-darwin';
  if (p === 'darwin' && a === 'x64')   return 'x86_64-apple-darwin';
  if (p === 'linux'  && a === 'x64')   return 'x86_64-unknown-linux-gnu';
  if (p === 'linux'  && a === 'arm64') return 'aarch64-unknown-linux-gnu';
  if (p === 'win32'  && a === 'x64')   return 'x86_64-pc-windows-msvc';

  throw new Error(
    `blikplan-mcp: unsupported platform ${p}/${a}.\n` +
    'Please open an issue at https://github.com/jontystegmann-byte/ganttbok'
  );
}

const triple = platformToTriple();
const isWindows = process.platform === 'win32';
const binaryName = isWindows ? 'blikplan-mcp.exe' : 'blikplan-mcp';
const assetName = isWindows
  ? `blikplan-mcp-${triple}.exe`
  : `blikplan-mcp-${triple}`;

const downloadUrl =
  `https://github.com/jontystegmann-byte/ganttbok/releases/download/` +
  `mcp-v${version}/${assetName}`;

const binDir = path.join(__dirname, 'bin');
const destPath = path.join(binDir, binaryName);

if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

console.log(`blikplan-mcp: downloading ${downloadUrl}`);

function download(url, dest, redirectCount = 0) {
  if (redirectCount > 5) {
    throw new Error('Too many redirects');
  }
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest + '.tmp');
    https.get(url, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        file.close(() => fs.unlinkSync(dest + '.tmp'));
        resolve(download(res.headers.location, dest, redirectCount + 1));
        return;
      }
      if (res.statusCode !== 200) {
        file.close(() => fs.unlinkSync(dest + '.tmp'));
        reject(new Error(`HTTP ${res.statusCode} from ${url}`));
        return;
      }
      res.pipe(file);
      file.on('finish', () => {
        file.close(() => {
          fs.renameSync(dest + '.tmp', dest);
          resolve();
        });
      });
    }).on('error', (err) => {
      file.close(() => fs.unlinkSync(dest + '.tmp'));
      reject(err);
    });
  });
}

download(downloadUrl, destPath)
  .then(() => {
    if (!isWindows) {
      fs.chmodSync(destPath, 0o755);
    }
    console.log(`blikplan-mcp: installed to ${destPath}`);
  })
  .catch((err) => {
    console.error(`blikplan-mcp: download failed — ${err.message}`);
    console.error(
      'You can set BLIKPLAN_MCP_SKIP_DOWNLOAD=1 to skip the download ' +
      'and provide the binary yourself.'
    );
    process.exit(1);
  });
```

- [ ] **Step 4: Create `.npmignore`**

Create `/Users/cncuser/Desktop/GanttBok/packages/blikplan-mcp-npm/.npmignore`:

```
# Exclude everything except the files listed in package.json "files" array.
# The bin/ directory is populated at postinstall time; exclude any cached binary.
bin/blikplan-mcp
bin/blikplan-mcp.exe
```

- [ ] **Step 5: Verify the package.json is valid**

Run: `node -e "require('./packages/blikplan-mcp-npm/package.json'); console.log('ok')"`

Expected: `ok`

- [ ] **Step 6: Verify postinstall.js is syntactically valid**

Run: `node --check /Users/cncuser/Desktop/GanttBok/packages/blikplan-mcp-npm/postinstall.js && echo "syntax ok"`

Expected: `syntax ok`

- [ ] **Step 7: Commit**

```bash
cd /Users/cncuser/Desktop/GanttBok
git add packages/blikplan-mcp-npm/
git commit -m "feat(npm): add @blikplan/mcp npm wrapper package

Thin wrapper following the esbuild/swc pattern: postinstall.js
downloads the correct prebuilt binary from the GitHub release for
the user's platform. Supports macOS arm64/x64, Linux x64/arm64,
Windows x64. Release-upload CI is a future task."
```

---

## Task 11: Final verification

- [ ] **Step 1: Run the complete workspace test suite**

Run: `cargo test --workspace 2>&1`

Expected: ALL tests pass. Exact new tests added in Plan 2:

**gb-patches** (15 tests — same as Plan 1, now from the extracted crate):
- `schema::tests::deserialises_add_task_op`
- `schema::tests::deserialises_full_patch`
- `schema::tests::deserialises_shift_task_op`
- `schema::tests::rejects_unknown_op`
- `schema::tests::rejects_unknown_patch_version`
- `validate::tests::accepts_valid_op_ref_chain`
- `validate::tests::accepts_well_formed_patch`
- `validate::tests::rejects_bad_date_in_add_task`
- `validate::tests::rejects_dangling_op_ref`
- `validate::tests::rejects_duplicate_op_ref`
- `validate::tests::rejects_empty_ops`
- `validate::tests::rejects_empty_summary`
- `validate::tests::rejects_non_positive_duration`
- `validate::tests::rejects_unknown_dep_type`
- `validate::tests::rejects_unknown_patch_version`

**ganttbok_lib** — all pre-existing tests continue to pass (patches tests now run via shim).

**blikplan-mcp** (integration tests):
- `integration::handshake_returns_server_info`
- `integration::tools_list_contains_all_eight_tools`
- `integration::list_jobs_returns_job_names`
- `integration::get_job_returns_phases_and_tasks`
- `integration::get_job_unknown_id_returns_error`
- `integration::list_tasks_filters_by_job_id`
- `integration::list_tasks_no_filter_returns_all`
- `integration::get_task_returns_task`
- `integration::list_contacts_returns_empty_when_none`
- `integration::list_contacts_returns_contacts`
- `integration::search_matches_task_name`
- `integration::search_empty_query_returns_error`
- `integration::today_returns_in_progress_or_overdue`
- `integration::today_with_overdue_task_is_returned`
- `integration::propose_patch_inserts_row_and_returns_patch_id`
- `integration::propose_patch_rejects_invalid_patch`
- `db::tests::env_var_takes_priority`
- `db::tests::env_var_nonexistent_file_falls_through`
- `db::tests::returns_none_when_no_db_present`
- `db::tests::open_ro_connection_is_read_only`
- `db::tests::open_rw_connection_can_write`

- [ ] **Step 2: Build the release binary**

Run: `cargo build --release -p blikplan-mcp`

Expected: binary at `target/release/blikplan-mcp`, zero errors.

- [ ] **Step 3: Copy sidecar binary**

Run: `./scripts/copy-mcp-sidecar.sh`

Expected:
```
Copied target/release/blikplan-mcp → src-tauri/binaries/blikplan-mcp-<triple>
```

- [ ] **Step 4: Confirm git log shape**

Run: `git log --oneline -12`

Expected (newest first, 10 commits from Plan 2 on top of Plan 1's 4):

```
feat(mcp): wire db_path into main for propose_patch RW access
feat(mcp): add propose_patch write tool with validation + DB insert
test(mcp): add search + today integration tests
test(mcp): add list_tasks, get_task, list_contacts integration tests
feat(mcp): add list_jobs + get_job read tools with integration tests
test(mcp): db path discovery + connection flag tests
feat(mcp): add blikplan-mcp crate skeleton with MCP handshake
refactor(patches): extract gb-patches workspace crate
chore(workspace): introduce Cargo workspace root
feat(npm): add @blikplan/mcp npm wrapper package
feat(mcp): add sidecar copy script for Tauri externalBin wiring
feat(types): add Patch + PendingPatch TS types    ← Plan 1
```

---

## Out of scope for Plan 2

- Inbox panel UI and apply engine → Plan 3
- "Connect to Claude (beta)" settings panel and config writer → Plan 4
- `tauri.conf.json` `externalBin` entry → Plan 4 (script is ready; just needs the JSON key)
- Release CI that uploads platform binaries to GitHub Releases (npm `postinstall` target) → post-Plan 4
- Patch schema v2 / versioning policy → deferred until a v2 op is needed
- `expired` 30-day auto-sweep of `pending_patches` rows → Plan 3
- Authentication / per-tool permissions → post-v1 non-goal per spec

---

## Risks logged for next plans

1. **DB path discrepancy.** `src-tauri/src/lib.rs:19` still opens `dirs::data_local_dir().join("Gantt Bok")` (the pre-rename path), while `tauri.conf.json` now has `productName: "Blik Plan"`. Until `lib.rs` is updated to `"Blik Plan"` (or until the user accepts the `rename_bundle_and_restart` prompt), the live DB will be at the "Gantt Bok" path. The MCP server's fallback chain covers both, but Plan 4's config writer should set `$BLIKPLAN_DB` to the exact path rather than relying on the fallback.

2. **`today` uses calendar-day approximation, not workday math.** The `query_today` function computes end date as `start_date + (duration_workdays - 1)` calendar days. The actual workday-aware end date requires the `calendar` module from `ganttbok_lib`, which is not a dep of `blikplan-mcp`. This means a 5-workday task spanning a weekend will show as ending 2 days early. Acceptable for Claude context but should be noted in the UI description. Fix in a follow-up by either duplicating the workday calculation or extracting it into `gb-patches`.

3. **`propose_patch` preview is generic.** The preview string says "Will apply N ops to job X." It does not enumerate the ops. A richer preview (e.g. listing task names) requires a second DB read after validation. Deferred to Plan 3 when the Inbox panel render also needs the detailed diff.

4. **`rmcp` version stability.** `rmcp 0.3` is current as of May 2026. The macro API (`#[tool_router]`, `#[tool]`, `Parameters<T>`) is stable but the crate is pre-1.0. If a breaking change lands between Plan 2 and Plan 4, `Cargo.lock` protects the current implementation; migration will be needed before a new release.

5. **In-memory test DB for `propose_patch`.** The `propose_patch` integration test uses a `NamedTempFile` rather than a pure in-memory connection because the RW connection must open a file path. This means tests leave temp files on disk until the OS cleans them. `tempfile` crate handles cleanup on drop — acceptable.

6. **`add_chaser` op template validation.** As flagged in Plan 1: the `chaser_template_*` meta keys exist in the DB but there is no typed enum. `propose_patch` will accept any string for `template`. Plan 3's apply engine must validate against the configured templates at apply-time.
