import type { BoqItem, Procurement } from '../types';

export type ColumnKey =
  | 'item' | 'qty' | 'unit' | 'rate' | 'cost' | 'trade' | 'full_spec'
  | 'w_mm' | 'd_mm' | 'h_mm' | 'dia_mm' | 'supplier' | 'location'
  | 'procurement' | 'lead_weeks' | 'invoice_no' | 'tut_ref_no' | 'organisation';

export interface ColumnDef {
  key: ColumnKey;
  label: string;
  numeric: boolean;
  computed?: boolean; // cost is derived, never edited
}

export const COLUMNS: ColumnDef[] = [
  { key: 'item',        label: 'Item',        numeric: false },
  { key: 'qty',         label: 'Qty',         numeric: true  },
  { key: 'unit',        label: 'Unit',        numeric: false },
  { key: 'rate',        label: 'Rate',        numeric: true  },
  { key: 'cost',        label: 'Cost',        numeric: true, computed: true },
  { key: 'trade',       label: 'Trade',       numeric: false },
  { key: 'full_spec',   label: 'Full Spec',   numeric: false },
  { key: 'w_mm',        label: 'W (mm)',      numeric: true  },
  { key: 'd_mm',        label: 'D (mm)',      numeric: true  },
  { key: 'h_mm',        label: 'H (mm)',      numeric: true  },
  { key: 'dia_mm',      label: 'Ø (mm)',      numeric: true  },
  { key: 'supplier',    label: 'Supplier',    numeric: false },
  { key: 'location',    label: 'Location',    numeric: false },
  { key: 'procurement', label: 'Procurement', numeric: false },
  { key: 'lead_weeks',  label: 'Lead (wks)',  numeric: true  },
  { key: 'invoice_no',  label: 'Invoice #',   numeric: false },
  { key: 'tut_ref_no',  label: 'Tut Ref No',  numeric: false },
  { key: 'organisation',label: 'Organisation',numeric: false },
];

export const DEFAULT_HIDDEN: ColumnKey[] = [
  'full_spec', 'w_mm', 'd_mm', 'h_mm', 'dia_mm', 'invoice_no', 'tut_ref_no', 'organisation',
];

export const PROCUREMENT_LABELS: Record<Procurement, string> = {
  not_ordered: 'Not ordered',
  quoted: 'Quoted',
  ordered: 'Ordered',
  delivered: 'Delivered',
};

export type StatusFilter = 'all' | Procurement;
export type SortDir = 'asc' | 'desc';

export function cost(it: BoqItem): number | null {
  return it.qty != null && it.rate != null ? it.qty * it.rate : null;
}

/** Value used for sorting a given column. */
function sortValue(it: BoqItem, key: ColumnKey): number | string | null {
  if (key === 'cost') return cost(it);
  const v = (it as unknown as Record<string, unknown>)[key];
  return (v as number | string | null) ?? null;
}

/** Stable sort. Nulls always sort last regardless of direction. */
export function sortItems(items: BoqItem[], key: ColumnKey | null, dir: SortDir): BoqItem[] {
  if (!key) return items;
  const col = COLUMNS.find(c => c.key === key);
  const numeric = col?.numeric ?? false;
  const factor = dir === 'asc' ? 1 : -1;
  return items
    .map((it, i) => [it, i] as const)
    .sort(([a, ai], [b, bi]) => {
      const av = sortValue(a, key);
      const bv = sortValue(b, key);
      if (av == null && bv == null) return ai - bi;
      if (av == null) return 1;   // nulls last
      if (bv == null) return -1;  // nulls last
      let cmp: number;
      if (numeric) cmp = (av as number) - (bv as number);
      else cmp = String(av).localeCompare(String(bv), undefined, { sensitivity: 'base' });
      return cmp !== 0 ? cmp * factor : ai - bi;
    })
    .map(([it]) => it);
}

const SEARCH_FIELDS: (keyof BoqItem)[] = ['item', 'full_spec', 'supplier', 'location', 'invoice_no'];

export function filterItems(items: BoqItem[], status: StatusFilter, search: string): BoqItem[] {
  const q = search.trim().toLowerCase();
  return items.filter(it => {
    if (status !== 'all' && it.procurement !== status) return false;
    if (!q) return true;
    return SEARCH_FIELDS.some(f => {
      const v = it[f];
      return typeof v === 'string' && v.toLowerCase().includes(q);
    });
  });
}
