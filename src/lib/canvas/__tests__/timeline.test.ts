import { describe, it, expect } from 'vitest';
import { pxToDate, dateToPx, pxToDayIndex } from '../timeline';
import type { ViewportDay } from '../../calendar';

const days: ViewportDay[] = [
  { date: '2026-06-08', weekday: 'M', dayOfMonth: 8,  projectWeekNumber: 1, isWeekend: false },
  { date: '2026-06-09', weekday: 'T', dayOfMonth: 9,  projectWeekNumber: 1, isWeekend: false },
  { date: '2026-06-10', weekday: 'W', dayOfMonth: 10, projectWeekNumber: 1, isWeekend: false },
  { date: '2026-06-11', weekday: 'T', dayOfMonth: 11, projectWeekNumber: 1, isWeekend: false },
  { date: '2026-06-12', weekday: 'F', dayOfMonth: 12, projectWeekNumber: 1, isWeekend: false },
];

describe('pxToDayIndex', () => {
  it('clamps to 0 below the timeline', () => {
    expect(pxToDayIndex(-50, 24, days.length)).toBe(0);
  });
  it('clamps to last index above the timeline', () => {
    expect(pxToDayIndex(10_000, 24, days.length)).toBe(4);
  });
  it('returns floor of px/cellW within range', () => {
    expect(pxToDayIndex(0, 24, days.length)).toBe(0);
    expect(pxToDayIndex(23, 24, days.length)).toBe(0);
    expect(pxToDayIndex(24, 24, days.length)).toBe(1);
    expect(pxToDayIndex(47, 24, days.length)).toBe(1);
    expect(pxToDayIndex(48, 24, days.length)).toBe(2);
  });
});

describe('pxToDate', () => {
  it('returns the date at a column boundary', () => {
    expect(pxToDate(0,  24, days)).toBe('2026-06-08');
    expect(pxToDate(24, 24, days)).toBe('2026-06-09');
    expect(pxToDate(96, 24, days)).toBe('2026-06-12');
  });
  it('returns the date at a column midpoint (same column)', () => {
    expect(pxToDate(12, 24, days)).toBe('2026-06-08');
  });
  it('clamps to the last day when past the timeline', () => {
    expect(pxToDate(10_000, 24, days)).toBe('2026-06-12');
  });
});

describe('dateToPx', () => {
  it('returns the column left-edge px for a date present in the viewport', () => {
    expect(dateToPx('2026-06-08', 24, days)).toBe(0);
    expect(dateToPx('2026-06-10', 24, days)).toBe(48);
    expect(dateToPx('2026-06-12', 24, days)).toBe(96);
  });
  it('returns -1 for a date not in the viewport', () => {
    // Sat 2026-06-13 is not in this weekends-excluded viewport.
    expect(dateToPx('2026-06-13', 24, days)).toBe(-1);
  });
});

describe('pxToDate + dateToPx round-trip', () => {
  it('round-trips every column at cellW=24', () => {
    for (const day of days) {
      const x = dateToPx(day.date, 24, days);
      expect(pxToDate(x, 24, days)).toBe(day.date);
    }
  });
  it('round-trips every column at cellW=12 (zoomed-out)', () => {
    for (const day of days) {
      const x = dateToPx(day.date, 12, days);
      expect(pxToDate(x, 12, days)).toBe(day.date);
    }
  });
});
