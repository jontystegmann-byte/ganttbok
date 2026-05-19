use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::db::models::{Task, NewTask};
use crate::{GbError, GbResult};

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
        "SELECT id, phase_id, name, start_date, duration_workdays, order_index, notes
         FROM task WHERE id = ?1",
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
        "SELECT id, phase_id, name, start_date, duration_workdays, order_index, notes
         FROM task WHERE phase_id = ?1 ORDER BY order_index ASC",
    )?;
    let rows = stmt.query_map([phase_id], row_to_task)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.phase_id, t.name, t.start_date, t.duration_workdays, t.order_index, t.notes
         FROM task t JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1
         ORDER BY p.order_index ASC, t.order_index ASC",
    )?;
    let rows = stmt.query_map([job_id], row_to_task)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, task: &Task) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE task SET phase_id = ?1, name = ?2, start_date = ?3,
                         duration_workdays = ?4, order_index = ?5, notes = ?6
         WHERE id = ?7",
        params![
            task.phase_id, task.name, task.start_date.to_string(),
            task.duration_workdays, task.order_index, task.notes, task.id,
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase};
    use crate::repo::{job, phase};

    fn setup(conn: &Connection) -> i64 {
        let j = job::create(conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
        }).unwrap();
        let p = phase::create(conn, &NewPhase {
            job_id: j.id, name: "Plumbing".into(), colour: "#3B82F6".into(),
            order_index: 0, collapsed: true,
        }).unwrap();
        p.id
    }

    fn sample(phase_id: i64, name: &str, order_index: i64) -> NewTask {
        NewTask {
            phase_id, name: name.into(),
            start_date: NaiveDate::from_ymd_opt(2026, 6, 8).unwrap(),
            duration_workdays: 3, order_index, notes: None,
        }
    }

    #[test]
    fn create_and_list() {
        let conn = open_in_memory().unwrap();
        let phase_id = setup(&conn);
        let a = create(&conn, &sample(phase_id, "First-fix", 0)).unwrap();
        let b = create(&conn, &sample(phase_id, "Second-fix", 1)).unwrap();
        let list = list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, a.id);
        assert_eq!(list[1].id, b.id);
    }

    #[test]
    fn duration_zero_is_rejected_by_check_constraint() {
        let conn = open_in_memory().unwrap();
        let phase_id = setup(&conn);
        let mut bad = sample(phase_id, "Bad", 0);
        bad.duration_workdays = 0;
        let r = create(&conn, &bad);
        assert!(r.is_err(), "expected CHECK violation");
    }

    #[test]
    fn reorder_works() {
        let conn = open_in_memory().unwrap();
        let phase_id = setup(&conn);
        let a = create(&conn, &sample(phase_id, "A", 0)).unwrap();
        let b = create(&conn, &sample(phase_id, "B", 1)).unwrap();
        let c = create(&conn, &sample(phase_id, "C", 2)).unwrap();
        reorder(&conn, phase_id, &[c.id, a.id, b.id]).unwrap();
        let list = list_for_phase(&conn, phase_id).unwrap();
        assert_eq!(list.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["C","A","B"]);
    }
}
