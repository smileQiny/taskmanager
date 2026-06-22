export interface Task {
  id: string;
  title: string;
  description?: string;
  status: 'todo' | 'in_progress' | 'done';
  priority: 'low' | 'medium' | 'high';
  start_time?: number;
  end_time?: number;
  all_day: boolean;
  recurrence?: string;
  tags?: string;
  created_at: number;
  updated_at: number;
}

export type TaskStatusFilter = 'all' | Task['status'];
export type TaskPriorityFilter = 'all' | Task['priority'];
export type TaskSortKey = 'schedule' | 'created' | 'priority' | 'title';

export interface TaskFilters {
  query: string;
  status: TaskStatusFilter;
  priority: TaskPriorityFilter;
}

export interface CreateTaskInput {
  title: string;
  description?: string;
  status?: Task['status'];
  priority?: Task['priority'];
  start_time?: number;
  end_time?: number;
  all_day?: boolean;
  recurrence?: string;
  tags?: string;
}

export interface UpdateTaskInput {
  id: string;
  title?: string;
  description?: string | null;
  status?: Task['status'];
  priority?: Task['priority'];
  start_time?: number | null;
  end_time?: number | null;
  all_day?: boolean;
  recurrence?: string | null;
  tags?: string | null;
}

export interface AppMetadata {
  data_dir: string;
  db_path: string;
}

export interface AppSettings {
  theme: 'light' | 'dark';
  default_calendar_view: 'month' | 'week' | 'day';
  pomodoro_minutes: number;
  cockpit_opacity: number;
}

export interface PomodoroSession {
  id: string;
  started_at: number;
  duration: number;
  completed: boolean;
}

export type SyncProvider = 'feishu' | 'macos' | 'wecom' | 'google';

export interface SyncAccount {
  id: string;
  provider: SyncProvider;
  enabled: boolean;
  expires_at?: number;
  config?: string;
  status: 'not_configured' | 'pending_auth' | 'needs_auth' | 'synced' | 'error';
}

export interface UpsertSyncAccountInput {
  provider: SyncProvider;
  enabled: boolean;
  config?: string;
}

export interface SyncRunResult {
  provider: SyncProvider;
  status: string;
  message: string;
  affected_tasks: number;
}

export interface TaskSyncState {
  task_id: string;
  provider: SyncProvider;
  sync_status: 'pending' | 'needs_auth' | 'synced' | 'conflict' | 'error';
  external_id?: string;
  last_synced?: number;
}

export interface SetTaskSyncInput {
  task_id: string;
  provider: SyncProvider;
  enabled: boolean;
}
