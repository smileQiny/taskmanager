import { invoke } from '@tauri-apps/api/core';
import {
  AppMetadata,
  AppSettings,
  CreateTaskInput,
  PomodoroSession,
  SyncAccount,
  SyncProvider,
  SyncRunResult,
  SetTaskSyncInput,
  Task,
  TaskSyncState,
  UpsertSyncAccountInput,
  UpdateTaskInput,
} from '../types/task';

const hasTauri = typeof window !== 'undefined' && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
const taskKey = 'taskmanager.tasks';
const settingsKey = 'taskmanager.settings';
const pomodoroKey = 'taskmanager.pomodoro';
const syncKey = 'taskmanager.syncAccounts';
const taskSyncKey = 'taskmanager.taskSyncStates';

export const taskService = {
  getAll: () => hasTauri ? invoke<Task[]>('get_tasks') : local(readTasks),
  getById: (id: string) => hasTauri ? invoke<Task | null>('get_task', { id }) : local(() => readTasks().find((task) => task.id === id) ?? null),
  create: (input: CreateTaskInput) => hasTauri ? invoke<Task>('create_task', { input }) : local(() => createLocalTask(input)),
  update: (input: UpdateTaskInput) => hasTauri ? invoke<Task | null>('update_task', { input }) : local(() => updateLocalTask(input)),
  delete: (id: string) => hasTauri ? invoke<boolean>('delete_task', { id }) : local(() => deleteLocalTask(id)),
  getByRange: (from: number, to: number) =>
    hasTauri ? invoke<Task[]>('get_tasks_by_range', { from, to }) : local(() => readTasks().filter((task) => {
      if (!task.start_time) return false;
      return task.start_time >= from && task.start_time <= to;
    })),
};

export const appService = {
  getMetadata: () => hasTauri ? invoke<AppMetadata>('get_app_metadata') : Promise.resolve({
    data_dir: 'Browser localStorage preview',
    db_path: 'Browser localStorage preview',
  }),
};

export const settingsService = {
  get: () => hasTauri ? invoke<AppSettings>('get_settings') : local(readSettings),
  update: (input: Partial<AppSettings>) => hasTauri ? invoke<AppSettings>('update_settings', { input }) : local(() => updateLocalSettings(input)),
};

export const pomodoroService = {
  createSession: (input: { started_at: number; duration: number; completed: boolean }) =>
    hasTauri ? invoke<PomodoroSession>('create_pomodoro_session', { input }) : Promise.resolve(createLocalPomodoroSession(input)),
  listSessions: (limit = 10) =>
    hasTauri ? invoke<PomodoroSession[]>('list_pomodoro_sessions', { limit }) : Promise.resolve(readPomodoroSessions().slice(0, limit)),
};

export const syncService = {
  listAccounts: () => hasTauri ? invoke<SyncAccount[]>('list_sync_accounts') : local(readSyncAccounts),
  upsertAccount: (input: UpsertSyncAccountInput) => hasTauri ? invoke<SyncAccount>('upsert_sync_account', { input }) : local(() => upsertLocalSyncAccount(input)),
  syncNow: (provider: SyncProvider) => hasTauri ? invoke<SyncRunResult>('sync_provider_now', { provider }) : local(() => syncLocalProvider(provider)),
  listTaskStates: (taskId: string) => hasTauri ? invoke<TaskSyncState[]>('list_task_sync_states', { taskId }) : local(() => readTaskSyncStates(taskId)),
  setTaskSync: (input: SetTaskSyncInput) => hasTauri ? invoke<TaskSyncState[]>('set_task_sync', { input }) : local(() => setLocalTaskSync(input)),
};

function local<T>(fn: () => T): Promise<T> {
  return Promise.resolve().then(fn);
}

function readTasks(): Task[] {
  return readJson<Task[]>(taskKey, []);
}

function writeTasks(tasks: Task[]) {
  localStorage.setItem(taskKey, JSON.stringify(tasks));
}

function createLocalTask(input: CreateTaskInput): Task {
  const now = Math.floor(Date.now() / 1000);
  const title = normalizeRequiredTitle(input.title);
  const status = validateStatus(input.status ?? 'todo');
  const priority = validatePriority(input.priority ?? 'medium');
  validateTimeRange(input.start_time, input.end_time);
  const task: Task = {
    id: crypto.randomUUID(),
    title,
    description: input.description,
    status,
    priority,
    start_time: input.start_time,
    end_time: input.end_time,
    all_day: input.all_day ?? false,
    recurrence: input.recurrence,
    tags: input.tags,
    created_at: now,
    updated_at: now,
  };
  writeTasks([task, ...readTasks()]);
  return task;
}

function updateLocalTask(input: UpdateTaskInput): Task | null {
  const tasks = readTasks();
  const task = tasks.find((item) => item.id === input.id);
  if (!task) return null;
  const startTime = input.start_time !== undefined ? input.start_time ?? undefined : task.start_time;
  const endTime = input.end_time !== undefined ? input.end_time ?? undefined : task.end_time;
  validateTimeRange(startTime, endTime);
  const updated: Task = {
    ...task,
    title: input.title !== undefined ? normalizeRequiredTitle(input.title) : task.title,
    description: input.description !== undefined ? input.description ?? undefined : task.description,
    status: input.status !== undefined ? validateStatus(input.status) : task.status,
    priority: input.priority !== undefined ? validatePriority(input.priority) : task.priority,
    start_time: startTime,
    end_time: endTime,
    all_day: input.all_day ?? task.all_day,
    recurrence: input.recurrence !== undefined ? input.recurrence ?? undefined : task.recurrence,
    tags: input.tags !== undefined ? input.tags ?? undefined : task.tags,
    updated_at: Math.floor(Date.now() / 1000),
  };
  writeTasks(tasks.map((item) => item.id === updated.id ? updated : item));
  return updated;
}

function normalizeRequiredTitle(title: string): string {
  const trimmed = title.trim();
  if (!trimmed) throw new Error('title cannot be blank');
  return trimmed;
}

function validateStatus(status: Task['status']): Task['status'] {
  if (status !== 'todo' && status !== 'in_progress' && status !== 'done') {
    throw new Error('status must be todo, in_progress, or done');
  }
  return status;
}

function validatePriority(priority: Task['priority']): Task['priority'] {
  if (priority !== 'low' && priority !== 'medium' && priority !== 'high') {
    throw new Error('priority must be low, medium, or high');
  }
  return priority;
}

function validateTimeRange(startTime?: number, endTime?: number) {
  if (startTime !== undefined && endTime !== undefined && endTime < startTime) {
    throw new Error('end_time cannot be before start_time');
  }
}

function deleteLocalTask(id: string): boolean {
  const tasks = readTasks();
  writeTasks(tasks.filter((task) => task.id !== id));
  return tasks.some((task) => task.id === id);
}

function readSettings(): AppSettings {
  return {
    ...defaultSettings(),
    ...readJson<Partial<AppSettings>>(settingsKey, {}),
  };
}

function defaultSettings(): AppSettings {
  return {
    theme: 'light',
    default_calendar_view: 'month',
    pomodoro_minutes: 25,
    cockpit_opacity: 92,
  };
}

function updateLocalSettings(input: Partial<AppSettings>): AppSettings {
  const settings = {
    ...readSettings(),
    ...input,
    cockpit_opacity: validateCockpitOpacity(input.cockpit_opacity ?? readSettings().cockpit_opacity),
  };
  localStorage.setItem(settingsKey, JSON.stringify(settings));
  return settings;
}

function validateCockpitOpacity(value: number): number {
  if (!Number.isFinite(value) || value < 60 || value > 100) {
    throw new Error('cockpit_opacity must be between 60 and 100');
  }
  return Math.round(value);
}

function readPomodoroSessions(): PomodoroSession[] {
  return readJson<PomodoroSession[]>(pomodoroKey, []);
}

function readSyncAccounts(): SyncAccount[] {
  const existing = readJson<SyncAccount[]>(syncKey, []);
  if (existing.length > 0) return existing;
  return ['feishu', 'macos', 'wecom', 'google'].map((provider) => ({
    id: provider,
    provider: provider as SyncProvider,
    enabled: false,
    status: 'not_configured',
  }));
}

function writeSyncAccounts(accounts: SyncAccount[]) {
  localStorage.setItem(syncKey, JSON.stringify(accounts));
}

function upsertLocalSyncAccount(input: UpsertSyncAccountInput): SyncAccount {
  const accounts = readSyncAccounts();
  const account: SyncAccount = {
    id: input.provider,
    provider: input.provider,
    enabled: input.enabled,
    config: input.config,
    status: input.enabled ? 'pending_auth' : 'not_configured',
  };
  writeSyncAccounts(accounts.map((item) => item.provider === input.provider ? account : item));
  return account;
}

function syncLocalProvider(provider: SyncProvider): SyncRunResult {
  const account = readSyncAccounts().find((item) => item.provider === provider);
  if (!account?.enabled) {
    return {
      provider,
      status: 'not_configured',
      message: 'Provider is disabled. Enable it in Settings before syncing.',
      affected_tasks: 0,
    };
  }
  const states = readAllTaskSyncStates();
  let affected = 0;
  const updated = states.map((state) => {
    if (state.provider !== provider) return state;
    affected += 1;
    return { ...state, sync_status: 'needs_auth' as const };
  });
  writeTaskSyncStates(updated);
  return {
    provider,
    status: 'needs_auth',
    message: `${providerLabel(provider)} requires authorization before live sync.`,
    affected_tasks: affected,
  };
}

function providerLabel(provider: SyncProvider): string {
  return {
    feishu: 'Feishu',
    macos: 'macOS Calendar',
    wecom: 'WeCom',
    google: 'Google Calendar',
  }[provider];
}

function createLocalPomodoroSession(input: { started_at: number; duration: number; completed: boolean }): PomodoroSession {
  const session: PomodoroSession = {
    id: crypto.randomUUID(),
    started_at: input.started_at,
    duration: input.duration,
    completed: input.completed,
  };
  localStorage.setItem(pomodoroKey, JSON.stringify([session, ...readPomodoroSessions()]));
  return session;
}

function readAllTaskSyncStates(): TaskSyncState[] {
  return readJson<TaskSyncState[]>(taskSyncKey, []);
}

function readTaskSyncStates(taskId: string): TaskSyncState[] {
  return readAllTaskSyncStates().filter((state) => state.task_id === taskId);
}

function writeTaskSyncStates(states: TaskSyncState[]) {
  localStorage.setItem(taskSyncKey, JSON.stringify(states));
}

function setLocalTaskSync(input: SetTaskSyncInput): TaskSyncState[] {
  const states = readAllTaskSyncStates().filter((state) => !(state.task_id === input.task_id && state.provider === input.provider));
  if (input.enabled) {
    states.push({
      task_id: input.task_id,
      provider: input.provider,
      sync_status: 'pending',
    });
  }
  writeTaskSyncStates(states);
  return readTaskSyncStates(input.task_id);
}

function readJson<T>(key: string, fallback: T): T {
  const value = localStorage.getItem(key);
  if (!value) return fallback;
  try {
    return JSON.parse(value) as T;
  } catch {
    return fallback;
  }
}
