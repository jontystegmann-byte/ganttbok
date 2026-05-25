use rusqlite::{Connection, params};
use crate::db::models::{Phase, NewPhase};
use crate::{GbError, GbResult};

pub fn create(conn: &Connection, new: &NewPhase) -> GbResult<Phase> {
    conn.execute(
        "INSERT INTO phase (job_id, name, colour, order_index, collapsed, notes)
         VALUES (?1, ?2, ?3, ?4, ?5, '')",
        params![new.job_id, new.name, new.colour, new.order_index, new.collapsed as i64],
    )?;
    get(conn, conn.last_insert_rowid())
}

const SELECT_COLS: &str = "id, job_id, name, colour, order_index, collapsed, notes";

pub fn get(conn: &Connection, id: i64) -> GbResult<Phase> {
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM phase WHERE id = ?1"),
        [id],
        row_to_phase,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("phase {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Phase>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM phase WHERE job_id = ?1 ORDER BY order_index ASC"),
    )?;
    let rows = stmt.query_map([job_id], row_to_phase)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, phase: &Phase) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE phase SET name = ?1, colour = ?2, order_index = ?3, collapsed = ?4, notes = ?5 WHERE id = ?6",
        params![phase.name, phase.colour, phase.order_index, phase.collapsed as i64, phase.notes, phase.id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("phase {}", phase.id))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM phase WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("phase {id}"))); }
    Ok(())
}

pub fn reorder(conn: &Connection, job_id: i64, ordered_ids: &[i64]) -> GbResult<()> {
    let tx = conn.unchecked_transaction()?;
    for (idx, id) in ordered_ids.iter().enumerate() {
        let n = tx.execute(
            "UPDATE phase SET order_index = ?1 WHERE id = ?2 AND job_id = ?3",
            params![idx as i64, id, job_id],
        )?;
        if n == 0 {
            return Err(GbError::Validation(format!("phase {id} not in job {job_id}")));
        }
    }
    tx.commit()?;
    Ok(())
}

fn row_to_phase(r: &rusqlite::Row) -> rusqlite::Result<Phase> {
    Ok(Phase {
        id: r.get(0)?,
        job_id: r.get(1)?,
        name: r.get(2)?,
        colour: r.get(3)?,
        order_index: r.get(4)?,
        collapsed: r.get::<_, i64>(5)? != 0,
        notes: r.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::NewJob;
    use crate::repo::job;
    use chrono::NaiveDate;

    fn make_job(conn: &Connection) -> i64 {
        let j = job::create(conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
            holidays_block_work: true,
            region: "ZA".into(),
            auto_shift_dependents: true,
        }).unwrap();
        j.id
    }

    #[test]
    fn create_and_list_phases() {
        let conn = open_in_memory().unwrap();
        let job_id = make_job(&conn);
        let a = create(&conn, &NewPhase { job_id, name: "Plumbing".into(), colour: "#3B82F6".into(), order_index: 0, collapsed: true }).unwrap();
        let b = create(&conn, &NewPhase { job_id, name: "Electrical".into(), colour: "#EF4444".into(), order_index: 1, collapsed: true }).unwrap();
        let phases = list_for_job(&conn, job_id).unwrap();
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].id, a.id);
        assert_eq!(phases[1].id, b.id);
    }

    #[test]
    fn reorder_swaps_order_indices() {
        let conn = open_in_memory().unwrap();
        let job_id = make_job(&conn);
        let a = create(&conn, &NewPhase { job_id, name: "A".into(), colour: "#000".into(), order_index: 0, collapsed: true }).unwrap();
        let b = create(&conn, &NewPhase { job_id, name: "B".into(), colour: "#000".into(), order_index: 1, collapsed: true }).unwrap();
        let c = create(&conn, &NewPhase { job_id, name: "C".into(), colour: "#000".into(), order_index: 2, collapsed: true }).unwrap();
        reorder(&conn, job_id, &[c.id, a.id, b.id]).unwrap();
        let phases = list_for_job(&conn, job_id).unwrap();
        assert_eq!(phases.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["C","A","B"]);
    }

    #[test]
    fn delete_phase_cascades_to_tasks() {
        let conn = open_in_memory().unwrap();
        let job_id = make_job(&conn);
        let p = create(&conn, &NewPhase { job_id, name: "Doomed".into(), colour: "#000".into(), order_index: 0, collapsed: true }).unwrap();
        // Insert a task directly via SQL so we don't depend on Task 16 yet
        conn.execute(
            "INSERT INTO task (phase_id, name, start_date, duration_workdays, order_index) VALUES (?1, 'T', '2026-06-05', 1, 0)",
            params![p.id],
        ).unwrap();
        delete(&conn, p.id).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM task WHERE phase_id = ?1", [p.id], |r| r.get(0)).unwrap();
        assert_eq!(count, 0);
    }
}
