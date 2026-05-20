import type { Phase, Task, Dependency, NoWorkDay } from './types';

export type Selection =
  | { kind: 'task'; id: number }
  | { kind: 'phase'; id: number }
  | { kind: 'dependency'; id: number }
  | null;

export interface Snapshot {
  phases: Phase[];
  tasks: Task[];
  dependencies: Dependency[];
  noWorkDays: NoWorkDay[];
  selection: Selection;
}

export class UndoStack {
  private past: Snapshot[] = [];
  private future: Snapshot[] = [];

  push(snap: Snapshot): void {
    // Deep-clone so future mutations don't bleed back into history.
    this.past.push(structuredClone(snap));
    this.future = [];
  }

  undo(): Snapshot | null {
    if (this.past.length < 2) return null;
    // The top of the stack is the current state. Drop it, peek at the previous.
    const current = this.past.pop()!;
    this.future.push(current);
    return structuredClone(this.past[this.past.length - 1]);
  }

  redo(): Snapshot | null {
    if (this.future.length === 0) return null;
    const next = this.future.pop()!;
    this.past.push(next);
    return structuredClone(next);
  }

  canUndo(): boolean { return this.past.length >= 2; }
  canRedo(): boolean { return this.future.length > 0; }

  clear(): void {
    this.past = [];
    this.future = [];
  }
}
