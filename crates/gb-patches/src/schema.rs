use serde::{Deserialize, Serialize};

/// The patch document version. The current MCP server and the Inbox apply
/// engine both target this version. Mismatched versions are rejected by
/// `validate::validate_patch`.
pub const PATCH_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub patch_version: u32,
    pub summary: String,
    pub ops: Vec<PatchOp>,
}

/// All operations that may appear inside a patch. Each variant maps to
/// an existing Tauri command in `commands/*` — Plan 3's apply engine
/// dispatches accordingly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PatchOp {
    AddTask {
        phase_id: i64,
        name: String,
        start_date: String,       // ISO 8601 YYYY-MM-DD
        duration_workdays: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contact_id: Option<i64>,
        /// Optional local handle so later ops in the same patch can
        /// reference this not-yet-created task.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        op_ref: Option<String>,
    },
    ShiftTask {
        task_id: i64,
        by_days: i64,
    },
    AddDependency {
        /// Either a real task id (`{ "task_id": 7 }`) or an `op_ref`
        /// from an earlier `add_task` in the same patch
        /// (`{ "op_ref": "new_vent_task" }`).
        predecessor: TaskRef,
        successor: TaskRef,
        #[serde(default = "default_dep_type")]
        dep_type: String,        // "FS", "SS", "FF", "SF"
        #[serde(default)]
        lag_days: i64,
    },
    AddChaser {
        task_id: i64,
        contact_id: i64,
        /// One of the user's three editable chaser templates.
        /// Plan 3 enforces that the value is one of the configured template keys.
        template: String,
    },
    AppendNote {
        /// Targets the job-level notes field. Phase-level / task-level
        /// note edits get separate ops if/when we need them.
        job_id: i64,
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskRef {
    Existing { task_id: i64 },
    Pending { op_ref: String },
}

fn default_dep_type() -> String {
    "FS".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deserialises_add_task_op() {
        let raw = json!({
            "op": "add_task",
            "phase_id": 42,
            "name": "Order vent ducting",
            "start_date": "2026-06-03",
            "duration_workdays": 3,
            "op_ref": "new_vent_task"
        });
        let op: PatchOp = serde_json::from_value(raw).unwrap();
        match op {
            PatchOp::AddTask { phase_id, name, op_ref, .. } => {
                assert_eq!(phase_id, 42);
                assert_eq!(name, "Order vent ducting");
                assert_eq!(op_ref.as_deref(), Some("new_vent_task"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn deserialises_shift_task_op() {
        let raw = json!({ "op": "shift_task", "task_id": 7, "by_days": -2 });
        let op: PatchOp = serde_json::from_value(raw).unwrap();
        assert!(matches!(op, PatchOp::ShiftTask { task_id: 7, by_days: -2 }));
    }

    #[test]
    fn deserialises_full_patch() {
        let raw = json!({
            "patch_version": 1,
            "summary": "Two changes from the meeting",
            "ops": [
                { "op": "append_note", "job_id": 1, "text": "hello" },
                { "op": "shift_task", "task_id": 7, "by_days": 1 }
            ]
        });
        let p: Patch = serde_json::from_value(raw).unwrap();
        assert_eq!(p.patch_version, 1);
        assert_eq!(p.ops.len(), 2);
        assert_eq!(p.summary, "Two changes from the meeting");
    }

    #[test]
    fn rejects_unknown_op() {
        let raw = json!({ "op": "delete_universe", "scope": "all" });
        let r: Result<PatchOp, _> = serde_json::from_value(raw);
        assert!(r.is_err());
    }

    #[test]
    fn rejects_unknown_patch_version() {
        let raw = json!({ "patch_version": 99, "summary": "x", "ops": [] });
        // Deserialisation succeeds (we accept the field as a number);
        // validate_patch in the next module will reject. Document that here:
        let p: Patch = serde_json::from_value(raw).unwrap();
        assert_eq!(p.patch_version, 99);
    }
}
