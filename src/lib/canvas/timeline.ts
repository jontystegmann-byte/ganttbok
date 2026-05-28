import type { ViewportDay } from '../calendar';

/**
 * Pure pixel ↔ date conversions driven by the already-rendered ViewportDay[].
 * The viewport list is the source of truth for which dates have columns; this
 * module never makes calendar decisions of its own.
 */

/** Clamped floor(px/cellW). Returns an index into `days` (0..days.length-1). */
export function pxToDayIndex(px: number, cellW: number, dayCount: number): number {
  if (dayCount === 0) return 0;
  if (cellW <= 0) return 0;
  const raw = Math.floor(px / cellW);
  if (raw < 0) return 0;
  if (raw >= dayCount) return dayCount - 1;
  return raw;
}

/** ISO date of the column under `px`. */
export function pxToDate(px: number, cellW: number, days: ViewportDay[]): string {
  if (days.length === 0) return '';
  const idx = pxToDayIndex(px, cellW, days.length);
  return days[idx].date;
}

/**
 * Left-edge pixel X of the column rendering `iso`. Returns -1 if the date is
 * not currently in the viewport (e.g. weekend with weekends hidden).
 */
export function dateToPx(iso: string, cellW: number, days: ViewportDay[]): number {
  const idx = days.findIndex(d => d.date === iso);
  if (idx === -1) return -1;
  return idx * cellW;
}
