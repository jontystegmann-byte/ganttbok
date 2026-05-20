import { describe, it, expect } from 'vitest';
import { hitZone, type Zone } from '../hit-test';

describe('hitZone', () => {
  // Bar width 100. Edge zone is 10% = 10px each side. Middle = 80px.
  it('left 10% is resize-start', () => {
    expect(hitZone({ relX: 0, width: 100 })).toBe<Zone>('resize-start');
    expect(hitZone({ relX: 9, width: 100 })).toBe<Zone>('resize-start');
  });
  it('right 10% is resize-end', () => {
    expect(hitZone({ relX: 91, width: 100 })).toBe<Zone>('resize-end');
    expect(hitZone({ relX: 100, width: 100 })).toBe<Zone>('resize-end');
  });
  it('middle 80% is move', () => {
    expect(hitZone({ relX: 10, width: 100 })).toBe<Zone>('move');
    expect(hitZone({ relX: 50, width: 100 })).toBe<Zone>('move');
    expect(hitZone({ relX: 89, width: 100 })).toBe<Zone>('move');
  });
  it('narrow bars cap edge zone at 8px so move zone stays usable', () => {
    expect(hitZone({ relX: 7, width: 20 })).toBe<Zone>('move');
    expect(hitZone({ relX: 3, width: 20 })).toBe<Zone>('resize-start');
  });
});
