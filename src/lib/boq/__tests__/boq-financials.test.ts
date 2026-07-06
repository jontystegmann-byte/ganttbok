import { describe, it, expect } from 'vitest';
import { financials, sectorRollups } from '../boq-financials';
import type { BoqItem } from '../../types';

function mk(p: Partial<BoqItem>): BoqItem {
  return {
    id: 1, job_id: 1, order_index: 0, item: '', qty: null, unit: null, rate: null,
    trade: null, full_spec: null, w_mm: null, d_mm: null, h_mm: null, dia_mm: null,
    supplier: null, location: null, procurement: 'not_ordered', delivered_date: null,
    lead_weeks: null, invoice_no: null, tut_ref_no: null, organisation: null,
    created_at: '2026-07-06', ...p,
  };
}

const items: BoqItem[] = [
  mk({ id: 1, qty: 1, rate: 510000, trade: 'HVAC', procurement: 'delivered' }),
  mk({ id: 2, qty: 1, rate: 130000, trade: 'HVAC', procurement: 'ordered' }),
  mk({ id: 3, qty: 1, rate: 240000, trade: 'GLAZING', procurement: 'quoted' }),
  mk({ id: 4, qty: 1, rate: 99999, trade: 'CARPENTER', procurement: 'not_ordered' }),
];

describe('financials', () => {
  it('spent = ordered + delivered; quoted separate; not_ordered excluded', () => {
    const f = financials(items, 2_000_000);
    expect(f.delivered).toBe(510000);
    expect(f.ordered).toBe(130000);
    expect(f.spent).toBe(640000);
    expect(f.quoted).toBe(240000);
    expect(f.remaining).toBe(1_360_000);
    expect(f.projected).toBe(880000);
    expect(f.overBudget).toBe(false);
  });

  it('remaining is null when no budget set', () => {
    const f = financials(items, null);
    expect(f.remaining).toBeNull();
    expect(f.spent).toBe(640000);
    expect(f.overBudget).toBe(false);
  });

  it('flags over budget when spent + quoted exceeds budget', () => {
    const f = financials(items, 700_000);
    expect(f.overBudget).toBe(true); // 640k + 240k = 880k > 700k
  });
});

describe('sectorRollups', () => {
  it('groups committed (ordered+delivered) and quoted by trade, sorted by committed desc', () => {
    const rollups = sectorRollups(items);
    expect(rollups[0]).toEqual({ trade: 'HVAC', committed: 640000, quoted: 0 });
    const glazing = rollups.find(r => r.trade === 'GLAZING');
    expect(glazing).toEqual({ trade: 'GLAZING', committed: 0, quoted: 240000 });
    const carpenter = rollups.find(r => r.trade === 'CARPENTER');
    expect(carpenter).toEqual({ trade: 'CARPENTER', committed: 0, quoted: 0 });
  });

  it('buckets null trade as "Untraded"', () => {
    const rollups = sectorRollups([mk({ qty: 1, rate: 100, trade: null, procurement: 'ordered' })]);
    expect(rollups[0].trade).toBe('Untraded');
    expect(rollups[0].committed).toBe(100);
  });
});
