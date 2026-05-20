import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../ipc', () => ({
  startupInfo:    vi.fn(async () => ({ clean_shutdown: true, last_open_job_id: null, last_save_at: null, sidebar_width: null })),
  listJobs:       vi.fn(async () => []),
  listTemplates:  vi.fn(async () => []),
  listPhases:     vi.fn(async () => []),
  listTasks:      vi.fn(async () => []),
  listDependencies: vi.fn(async () => []),
  listNoWorkDays:   vi.fn(async () => []),
  getJob:         vi.fn(),
  setLastOpenJob: vi.fn(async () => {}),
}));

import { state } from '../store.svelte';

describe('Store', () => {
  beforeEach(() => {
    state.tasks = [];
    state.selection = null;
  });

  it('applyDragResult patches tasks by id', () => {
    state.tasks = [
      { id: 1, phase_id: 1, name: 'A', start_date: '2026-06-08', duration_workdays: 1, order_index: 0, notes: null },
      { id: 2, phase_id: 1, name: 'B', start_date: '2026-06-09', duration_workdays: 1, order_index: 1, notes: null },
    ];
    state.applyDragResult([
      { id: 1, phase_id: 1, name: 'A', start_date: '2026-06-10', duration_workdays: 1, order_index: 0, notes: null },
    ]);
    expect(state.tasks[0].start_date).toBe('2026-06-10');
    expect(state.tasks[1].start_date).toBe('2026-06-09'); // untouched
  });

  it('select stores the selection', () => {
    state.select({ kind: 'task', id: 42 });
    expect(state.selection).toEqual({ kind: 'task', id: 42 });
  });
});
