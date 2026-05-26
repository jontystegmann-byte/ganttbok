// Mirror of src-tauri/src/db/models.rs row structs.
// All dates are ISO YYYY-MM-DD strings on the wire; we keep them as strings
// in the frontend store and only convert to Date for math via lib/calendar.ts.

export interface Job {
  id: number;
  name: string;
  client: string | null;
  address: string | null;
  project_start_date: string;
  is_template: boolean;
  archived: boolean;
  created_at: string;
  holidays_block_work: boolean;
  region: string;
  auto_shift_dependents: boolean;
}

export interface Phase {
  id: number;
  job_id: number;
  name: string;
  colour: string;
  order_index: number;
  collapsed: boolean;
  notes: string;
}

export interface Contact {
  id: number;
  name: string;
  telegram_chat_id: string | null;
  telegram_handle: string | null;
  notes: string;
  created_at: string;
}

export interface NudgeResult {
  task_id: number;
  task_name: string;
  contact_name: string;
  template_key: string;
  days: number;
  success: boolean;
  error: string | null;
}

export interface Task {
  id: number;
  phase_id: number;
  name: string;
  start_date: string;
  duration_workdays: number;
  order_index: number;
  notes: string | null;
  contact_id: number | null;
  last_chaser_sent_at: string | null;
  status: TaskStatus;
  completion_date: string | null;
}

export interface Dependency {
  id: number;
  predecessor_id: number;
  successor_id: number;
  type: string;
  lag_days: number;
}

export interface NoWorkDay {
  id: number;
  job_id: number;
  date: string;
  reason: string;
  source: 'sa_public_holiday' | 'manual';
}

export interface StartupInfo {
  clean_shutdown: boolean;
  last_open_job_id: number | null;
  last_save_at: string | null;
  sidebar_width: number | null;
  duration_unit: 'weeks' | 'days' | null;
  holidays_block_work_default: boolean | null;
  include_weekends: boolean | null;
  ui_scale: number | null;
  region_default: string | null;
}

export interface DragResult {
  updated_tasks: Task[];
}

// Args structs (match the Rust #[derive(Deserialize)] structs)
export interface CreateJobArgs {
  name: string;
  client: string | null;
  address: string | null;
  project_start_date: string;
  is_template: boolean;
  holidays_block_work: boolean;
  region: string;
}

export interface CreatePhaseArgs {
  job_id: number;
  name: string;
  colour: string;
}

export interface CreateTaskArgs {
  phase_id: number;
  name: string;
  start_date: string;
  duration_workdays: number;
}

export interface CreateDepArgs {
  predecessor_id: number;
  successor_id: number;
  lag_days: number;
}

export interface AddManualArgs {
  job_id: number;
  date: string;
  reason: string;
}

export interface SyncSaArgs {
  job_id: number;
  from: string;
  to: string;
}

export interface InstantiateArgs {
  template_id: number;
  new_name: string;
  client: string | null;
  address: string | null;
  project_start_date: string;
}

export interface DragTaskArgs {
  job_id: number;
  task_id: number;
  new_start_date: string;
}

// ---------------------------------------------------------------
// Patch schema — shared with the MCP server and the Inbox panel.
// Source of truth: src-tauri/src/patches/schema.rs (PATCH_VERSION = 1).
// Keep these two definitions in sync; any change here needs a
// matching change there.
// ---------------------------------------------------------------

export const PATCH_VERSION = 1;

export type TaskRef =
  | { task_id: number }
  | { op_ref: string };

export type PatchOp =
  | {
      op: 'add_task';
      phase_id: number;
      name: string;
      start_date: string;          // YYYY-MM-DD
      duration_workdays: number;
      notes?: string;
      contact_id?: number;
      op_ref?: string;
    }
  | {
      op: 'shift_task';
      task_id: number;
      by_days: number;
    }
  | {
      op: 'add_dependency';
      predecessor: TaskRef;
      successor: TaskRef;
      dep_type?: 'FS' | 'SS' | 'FF' | 'SF';   // default FS
      lag_days?: number;
    }
  | {
      op: 'add_chaser';
      task_id: number;
      contact_id: number;
      template: string;
    }
  | {
      op: 'append_note';
      job_id: number;
      text: string;
    };

export interface Patch {
  patch_version: number;
  summary: string;
  ops: PatchOp[];
}

export type TaskStatus = 'on_track' | 'done' | 'late';

export type PatchStatus =
  | 'proposed'
  | 'accepted'
  | 'applied'
  | 'rejected'
  | 'apply_failed'
  | 'expired';

export interface PendingPatch {
  id: string;
  job_id: number;
  patch: Patch;            // parsed from patch_json at the IPC boundary
  summary: string;
  source: string;          // 'mcp' for v1
  status: PatchStatus;
  created_at: number;      // unix seconds
  resolved_at: number | null;
  error: string | null;
}

// ---------------------------------------------------------------
// Claude connector — Settings → Integrations.
// Mirrors src-tauri/src/commands/claude.rs.
// ---------------------------------------------------------------

export type ClaudeSurface = 'code' | 'desktop';

export interface ClaudeDetection {
  surface: ClaudeSurface;
  display_name: string;
  config_path: string;
  config_exists: boolean;
  blikplan_connected: boolean;
}

export interface ClaudeDetectionResult {
  surfaces: ClaudeDetection[];
}
