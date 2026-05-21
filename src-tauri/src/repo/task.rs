use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::db::models::{Task, NewTask};
use crate::{GbError, GbResult};

const SELECT_COLS: &str = "id, phase_id, name, start_date, duration_workdays, order_index, notes, contact_id, last_chaser_sent_at";

pub fn create(conn: &Connection, new: &NewTask) -> GbResult<Task> {
    conn.execute(
        "INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            new.phase_id, new.name, new.start_date.to_string(),
            new.duration_workdays, new.order_index, new.notes,
        ],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Task> {
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM task WHERE id = ?1"),
        [id],
        row_to_task,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("task {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_for_phase(conn: &Connection, phase_id: i64) -> GbResult<Vec<Task>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM task WHERE phase_id = ?1 ORDER BY order_index ASC"),
    )?;
    let rows = stmt.query_map([phase_id], row_to_task)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Task>> {
    let select_cols_t = SELECT_COLS.split(", ")
        .map(|c| format!("t.{c}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut stmt = conn.prepare(&format!(
        "SELECT {select_cols_t}
         FROM task t JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1
         ORDER BY p.order_index ASC, t.order_index ASC"
    ))?;
    let rows = stmt.query_map([job_id], row_to_task)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, task: &Task) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE task SET phase_id = ?1, name = ?2, start_date = ?3,
                         duration_workdays = ?4, order_index = ?5, notes = ?6,
                         contact_id = ?7, last_chaser_sent_at = ?8
         WHERE id = ?9",
        params![
            task.phase_id, task.name, task.start_date.to_string(),
            task.duration_workdays, task.order_index, task.notes,
            task.contact_id, task.last_chaser_sent_at,
            task.id,
        ],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("task {}", task.id))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM task WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("task {id}"))); }
    Ok(())
}

pub fn reorder(conn: &Connection, phase_id: i64, ordered_ids: &[i64]) -> GbResult<()> {
    let tx = conn.unchecked_transaction()?;
    for (idx, id) in ordered_ids.iter().enumerate() {
        let n = tx.execute(
            "UPDATE task SET order_index = ?1 WHERE id = ?2 AND phase_id = ?3",
            params![idx as i64, id, phase_id],
        )?;
        if n == 0 {
            return Err(GbError::Validation(format!("task {id} not in phase {phase_id}")));
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn mark_chaser_sent(conn: &Connection, id: i64, sent_at: &str) -> GbResult<()> {
    conn.execute(
        "UPDATE task SET last_chaser_sent_at = ?1 WHERE id = ?2",
        params![sent_at, id],
    )?;
    Ok(())
}

fn row_to_task(r: &rusqlite::Row) -> rusqlite::Result<Task> {
    let date_str: String = r.get(3)?;
    let start_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok(Task {
        id: r.get(0)?,
        phase_id: r.get(1)?,
        name: r.get(2)?,
        start_date,
        duration_workdays: r.get(4)?,
        order_index: r.get(5)?,
        notes: r.get(6)?,
        contact_id: r.get(7)?,
        last_chaser_sent_at: r.get(8)?,
    })
}
