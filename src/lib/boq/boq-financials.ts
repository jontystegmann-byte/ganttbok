import type { BoqItem } from '../types';
import { cost } from './boq-grid';

export interface Financials {
  delivered: number;   // Σ cost where procurement = delivered
  ordered: number;     // Σ cost where procurement = ordered
  spent: number;       // delivered + ordered (money that left the account)
  quoted: number;      // Σ cost where procurement = quoted (provisional)
  remaining: number | null; // budget - spent, or null when no budget
  projected: number;   // spent + quoted
  overBudget: boolean; // budget set AND spent + quoted > budget
}

export interface SectorRollup {
  trade: string;
  committed: number; // ordered + delivered within the trade
  quoted: number;
}

function sumWhere(items: BoqItem[], pred: (it: BoqItem) => boolean): number {
  return items.reduce((acc, it) => pred(it) ? acc + (cost(it) ?? 0) : acc, 0);
}

export function financials(items: BoqItem[], budget: number | null): Financials {
  const delivered = sumWhere(items, it => it.procurement === 'delivered');
  const ordered = sumWhere(items, it => it.procurement === 'ordered');
  const spent = delivered + ordered;
  const quoted = sumWhere(items, it => it.procurement === 'quoted');
  const projected = spent + quoted;
  return {
    delivered, ordered, spent, quoted, projected,
    remaining: budget == null ? null : budget - spent,
    overBudget: budget != null && projected > budget,
  };
}

export function sectorRollups(items: BoqItem[]): SectorRollup[] {
  const map = new Map<string, SectorRollup>();
  for (const it of items) {
    const trade = it.trade ?? 'Untraded';
    const r = map.get(trade) ?? { trade, committed: 0, quoted: 0 };
    const c = cost(it) ?? 0;
    if (it.procurement === 'ordered' || it.procurement === 'delivered') r.committed += c;
    else if (it.procurement === 'quoted') r.quoted += c;
    map.set(trade, r);
  }
  return [...map.values()].sort((a, b) => b.committed - a.committed || b.quoted - a.quoted);
}
