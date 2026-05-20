import { describe, it, expect } from 'vitest';
import { magneticSnap } from '../snap';

describe('magneticSnap', () => {
  it('exact cell boundary stays put', () => {
    expect(magneticSnap({ pxDelta: 0, cellW: 24 })).toBe(0);
    expect(magneticSnap({ pxDelta: 24, cellW: 24 })).toBe(24);
    expect(magneticSnap({ pxDelta: -24, cellW: 24 })).toBe(-24);
  });
  it('within 30% pull snaps to nearest cell', () => {
    // 4px into a 24px cell = ~17% — pulls back to 0.
    expect(magneticSnap({ pxDelta: 4, cellW: 24 })).toBe(0);
    // 20px = 83% (closer to next cell). Pulls forward to 24.
    expect(magneticSnap({ pxDelta: 20, cellW: 24 })).toBe(24);
  });
  it('within free zone tracks pointer faithfully', () => {
    // 8px = 33% — outside hard pull, but eased toward 0 a bit.
    const result = magneticSnap({ pxDelta: 8, cellW: 24 });
    expect(result).toBeGreaterThanOrEqual(0);
    expect(result).toBeLessThanOrEqual(8);
  });
});
