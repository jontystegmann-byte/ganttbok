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
