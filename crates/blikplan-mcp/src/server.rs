use std::future::Future;
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use rmcp::{
    ServerHandler,
    handler::server::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

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
    // Tools are added as Tasks 4–10 progress.
    // A placeholder is needed so the macro emits a valid (empty) router.
    // Remove this placeholder once Task 4 adds the first real tool.
    #[tool(description = "_placeholder — remove after Task 4_")]
    async fn placeholder(&self) -> String {
        "placeholder".into()
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
                "Read and propose patches to a Blik Plan schedule.".into(),
            ),
            ..Default::default()
        }
    }
}
