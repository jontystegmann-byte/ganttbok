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
    /// Advertise the real object schema (via `Patch`) so clients send an
    /// object; the field stays `Value` so the handler can also tolerate a
    /// JSON-encoded string from clients that stringify nested arguments.
    #[schemars(with = "Patch")]
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

/// Render a string as a JSON string literal (quoted + escaped), so error
/// detail messages containing quotes or backslashes stay valid JSON.
fn json_str(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
}

/// Accept the `patch` argument whether the client sent it as a JSON object
/// or as a JSON-encoded string. The schema advertises an object, but some MCP
/// clients (including the Claude tool-use encoder) serialise the nested
/// document as a string; tolerate both so the tool works regardless of client.
fn coerce_patch_value(raw: &serde_json::Value) -> Result<serde_json::Value, serde_json::Error> {
    match raw {
        serde_json::Value::String(s) => serde_json::from_str(s),
        other => Ok(other.clone()),
    }
}

pub async fn handle_propose_patch(server: &BlikPlanServer, params: ProposePatchParams) -> String {
    // 1. Normalise then deserialise and validate the patch document.
    let patch_value = match coerce_patch_value(&params.patch) {
        Ok(v) => v,
        Err(e) => return format!("{{\"error\":\"parse_error\",\"detail\":{}}}", json_str(&e.to_string())),
    };
    let patch: Patch = match serde_json::from_value(patch_value.clone()) {
        Ok(p) => p,
        Err(e) => return format!("{{\"error\":\"parse_error\",\"detail\":{}}}", json_str(&e.to_string())),
    };
    if let Err(e) = validate_patch(&patch) {
        return format!("{{\"error\":\"validation_error\",\"detail\":{}}}", json_str(&e.to_string()));
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
    let patch_json = patch_value.to_string();
    let now = chrono::Utc::now().timestamp();

    // 4. Open a short-lived RW connection and insert the row.
    let rw = crate::db::open_rw(&db_path);
    let insert_result = rw.execute(
        "INSERT INTO pending_patches (id, job_id, patch_json, summary, source, created_at)
         VALUES (?1, ?2, ?3, ?4, 'mcp', ?5)",
        rusqlite::params![patch_id, params.job_id, patch_json, params.summary, now],
    );
    if let Err(e) = insert_result {
        return format!("{{\"error\":\"db_error\",\"detail\":{}}}", json_str(&e.to_string()));
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
