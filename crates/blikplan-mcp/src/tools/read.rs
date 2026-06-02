use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use chrono::NaiveDate;

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

// ──────────────────────────────────────────────────────────────────────────────
// Search output type and params
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub kind: String,       // "job" | "phase" | "task"
    pub id: i64,
    pub name: String,
    pub snippet: String,    // the matching field value
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct SearchParams {
    /// Free-text query. Case-insensitive substring match across job names,
    /// phase names, task names, and task notes.
    pub query: String,
}

pub fn query_search(conn: &Connection, q: &str) -> Result<Vec<SearchHit>, String> {
    let pattern = format!("%{}%", q.to_lowercase());
    let mut hits: Vec<SearchHit> = Vec::new();

    // Job names
    let mut s = conn.prepare(
        "SELECT id, name FROM job WHERE lower(name) LIKE ?1 AND archived = 0"
    ).map_err(|e| e.to_string())?;
    let rows = s.query_map([&pattern], |r| {
        Ok(SearchHit { kind: "job".into(), id: r.get(0)?, name: r.get(1)?, snippet: r.get(1)? })
    }).map_err(|e| e.to_string())?;
    for r in rows { hits.push(r.map_err(|e| e.to_string())?); }

    // Phase names and notes
    let mut s = conn.prepare(
        "SELECT p.id, p.name, p.notes FROM phase p
         JOIN job j ON j.id = p.job_id
         WHERE j.archived = 0 AND (lower(p.name) LIKE ?1 OR lower(coalesce(p.notes,'')) LIKE ?1)"
    ).map_err(|e| e.to_string())?;
    let rows = s.query_map([&pattern], |r| {
        let pname: String = r.get(1)?;
        let pnotes: String = r.get(2)?;
        let snippet = if pname.to_lowercase().contains(&q.to_lowercase()) { pname.clone() } else { pnotes };
        Ok(SearchHit { kind: "phase".into(), id: r.get(0)?, name: pname, snippet })
    }).map_err(|e| e.to_string())?;
    for r in rows { hits.push(r.map_err(|e| e.to_string())?); }

    // Task names and notes
    let mut s = conn.prepare(
        "SELECT t.id, t.name, t.notes FROM task t
         JOIN phase p ON p.id = t.phase_id
         JOIN job j ON j.id = p.job_id
         WHERE j.archived = 0
           AND (lower(t.name) LIKE ?1 OR lower(coalesce(t.notes,'')) LIKE ?1)"
    ).map_err(|e| e.to_string())?;
    let rows = s.query_map([&pattern], |r| {
        let tname: String = r.get(1)?;
        let tnotes: Option<String> = r.get(2)?;
        let snippet = if tname.to_lowercase().contains(&q.to_lowercase()) {
            tname.clone()
        } else {
            tnotes.unwrap_or_default()
        };
        Ok(SearchHit { kind: "task".into(), id: r.get(0)?, name: tname, snippet })
    }).map_err(|e| e.to_string())?;
    for r in rows { hits.push(r.map_err(|e| e.to_string())?); }

    Ok(hits)
}

// ──────────────────────────────────────────────────────────────────────────────
// Today output type and params
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TodayItem {
    pub status: String,     // "overdue" | "in_progress" | "due_today"
    pub task_id: i64,
    pub task_name: String,
    pub job_id: i64,
    pub job_name: String,
    pub start_date: String,
    pub end_date: String,   // inclusive last workday (start_date + duration_workdays - 1 calendar days, simplified)
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct TodayParams {
    /// Restrict results to a single job when provided.
    pub job_id: Option<i64>,
}

pub fn query_today(conn: &Connection, job_id: Option<i64>) -> Result<Vec<TodayItem>, String> {
    // Filtering is done in Rust; SQL just fetches candidates from non-archived jobs.
    let today = chrono::Local::now().date_naive().to_string();

    // Done tasks are never "overdue" — exclude them at the SQL layer so the
    // today/overdue view matches the in-app inbox (list_overdue_reviews_inner).
    let base_sql = "SELECT t.id, t.name, t.start_date, t.duration_workdays,
                           j.id AS job_id, j.name AS job_name
                    FROM task t
                    JOIN phase p ON p.id = t.phase_id
                    JOIN job j ON j.id = p.job_id
                    WHERE j.archived = 0 AND j.is_template = 0
                      AND t.status != 'done'";

    let filter = if job_id.is_some() { " AND j.id = ?1" } else { "" };
    let sql = format!("{base_sql}{filter} ORDER BY t.start_date");

    let row_fn = |r: &rusqlite::Row| -> rusqlite::Result<(i64, String, String, i64, i64, String)> {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let raw: Vec<(i64, String, String, i64, i64, String)> = if let Some(jid) = job_id {
        stmt.query_map([jid], row_fn)
    } else {
        stmt.query_map([], row_fn)
    }.map_err(|e| e.to_string())?
    .map(|r| r.map_err(|e| e.to_string()))
    .collect::<Result<Vec<_>, _>>()?;

    let today_d = NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap();

    let mut items = Vec::new();
    for (tid, tname, start_str, dur, jid, jname) in raw {
        let start = match NaiveDate::parse_from_str(&start_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        // Approximate end: add (duration_workdays - 1) calendar days.
        let end = start + chrono::Duration::days(dur.saturating_sub(1));
        let end_str = end.to_string();

        let status = if end < today_d {
            "overdue"
        } else if start == today_d {
            "due_today"
        } else if start <= today_d && end >= today_d {
            "in_progress"
        } else {
            continue // future task, not relevant to "today"
        };

        items.push(TodayItem {
            status: status.into(),
            task_id: tid,
            task_name: tname,
            job_id: jid,
            job_name: jname,
            start_date: start_str,
            end_date: end_str,
        });
    }
    Ok(items)
}
