use std::future::Future;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    handler::server::tool::Parameters,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

use crate::tools::read::{GetJobParams, query_list_jobs, query_get_job};

pub struct BlikPlanServer {
    pub(crate) db: Arc<Mutex<Connection>>,
    tool_router: ToolRouter<Self>,
}

impl BlikPlanServer {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self {
            db,
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
