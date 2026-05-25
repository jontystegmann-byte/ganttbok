use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::db::models::{Job, NewJob};
use crate::{GbError, GbResult};

pub fn create(conn: &Connection, new: &NewJob) -> GbResult<Job> {
    conn.execute(
        "INSERT INTO job (name, client, address, project_start_date, is_template, holidays_block_work, region, auto_shift_dependents)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            new.name,
            new.client,
            new.address,
            new.project_start_date.to_string(),
            new.is_template as i64,
            new.holidays_block_work as i64,
            new.region,
            new.auto_shift_dependents as i64,
        ],
    )?;
    let id = conn.last_insert_rowid();
    get(conn, id)
}

const SELECT_COLS: &str = "id, name, client, address, project_start_date, \
    is_template, archived, created_at, holidays_block_work, region, auto_shift_dependents";

pub fn get(conn: &Connection, id: i64) -> GbResult<Job> {
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM job WHERE id = ?1"),
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
        &format!("SELECT {SELECT_COLS} FROM job \
                  WHERE archived = 0 AND is_template = 0 \
                  ORDER BY created_at DESC"),
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_archived(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM job \
                  WHERE archived = 1 AND is_template = 0 \
                  ORDER BY created_at DESC"),
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn list_templates(conn: &Connection) -> GbResult<Vec<Job>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM job \
                  WHERE is_template = 1 \
                  ORDER BY name"),
    )?;
    let rows = stmt.query_map([], row_to_job)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn update(conn: &Connection, job: &Job) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE job SET name = ?1, client = ?2, address = ?3,
                        project_start_date = ?4, is_template = ?5, archived = ?6,
                        holidays_block_work = ?7, region = ?8, auto_shift_dependents = ?9
         WHERE id = ?10",
        params![
            job.name, job.client, job.address,
            job.project_start_date.to_string(),
            job.is_template as i64,
            job.archived as i64,
            job.holidays_block_work as i64,
            job.region,
            job.auto_shift_dependents as i64,
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
        holidays_block_work: r.get::<_, i64>(8)? != 0,
        region: r.get(9)?,
        auto_shift_dependents: r.get::<_, i64>(10)? != 0,
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
            holidays_block_work: true,
            region: "ZA".into(),
            auto_shift_dependents: true,
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
        assert!(fetched.holidays_block_work);
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
    fn list_archived_returns_only_archived() {
        let conn = open_in_memory().unwrap();
        let a = create(&conn, &sample("A")).unwrap();
        create(&conn, &sample("B")).unwrap();
        set_archived(&conn, a.id, true).unwrap();
        let archived = list_archived(&conn).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].name, "A");
    }

    #[test]
    fn update_changes_name() {
        let conn = open_in_memory().unwrap();
        let mut job = create(&conn, &sample("Old")).unwrap();
        job.name = "New".into();
        job.holidays_block_work = false;
        update(&conn, &job).unwrap();
        let fetched = get(&conn, job.id).unwrap();
        assert_eq!(fetched.name, "New");
        assert!(!fetched.holidays_block_work);
    }

    #[test]
    fn delete_removes_row() {
        let conn = open_in_memory().unwrap();
        let job = create(&conn, &sample("Doomed")).unwrap();
        delete(&conn, job.id).unwrap();
        assert!(matches!(get(&conn, job.id), Err(GbError::NotFound(_))));
    }
}
