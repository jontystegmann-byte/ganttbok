import { describe, it, expect } from 'vitest';
import {
  COLUMNS, DEFAULT_HIDDEN, cost, sortItems, filterItems, type ColumnKey,
} from '../boq-grid';
import type { BoqItem } from '../../types';

function mk(partial: Partial<BoqItem>): BoqItem {
  return {
    id: 1, job_id: 1, order_index: 0, item: '', qty: null, unit: null, rate: null,
    trade: null, full_spec: null, w_mm: null, d_mm: null, h_mm: null, dia_mm: null,
    supplier: null, location: null, procurement: 'not_ordered', delivered_date: null,
    lead_weeks: null, invoice_no: null, tut_ref_no: null, organisation: null,
    created_at: '2026-07-06T00:00:00', ...partial,
  };
}

describe('cost', () => {
  it('is qty*rate when both present, else null', () => {
    expect(cost(mk({ qty: 2, rate: 100 }))).toBe(200);
    expect(cost(mk({ qty: null, rate: 100 }))).toBeNull();
    expect(cost(mk({ qty: 2, rate: null }))).toBeNull();
  });
});

describe('COLUMNS / DEFAULT_HIDDEN', () => {
  it('includes item first and cost as computed', () => {
    expect(COLUMNS[0].key).toBe('item');
    expect(COLUMNS.find(c => c.key === 'cost')?.computed).toBe(true);
  });
  it('default-hides dimension + ref columns', () => {
    for (const k of ['full_spec','w_mm','d_mm','h_mm','dia_mm','invoice_no','tut_ref_no','organisation'] as ColumnKey[]) {
      expect(DEFAULT_HIDDEN).toContain(k);
    }
  });
});

describe('sortItems', () => {
  const items = [
    mk({ id: 1, item: 'Beta',  qty: 1, rate: 300 }), // cost 300
    mk({ id: 2, item: 'Alpha', qty: 2, rate: 50 }),  // cost 100
    mk({ id: 3, item: 'Gamma', qty: null, rate: null }), // cost null
  ];
  it('sorts numeric cost ascending with nulls last', () => {
    const out = sortItems(items, 'cost', 'asc').map(i => i.id);
    expect(out).toEqual([2, 1, 3]);
  });
  it('sorts numeric cost descending with nulls last', () => {
    const out = sortItems(items, 'cost', 'desc').map(i => i.id);
    expect(out).toEqual([1, 2, 3]);
  });
  it('sorts text case-insensitively', () => {
    const out = sortItems(items, 'item', 'asc').map(i => i.item);
    expect(out).toEqual(['Alpha', 'Beta', 'Gamma']);
  });
  it('returns original order when column is null', () => {
    const out = sortItems(items, null, 'asc').map(i => i.id);
    expect(out).toEqual([1, 2, 3]);
  });
});

describe('filterItems', () => {
  const items = [
    mk({ id: 1, item: 'Heat pump', supplier: 'Hydrofire', procurement: 'ordered' }),
    mk({ id: 2, item: 'Skylight', supplier: 'OZ', procurement: 'quoted' }),
    mk({ id: 3, item: 'Gate', full_spec: 'timber hobbit', procurement: 'not_ordered' }),
  ];
  it('filters by procurement status', () => {
    expect(filterItems(items, 'quoted', '').map(i => i.id)).toEqual([2]);
    expect(filterItems(items, 'all', '').length).toBe(3);
  });
  it('filters by case-insensitive search across item/supplier/full_spec/location/invoice', () => {
    expect(filterItems(items, 'all', 'hydro').map(i => i.id)).toEqual([1]);
    expect(filterItems(items, 'all', 'HOBBIT').map(i => i.id)).toEqual([3]);
  });
  it('combines status + search (AND)', () => {
    expect(filterItems(items, 'ordered', 'heat').map(i => i.id)).toEqual([1]);
    expect(filterItems(items, 'quoted', 'heat').length).toBe(0);
  });
});
