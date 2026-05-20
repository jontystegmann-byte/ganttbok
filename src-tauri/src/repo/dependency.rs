use rusqlite::{Connection, params};
use crate::db::models::{Dependency, NewDependency};
use crate::deps::graph::{build_adjacency, would_cycle};
use crate::{GbError, GbResult};

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<Dependency>> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.predecessor_id, d.successor_id, d.type, d.lag_days
         FROM dependency d
         JOIN task t ON t.id = d.predecessor_id
         JOIN phase p ON p.id = t.phase_id
         WHERE p.job_id = ?1",
    )?;
    let rows = stmt.query_map([job_id], row_to_dep)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn create(conn: &Connection, new: &NewDependency) -> GbResult<Dependency> {
    let job_id = job_id_for_task(conn, new.predecessor_id)?;
    let existing = list_for_job(conn, job_id)?;
    let adj = build_adjacency(&existing);
    if would_cycle(&adj, new.predecessor_id, new.successor_id) {
        return Err(GbError::DependencyCycle(new.predecessor_id, new.successor_id));
    }
    conn.execute(
        "INSERT INTO dependency (predecessor_id, successor_id, type, lag_days)
         VALUES (?1, ?2, 'FS', ?3)",
        params![new.predecessor_id, new.successor_id, new.lag_days],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, predecessor_id, successor_id, type, lag_days FROM dependency WHERE id = ?1",
        [id],
        row_to_dep,
    ).map_err(GbError::from)
}

pub fn update_lag(conn: &Connection, id: i64, lag_days: i64) -> GbResult<()> {
    let n = conn.execute("UPDATE dependency SET lag_days = ?1 WHERE id = ?2", params![lag_days, id])?;
    if n == 0 { return Err(GbError::NotFound(format!("dependency {id}"))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM dependency WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("dependency {id}"))); }
    Ok(())
}

fn job_id_for_task(conn: &Connection, task_id: i64) -> GbResult<i64> {
    conn.query_row(
        "SELECT p.job_id FROM task t JOIN phase p ON p.id = t.phase_id WHERE t.id = ?1",
        [task_id],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("task {task_id}")),
        other => GbError::Sqlite(other),
    })
}

fn row_to_dep(r: &rusqlite::Row) -> rusqlite::Result<Dependency> {
    Ok(Dependency {
        id: r.get(0)?,
        predecessor_id: r.get(1)?,
        successor_id: r.get(2)?,
        r#type: r.get(3)?,
        lag_days: r.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::db::connection::open_in_memory;
    use crate::db::models::{NewJob, NewPhase, NewTask};
    use crate::repo::{job, phase, task};

    fn three_tasks() -> (rusqlite::Connection, i64, i64, i64) {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,6,5).unwrap(),
            is_template: false,
        }).unwrap();
        let p = phase::create(&conn, &NewPhase {
            job_id: j.id, name: "P".into(), colour: "#000".into(),
            order_index: 0, collapsed: false,
        }).unwrap();
        let t1 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T1".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,8).unwrap(),
            duration_workdays: 1, order_index: 0, notes: None,
        }).unwrap();
        let t2 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T2".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,9).unwrap(),
            duration_workdays: 1, order_index: 1, notes: None,
        }).unwrap();
        let t3 = task::create(&conn, &NewTask {
            phase_id: p.id, name: "T3".into(),
            start_date: NaiveDate::from_ymd_opt(2026,6,10).unwrap(),
            duration_workdays: 1, order_index: 2, notes: None,
        }).unwrap();
        (conn, t1.id, t2.id, t3.id)
    }

    #[test]
    fn create_dependency_then_list() {
        let (conn, t1, t2, _) = three_tasks();
        let d = create(&conn, &NewDependency { predecessor_id: t1, successor_id: t2, lag_days: 0 }).unwrap();
        let list = list_for_job(&conn, 1).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, d.id);
    }

    #[test]
    fn create_cycle_is_rejected() {
        let (conn, t1, t2, t3) = three_tasks();
        create(&conn, &NewDependency { predecessor_id: t1, successor_id: t2, lag_days: 0 }).unwrap();
        create(&conn, &NewDependency { predecessor_id: t2, successor_id: t3, lag_days: 0 }).unwrap();
        let bad = create(&conn, &NewDependency { predecessor_id: t3, successor_id: t1, lag_days: 0 });
        assert!(matches!(bad, Err(GbError::DependencyCycle(_,_))));
    }
}
