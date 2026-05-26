import { invoke } from '@tauri-apps/api/core';
import type {
  Job, Phase, Task, Dependency, NoWorkDay, StartupInfo, DragResult,
  CreateJobArgs, CreatePhaseArgs, CreateTaskArgs, CreateDepArgs,
  AddManualArgs, SyncSaArgs, InstantiateArgs, DragTaskArgs,
  Contact, NudgeResult,
  PendingPatch,
  ClaudeSurface, ClaudeDetectionResult,
  TaskStatus,
} from './types';

// Jobs
export const listJobs        = ()                                    => invoke<Job[]>('list_jobs');
export const listTemplates   = ()                                    => invoke<Job[]>('list_templates');
export const listArchived    = ()                                    => invoke<Job[]>('list_archived');
export const getJob          = (id: number)                          => invoke<Job>('get_job', { id });
export const createJob       = (args: CreateJobArgs)                 => invoke<Job>('create_job', { args });
export const updateJob       = (job: Job)                            => invoke<void>('update_job', { job });
export const archiveJob      = (id: number, archived: boolean)       => invoke<void>('archive_job', { id, archived });
export const deleteJob       = (id: number)                          => invoke<void>('delete_job', { id });

// Templates
export const saveAsTemplate     = (sourceJobId: number, templateName: string) =>
  invoke<Job>('save_as_template', { sourceJobId, templateName });
export const instantiateTemplate = (args: InstantiateArgs) =>
  invoke<Job>('instantiate_template', { args });

// Phases
export const listPhases    = (jobId: number)                         => invoke<Phase[]>('list_phases',  { jobId });
export const createPhase   = (args: CreatePhaseArgs)                 => invoke<Phase>('create_phase',   { args });
export const updatePhase   = (phase: Phase)                          => invoke<void>('update_phase',    { phase });
export const deletePhase   = (id: number)                            => invoke<void>('delete_phase',    { id });
export const reorderPhases = (jobId: number, orderedIds: number[])   => invoke<void>('reorder_phases',  { jobId, orderedIds });

// Tasks
export const listTasks    = (jobId: number)                          => invoke<Task[]>('list_tasks', { jobId });
export const createTask   = (args: CreateTaskArgs)                   => invoke<Task>('create_task',  { args });
export const updateTask   = (task: Task)                             => invoke<void>('update_task',  { task });
export const deleteTask   = (id: number)                             => invoke<void>('delete_task',  { id });
export const reorderTasks = (phaseId: number, orderedIds: number[])  => invoke<void>('reorder_tasks', { phaseId, orderedIds });

// Drag
export const dragTask = (args: DragTaskArgs) => invoke<DragResult>('drag_task', { args });

export const setTaskStatus = (
  id: number,
  status: TaskStatus,
  completionDate: string | null,
) => invoke<void>('set_task_status', { id, status, completionDate });

export const setJobAutoShift = (id: number, enabled: boolean) =>
  invoke<void>('set_job_auto_shift', { id, enabled });

export const autoTransitionStartedTasks = (today: string) =>
  invoke<number>('auto_transition_started_tasks', { today });

// Dependencies
export const listDependencies     = (jobId: number)                  => invoke<Dependency[]>('list_dependencies', { jobId });
export const createDependency     = (args: CreateDepArgs)            => invoke<Dependency>('create_dependency', { args });
export const updateDependencyLag  = (id: number, lagDays: number)    => invoke<void>('update_dependency_lag', { id, lagDays });
export const deleteDependency     = (id: number)                     => invoke<void>('delete_dependency', { id });

// No-work days
export const listNoWorkDays         = (jobId: number)                => invoke<NoWorkDay[]>('list_no_work_days', { jobId });
export const addManualNoWorkDay     = (args: AddManualArgs)          => invoke<NoWorkDay>('add_manual_no_work_day', { args });
export const deleteNoWorkDay        = (id: number)                   => invoke<void>('delete_no_work_day', { id });
export const syncSaHolidays         = (args: SyncSaArgs)             => invoke<number>('sync_sa_holidays', { args });
export const syncHolidays = (args: { job_id: number; region: string; from: string; to: string }) =>
  invoke<number>('sync_holidays', { args });

// Meta
export const startupInfo        = ()                                 => invoke<StartupInfo>('startup_info');
export const markCleanShutdown  = ()                                 => invoke<void>('mark_clean_shutdown');
export const setLastOpenJob     = (jobId: number)                    => invoke<void>('set_last_open_job', { jobId });
export const setSidebarWidth    = (width: number)                    => invoke<void>('set_sidebar_width', { width });
export const touchLastSave      = ()                                 => invoke<void>('touch_last_save');
export const setDurationUnit    = (unit: 'weeks' | 'days')           => invoke<void>('set_duration_unit', { unit });
export const setHolidaysBlockWorkDefault = (value: boolean)          => invoke<void>('set_holidays_block_work_default', { value });
export const setIncludeWeekends = (value: boolean)                   => invoke<void>('set_include_weekends', { value });
export const setUiScale         = (value: number)                    => invoke<void>('set_ui_scale', { value });
export const setRegionDefault   = (region: string)                   => invoke<void>('set_region_default', { region });
export const setMetaValue       = (key: string, value: string)       => invoke<void>('set_meta_value', { key, value });
export const getMetaValue       = (key: string)                      => invoke<string | null>('get_meta_value', { key });

// Chaser
type ContactArgs = { id?: number | null; name: string; telegram_chat_id: string | null; telegram_handle: string | null; notes: string };
export const listContacts        = ()                                  => invoke<Contact[]>('list_contacts');
export const createContact       = (args: ContactArgs)                 => invoke<Contact>('create_contact', { args });
export const updateContact       = (args: ContactArgs)                 => invoke<void>('update_contact', { args });
export const deleteContact       = (id: number)                        => invoke<void>('delete_contact', { id });
export const assignTaskContact   = (args: { task_id: number; contact_id: number | null }) =>
  invoke<void>('assign_task_contact', { args });
export const sendChaser          = (args: { task_id: number; template_key: string; custom_text?: string | null }) =>
  invoke<void>('send_chaser', { args });
export const testTelegram        = (args: { token: string; chat_id: string }) =>
  invoke<void>('test_telegram', { args });
export const runChaserCheck      = ()                                  => invoke<NudgeResult[]>('run_chaser_check');

// Resync (used by undo/redo + manual ⌘S to push local state to backend in one transaction)
export interface ResyncArgs {
  job_id: number;
  phases: Phase[];
  tasks: Task[];
  dependencies: Dependency[];
  no_work_days: NoWorkDay[];
}
export const resyncJobState = (args: ResyncArgs) => invoke<void>('resync_job_state', { args });

// Inbox / Patches
export const listPendingPatches = (statusFilter?: string) =>
  invoke<PendingPatch[]>('list_pending_patches', { statusFilter: statusFilter ?? null });

export const getPendingPatch = (id: string) =>
  invoke<PendingPatch>('get_pending_patch', { id });

export const acceptPatch = (id: string) =>
  invoke<void>('accept_patch', { id });

export const rejectPatch = (id: string) =>
  invoke<void>('reject_patch', { id });

export const clearResolvedPatches = () =>
  invoke<number>('clear_resolved_patches');

export const expireStalePatches = () =>
  invoke<number>('expire_stale_patches');

// Claude connector
export async function detectClaudeSurfaces(): Promise<ClaudeDetectionResult> {
  return invoke('detect_claude_surfaces');
}

export async function connectToClaude(surfaces: ClaudeSurface[]): Promise<ClaudeDetectionResult> {
  return invoke('connect_to_claude', { surfaces });
}

export async function disconnectFromClaude(surfaces: ClaudeSurface[]): Promise<ClaudeDetectionResult> {
  return invoke('disconnect_from_claude', { surfaces });
}
