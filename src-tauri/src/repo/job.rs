use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::db::models::{Job, NewJob};
use crate::{GbError, GbResult};

pub fn create(conn: &Connection, new: &NewJob) -> GbResult<Job> {
    conn.execute(
        "INSERT INTO job (name, client, address, project_start_date, is_template)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            new.name,
            new.client,
            new.address,
            new.project_start_date.to_string(),
            new.is_template as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get(conn, id)
}

pub fn get(conn: &Connection, id: i64) -> GbResult<Job> {
    conn.query_row(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job WHERE id = ?1",
        [id],
        row_to_job,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("job {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_active(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job
         WHERE archived = 0 AND is_template = 0
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_templates(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, client, address, project_start_date,
                is_template, archived, created_at
         FROM job
         WHERE is_template = 1
         ORDER BY name",
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, job: &Job) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE job SET name = ?1, client = ?2, address = ?3,
                        project_start_date = ?4, is_template = ?5, archived = ?6
         WHERE id = ?7",
        params![
            job.name, job.client, job.address,
            job.project_start_date.to_string(),
            job.is_template as i64,
            job.archived as i64,
            job.id,
        ],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("job {}", job.id))); }
    Ok(())
}

pub fn set_archived(conn: &Connection, id: i64, archived: bool) -> GbResult<()> {
    let n = conn.execute("UPDATE job SET archived = ?1 WHERE id = ?2", params![archived as i64, id])?;
    if n == 0 { return Err(GbError::NotFound(format!("job {id}"))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM job WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("job {id}"))); }
    Ok(())
}

fn row_to_job(r: &rusqlite::Row) -> rusqlite::Result<Job> {
    let date_str: String = r.get(4)?;
    let project_start_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok(Job {
        id: r.get(0)?,
        name: r.get(1)?,
        client: r.get(2)?,
        address: r.get(3)?,
        project_start_date,
        is_template: r.get::<_, i64>(5)? != 0,
        archived: r.get::<_, i64>(6)? != 0,
        created_at: r.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;

    fn sample(name: &str) -> NewJob {
        NewJob {
            name: name.into(),
            client: None,
            address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
            is_template: false,
        }
    }

    #[test]
    fn create_and_get_roundtrip() {
        let conn = open_in_memory().unwrap();
        let job = create(&conn, &sample("Sea Point")).unwrap();
        assert!(job.id > 0);
        assert_eq!(job.name, "Sea Point");
        let fetched = get(&conn, job.id).unwrap();
        assert_eq!(fetched.name, "Sea Point");
    }

    #[test]
    fn list_active_excludes_archived() {
        let conn = open_in_memory().unwrap();
        let a = create(&conn, &sample("A")).unwrap();
        let _b = create(&conn, &sample("B")).unwrap();
        set_archived(&conn, a.id, true).unwrap();
        let list = list_active(&conn).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "B");
    }

    #[test]
    fn update_changes_name() {
        let conn = open_in_memory().unwrap();
        let mut job = create(&conn, &sample("Old")).unwrap();
        job.name = "New".into();
        update(&conn, &job).unwrap();
        assert_eq!(get(&conn, job.id).unwrap().name, "New");
    }

    #[test]
    fn delete_removes_row() {
        let conn = open_in_memory().unwrap();
        let job = create(&conn, &sample("Doomed")).unwrap();
        delete(&conn, job.id).unwrap();
        assert!(matches!(get(&conn, job.id), Err(GbError::NotFound(_))));
    }
}
