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
pub struct TaskSummary {
    pub id: i64,
    pub name: String,
    pub start_date: String,
    pub duration_workdays: i64,
    pub notes: Option<String>,
    pub contact_id: Option<i64>,
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
pub struct DepSummary {
    pub id: i64,
    pub predecessor_id: i64,
    pub successor_id: i64,
    pub dep_type: String,
    pub lag_days: i64,
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

// ──────────────────────────────────────────────────────────────────────────────
// Contact output type
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ContactSummary {
    pub id: i64,
    pub name: String,
    pub telegram_handle: Option<String>,
    pub notes: String,
}

// ──────────────────────────────────────────────────────────────────────────────
// Input parameter structs
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetJobParams {
    /// DB integer id of the job to fetch.
    pub job_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListTasksParams {
    /// Optional job id to filter tasks by. If omitted, returns tasks from all jobs.
    #[serde(default)]
    pub job_id: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetTaskParams {
    /// DB integer id of the task to fetch.
    pub task_id: i64,
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

pub fn query_list_tasks(conn: &Connection, job_id: Option<i64>) -> Result<Vec<TaskSummary>, String> {
    let row_mapper = |r: &rusqlite::Row<'_>| Ok(TaskSummary {
        id: r.get(0)?,
        name: r.get(1)?,
        start_date: r.get(2)?,
        duration_workdays: r.get(3)?,
        notes: r.get(4)?,
        contact_id: r.get(5)?,
    });
    if let Some(jid) = job_id {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.start_date, t.duration_workdays, t.notes, t.contact_id
             FROM task t
             JOIN phase p ON p.id = t.phase_id
             WHERE p.job_id = ?1
             ORDER BY t.start_date, t.order_index"
        ).map_err(|e| e.to_string())?;
        let result: Result<Vec<_>, _> = stmt.query_map([jid], row_mapper)
            .map_err(|e| e.to_string())?
            .map(|r| r.map_err(|e| e.to_string()))
            .collect();
        result
    } else {
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.start_date, t.duration_workdays, t.notes, t.contact_id
             FROM task t
             ORDER BY t.start_date, t.order_index"
        ).map_err(|e| e.to_string())?;
        let result: Result<Vec<_>, _> = stmt.query_map([], row_mapper)
            .map_err(|e| e.to_string())?
            .map(|r| r.map_err(|e| e.to_string()))
            .collect();
        result
    }
}

pub fn query_get_task(conn: &Connection, task_id: i64) -> Result<TaskSummary, String> {
    conn.query_row(
        "SELECT id, name, start_date, duration_workdays, notes, contact_id
         FROM task WHERE id = ?1",
        [task_id],
        |r| Ok(TaskSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            start_date: r.get(2)?,
            duration_workdays: r.get(3)?,
            notes: r.get(4)?,
            contact_id: r.get(5)?,
        }),
    ).map_err(|e| format!("task {task_id} not found: {e}"))
}

pub fn query_list_contacts(conn: &Connection) -> Result<Vec<ContactSummary>, String> {
    let mut stmt = conn.prepare(
        "SELECT id, name, telegram_handle, notes FROM contact ORDER BY name"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(ContactSummary {
        id: r.get(0)?,
        name: r.get(1)?,
        telegram_handle: r.get(2)?,
        notes: r.get(3)?,
    })).map_err(|e| e.to_string())?;
    rows.map(|r| r.map_err(|e| e.to_string())).collect()
}
