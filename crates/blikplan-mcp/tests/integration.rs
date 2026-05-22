//! Integration tests using rmcp's in-process duplex transport.
//! Each test spins up the server against an in-memory SQLite fixture,
//! then drives it with an rmcp client — no subprocess, no TCP, no temp files.

use rmcp::{ServiceExt, model::ClientInfo};
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
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let _server_handle = tokio::spawn(server.serve(server_transport));
    let client = ClientInfo::default()
        .serve(client_transport)
        .await
        .unwrap();
    let info = client.peer_info().unwrap();
    assert_eq!(info.server_info.name, "blikplan-mcp");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn tools_list_contains_all_eight_tools() {
    let db = std::sync::Arc::new(std::sync::Mutex::new(fixture_db()));
    let server = BlikPlanServer::new(db);
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    let _server_handle = tokio::spawn(server.serve(server_transport));
    let client = ClientInfo::default()
        .serve(client_transport)
        .await
        .unwrap();
    let list = client.list_tools(Default::default()).await.unwrap();
    let names: Vec<&str> = list.tools.iter().map(|t| t.name.as_ref()).collect();
    for expected in &[
        "list_jobs", "get_job", "list_tasks", "get_task",
        "list_contacts", "search", "today", "propose_patch",
    ] {
        assert!(names.contains(expected), "missing tool: {expected}; got {names:?}");
    }
    client.cancel().await.unwrap();
}

// ──────────────────────────────────────────────────────────────────────────────
// Fixtures and helpers for Task 4 tests
// ──────────────────────────────────────────────────────────────────────────────

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
    -> rmcp::service::RunningService<rmcp::RoleClient, rmcp::model::ClientInfo>
{
    use std::sync::{Arc, Mutex};
    use blikplan_mcp::server::BlikPlanServer;

    let server = BlikPlanServer::new(Arc::new(Mutex::new(db)));
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_transport).await {
            let _ = running.waiting().await;
        }
    });
    ClientInfo::default()
        .serve(client_transport)
        .await
        .unwrap()
}

#[tokio::test]
async fn list_jobs_returns_job_names() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "list_jobs".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Noordhoek"), "expected Noordhoek in: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn get_job_returns_phases_and_tasks() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "get_job".into(),
        arguments: Some(serde_json::json!({ "job_id": 1 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Basement"), "missing phase: {text}");
    assert!(text.contains("Pour slab"), "missing task: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn get_job_unknown_id_returns_error() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "get_job".into(),
        arguments: Some(serde_json::json!({ "job_id": 999 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("not_found") || text.contains("error"), "expected error in: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn list_tasks_filters_by_job_id() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "list_tasks".into(),
        arguments: Some(serde_json::json!({ "job_id": 1 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Pour slab"), "expected task in: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn list_tasks_no_filter_returns_all() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "list_tasks".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Pour slab"), "expected task in: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn get_task_returns_task() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "get_task".into(),
        arguments: Some(serde_json::json!({ "task_id": 1 }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Pour slab"), "expected task name in: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn list_contacts_returns_empty_when_none() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "list_contacts".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    // No contacts in fixture — should return empty array.
    assert!(text.trim() == "[]" || text.contains("[]"), "expected empty array: {text}");
    client.cancel().await.unwrap();
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
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "list_contacts".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Doug"), "expected Doug in: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn search_matches_task_name() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "search".into(),
        arguments: Some(serde_json::json!({ "query": "slab" }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("Pour slab") || text.contains("task"), "expected hit: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn search_empty_query_returns_error() {
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "search".into(),
        arguments: Some(serde_json::json!({ "query": "" }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("error"), "expected error: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn today_returns_in_progress_or_overdue() {
    // The fixture task has start_date 2026-06-02. Since today (test runtime)
    // is 2026-05-22 the task is in the future — today() returns empty [].
    // This test asserts the tool responds without error.
    let client = make_client(fixture::with_one_job()).await;
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "today".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    // Valid outcomes: empty array or a list of items; no "error" key.
    let val: serde_json::Value = serde_json::from_str(text).unwrap_or(serde_json::Value::Null);
    assert!(val.is_array(), "expected JSON array, got: {text}");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn propose_patch_inserts_row_and_returns_patch_id() {
    use std::sync::{Arc, Mutex};
    use blikplan_mcp::server::BlikPlanServer;
    use tempfile::NamedTempFile;

    // propose_patch needs a RW connection opened from a real path — use a tempfile.
    let tmp = NamedTempFile::new().unwrap();
    {
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
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_transport).await {
            let _ = running.waiting().await;
        }
    });
    let client = ClientInfo::default().serve(client_transport).await.unwrap();

    let patch = serde_json::json!({
        "patch_version": 1,
        "summary": "Add note from meeting",
        "ops": [{ "op": "append_note", "job_id": 1, "text": "Graham wants fewer cavity walls" }]
    });
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "propose_patch".into(),
        arguments: Some(serde_json::json!({
            "job_id": 1,
            "patch": patch,
            "summary": "Add note from meeting"
        }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    let val: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(val.get("patch_id").is_some(), "expected patch_id: {text}");
    assert_eq!(val["status"], "proposed");
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn propose_patch_rejects_invalid_patch() {
    use std::sync::{Arc, Mutex};
    use blikplan_mcp::server::BlikPlanServer;
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
    let (server_transport, client_transport) = tokio::io::duplex(4096);
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_transport).await {
            let _ = running.waiting().await;
        }
    });
    let client = ClientInfo::default().serve(client_transport).await.unwrap();

    // Invalid: empty ops list.
    let bad_patch = serde_json::json!({ "patch_version": 1, "summary": "x", "ops": [] });
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "propose_patch".into(),
        arguments: Some(serde_json::json!({
            "job_id": 1,
            "patch": bad_patch,
            "summary": "x"
        }).as_object().unwrap().clone()),
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("error") || text.contains("validation"), "expected error: {text}");
    client.cancel().await.unwrap();
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
    let result = client.call_tool(rmcp::model::CallToolRequestParam {
        name: "today".into(),
        arguments: None,
    }).await.unwrap();
    let text = result.content.first()
        .and_then(|c| c.raw.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(text.contains("overdue"), "expected overdue: {text}");
    client.cancel().await.unwrap();
}
