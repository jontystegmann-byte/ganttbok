import * as ipc from './ipc';
import type { Job, Phase, Task, Dependency, NoWorkDay } from './types';
import type { Zone } from './hit-test';

export interface DragState {
  taskId: number;
  zone: Zone;
  startX: number;
  originalStart: string;
  originalDuration: number;
  liveDelta: number;
}

export type Selection =
  | { kind: 'task'; id: number }
  | { kind: 'phase'; id: number }
  | { kind: 'dependency'; id: number }
  | null;

class Store {
  // Top-level reactive state
  jobs       = $state<Job[]>([]);
  templates  = $state<Job[]>([]);
  currentJob = $state<Job | null>(null);

  phases       = $state<Phase[]>([]);
  tasks        = $state<Task[]>([]);
  dependencies = $state<Dependency[]>([]);
  noWorkDays   = $state<NoWorkDay[]>([]);

  selection     = $state<Selection>(null);
  sidebarWidth  = $state<number>(240);
  showNewJobModal = $state<boolean>(false);
  archivedJobs = $state<Job[]>([]);
  hoveredTaskId = $state<number | null>(null);
  dragState     = $state<DragState | null>(null);

  cancelDrag(): void {
    this.dragState = null;
  }

  async refreshArchived(): Promise<void> {
    // Backend doesn't have a list_archived command; fetch all by toggling and use a generic.
    // For Plan 2 we fake it by calling list_jobs with a future extension. Until backend exposes it,
    // archived stays empty. (Backend extension is a 5-line task scheduled for Plan 3.)
    this.archivedJobs = [];
  }

  async createJob(args: { name: string; client: string | null; address: string | null; project_start_date: string; }): Promise<void> {
    const job = await ipc.createJob({ ...args, is_template: false });
    await this.refreshSidebar();
    await this.openJob(job.id);
    this.showNewJobModal = false;
  }

  // Derived helpers
  tasksByPhase = $derived.by(() => {
    const m = new Map<number, Task[]>();
    for (const t of this.tasks) {
      const list = m.get(t.phase_id) ?? [];
      list.push(t);
      m.set(t.phase_id, list);
    }
    for (const list of m.values()) list.sort((a, b) => a.order_index - b.order_index);
    return m;
  });

  // Bootstrap: load app meta + jobs at startup.
  async bootstrap(): Promise<void> {
    const meta = await ipc.startupInfo();
    if (meta.sidebar_width) this.sidebarWidth = meta.sidebar_width;
    await this.refreshSidebar();
    if (meta.last_open_job_id) {
      try { await this.openJob(meta.last_open_job_id); }
      catch { /* job may have been deleted */ }
    }
  }

  async refreshSidebar(): Promise<void> {
    this.jobs       = await ipc.listJobs();
    this.templates  = await ipc.listTemplates();
  }

  async openJob(jobId: number): Promise<void> {
    this.currentJob   = await ipc.getJob(jobId);
    if (!this.currentJob.is_template) {
      const start = this.currentJob.project_start_date;
      const startDate = new Date(start);
      const end = new Date(startDate);
      end.setMonth(end.getMonth() + 18);
      await ipc.syncSaHolidays({
        job_id: jobId,
        from: start,
        to: end.toISOString().slice(0, 10),
      });
    }
    this.phases       = await ipc.listPhases(jobId);
    this.tasks        = await ipc.listTasks(jobId);
    this.dependencies = await ipc.listDependencies(jobId);
    this.noWorkDays   = await ipc.listNoWorkDays(jobId);
    this.selection    = null;
    await ipc.setLastOpenJob(jobId);
  }

  select(s: Selection): void {
    this.selection = s;
  }

  async reorderTasksInPhase(phaseId: number, orderedIds: number[]): Promise<void> {
    await ipc.reorderTasks(phaseId, orderedIds);
    const idx = new Map(orderedIds.map((id, i) => [id, i]));
    this.tasks = this.tasks.map(t => t.phase_id === phaseId ? { ...t, order_index: idx.get(t.id) ?? t.order_index } : t);
  }

  async reorderPhases(orderedIds: number[]): Promise<void> {
    if (!this.currentJob) return;
    await ipc.reorderPhases(this.currentJob.id, orderedIds);
    const idx = new Map(orderedIds.map((id, i) => [id, i]));
    this.phases = this.phases.map(p => ({ ...p, order_index: idx.get(p.id) ?? p.order_index }))
                              .sort((a, b) => a.order_index - b.order_index);
  }

  async createPhase(name: string): Promise<void> {
    if (!this.currentJob) return;
    const palette = ['#3B82F6', '#EF4444', '#10B981', '#F59E0B', '#8B5CF6', '#EC4899', '#14B8A6'];
    const colour = palette[this.phases.length % palette.length];
    const phase = await ipc.createPhase({ job_id: this.currentJob.id, name, colour });
    this.phases = [...this.phases, phase].sort((a, b) => a.order_index - b.order_index);
  }

  async createTaskInPhase(phaseId: number, name: string): Promise<void> {
    if (!this.currentJob) return;
    const start = this.currentJob.project_start_date;
    const task = await ipc.createTask({
      phase_id: phaseId, name, start_date: start, duration_workdays: 3,
    });
    this.tasks = [...this.tasks, task];
  }

  // Optimistic local update applied after an IPC mutation returns updated rows.
  applyDragResult(updated: Task[]): void {
    const byId = new Map(updated.map(t => [t.id, t]));
    this.tasks = this.tasks.map(t => byId.get(t.id) ?? t);
  }
}

export const state = new Store();
