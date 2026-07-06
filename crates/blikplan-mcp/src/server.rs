use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    handler::server::tool::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::tools::read::{
    GetJobParams, ListTasksParams, GetTaskParams, SearchParams, TodayParams, ListBoqParams,
    query_list_jobs, query_get_job,
    query_list_tasks, query_get_task, query_list_contacts,
    query_search, query_today, query_list_boq,
};
use crate::tools::write::{ProposePatchParams, handle_propose_patch};

pub struct BlikPlanServer {
    pub(crate) db: Arc<Mutex<Connection>>,
    /// Path used to open short-lived RW connections for write tools.
    /// `None` in unit tests that use an in-memory connection only.
    pub(crate) db_path: Option<PathBuf>,
    tool_router: ToolRouter<Self>,
}

impl BlikPlanServer {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self {
            db,
            db_path: None,
            tool_router: Self::tool_router(),
        }
    }

    /// Constructor used by integration tests and the real binary, which need
    /// a writable DB path for `propose_patch`.
    pub fn new_with_path(db: Arc<Mutex<Connection>>, db_path: PathBuf) -> Self {
        Self {
            db,
            db_path: Some(db_path),
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
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
            Err(e) => format!("{{\"error\":\"not_found\",\"detail\":\"{e}\"}}"),
        }
    }

    #[tool(description = "List tasks, optionally filtered by job_id. Returns id, name, start_date, duration_workdays, notes, contact_id.")]
    async fn list_tasks(&self, Parameters(p): Parameters<ListTasksParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_list_tasks(&conn, p.job_id) {
            Ok(tasks) => serde_json::to_string_pretty(&tasks).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "Get a single task by id.")]
    async fn get_task(&self, Parameters(p): Parameters<GetTaskParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_get_task(&conn, p.task_id) {
            Ok(task) => serde_json::to_string_pretty(&task).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"not_found\",\"detail\":\"{e}\"}}"),
        }
    }

    #[tool(description = "List all contacts. Returns id, name, telegram_handle, notes.")]
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

    #[tool(description = "List Bill of Quantities line items for a job. Returns id, item, qty, rate, cost, trade, supplier, location, procurement (not_ordered|quoted|ordered|delivered).")]
    async fn list_boq(&self, Parameters(p): Parameters<ListBoqParams>) -> String {
        let conn = self.db.lock().unwrap();
        match query_list_boq(&conn, p.job_id) {
            Ok(items) => serde_json::to_string_pretty(&items).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("{{\"error\":\"{e}\"}}"),
        }
    }

    #[tool(description = "Propose a patch to a Blik Plan schedule. The patch is validated and inserted into the pending_patches inbox for the user to review and accept or reject. Returns patch_id, status, preview, and inbox_count.")]
    async fn propose_patch(&self, Parameters(p): Parameters<ProposePatchParams>) -> String {
        handle_propose_patch(self, p).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for BlikPlanServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "blikplan-mcp".into(),
                version: "0.1.0".into(),
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Read and propose patches to a Blik Plan schedule. Use list_jobs first to discover job ids, then get_job for full context.".into(),
            ),
            ..Default::default()
        }
    }
}
