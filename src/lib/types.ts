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

export interface Task {
  id: number;
  phase_id: number;
  name: string;
  start_date: string;
  duration_workdays: number;
  order_index: number;
  notes: string | null;
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
