#!/usr/bin/env python3
"""One-time import of the LibreOffice BoQ sheet into Blik Plan's SQLite DB.

Usage:
    python3 scripts/import_boq_from_ods.py <job_id> [--ods PATH] [--db PATH] [--dry-run]

Maps the old free-text Status column onto the Procurement lifecycle:
    Complete                                   -> delivered
    In Progress                                -> ordered
    everything else (Not Started / Awaiting
    Decision / Ready to order / blank)         -> not_ordered
Rows with a filled Rate but Status 'Complete' stay 'delivered'.
Review + correct edge cases in-app after import.
"""
import argparse, os, sqlite3, sys, zipfile
import xml.etree.ElementTree as ET

T = 'urn:oasis:names:tc:opendocument:xmlns:table:1.0'
TEXTNS = 'urn:oasis:names:tc:opendocument:xmlns:text:1.0'
OFFICE = 'urn:oasis:names:tc:opendocument:xmlns:office:1.0'

DEFAULT_ODS = os.path.expanduser('~/Downloads/Bill_of_Quantities.ods')

def default_db():
    base = os.path.expanduser('~/Library/Application Support')
    for name in ('Blik Plan', 'Gantt Bok'):
        p = os.path.join(base, name, 'ganttbok.db')
        if os.path.exists(p):
            return p
    return os.path.join(base, 'Gantt Bok', 'ganttbok.db')

# Sheet column index (0-based) -> boq_item column.
COLS = ['item','qty','unit','rate',None,'trade','full_spec','w_mm','d_mm','h_mm',
        'dia_mm','supplier','location','status','lead_weeks','invoice_no','tut_ref_no','organisation']
NUMERIC = {'qty','rate','w_mm','d_mm','h_mm','dia_mm','lead_weeks'}

def cell_text(c):
    return ' '.join(''.join(p.itertext()) for p in c.iter(f'{{{TEXTNS}}}p')).strip()

def read_boq_rows(ods_path):
    z = zipfile.ZipFile(ods_path)
    root = ET.fromstring(z.read('content.xml'))
    for tbl in root.iter(f'{{{T}}}table'):
        if tbl.get(f'{{{T}}}name') != 'BoQ':
            continue
        rows = []
        for ri, row in enumerate(tbl.iter(f'{{{T}}}table-row')):
            if ri == 0:
                continue  # header
            cells = []
            for c in row.findall(f'{{{T}}}table-cell'):
                rep = int(c.get(f'{{{T}}}number-columns-repeated', '1'))
                val = c.get(f'{{{OFFICE}}}value') or cell_text(c)
                cells.extend([val] * min(rep, 50))
            if any(x for x in cells):
                rows.append(cells)
        return rows
    raise SystemExit('No "BoQ" sheet found in the .ods')

def to_procurement(status):
    s = (status or '').strip().lower()
    if s == 'complete':
        return 'delivered'
    if s == 'in progress':
        return 'ordered'
    return 'not_ordered'

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('job_id', type=int)
    ap.add_argument('--ods', default=DEFAULT_ODS)
    ap.add_argument('--db', default=default_db())
    ap.add_argument('--dry-run', action='store_true')
    a = ap.parse_args()

    rows = read_boq_rows(a.ods)
    conn = sqlite3.connect(a.db)
    conn.execute('PRAGMA foreign_keys = ON')
    if not conn.execute('SELECT 1 FROM job WHERE id = ?', (a.job_id,)).fetchone():
        sys.exit(f'job {a.job_id} not found in {a.db}')

    start = conn.execute(
        'SELECT COALESCE(MAX(order_index)+1, 0) FROM boq_item WHERE job_id = ?', (a.job_id,)
    ).fetchone()[0]

    inserted = 0
    for i, cells in enumerate(rows):
        rec = {'job_id': a.job_id, 'order_index': start + i, 'procurement': 'not_ordered'}
        for ci, key in enumerate(COLS):
            if key is None or ci >= len(cells):
                continue
            raw = (cells[ci] or '').strip()
            if key == 'status':
                rec['procurement'] = to_procurement(raw)
            elif key in NUMERIC:
                try: rec[key] = float(raw) if raw else None
                except ValueError: rec[key] = None
            else:
                rec[key] = raw or ('' if key == 'item' else None)
        cols = ','.join(rec.keys())
        ph = ','.join(['?'] * len(rec))
        if a.dry_run:
            print(rec.get('item'), '->', rec['procurement'])
        else:
            conn.execute(f'INSERT INTO boq_item ({cols}) VALUES ({ph})', list(rec.values()))
            inserted += 1
    if not a.dry_run:
        conn.commit()
    print(f"{'(dry-run) ' if a.dry_run else ''}rows read: {len(rows)}, inserted: {inserted}")

if __name__ == '__main__':
    main()
