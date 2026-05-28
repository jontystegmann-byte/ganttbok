import type { ViewportDay } from '../calendar';
import { snapToNearestWorkable } from '../calendar';

/** Round a raw pointer-pixel delta to a signed integer column count.
 * Uses round-half-away-from-zero so +12px and -12px both yield ±1 column. */
export function pxDeltaToColsMoved(pxDelta: number, cellW: number): number {
  if (cellW <= 0) return 0;
  const raw = pxDelta / cellW;
  return Math.sign(raw) * Math.round(Math.abs(raw));
}

export interface ComputeGhostArgs {
  originalStart: string;            // ISO date of the bar's leading edge at drag start
  pxDelta: number;                  // raw pointer delta in pixels since pointerdown
  cellW: number;                    // px per viewport column
  days: ViewportDay[];              // the rendered viewport
  noWorkSet: Set<string>;           // effective no-work dates (post-toggle)
  includeWeekends: boolean;
}

/**
 * Pure: given a bar's original ISO start + a pixel delta, return the ISO date the
 * bar will commit to if the user releases now. Walks the viewport's rendered
 * `days` by the column delta, then snaps to the nearest workable date.
 *
 * Returns originalStart unchanged when the bar's start is outside the viewport
 * (defensive — caller doesn't have to special-case).
 */
export function computeGhostDate(args: ComputeGhostArgs): string {
  const { originalStart, pxDelta, cellW, days, noWorkSet, includeWeekends } = args;
  if (days.length === 0) return originalStart;
  const idxAtStart = days.findIndex(d => d.date === originalStart);
  if (idxAtStart === -1) return originalStart;
  const colsMoved = pxDeltaToColsMoved(pxDelta, cellW);
  const candidateIdx = Math.max(0, Math.min(days.length - 1, idxAtStart + colsMoved));
  const candidate = days[candidateIdx].date;
  // Tasks never start on a weekend day, even when weekend columns are visible.
  // Pass includeWeekends=false so weekends are always treated as non-workable for placement.
  return snapToNearestWorkable(candidate, noWorkSet, false);
}
