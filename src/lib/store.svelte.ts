import * as ipc from './ipc';
import type { Job, Phase, Task, Dependency, NoWorkDay, Contact, PendingPatch, TaskStatus, OverdueReview, BoqItem, Procurement } from './types';
import type { Zone } from './hit-test';
import { UndoStack, type Snapshot as UndoSnapshot } from './undo';

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

  private undoStack = new UndoStack();
  hasUnsavedUndo = $state<boolean>(false);
  private resyncTimer: number | null = null;

  // Print
  showPrintOptions = $state<boolean>(false);
  printScaling     = $state<'fit' | 'multi'>('fit');
  printShowNotes   = $state<boolean>(false);

  // User prefs
  durationUnit = $state<'weeks' | 'days'>('weeks');
  holidaysBlockWorkDefault = $state<boolean>(true);
  includeWeekends = $state<boolean>(false);
  uiScale = $state<number>(1);
  hoveredDayIndex = $state<number | null>(null);
  todayIso = $state<string>(new Date().toISOString().slice(0, 10));
  regionDefault = $state<string>('ZA');

  // Chaser
  contacts = $state<Contact[]>([]);

  // Inbox — proposed patches from MCP / external sources
  inboxPatches    = $state<PendingPatch[]>([]);
  private inboxPollTimer: number | null = null;

  // Exactly one right-hand-side tool panel may be open at a time.
  // Toggling a tool with itself closes it; picking another tool replaces it.
  activeTool = $state<'inbox' | 'notes' | 'contacts' | 'settings' | null>(null);
  toggleTool(tool: 'inbox' | 'notes' | 'contacts' | 'settings'): void {
    this.activeTool = this.activeTool === tool ? null : tool;
  }

  // Top-level view: the schedule (Gantt) or the Bill of Quantities. Co-equal pages.
  activeView = $state<'schedule' | 'boq'>('schedule');
  setView(view: 'schedule' | 'boq'): void { this.activeView = view; }

  // Bill of Quantities line items for the open job.
  boqItems = $state<BoqItem[]>([]);
  boqBudget = $state<number | null>(null);
  showBoqFinancials = $state<boolean>(false);

  async toggleDurationUnit(): Promise<void> {
    this.durationUnit = this.durationUnit === 'weeks' ? 'days' : 'weeks';
    await ipc.setDurationUnit(this.durationUnit);
  }

  async setIncludeWeekends(value: boolean): Promise<void> {
    this.includeWeekends = value;
    await ipc.setIncludeWeekends(value);
  }

  async setUiScale(value: number): Promise<void> {
    this.uiScale = value;
    document.documentElement.style.setProperty('--ui-scale', String(value));
    await ipc.setUiScale(value);
  }

  async refreshContacts(): Promise<void> {
    this.contacts = await ipc.listContacts();
  }

  async refreshBoqItems(): Promise<void> {
    if (!this.currentJob) { this.boqItems = []; return; }
    this.boqItems = await ipc.listBoqItems(this.currentJob.id);
  }

  async refreshBoqBudget(): Promise<void> {
    if (!this.currentJob) { this.boqBudget = null; return; }
    this.boqBudget = await ipc.getJobBudget(this.currentJob.id);
  }

  async setBoqBudget(budget: number | null): Promise<void> {
    if (!this.currentJob) return;
    await ipc.setJobBudget(this.currentJob.id, budget);
    this.boqBudget = budget;
    await ipc.touchLastSave();
  }

  async createBoqItem(): Promise<void> {
    if (!this.currentJob) return;
    const created = await ipc.createBoqItem(this.currentJob.id);
    this.boqItems = [...this.boqItems, created];
    await ipc.touchLastSave();
  }

  async updateBoqItem(item: BoqItem): Promise<void> {
    await ipc.updateBoqItem($state.snapshot(item));
    this.boqItems = this.boqItems.map(b => b.id === item.id ? { ...item } : b);
    await ipc.touchLastSave();
  }

  async setBoqProcurement(id: number, procurement: Procurement, deliveredDate: string | null): Promise<void> {
    await ipc.setBoqProcurement({ id, procurement, delivered_date: deliveredDate });
    // Backend owns delivered_date: it stores it only when procurement === 'delivered'.
    const resolved = procurement === 'delivered'
      ? (deliveredDate ?? this.todayIso)
      : null;
    this.boqItems = this.boqItems.map(b =>
      b.id === id ? { ...b, procurement, delivered_date: resolved } : b);
    await ipc.touchLastSave();
  }

  async deleteBoqItem(id: number): Promise<void> {
    await ipc.deleteBoqItem(id);
    this.boqItems = this.boqItems.filter(b => b.id !== id);
    await ipc.touchLastSave();
  }

  async createContact(args: { name: string; telegram_chat_id: string | null; telegram_handle: string | null; notes: string }): Promise<Contact> {
    const created = await ipc.createContact(args);
    await this.refreshContacts();
    return created;
  }

  async updateContact(c: Contact): Promise<void> {
    await ipc.updateContact({
      id: c.id, name: c.name,
      telegram_chat_id: c.telegram_chat_id,
      telegram_handle: c.telegram_handle,
      notes: c.notes,
    });
    await this.refreshContacts();
  }

  async deleteContact(id: number): Promise<void> {
    await ipc.deleteContact(id);
    await this.refreshContacts();
    // Clear contact_id on any tasks in memory that referenced it
    this.tasks = this.tasks.map(t => t.contact_id === id ? { ...t, contact_id: null } : t);
  }

  async assignTaskContact(task_id: number, contact_id: number | null): Promise<void> {
    await ipc.assignTaskContact({ task_id, contact_id });
    this.tasks = this.tasks.map(t => t.id === task_id ? { ...t, contact_id } : t);
  }

  /** Run the auto-nudge sweep — fired on launch + on focus. Surfaces results via toast. */
  async runChaserCheck(): Promise<void> {
    try {
      const results = await ipc.runChaserCheck();
      for (const r of results) {
        if (r.success) {
          // Best-effort toast; if Toast component isn't ready it's a no-op
          (window as unknown as { __toast?: (msg: string) => void }).__toast?.(
            `Pinged ${r.contact_name} about "${r.task_name}"`
          );
        }
      }
    } catch (e) {
      console.warn('chaser check failed', e);
    }
  }

  /** Poll interval for the Inbox. 5 seconds while the window is open. */
  static readonly INBOX_POLL_MS = 5000;

  async refreshInbox(): Promise<void> {
    try {
      this.inboxPatches = await ipc.listPendingPatches('proposed');
    } catch (e) {
      console.warn('inbox refresh failed', e);
    }
  }

  startInboxPoll(): void {
    if (this.inboxPollTimer !== null) return;
    this.inboxPollTimer = window.setInterval(
      () => this.refreshInbox(),
      Store.INBOX_POLL_MS,
    );
  }

  stopInboxPoll(): void {
    if (this.inboxPollTimer !== null) {
      clearInterval(this.inboxPollTimer);
      this.inboxPollTimer = null;
    }
  }

  async acceptInboxPatch(id: string): Promise<void> {
    await ipc.acceptPatch(id);
    await this.refreshInbox();
    // Re-load the current job to reflect the applied changes.
    if (this.currentJob) {
      await this.openJob(this.currentJob.id);
    }
  }

  async rejectInboxPatch(id: string): Promise<void> {
    await ipc.rejectPatch(id);
    await this.refreshInbox();
  }

  async clearResolvedPatches(): Promise<void> {
    await ipc.clearResolvedPatches();
    await this.refreshInbox();
  }

  async setCurrentJobRegion(region: string): Promise<void> {
    if (!this.currentJob) return;
    if (region === this.currentJob.region) return;
    this.currentJob.region = region;
    await ipc.updateJob($state.snapshot(this.currentJob));
    // Re-sync holidays for the new region across the project window.
    const from = this.currentJob.project_start_date;
    const startDate = new Date(from);
    const end = new Date(startDate);
    end.setMonth(end.getMonth() + 18);
    await ipc.syncHolidays({
      job_id: this.currentJob.id,
      region,
      from,
      to: end.toISOString().slice(0, 10),
    });
    this.noWorkDays = await ipc.listNoWorkDays(this.currentJob.id);
    // Save as new default for future jobs.
    this.regionDefault = region;
    await ipc.setRegionDefault(region);
  }

  // Dep creation gesture
  depCreator = $state<{
    fromTaskId: number;
    fromX: number;
    fromY: number;
    mouseX: number;
    mouseY: number;
    hoverTaskId: number | null;
  } | null>(null);

  cancelDrag(): void {
    this.dragState = null;
  }

  /** Snapshot current job state into the undo stack. Called after every mutation. */
  recordHistory(): void {
    this.undoStack.push({
      phases: $state.snapshot(this.phases),
      tasks: $state.snapshot(this.tasks),
      dependencies: $state.snapshot(this.dependencies),
      noWorkDays: $state.snapshot(this.noWorkDays),
      selection: $state.snapshot(this.selection),
    });
  }

  canUndo(): boolean { return this.undoStack.canUndo(); }
  canRedo(): boolean { return this.undoStack.canRedo(); }

  undo(): void {
    const snap = this.undoStack.undo();
    if (snap) { this.applySnapshot(snap); this.scheduleResync(); }
  }

  redo(): void {
    const snap = this.undoStack.redo();
    if (snap) { this.applySnapshot(snap); this.scheduleResync(); }
  }

  private applySnapshot(snap: UndoSnapshot): void {
    this.phases       = snap.phases;
    this.tasks        = snap.tasks;
    this.dependencies = snap.dependencies;
    this.noWorkDays   = snap.noWorkDays;
    this.selection    = snap.selection;
    this.hasUnsavedUndo = true;
  }

  private scheduleResync(): void {
    if (this.resyncTimer !== null) clearTimeout(this.resyncTimer);
    this.resyncTimer = window.setTimeout(() => {
      this.resyncTimer = null;
      void this.resyncJobState();
    }, 300);
  }

  async resyncJobState(): Promise<void> {
    if (!this.currentJob) return;
    await ipc.resyncJobState({
      job_id: this.currentJob.id,
      phases: $state.snapshot(this.phases),
      tasks: $state.snapshot(this.tasks),
      dependencies: $state.snapshot(this.dependencies),
      no_work_days: $state.snapshot(this.noWorkDays),
    });
    await ipc.touchLastSave();
    this.hasUnsavedUndo = false;
  }

  mutateAndRecord<T>(fn: () => T): T {
    const result = fn();
    this.recordHistory();
    return result;
  }

  async refreshArchived(): Promise<void> {
    this.archivedJobs = await ipc.listArchived();
  }

  async createJob(args: { name: string; client: string | null; address: string | null; project_start_date: string; }): Promise<void> {
    const job = await ipc.createJob({
      ...args,
      is_template: false,
      holidays_block_work: this.holidaysBlockWorkDefault,
      region: this.regionDefault,
    });
    await this.refreshSidebar();
    await this.openJob(job.id);
    await ipc.touchLastSave();
    this.showNewJobModal = false;
  }

  async setJobHolidaysBlockWork(value: boolean): Promise<void> {
    if (!this.currentJob) return;
    this.currentJob.holidays_block_work = value;
    await ipc.updateJob($state.snapshot(this.currentJob));
    this.holidaysBlockWorkDefault = value;
    await ipc.setHolidaysBlockWorkDefault(value);
  }

  async renameCurrentJob(newName: string): Promise<void> {
    if (!this.currentJob) return;
    const trimmed = newName.trim();
    if (!trimmed || trimmed === this.currentJob.name) return;
    this.currentJob.name = trimmed;
    await ipc.updateJob($state.snapshot(this.currentJob));
    await this.refreshSidebar();
  }

  async setCurrentJobStartDate(newDate: string): Promise<void> {
    if (!this.currentJob) return;
    if (newDate === this.currentJob.project_start_date) return;
    this.currentJob.project_start_date = newDate;
    await ipc.updateJob($state.snapshot(this.currentJob));
  }

  async createFromTemplate(
    templateId: number,
    args: { new_name: string; client: string | null; address: string | null; project_start_date: string },
  ): Promise<void> {
    const job = await ipc.instantiateTemplate({ template_id: templateId, ...args });
    await this.refreshSidebar();
    await this.openJob(job.id);
    await ipc.touchLastSave();
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

  /** Refresh todayIso to the actual current date. Called on bootstrap, focus, and a minute timer. */
  overdueReviews = $state<OverdueReview[]>([]);

  async refreshOverdueReviews(): Promise<void> {
    if (!this.currentJob) { this.overdueReviews = []; return; }
    try {
      this.overdueReviews = await ipc.listOverdueReviews(this.currentJob.id, this.todayIso);
    } catch (e) {
      console.error('refreshOverdueReviews', e);
    }
  }

  /** Daily tick — runs on bootstrap + day rollover.
   *  Backend catch-up extends every Late task in the open job so its bar
   *  reaches today; idempotent, so safe to run multiple times per day. */
  async runLateTasksTick(): Promise<void> {
    if (!this.currentJob) return;
    try {
      const extended = await ipc.tickLateTasks(this.currentJob.id, this.todayIso);
      if (extended > 0) {
        this.tasks = await ipc.listTasks(this.currentJob.id);
      }
    } catch (e) {
      console.error('runLateTasksTick', e);
    }
  }

  /** Mark an overdue task Done on a specific completion date.
   *  Backend adjusts duration and ripples dependents if auto_shift_dependents is on. */
  async resolveOverdueAsDone(taskId: number, completionDate: string): Promise<void> {
    if (!this.currentJob) return;
    const jobId = this.currentJob.id;
    await ipc.markTaskDoneOnDate(jobId, taskId, completionDate);
    this.tasks = await ipc.listTasks(jobId);
    this.recordHistory();
    await this.refreshOverdueReviews();
    await ipc.touchLastSave();
  }

  /** Flag an overdue task as Running Late. Backend catch-up extends the duration
   *  so the bar reaches today, then pushes downstream dependents if auto_shift is on. */
  async resolveOverdueAsRunningLate(taskId: number): Promise<void> {
    if (!this.currentJob) return;
    const jobId = this.currentJob.id;
    await ipc.markTaskRunningLate(jobId, taskId, this.todayIso);
    this.tasks = await ipc.listTasks(jobId);
    this.recordHistory();
    await this.refreshOverdueReviews();
    await ipc.touchLastSave();
  }

  tickToday(): void {
    const iso = new Date().toISOString().slice(0, 10);
    if (iso !== this.todayIso) {
      this.todayIso = iso;
      // Day rolled over: extend any Late bars, then refresh the overdue review list.
      this.runLateTasksTick().then(() => this.refreshOverdueReviews()).catch(() => {});
    }
  }

  /** Scroll the canvas .grid-area so today's column sits ~1 week from the left visible edge. */
  scrollToToday(): void {
    requestAnimationFrame(() => {
      const grid = document.querySelector('.grid-area') as HTMLElement | null;
      if (!grid) return;
      const todayCol = document.querySelector(`.rows`)?.querySelector('.today-line') as HTMLElement | null;
      if (!todayCol) return;
      const x = parseFloat(todayCol.style.left) || 0;
      const CELL = 24;
      grid.scrollLeft = Math.max(0, x - CELL * 5);
    });
  }

  // Bootstrap: load app meta + jobs at startup.
  async bootstrap(): Promise<void> {
    const meta = await ipc.startupInfo();
    if (meta.sidebar_width) this.sidebarWidth = meta.sidebar_width;
    if (meta.duration_unit === 'days' || meta.duration_unit === 'weeks') this.durationUnit = meta.duration_unit;
    if (meta.holidays_block_work_default !== null) this.holidaysBlockWorkDefault = meta.holidays_block_work_default;
    if (meta.include_weekends !== null) this.includeWeekends = meta.include_weekends;
    if (meta.region_default) this.regionDefault = meta.region_default;
    if (meta.ui_scale !== null && meta.ui_scale > 0) {
      this.uiScale = meta.ui_scale;
      document.documentElement.style.setProperty('--ui-scale', String(meta.ui_scale));
    }
    await this.refreshSidebar();
    await this.refreshArchived();
    if (meta.last_open_job_id) {
      try { await this.openJob(meta.last_open_job_id); }
      catch { /* job may have been deleted */ }
    }

    await this.refreshContacts();

    // Keep today's date fresh: tick every minute, plus on window focus.
    setInterval(() => this.tickToday(), 60_000);
    window.addEventListener('focus', () => this.tickToday());

    // Auto-nudges: 3s after boot, and on window focus (debounced once per 5 min).
    setTimeout(() => this.runChaserCheck(), 3000);
    let lastNudgeAt = 0;
    window.addEventListener('focus', () => {
      const now = Date.now();
      if (now - lastNudgeAt > 5 * 60 * 1000) {
        lastNudgeAt = now;
        this.runChaserCheck();
      }
    });

    // Inbox: initial fetch + 5-second poll + refresh on focus.
    await this.refreshInbox();
    this.startInboxPoll();
    window.addEventListener('focus', () => this.refreshInbox());

    // Overdue task reviews — recheck on bootstrap and on window focus.
    await this.refreshOverdueReviews();
    window.addEventListener('focus', () => this.refreshOverdueReviews());
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
      await ipc.syncHolidays({
        job_id: jobId,
        region: this.currentJob.region || 'ZA',
        from: start,
        to: end.toISOString().slice(0, 10),
      });
    }
    this.phases       = await ipc.listPhases(jobId);
    this.tasks        = await ipc.listTasks(jobId);
    this.dependencies = await ipc.listDependencies(jobId);
    this.noWorkDays   = await ipc.listNoWorkDays(jobId);
    this.boqItems = await ipc.listBoqItems(jobId);
    this.boqBudget = await ipc.getJobBudget(jobId);
    this.selection    = null;
    this.activeView = 'schedule';
    await ipc.setLastOpenJob(jobId);
    this.undoStack.clear();
    this.recordHistory(); // seed
    this.hasUnsavedUndo = false;
    this.scrollToToday();
    await this.runLateTasksTick();
    await this.refreshOverdueReviews();
  }

  select(s: Selection): void {
    this.selection = s;
  }

  async deleteDependency(id: number): Promise<void> {
    await ipc.deleteDependency(id);
    this.dependencies = this.dependencies.filter(d => d.id !== id);
    if (this.selection?.kind === 'dependency' && this.selection.id === id) {
      this.selection = null;
    }
    await ipc.touchLastSave();
    this.recordHistory();
  }

  async reorderTasksInPhase(phaseId: number, orderedIds: number[]): Promise<void> {
    await ipc.reorderTasks(phaseId, orderedIds);
    const idx = new Map(orderedIds.map((id, i) => [id, i]));
    this.tasks = this.tasks.map(t => t.phase_id === phaseId ? { ...t, order_index: idx.get(t.id) ?? t.order_index } : t);
    await ipc.touchLastSave();
    this.recordHistory();
  }

  async reorderPhases(orderedIds: number[]): Promise<void> {
    if (!this.currentJob) return;
    await ipc.reorderPhases(this.currentJob.id, orderedIds);
    const idx = new Map(orderedIds.map((id, i) => [id, i]));
    this.phases = this.phases.map(p => ({ ...p, order_index: idx.get(p.id) ?? p.order_index }))
                              .sort((a, b) => a.order_index - b.order_index);
    await ipc.touchLastSave();
    this.recordHistory();
  }

  async createPhase(name: string): Promise<void> {
    if (!this.currentJob) return;
    const palette = ['#3B82F6', '#EF4444', '#10B981', '#F59E0B', '#8B5CF6', '#EC4899', '#14B8A6'];
    const colour = palette[this.phases.length % palette.length];
    const phase = await ipc.createPhase({ job_id: this.currentJob.id, name, colour });
    // Newly-created phases come back expanded (collapsed=false from the IPC command).
    this.phases = [...this.phases, phase].sort((a, b) => a.order_index - b.order_index);
    this.selection = { kind: 'phase', id: phase.id };
    await ipc.touchLastSave();
    this.recordHistory();
  }

  async createTaskInPhase(phaseId: number, name: string): Promise<void> {
    if (!this.currentJob) return;
    const start = this.currentJob.project_start_date;
    const task = await ipc.createTask({
      phase_id: phaseId, name, start_date: start, duration_workdays: 3,
    });
    this.tasks = [...this.tasks, task];
    this.selection = { kind: 'task', id: task.id };
    await ipc.touchLastSave();
    this.recordHistory();
  }

  async toggleNoWorkDay(date: string): Promise<void> {
    if (!this.currentJob) return;
    const existing = this.noWorkDays.find(n => n.date === date && n.source === 'manual');
    if (existing) {
      await ipc.deleteNoWorkDay(existing.id);
      this.noWorkDays = this.noWorkDays.filter(n => n.id !== existing.id);
    } else {
      const created = await ipc.addManualNoWorkDay({
        job_id: this.currentJob.id, date, reason: 'Site closed',
      });
      this.noWorkDays = [...this.noWorkDays, created];
    }
    this.recordHistory();
    await ipc.touchLastSave();
  }

  async setTaskStatus(taskId: number, status: TaskStatus, completionDate: string | null): Promise<void> {
    const idx = this.tasks.findIndex(t => t.id === taskId);
    if (idx >= 0) {
      this.tasks[idx] = { ...this.tasks[idx], status, completion_date: completionDate };
    }
    await ipc.setTaskStatus(taskId, status, completionDate);
    this.recordHistory();
    await ipc.touchLastSave();
  }

  // Optimistic local update applied after an IPC mutation returns updated rows.
  applyDragResult(updated: Task[]): void {
    const byId = new Map(updated.map(t => [t.id, t]));
    this.tasks = this.tasks.map(t => byId.get(t.id) ?? t);
    this.recordHistory();
  }
}

export const store = new Store();
