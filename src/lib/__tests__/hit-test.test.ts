import { describe, it, expect } from 'vitest';
import { hitZone, type Zone } from '../hit-test';

describe('hitZone', () => {
  // Bar width 100. Edge zone is 15% = 15px each side. Middle = 70px.
  it('left edge is resize-start', () => {
    expect(hitZone({ relX: 0, width: 100 })).toBe<Zone>('resize-start');
    expect(hitZone({ relX: 13, width: 100 })).toBe<Zone>('resize-start');
  });
  it('right edge is resize-end', () => {
    expect(hitZone({ relX: 87, width: 100 })).toBe<Zone>('resize-end');
    expect(hitZone({ relX: 100, width: 100 })).toBe<Zone>('resize-end');
  });
  it('middle is move', () => {
    expect(hitZone({ relX: 20, width: 100 })).toBe<Zone>('move');
    expect(hitZone({ relX: 50, width: 100 })).toBe<Zone>('move');
    expect(hitZone({ relX: 80, width: 100 })).toBe<Zone>('move');
  });
  it('narrow bars use a 9px floor for resize handles', () => {
    expect(hitZone({ relX: 5, width: 30 })).toBe<Zone>('resize-start');
    expect(hitZone({ relX: 15, width: 30 })).toBe<Zone>('move');
    expect(hitZone({ relX: 25, width: 30 })).toBe<Zone>('resize-end');
  });
  it('wide bars cap edge zone at 14px so move zone dominates', () => {
    expect(hitZone({ relX: 13, width: 500 })).toBe<Zone>('resize-start');
    expect(hitZone({ relX: 15, width: 500 })).toBe<Zone>('move');
    expect(hitZone({ relX: 485, width: 500 })).toBe<Zone>('move');
    expect(hitZone({ relX: 487, width: 500 })).toBe<Zone>('resize-end');
  });
});
