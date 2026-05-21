use rusqlite::{Connection, params};
use crate::db::models::{Contact, NewContact};
use crate::{GbError, GbResult};

const SELECT_COLS: &str = "id, name, telegram_chat_id, telegram_handle, notes, created_at";

pub fn create(conn: &Connection, new: &NewContact) -> GbResult<Contact> {
    conn.execute(
        "INSERT INTO contact (name, telegram_chat_id, telegram_handle, notes)
         VALUES (?1, ?2, ?3, ?4)",
        params![new.name, new.telegram_chat_id, new.telegram_handle, new.notes],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Contact> {
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM contact WHERE id = ?1"),
        [id],
        row_to_contact,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("contact {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_all(conn: &Connection) -> GbResult<Vec<Contact>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM contact ORDER BY name COLLATE NOCASE ASC"),
    )?;
    let rows = stmt.query_map([], row_to_contact)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, c: &Contact) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE contact SET name = ?1, telegram_chat_id = ?2, telegram_handle = ?3, notes = ?4
         WHERE id = ?5",
        params![c.name, c.telegram_chat_id, c.telegram_handle, c.notes, c.id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("contact {}", c.id))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM contact WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("contact {id}"))); }
    Ok(())
}

fn row_to_contact(r: &rusqlite::Row) -> rusqlite::Result<Contact> {
    Ok(Contact {
        id: r.get(0)?,
        name: r.get(1)?,
        telegram_chat_id: r.get(2)?,
        telegram_handle: r.get(3)?,
        notes: r.get(4)?,
        created_at: r.get(5)?,
    })
}
