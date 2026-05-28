import { describe, it, expect } from 'vitest';
import { pxDeltaToColsMoved, computeGhostDate } from '../drag-physics';
import type { ViewportDay } from '../../calendar';

const mkDays = (isos: string[]): ViewportDay[] =>
  isos.map(iso => ({ date: iso, weekday: 'M', dayOfMonth: 1, projectWeekNumber: 1, isWeekend: false }));

describe('pxDeltaToColsMoved', () => {
  it('rounds half-up at the cell midpoint', () => {
    expect(pxDeltaToColsMoved(0,  24)).toBe(0);
    expect(pxDeltaToColsMoved(11, 24)).toBe(0);
    expect(pxDeltaToColsMoved(12, 24)).toBe(1);
    expect(pxDeltaToColsMoved(-12, 24)).toBe(-1);
    expect(pxDeltaToColsMoved(36, 24)).toBe(2);
  });
  it('returns 0 for degenerate cellW', () => {
    expect(pxDeltaToColsMoved(100, 0)).toBe(0);
    expect(pxDeltaToColsMoved(100, -1)).toBe(0);
  });
});

describe('computeGhostDate', () => {
  // 5 weekday cols: Mon..Fri 8-12 Jun 2026
  const days = mkDays(['2026-06-08', '2026-06-09', '2026-06-10', '2026-06-11', '2026-06-12']);
  const noWork = new Set<string>();

  it('returns originalStart when delta is zero', () => {
    const ghost = computeGhostDate({
      originalStart: '2026-06-10', pxDelta: 0, cellW: 24, days, noWorkSet: noWork, includeWeekends: false,
    });
    expect(ghost).toBe('2026-06-10');
  });

  it('moves one viewport column right', () => {
    const ghost = computeGhostDate({
      originalStart: '2026-06-10', pxDelta: 24, cellW: 24, days, noWorkSet: noWork, includeWeekends: false,
    });
    expect(ghost).toBe('2026-06-11');
  });

  it('clamps when delta would push past the viewport edge', () => {
    const ghost = computeGhostDate({
      originalStart: '2026-06-10', pxDelta: 10_000, cellW: 24, days, noWorkSet: noWork, includeWeekends: false,
    });
    expect(ghost).toBe('2026-06-12');
  });

  it('skips weekend cells when weekends are visible', () => {
    // Days list includes Sat/Sun visually; landing on a weekend snaps to nearest workable.
    const daysWithWknd = mkDays([
      '2026-06-12', // Fri
      '2026-06-13', // Sat
      '2026-06-14', // Sun
      '2026-06-15', // Mon
    ]);
    // pxDelta = 2*24 = 48 from Fri → 2 columns right = Sun. Snap nearest workable = Mon (1 fwd vs Fri 2 back).
    const ghost = computeGhostDate({
      originalStart: '2026-06-12', pxDelta: 48, cellW: 24, days: daysWithWknd, noWorkSet: noWork, includeWeekends: true,
    });
    expect(ghost).toBe('2026-06-15');
  });

  it('skips a no-work day (e.g. ZA Freedom Day 27 Apr 2026)', () => {
    // Mon 27 Apr 2026 is in noWorkSet. Drag from Fri 24 Apr by 1 column right → 27 Apr → snap fwd to Tue 28 Apr.
    const aprDays = mkDays([
      '2026-04-24', // Fri
      '2026-04-27', // Mon (Freedom Day)
      '2026-04-28', // Tue
      '2026-04-29', // Wed
    ]);
    const za = new Set(['2026-04-27']);
    const ghost = computeGhostDate({
      originalStart: '2026-04-24', pxDelta: 24, cellW: 24, days: aprDays, noWorkSet: za, includeWeekends: false,
    });
    expect(ghost).toBe('2026-04-28');
  });

  it('returns originalStart when originalStart is not in the viewport', () => {
    const ghost = computeGhostDate({
      originalStart: '2025-01-01', pxDelta: 24, cellW: 24, days, noWorkSet: noWork, includeWeekends: false,
    });
    expect(ghost).toBe('2025-01-01');
  });
});
