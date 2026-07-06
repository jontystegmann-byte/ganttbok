use std::path::PathBuf;
use rust_xlsxwriter::{Workbook, Formula};
use tauri::State;

use crate::commands::Db;
use crate::db::models::{BoqItem, Procurement};
use crate::repo::boq as boq_repo;
use crate::{GbError, GbResult};

const HEADERS: &[&str] = &[
    "Item", "Qty", "Unit", "Rate", "Cost", "Trade", "Full Spec",
    "W (mm)", "D (mm)", "H (mm)", "Ø (mm)", "Supplier", "Location",
    "Procurement", "Lead (wks)", "Invoice #", "Tut Ref No", "Organisation",
];

fn proc_label(p: Procurement) -> &'static str {
    match p {
        Procurement::NotOrdered => "Not ordered",
        Procurement::Quoted => "Quoted",
        Procurement::Ordered => "Ordered",
        Procurement::Delivered => "Delivered",
    }
}

/// Build an .xlsx workbook (one BoQ sheet) as bytes.
/// Cost cells are LIVE formulas (=Qty*Rate); a grand-total row uses =SUM.
pub fn build_xlsx(items: &[BoqItem]) -> GbResult<Vec<u8>> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet().set_name("BoQ").map_err(xlsx_err)?;

    for (c, h) in HEADERS.iter().enumerate() {
        ws.write_string(0, c as u16, *h).map_err(xlsx_err)?;
    }

    for (i, it) in items.iter().enumerate() {
        let r = (i + 1) as u32; // 0-based row index; excel row = r+1
        let xl = r + 1;
        ws.write_string(r, 0, &it.item).map_err(xlsx_err)?;
        if let Some(q) = it.qty { ws.write_number(r, 1, q).map_err(xlsx_err)?; }
        if let Some(u) = &it.unit { ws.write_string(r, 2, u).map_err(xlsx_err)?; }
        if let Some(rate) = it.rate { ws.write_number(r, 3, rate).map_err(xlsx_err)?; }
        // Cost = Qty*Rate as a live formula (blank-safe: shows 0 when either is empty).
        ws.write_formula(r, 4, Formula::new(format!("=IF(OR(B{xl}=\"\",D{xl}=\"\"),\"\",B{xl}*D{xl})"))).map_err(xlsx_err)?;
        if let Some(t) = &it.trade { ws.write_string(r, 5, t).map_err(xlsx_err)?; }
        if let Some(s) = &it.full_spec { ws.write_string(r, 6, s).map_err(xlsx_err)?; }
        if let Some(v) = it.w_mm { ws.write_number(r, 7, v).map_err(xlsx_err)?; }
        if let Some(v) = it.d_mm { ws.write_number(r, 8, v).map_err(xlsx_err)?; }
        if let Some(v) = it.h_mm { ws.write_number(r, 9, v).map_err(xlsx_err)?; }
        if let Some(v) = it.dia_mm { ws.write_number(r, 10, v).map_err(xlsx_err)?; }
        if let Some(s) = &it.supplier { ws.write_string(r, 11, s).map_err(xlsx_err)?; }
        if let Some(s) = &it.location { ws.write_string(r, 12, s).map_err(xlsx_err)?; }
        ws.write_string(r, 13, proc_label(it.procurement)).map_err(xlsx_err)?;
        if let Some(v) = it.lead_weeks { ws.write_number(r, 14, v).map_err(xlsx_err)?; }
        if let Some(s) = &it.invoice_no { ws.write_string(r, 15, s).map_err(xlsx_err)?; }
        if let Some(s) = &it.tut_ref_no { ws.write_string(r, 16, s).map_err(xlsx_err)?; }
        if let Some(s) = &it.organisation { ws.write_string(r, 17, s).map_err(xlsx_err)?; }
    }

    // Grand total (live SUM over the Cost column), one blank row below the data.
    if !items.is_empty() {
        let total_row = (items.len() + 2) as u32;
        ws.write_string(total_row, 3, "TOTAL").map_err(xlsx_err)?;
        ws.write_formula(total_row, 4, Formula::new(format!("=SUM(E2:E{})", items.len() + 1))).map_err(xlsx_err)?;
    }

    wb.save_to_buffer().map_err(xlsx_err)
}

fn xlsx_err(e: rust_xlsxwriter::XlsxError) -> GbError {
    GbError::Validation(format!("xlsx: {e}"))
}

/// Export a job's BoQ to Downloads. `format` is "xlsx" or "ods".
/// XLSX is written directly; ODS is produced by converting the XLSX via
/// headless LibreOffice (`soffice`), which must be installed for ODS.
#[tauri::command]
pub fn export_boq(db: State<Db>, job_id: i64, format: String) -> GbResult<String> {
    let items = {
        let conn = db.0.lock().unwrap();
        boq_repo::list_by_job(&conn, job_id)?
    };
    let bytes = build_xlsx(&items)?;

    let dir = dirs::download_dir()
        .ok_or_else(|| GbError::Validation("no Downloads directory".into()))?;
    let xlsx_path = dir.join("Bill_of_Quantities_export.xlsx");
    std::fs::write(&xlsx_path, &bytes)?;

    match format.as_str() {
        "xlsx" => Ok(xlsx_path.to_string_lossy().into_owned()),
        "ods" => convert_to_ods(&xlsx_path, &dir),
        other => Err(GbError::Validation(format!("unknown export format: {other}"))),
    }
}

/// Convert an .xlsx to .ods using headless LibreOffice. Returns the .ods path.
fn convert_to_ods(xlsx_path: &std::path::Path, out_dir: &std::path::Path) -> GbResult<String> {
    let status = std::process::Command::new("soffice")
        .args(["--headless", "--convert-to", "ods", "--outdir"])
        .arg(out_dir)
        .arg(xlsx_path)
        .status()
        .map_err(|e| GbError::Validation(format!("LibreOffice (soffice) not found for ODS export: {e}")))?;
    if !status.success() {
        return Err(GbError::Validation("soffice conversion to ODS failed".into()));
    }
    let ods_path: PathBuf = out_dir.join("Bill_of_Quantities_export.ods");
    Ok(ods_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::Procurement;

    fn mk(item: &str, qty: Option<f64>, rate: Option<f64>, proc: Procurement) -> BoqItem {
        BoqItem {
            id: 1, job_id: 1, order_index: 0, item: item.into(), qty, unit: None, rate,
            trade: Some("HVAC".into()), full_spec: None, w_mm: None, d_mm: None, h_mm: None,
            dia_mm: None, supplier: None, location: None, procurement: proc, delivered_date: None,
            lead_weeks: None, invoice_no: None, tut_ref_no: None, organisation: None,
            created_at: "2026-07-06".into(),
        }
    }

    #[test]
    fn build_xlsx_produces_a_valid_zip() {
        let items = vec![
            mk("Heat pump", Some(1.0), Some(49444.25), Procurement::Ordered),
            mk("Buffer tank", Some(2.0), Some(48836.0), Procurement::Delivered),
        ];
        let bytes = build_xlsx(&items).unwrap();
        // .xlsx is a zip → starts with "PK\x03\x04".
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[0..2], b"PK");
    }

    #[test]
    fn build_xlsx_handles_empty() {
        let bytes = build_xlsx(&[]).unwrap();
        assert_eq!(&bytes[0..2], b"PK");
    }
}
