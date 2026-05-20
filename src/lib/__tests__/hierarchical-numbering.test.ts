import { describe, it, expect } from 'vitest';
import type { Phase, Task } from '../types';

// The rule: phase[i] → "i+1.", task[i] in phase[j] → "j+1.i+1"
// (computed in template, but we test the algorithm here for clarity)
function phaseLabel(orderIndex: number) { return `${orderIndex + 1}.`; }
function taskLabel(phaseIndex: number, taskIndex: number) {
  return `${phaseIndex + 1}.${taskIndex + 1}`;
}

describe('hierarchical numbering', () => {
  it('phases number 1, 2, 3, ... by order_index', () => {
    expect(phaseLabel(0)).toBe('1.');
    expect(phaseLabel(1)).toBe('2.');
    expect(phaseLabel(2)).toBe('3.');
  });

  it('tasks number 1.1, 1.2, 2.1, 2.2 ...', () => {
    expect(taskLabel(0, 0)).toBe('1.1');
    expect(taskLabel(0, 1)).toBe('1.2');
    expect(taskLabel(1, 0)).toBe('2.1');
  });

  it('numbering does not depend on id, only order_index', () => {
    const phases: Phase[] = [
      { id: 99, job_id: 1, name: 'P1', colour: '#000', order_index: 0, collapsed: false },
      { id:  1, job_id: 1, name: 'P2', colour: '#000', order_index: 1, collapsed: false },
    ];
    expect(phaseLabel(phases[0].order_index)).toBe('1.');
    expect(phaseLabel(phases[1].order_index)).toBe('2.');
  });
});
