import { describe, it, expect, beforeEach } from 'vitest';
import { UndoStack, type Snapshot } from '../undo';

function snap(phases: any[] = [], tasks: any[] = []): Snapshot {
  return { phases, tasks, dependencies: [], noWorkDays: [], selection: null };
}

describe('UndoStack', () => {
  let stack: UndoStack;
  beforeEach(() => { stack = new UndoStack(); });

  it('starts with no undo/redo available', () => {
    expect(stack.canUndo()).toBe(false);
    expect(stack.canRedo()).toBe(false);
  });

  it('push records snapshot; undo returns the previous one', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([{ id: 1 }, { id: 2 }]));
    expect(stack.canUndo()).toBe(true);
    const prev = stack.undo();
    expect(prev?.phases.length).toBe(1);
  });

  it('redo restores what was just undone', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([{ id: 1 }, { id: 2 }]));
    stack.undo();
    expect(stack.canRedo()).toBe(true);
    const restored = stack.redo();
    expect(restored?.phases.length).toBe(2);
  });

  it('push after undo clears redo stack', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([{ id: 1 }, { id: 2 }]));
    stack.undo();
    stack.push(snap([{ id: 1 }, { id: 3 }]));
    expect(stack.canRedo()).toBe(false);
  });

  it('clear empties both stacks', () => {
    stack.push(snap([{ id: 1 }]));
    stack.push(snap([]));
    stack.undo();
    stack.clear();
    expect(stack.canUndo()).toBe(false);
    expect(stack.canRedo()).toBe(false);
  });
});
