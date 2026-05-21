use chrono::Local;
use serde::Deserialize;
use tauri::State;

use crate::chaser::{nudge::{self, NudgeResult}, telegram, templates::{self, TemplateContext}};
use crate::commands::Db;
use crate::db::models::{meta_get, Contact, NewContact};
use crate::repo::{contact as contact_repo, task as task_repo};
use crate::{GbError, GbResult};

#[derive(Debug, Deserialize)]
pub struct ContactPayload {
    pub id: Option<i64>,
    pub name: String,
    pub telegram_chat_id: Option<String>,
    pub telegram_handle: Option<String>,
    pub notes: String,
}

#[tauri::command]
pub fn list_contacts(db: State<Db>) -> GbResult<Vec<Contact>> {
    let conn = db.0.lock().unwrap();
    contact_repo::list_all(&conn)
}

#[tauri::command]
pub fn create_contact(db: State<Db>, args: ContactPayload) -> GbResult<Contact> {
    let conn = db.0.lock().unwrap();
    contact_repo::create(&conn, &NewContact {
        name: args.name,
        telegram_chat_id: args.telegram_chat_id,
        telegram_handle: args.telegram_handle,
        notes: args.notes,
    })
}

#[tauri::command]
pub fn update_contact(db: State<Db>, args: ContactPayload) -> GbResult<()> {
    let id = args.id.ok_or_else(|| GbError::Validation("contact id required".into()))?;
    let conn = db.0.lock().unwrap();
    let mut c = contact_repo::get(&conn, id)?;
    c.name = args.name;
    c.telegram_chat_id = args.telegram_chat_id;
    c.telegram_handle = args.telegram_handle;
    c.notes = args.notes;
    contact_repo::update(&conn, &c)
}

#[tauri::command]
pub fn delete_contact(db: State<Db>, id: i64) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    contact_repo::delete(&conn, id)
}

#[derive(Debug, Deserialize)]
pub struct AssignArgs {
    pub task_id: i64,
    pub contact_id: Option<i64>,
}

#[tauri::command]
pub fn assign_task_contact(db: State<Db>, args: AssignArgs) -> GbResult<()> {
    let conn = db.0.lock().unwrap();
    let mut task = task_repo::get(&conn, args.task_id)?;
    task.contact_id = args.contact_id;
    task_repo::update(&conn, &task)
}

#[derive(Debug, Deserialize)]
pub struct SendChaserArgs {
    pub task_id: i64,
    /// "manual" | "approaching" | "overdue" | "custom"
    pub template_key: String,
    /// Only used when template_key == "custom"
    pub custom_text: Option<String>,
}

/// Send a chaser to the contact assigned to a task. Reads the bot token + templates from meta.
#[tauri::command]
pub fn send_chaser(db: State<Db>, args: SendChaserArgs) -> Result<(), String> {
    let conn = db.0.lock().unwrap();

    let token = meta_get(&conn, "telegram_bot_token").map_err(|e| e.to_string())?
        .filter(|t| !t.is_empty())
        .ok_or_else(|| "Telegram bot token not set — open Settings → Chaser".to_string())?;

    let task = task_repo::get(&conn, args.task_id).map_err(|e| e.to_string())?;
    let contact_id = task.contact_id.ok_or_else(|| "task has no assigned contact".to_string())?;
    let contact = contact_repo::get(&conn, contact_id).map_err(|e| e.to_string())?;
    let chat_id = contact.telegram_chat_id.as_deref()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{} has no Telegram chat_id", contact.name))?;

    // Job name (for {job_name} placeholder)
    let job_name: String = conn.query_row(
        "SELECT j.name FROM job j JOIN phase p ON p.job_id = j.id WHERE p.id = ?1",
        [task.phase_id], |r| r.get(0),
    ).unwrap_or_else(|_| "(unknown)".into());

    // Pick template
    let template_str: String = match args.template_key.as_str() {
        "manual" => meta_get(&conn, "chaser_template_manual").ok().flatten()
            .unwrap_or_else(|| templates::DEFAULT_MANUAL.into()),
        "approaching" => meta_get(&conn, "chaser_template_approaching").ok().flatten()
            .unwrap_or_else(|| templates::DEFAULT_APPROACHING.into()),
        "overdue" => meta_get(&conn, "chaser_template_overdue").ok().flatten()
            .unwrap_or_else(|| templates::DEFAULT_OVERDUE.into()),
        "custom" => args.custom_text.unwrap_or_default(),
        other => return Err(format!("unknown template key: {other}")),
    };

    if template_str.is_empty() {
        return Err("empty message — write something or pick a template".into());
    }

    // days = today - end_date (signed)
    let days = 0i64; // for manual/custom we don't compute days — leaves {days} as 0
    let ctx = TemplateContext {
        task_name: &task.name,
        job_name: &job_name,
        contact_name: &contact.name,
        days,
    };
    let body = templates::render(&template_str, &ctx);

    telegram::send_message(&token, chat_id, &body).map_err(|e| e.to_string())?;

    // Mark sent
    let now = Local::now().naive_local().format("%Y-%m-%dT%H:%M:%S").to_string();
    task_repo::mark_chaser_sent(&conn, task.id, &now).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct TestTelegramArgs {
    pub token: String,
    pub chat_id: String,
}

#[tauri::command]
pub fn test_telegram(args: TestTelegramArgs) -> Result<(), String> {
    telegram::send_message(&args.token, &args.chat_id, "✅ Test message from Blik Plan — your bot is wired up correctly.")
        .map_err(|e| e.to_string())
}

/// Run an auto-nudge sweep — typically called on app launch + focus.
#[tauri::command]
pub fn run_chaser_check(db: State<Db>) -> Vec<NudgeResult> {
    let conn = db.0.lock().unwrap();
    let today = chrono::Local::now().naive_local().date();
    nudge::run_auto_nudges(&conn, today)
}
