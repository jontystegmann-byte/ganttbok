/// Auto-nudge engine. Evaluates all assigned tasks and decides which ones need a
/// chaser sent today, then dispatches via the Telegram helper.
///
/// Idempotency: a task won't be re-nudged within 24h of the last send (tracked by
/// task.last_chaser_sent_at). Threshold for "deadline approaching" comes from meta.
use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;

use crate::calendar::workday::add_workdays_excluding;
use crate::chaser::{telegram, templates::{self, TemplateContext}};
use crate::db::models::{meta_get, Task, TaskStatus};
use crate::repo::{contact as contact_repo, no_work_day as nwd_repo, task as task_repo};

#[derive(Debug, Serialize, Clone)]
pub struct NudgeResult {
    pub task_id: i64,
    pub task_name: String,
    pub contact_name: String,
    pub template_key: String,   // "approaching" | "overdue"
    pub days: i64,
    pub success: bool,
    pub error: Option<String>,
}

/// Run an auto-nudge sweep. Returns a result per task that was eligible, whether the send
/// succeeded or not. Tasks not eligible (no contact, no deadline match, within 24h throttle)
/// are silently skipped.
pub fn run_auto_nudges(conn: &Connection, today: NaiveDate) -> Vec<NudgeResult> {
    let mut out = Vec::new();

    let auto_enabled = meta_get(conn, "chaser_auto_enabled")
        .ok().flatten()
        .map(|s| s == "1")
        .unwrap_or(true);
    if !auto_enabled { return out; }

    let token = match meta_get(conn, "telegram_bot_token").ok().flatten() {
        Some(t) if !t.is_empty() => t,
        _ => return out,
    };

    let threshold: i64 = meta_get(conn, "chaser_threshold_days").ok().flatten()
        .and_then(|s| s.parse().ok()).unwrap_or(3);

    let template_approaching = meta_get(conn, "chaser_template_approaching")
        .ok().flatten()
        .unwrap_or_else(|| templates::DEFAULT_APPROACHING.into());
    let template_overdue = meta_get(conn, "chaser_template_overdue")
        .ok().flatten()
        .unwrap_or_else(|| templates::DEFAULT_OVERDUE.into());

    // Pull all tasks across all jobs in one query
    let mut stmt = match conn.prepare(
        "SELECT t.id, t.phase_id, t.name, t.start_date, t.duration_workdays,
                t.order_index, t.notes, t.contact_id, t.last_chaser_sent_at,
                p.job_id
         FROM task t JOIN phase p ON p.id = t.phase_id
         WHERE t.contact_id IS NOT NULL",
    ) {
        Ok(s) => s,
        Err(_) => return out,
    };

    let rows: Vec<(Task, i64)> = stmt.query_map([], |r| {
        let date_str: String = r.get(3)?;
        let start_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?;
        let task = Task {
            id: r.get(0)?, phase_id: r.get(1)?, name: r.get(2)?, start_date,
            duration_workdays: r.get(4)?, order_index: r.get(5)?, notes: r.get(6)?,
            contact_id: r.get(7)?, last_chaser_sent_at: r.get(8)?,
            status: TaskStatus::default(), completion_date: None,
        };
        let job_id: i64 = r.get(9)?;
        Ok((task, job_id))
    }).map(|it| it.filter_map(|r| r.ok()).collect()).unwrap_or_default();

    let now_iso = chrono::Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();

    for (task, job_id) in rows {
        // 24h throttle
        if let Some(last) = &task.last_chaser_sent_at {
            if let Ok(last_dt) = chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%dT%H:%M:%S") {
                let now_dt = chrono::Local::now().naive_local();
                if (now_dt - last_dt).num_hours() < 24 { continue; }
            }
        }

        // Compute end date — calendar end of the task respecting no-work-days for the job.
        let nwds_iso: Vec<NaiveDate> = nwd_repo::list_for_job(conn, job_id)
            .unwrap_or_default()
            .into_iter()
            .map(|n| n.date)
            .collect();
        let nwds_set: std::collections::HashSet<NaiveDate> = nwds_iso.into_iter().collect();

        let include_weekends = meta_get(conn, "include_weekends")
            .ok().flatten()
            .map(|s| s == "1")
            .unwrap_or(false);
        let end = add_workdays_excluding(task.start_date, task.duration_workdays - 1, &nwds_set, include_weekends);
        let days_to_deadline = (end - today).num_days();

        let (template, template_key) = if days_to_deadline >= 0 && days_to_deadline <= threshold {
            (&template_approaching, "approaching")
        } else if days_to_deadline < 0 {
            (&template_overdue, "overdue")
        } else {
            continue;
        };

        // Resolve contact + job names
        let contact_id = task.contact_id.unwrap();
        let contact = match contact_repo::get(conn, contact_id) { Ok(c) => c, Err(_) => continue };
        let chat_id = match &contact.telegram_chat_id { Some(c) if !c.is_empty() => c, _ => continue };

        let job_name: String = conn.query_row(
            "SELECT name FROM job WHERE id = ?1", [job_id], |r| r.get(0),
        ).unwrap_or_else(|_| "(unknown job)".into());

        let ctx = TemplateContext {
            task_name: &task.name,
            job_name: &job_name,
            contact_name: &contact.name,
            days: days_to_deadline,
        };
        let body = templates::render(template, &ctx);

        let (success, error) = match telegram::send_message(&token, chat_id, &body) {
            Ok(()) => (true, None),
            Err(e) => (false, Some(e.to_string())),
        };
        if success {
            let _ = task_repo::mark_chaser_sent(conn, task.id, &now_iso);
        }

        out.push(NudgeResult {
            task_id: task.id,
            task_name: task.name,
            contact_name: contact.name,
            template_key: template_key.into(),
            days: days_to_deadline,
            success,
            error,
        });
    }

    out
}
