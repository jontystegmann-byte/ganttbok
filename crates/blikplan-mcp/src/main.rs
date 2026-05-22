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
    match server.serve((stdin, stdout)).await {
        Ok(running) => {
            let _ = running.waiting().await;
        }
        Err(e) => eprintln!("blikplan-mcp error: {e}"),
    }
}
