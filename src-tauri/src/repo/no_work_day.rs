use chrono::NaiveDate;
use rusqlite::{Connection, params};
use crate::calendar::sa_holidays::sa_holidays_for_range;
use crate::db::models::{NoWorkDay, NewNoWorkDay};
use crate::{GbError, GbResult};

pub fn list_for_job(conn: &Connection, job_id: i64) -> GbResult<Vec<NoWorkDay>> {
    let mut stmt = conn.prepare(
        "SELECT id, job_id, date, reason, source FROM no_work_day WHERE job_id = ?1 ORDER BY date",
    )?;
    let rows = stmt.query_map([job_id], row_to_nwd)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

pub fn create(conn: &Connection, new: &NewNoWorkDay) -> GbResult<NoWorkDay> {
    conn.execute(
        "INSERT INTO no_work_day (job_id, date, reason, source)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(job_id, date) DO UPDATE SET reason = excluded.reason, source = excluded.source",
        params![new.job_id, new.date.to_string(), new.reason, new.source],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, job_id, date, reason, source FROM no_work_day WHERE id = ?1",
        [id],
        row_to_nwd,
    ).map_err(GbError::from)
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM no_work_day WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("no_work_day {id}"))); }
    Ok(())
}

/// Insert SA public holidays into `no_work_day` for [from..to] inclusive, *without overwriting* manual entries.
/// Kept for backwards-compat — new code should call `sync_holidays` with a region.
pub fn sync_sa_holidays(conn: &Connection, job_id: i64, from: NaiveDate, to: NaiveDate) -> GbResult<i64> {
    sync_holidays(conn, job_id, "ZA", from, to)
}

/// Sync public holidays for the given region into `no_work_day` for [from..to] inclusive.
/// `region` must be one of: "ZA" | "US" | "GB" | "IN" | "CN".
/// Manual entries are preserved. All *_holiday entries for OTHER regions in the range are cleared
/// (so switching a job's region replaces the holiday set cleanly).
pub fn sync_holidays(conn: &Connection, job_id: i64, region: &str, from: NaiveDate, to: NaiveDate) -> GbResult<i64> {
    use crate::calendar::{
        sa_holidays::sa_holidays_for_range,
        us_holidays::us_holidays_for_range,
        gb_holidays::gb_holidays_for_range,
        in_holidays::in_holidays_for_range,
        cn_holidays::cn_holidays_for_range,
        sa_holidays::Holiday,
    };

    let (holidays, source): (Vec<Holiday>, &'static str) = match region {
        "ZA" => (sa_holidays_for_range(from, to), "za_holiday"),
        "US" => (us_holidays_for_range(from, to), "us_holiday"),
        "GB" => (gb_holidays_for_range(from, to), "gb_holiday"),
        "IN" => (in_holidays_for_range(from, to), "in_holiday"),
        "CN" => (cn_holidays_for_range(from, to), "cn_holiday"),
        _ => return Err(crate::GbError::Validation(format!("unknown region {region}"))),
    };

    let tx = conn.unchecked_transaction()?;

    // Clear ALL holiday rows in range (including legacy sa_public_holiday + other regions).
    tx.execute(
        "DELETE FROM no_work_day
         WHERE job_id = ?1 AND date >= ?2 AND date <= ?3
           AND source IN ('za_holiday','us_holiday','gb_holiday','in_holiday','cn_holiday','sa_public_holiday')",
        params![job_id, from.to_string(), to.to_string()],
    )?;

    let mut inserted: i64 = 0;
    for h in holidays {
        let manual_exists: i64 = tx.query_row(
            "SELECT COUNT(*) FROM no_work_day WHERE job_id = ?1 AND date = ?2 AND source = 'manual'",
            params![job_id, h.date.to_string()],
            |r| r.get(0),
        )?;
        if manual_exists == 0 {
            tx.execute(
                "INSERT OR IGNORE INTO no_work_day (job_id, date, reason, source)
                 VALUES (?1, ?2, ?3, ?4)",
                params![job_id, h.date.to_string(), h.name, source],
            )?;
            inserted += 1;
        }
    }
    tx.commit()?;
    Ok(inserted)
}

fn row_to_nwd(r: &rusqlite::Row) -> rusqlite::Result<NoWorkDay> {
    let date_str: String = r.get(2)?;
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e)))?;
    Ok(NoWorkDay {
        id: r.get(0)?,
        job_id: r.get(1)?,
        date,
        reason: r.get(3)?,
        source: r.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::models::NewJob;
    use crate::repo::job;

    #[test]
    fn sync_2026_inserts_twelve_holidays() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            is_template: false,
            holidays_block_work: true,
            region: "ZA".into(),
            auto_shift_dependents: true,
        }).unwrap();
        let n = sync_sa_holidays(
            &conn, j.id,
            NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            NaiveDate::from_ymd_opt(2026,12,31).unwrap(),
        ).unwrap();
        assert_eq!(n, 12);
    }

    #[test]
    fn sync_does_not_overwrite_manual_entries() {
        let conn = open_in_memory().unwrap();
        let j = job::create(&conn, &NewJob {
            name: "J".into(), client: None, address: None,
            project_start_date: NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            is_template: false,
            holidays_block_work: true,
            region: "ZA".into(),
            auto_shift_dependents: true,
        }).unwrap();
        create(&conn, &NewNoWorkDay {
            job_id: j.id,
            date: NaiveDate::from_ymd_opt(2026, 6, 16).unwrap(),
            reason: "Team building".into(),
            source: "manual".into(),
        }).unwrap();
        let n = sync_sa_holidays(
            &conn, j.id,
            NaiveDate::from_ymd_opt(2026,1,1).unwrap(),
            NaiveDate::from_ymd_opt(2026,12,31).unwrap(),
        ).unwrap();
        assert_eq!(n, 11);
        let list = list_for_job(&conn, j.id).unwrap();
        let youth_day = list.iter().find(|r| r.date == NaiveDate::from_ymd_opt(2026,6,16).unwrap()).unwrap();
        assert_eq!(youth_day.source, "manual");
        assert_eq!(youth_day.reason, "Team building");
    }
}
