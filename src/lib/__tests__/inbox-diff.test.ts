import { describe, it, expect } from 'vitest';
import { renderPatchOp } from '../inbox-diff';
import type { PatchOp, Phase, Task, Contact } from '../types';

const phases: Phase[] = [
  { id: 1, job_id: 1, name: 'Foundation', colour: '#3B82F6', order_index: 0, collapsed: false, notes: '' },
];
const tasks: Task[] = [
  { id: 10, phase_id: 1, name: 'Order windows', start_date: '2026-06-08',
    duration_workdays: 5, order_index: 0, notes: null, contact_id: null, last_chaser_sent_at: null },
];
const contacts: Contact[] = [
  { id: 100, name: 'Doug Supplies', telegram_chat_id: null, telegram_handle: null,
    notes: '', created_at: '2026-05-01' },
];

describe('renderPatchOp', () => {
  it('renders add_task', () => {
    const op: PatchOp = {
      op: 'add_task', phase_id: 1, name: 'Order vent ducting',
      start_date: '2026-06-10', duration_workdays: 3,
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order vent ducting');
    expect(line).toContain('Foundation');
    expect(line).toContain('2026-06-10');
  });

  it('renders shift_task with positive delta', () => {
    const op: PatchOp = { op: 'shift_task', task_id: 10, by_days: 7 };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
    expect(line).toContain('+7');
  });

  it('renders shift_task with negative delta', () => {
    const op: PatchOp = { op: 'shift_task', task_id: 10, by_days: -3 };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
    expect(line).toContain('-3');
  });

  it('renders add_dependency with known task names', () => {
    const op: PatchOp = {
      op: 'add_dependency',
      predecessor: { task_id: 10 },
      successor: { task_id: 10 },
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
  });

  it('renders add_chaser with contact name', () => {
    const op: PatchOp = {
      op: 'add_chaser', task_id: 10, contact_id: 100, template: 'manual',
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Order windows');
    expect(line).toContain('Doug Supplies');
  });

  it('renders append_note', () => {
    const op: PatchOp = { op: 'append_note', job_id: 1, text: 'Graham wants fewer cavity walls' };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Graham wants fewer cavity walls');
  });

  it('falls back gracefully for unknown task_id', () => {
    const op: PatchOp = { op: 'shift_task', task_id: 9999, by_days: 1 };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('task #9999');
  });

  it('falls back gracefully for unknown phase_id', () => {
    const op: PatchOp = {
      op: 'add_task', phase_id: 9999, name: 'Mystery', start_date: '2026-06-10', duration_workdays: 1,
    };
    const line = renderPatchOp(op, phases, tasks, contacts);
    expect(line).toContain('Mystery');
    expect(line).toContain('phase #9999');
  });
});
