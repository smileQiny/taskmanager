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
  note?: string | null;
  task_id?: string | null;
}

export interface PomodoroTimerState {
  duration: number;
  remaining: number;
  running: boolean;
  started_at?: number | null;
  updated_at: number;
}

export interface PomodoroTimerCompletion {
  session: PomodoroSession;
  completed_at: number;
  duration: number;
}

export interface PomodoroTimerSnapshot {
  timer: PomodoroTimerState;
  completion?: PomodoroTimerCompletion | null;
}

export type SyncProvider = 'feishu' | 'macos' | 'wecom' | 'google';

export interface SyncAccount {
  id: string;
  provider: SyncProvider;
  enabled: boolean;
  expires_at?: number;
  config?: string;
  status: 'not_configured' | 'pending_auth' | 'needs_auth' | 'authenticated' | 'synced' | 'error';
  last_error?: string | null;
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

export interface GoogleConnection {
  status: string;
  connected: boolean;
  client_id: string;
  calendar_id: string;
  has_client_secret: boolean;
  last_error?: string | null;
}

export interface SaveGoogleSettingsInput {
  client_id: string;
  calendar_id?: string;
  client_secret?: string;
  enabled?: boolean;
}

export interface ResolveSyncConflictInput {
  task_id: string;
  provider: SyncProvider;
  strategy: 'local' | 'remote';
}

export interface MacosCalendarInfo {
  id: string;
  name: string;
}

export interface MacosCalendarConnection {
  supported: boolean;
  status: string;
  connected: boolean;
  permission: 'unsupported' | 'not_determined' | 'restricted' | 'denied' | 'write_only' | 'full_access' | string;
  calendar_id: string;
  calendar_name: string;
  calendars: MacosCalendarInfo[];
  last_error?: string | null;
}

export interface SaveMacosCalendarSettingsInput {
  calendar_id?: string;
  calendar_name?: string;
  enabled?: boolean;
}
