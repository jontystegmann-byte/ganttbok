use rusqlite::{Connection, params};
use crate::db::models::{BoqItem, Procurement};
use crate::{GbError, GbResult};

const SELECT_COLS: &str = "id, job_id, order_index, item, qty, unit, rate, trade, \
    full_spec, w_mm, d_mm, h_mm, dia_mm, supplier, location, procurement, \
    delivered_date, lead_weeks, invoice_no, tut_ref_no, organisation, created_at";

/// Append a blank line item to a job. order_index = current max + 1.
pub fn create(conn: &Connection, job_id: i64) -> GbResult<BoqItem> {
    let next_index: i64 = conn.query_row(
        "SELECT COALESCE(MAX(order_index) + 1, 0) FROM boq_item WHERE job_id = ?1",
        [job_id], |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO boq_item (job_id, order_index, item) VALUES (?1, ?2, '')",
        params![job_id, next_index],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> GbResult<BoqItem> {
    conn.query_row(
        &format!("SELECT {SELECT_COLS} FROM boq_item WHERE id = ?1"),
        [id],
        row_to_boq_item,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("boq_item {id}")),
        other => GbError::Sqlite(other),
    })
}

pub fn list_by_job(conn: &Connection, job_id: i64) -> GbResult<Vec<BoqItem>> {
    let mut stmt = conn.prepare(
        &format!("SELECT {SELECT_COLS} FROM boq_item WHERE job_id = ?1 ORDER BY order_index ASC"),
    )?;
    let rows = stmt.query_map([job_id], row_to_boq_item)?;
    let mut out = Vec::new();
    for r in rows { out.push(r?); }
    Ok(out)
}

/// Update CONTENT fields only. Deliberately does NOT write `procurement` or
/// `delivered_date` — those are owned by `set_procurement`, mirroring the
/// task.rs guard so grid edits never clobber procurement state.
pub fn update(conn: &Connection, b: &BoqItem) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE boq_item SET
            item = ?1, qty = ?2, unit = ?3, rate = ?4, trade = ?5, full_spec = ?6,
            w_mm = ?7, d_mm = ?8, h_mm = ?9, dia_mm = ?10, supplier = ?11,
            location = ?12, lead_weeks = ?13, invoice_no = ?14, tut_ref_no = ?15,
            organisation = ?16
         WHERE id = ?17",
        params![
            b.item, b.qty, b.unit, b.rate, b.trade, b.full_spec,
            b.w_mm, b.d_mm, b.h_mm, b.dia_mm, b.supplier,
            b.location, b.lead_weeks, b.invoice_no, b.tut_ref_no,
            b.organisation, b.id,
        ],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {}", b.id))); }
    Ok(())
}

/// The ONLY writer of procurement/delivered_date.
/// When status == Delivered, `delivered_date` is stored; otherwise it is cleared.
pub fn set_procurement(
    conn: &Connection,
    id: i64,
    status: Procurement,
    delivered_date: Option<&str>,
) -> GbResult<()> {
    let stored_date = if status == Procurement::Delivered { delivered_date } else { None };
    let n = conn.execute(
        "UPDATE boq_item SET procurement = ?1, delivered_date = ?2 WHERE id = ?3",
        params![status.as_db_str(), stored_date, id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {id}"))); }
    Ok(())
}

pub fn reorder(conn: &Connection, id: i64, order_index: i64) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE boq_item SET order_index = ?1 WHERE id = ?2",
        params![order_index, id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {id}"))); }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> GbResult<()> {
    let n = conn.execute("DELETE FROM boq_item WHERE id = ?1", [id])?;
    if n == 0 { return Err(GbError::NotFound(format!("boq_item {id}"))); }
    Ok(())
}

pub fn set_job_budget(conn: &Connection, job_id: i64, budget: Option<f64>) -> GbResult<()> {
    let n = conn.execute(
        "UPDATE job SET budget = ?1 WHERE id = ?2",
        params![budget, job_id],
    )?;
    if n == 0 { return Err(GbError::NotFound(format!("job {job_id}"))); }
    Ok(())
}

pub fn get_job_budget(conn: &Connection, job_id: i64) -> GbResult<Option<f64>> {
    conn.query_row("SELECT budget FROM job WHERE id = ?1", [job_id], |r| r.get(0))
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => GbError::NotFound(format!("job {job_id}")),
            other => GbError::Sqlite(other),
        })
}

fn row_to_boq_item(r: &rusqlite::Row) -> rusqlite::Result<BoqItem> {
    let proc_str: String = r.get(15)?;
    Ok(BoqItem {
        id: r.get(0)?,
        job_id: r.get(1)?,
        order_index: r.get(2)?,
        item: r.get(3)?,
        qty: r.get(4)?,
        unit: r.get(5)?,
        rate: r.get(6)?,
        trade: r.get(7)?,
        full_spec: r.get(8)?,
        w_mm: r.get(9)?,
        d_mm: r.get(10)?,
        h_mm: r.get(11)?,
        dia_mm: r.get(12)?,
        supplier: r.get(13)?,
        location: r.get(14)?,
        procurement: Procurement::from_db_str(&proc_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(15, "procurement".into(), rusqlite::types::Type::Text))?,
        delivered_date: r.get(16)?,
        lead_weeks: r.get(17)?,
        invoice_no: r.get(18)?,
        tut_ref_no: r.get(19)?,
        organisation: r.get(20)?,
        created_at: r.get(21)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Procurement;

    fn seed_job(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO job (name, project_start_date) VALUES ('t', '2026-01-01')",
            [],
        ).unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn create_appends_and_increments_order_index() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let a = create(&conn, job).unwrap();
        let b = create(&conn, job).unwrap();
        assert_eq!(a.order_index, 0);
        assert_eq!(b.order_index, 1);
        assert_eq!(a.procurement, Procurement::NotOrdered);
        assert_eq!(list_by_job(&conn, job).unwrap().len(), 2);
    }

    #[test]
    fn update_changes_content_but_preserves_procurement() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let mut item = create(&conn, job).unwrap();

        // Move it to Ordered via the dedicated setter.
        set_procurement(&conn, item.id, Procurement::Ordered, None).unwrap();

        // Now a content edit that (maliciously) carries a different procurement value.
        item.item = "Heat pump".into();
        item.rate = Some(49444.25);
        item.procurement = Procurement::NotOrdered; // must be ignored by update()
        update(&conn, &item).unwrap();

        let fetched = get(&conn, item.id).unwrap();
        assert_eq!(fetched.item, "Heat pump");
        assert_eq!(fetched.rate, Some(49444.25));
        assert_eq!(fetched.procurement, Procurement::Ordered, "update must not clobber procurement");
    }

    #[test]
    fn set_procurement_sets_and_clears_delivered_date() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let item = create(&conn, job).unwrap();

        set_procurement(&conn, item.id, Procurement::Delivered, Some("2026-07-06")).unwrap();
        assert_eq!(get(&conn, item.id).unwrap().delivered_date, Some("2026-07-06".into()));

        // Moving back off Delivered clears the date.
        set_procurement(&conn, item.id, Procurement::Ordered, None).unwrap();
        assert_eq!(get(&conn, item.id).unwrap().delivered_date, None);
    }

    #[test]
    fn budget_set_and_get_roundtrip() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        assert_eq!(get_job_budget(&conn, job).unwrap(), None);
        set_job_budget(&conn, job, Some(2_000_000.0)).unwrap();
        assert_eq!(get_job_budget(&conn, job).unwrap(), Some(2_000_000.0));
    }

    #[test]
    fn delete_removes_row() {
        let conn = crate::db::connection::open_in_memory().unwrap();
        let job = seed_job(&conn);
        let item = create(&conn, job).unwrap();
        delete(&conn, item.id).unwrap();
        assert!(get(&conn, item.id).is_err());
    }
}
