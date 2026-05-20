/**
 * Magnetic snap. Returns the rendered pixel position given a pointer delta in pixels.
 * - <20% into a cell: pulls hard to the nearest day-edge (snap).
 * - 20–50%: eased pull (free-with-bias).
 * - >50%: no pull (faithful tracking).
 *
 * Pure function — no DOM.
 */
export function magneticSnap({ pxDelta, cellW }: { pxDelta: number; cellW: number }): number {
  if (cellW <= 0) return pxDelta;
  const cells = pxDelta / cellW;
  const nearest = Math.round(cells);
  const fractional = cells - nearest; // -0.5 .. 0.5
  const absFrac = Math.abs(fractional);
  let pull: number;
  if (absFrac < 0.2) pull = 1.0;
  else if (absFrac > 0.5) pull = 0.0;
  else {
    pull = 1.0 - (absFrac - 0.2) / 0.3;
  }
  const snappedFrac = fractional * (1 - pull);
  return (nearest + snappedFrac) * cellW;
}
