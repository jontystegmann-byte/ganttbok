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
