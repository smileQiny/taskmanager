import { invoke } from '@tauri-apps/api/core';
import {
  AppMetadata,
  AppSettings,
  CreateTaskInput,
  PomodoroSession,
  PomodoroTimerSnapshot,
  FeishuConnection,
  GoogleConnection,
  MacosCalendarConnection,
  ResolveSyncConflictInput,
  SaveFeishuSettingsInput,
  SaveGoogleSettingsInput,
  SaveMacosCalendarSettingsInput,
  SaveWecomSettingsInput,
  SyncAccount,
  SyncProvider,
  SyncRunResult,
  SetTaskSyncInput,
  Task,
  WecomConnection,
  TaskSyncState,
  UpsertSyncAccountInput,
  UpdateTaskInput,
} from '../types/task';

const hasTauri = typeof window !== 'undefined' && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
const taskKey = 'taskmanager.tasks';
const settingsKey = 'taskmanager.settings';
const pomodoroKey = 'taskmanager.pomodoro';
const pomodoroTimerKey = 'taskmanager.pomodoro.timer-state';
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
  updateSessionRecord: (input: { id: string; note?: string | null; task_id?: string | null }) =>
    hasTauri ? invoke<PomodoroSession | null>('update_pomodoro_session_record', { input }) : local(() => updateLocalPomodoroSessionRecord(input)),
  getTimer: () =>
    hasTauri ? invoke<PomodoroTimerSnapshot>('get_pomodoro_timer') : local(() => syncLocalPomodoroTimer()),
  startTimer: () =>
    hasTauri ? invoke<PomodoroTimerSnapshot>('start_pomodoro_timer') : local(() => startLocalPomodoroTimer()),
  pauseTimer: () =>
    hasTauri ? invoke<PomodoroTimerSnapshot>('pause_pomodoro_timer') : local(() => pauseLocalPomodoroTimer()),
  resetTimer: () =>
    hasTauri ? invoke<PomodoroTimerSnapshot>('reset_pomodoro_timer') : local(() => resetLocalPomodoroTimer()),
  setTimerDuration: (minutes: number) =>
    hasTauri ? invoke<PomodoroTimerSnapshot>('set_pomodoro_timer_duration', { minutes }) : local(() => setLocalPomodoroTimerDuration(minutes)),
  completeTimer: () =>
    hasTauri ? invoke<PomodoroTimerSnapshot>('complete_pomodoro_timer') : local(() => completeLocalPomodoroTimer()),
};

export const syncService = {
  listAccounts: () => hasTauri ? invoke<SyncAccount[]>('list_sync_accounts') : local(readSyncAccounts),
  upsertAccount: (input: UpsertSyncAccountInput) => hasTauri ? invoke<SyncAccount>('upsert_sync_account', { input }) : local(() => upsertLocalSyncAccount(input)),
  syncNow: (provider: SyncProvider) => hasTauri ? invoke<SyncRunResult>('sync_provider_now', { provider }) : local(() => syncLocalProvider(provider)),
  listTaskStates: (taskId: string) => hasTauri ? invoke<TaskSyncState[]>('list_task_sync_states', { taskId }) : local(() => readTaskSyncStates(taskId)),
  setTaskSync: (input: SetTaskSyncInput) => hasTauri ? invoke<TaskSyncState[]>('set_task_sync', { input }) : local(() => setLocalTaskSync(input)),
  googleStatus: () => hasTauri ? invoke<GoogleConnection>('google_connection_status') : local(readLocalGoogleConnection),
  saveGoogleSettings: (input: SaveGoogleSettingsInput) => hasTauri
    ? invoke<GoogleConnection>('save_google_settings', { input })
    : Promise.reject(new Error('Google Calendar connection is available in the desktop app only.')),
  connectGoogle: () => hasTauri
    ? invoke<GoogleConnection>('connect_google_calendar')
    : Promise.reject(new Error('Google Calendar connection is available in the desktop app only.')),
  disconnectGoogle: () => hasTauri
    ? invoke<GoogleConnection>('disconnect_google_calendar')
    : Promise.reject(new Error('Google Calendar connection is available in the desktop app only.')),
  resolveConflict: (input: ResolveSyncConflictInput) => hasTauri
    ? invoke<SyncRunResult>('resolve_task_sync_conflict', { input })
    : Promise.reject(new Error(
      input.provider === 'macos'
        ? 'macOS Calendar sync is only available on macOS.'
        : input.provider === 'feishu'
          ? 'Feishu Calendar sync is available in the desktop app only.'
          : input.provider === 'wecom'
            ? 'WeCom Calendar sync is available in the desktop app only.'
            : 'Calendar sync conflict resolution is available in the desktop app only.',
    )),
  macosStatus: () => hasTauri
    ? invoke<MacosCalendarConnection>('macos_calendar_status')
    : local(readLocalMacosConnection),
  saveMacosSettings: (input: SaveMacosCalendarSettingsInput) => hasTauri
    ? invoke<MacosCalendarConnection>('save_macos_calendar_settings', { input })
    : Promise.reject(new Error('macOS Calendar sync is only available on macOS.')),
  requestMacosAccess: () => hasTauri
    ? invoke<MacosCalendarConnection>('request_macos_calendar_access')
    : Promise.reject(new Error('macOS Calendar sync is only available on macOS.')),
  feishuStatus: () => hasTauri
    ? invoke<FeishuConnection>('feishu_connection_status')
    : local(readLocalFeishuConnection),
  saveFeishuSettings: (input: SaveFeishuSettingsInput) => hasTauri
    ? invoke<FeishuConnection>('save_feishu_settings', { input })
    : Promise.reject(new Error('Feishu Calendar sync is available in the desktop app only.')),
  connectFeishu: () => hasTauri
    ? invoke<FeishuConnection>('connect_feishu_calendar')
    : Promise.reject(new Error('Feishu Calendar sync is available in the desktop app only.')),
  disconnectFeishu: () => hasTauri
    ? invoke<FeishuConnection>('disconnect_feishu_calendar')
    : Promise.reject(new Error('Feishu Calendar sync is available in the desktop app only.')),
  wecomStatus: () => hasTauri
    ? invoke<WecomConnection>('wecom_connection_status')
    : local(readLocalWecomConnection),
  saveWecomSettings: (input: SaveWecomSettingsInput) => hasTauri
    ? invoke<WecomConnection>('save_wecom_settings', { input })
    : Promise.reject(new Error('WeCom Calendar sync is available in the desktop app only.')),
  connectWecom: () => hasTauri
    ? invoke<WecomConnection>('connect_wecom_calendar')
    : Promise.reject(new Error('WeCom Calendar sync is available in the desktop app only.')),
  disconnectWecom: () => hasTauri
    ? invoke<WecomConnection>('disconnect_wecom_calendar')
    : Promise.reject(new Error('WeCom Calendar sync is available in the desktop app only.')),
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
  if (!Number.isFinite(value) || value < 35 || value > 100) {
    throw new Error('cockpit_opacity must be between 35 and 100');
  }
  return Math.round(value);
}

function readPomodoroSessions(): PomodoroSession[] {
  return readJson<PomodoroSession[]>(pomodoroKey, []);
}

function syncLocalPomodoroTimer(): PomodoroTimerSnapshot {
  const timer = readLocalPomodoroTimer();
  if (timer.running && getLocalTimerRemaining(timer) <= 0) {
    return completeLocalPomodoroTimer();
  }
  return { timer: normalizeLocalPomodoroTimer(timer) };
}

function readLocalGoogleConnection(): GoogleConnection {
  const account = readSyncAccounts().find((item) => item.provider === 'google');
  return {
    status: account?.status ?? 'not_configured',
    connected: false,
    client_id: '',
    calendar_id: 'primary',
    has_client_secret: false,
    last_error: 'Google Calendar connection is available in the desktop app only.',
  };
}

function readSyncAccounts(): SyncAccount[] {
  const existing = readJson<SyncAccount[]>(syncKey, []);
  if (existing.length > 0) {
    return existing.map((account) => account.provider === 'wecom'
      ? { ...account, config: publicWecomConfig(account.config) }
      : account);
  }
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
  const config = input.provider === 'wecom' ? publicWecomConfig(input.config) : input.config;
  const account: SyncAccount = {
    id: input.provider,
    provider: input.provider,
    enabled: input.enabled,
    config,
    status: input.enabled ? 'pending_auth' : 'not_configured',
  };
  writeSyncAccounts(accounts.map((item) => item.provider === input.provider ? account : item));
  return account;
}

function readLocalFeishuConnection(): FeishuConnection {
  const account = readSyncAccounts().find((item) => item.provider === 'feishu');
  let appId = '';
  let calendarId = 'primary';
  let domain = 'feishu';
  try {
    const parsed = JSON.parse(account?.config || '{}') as { app_id?: string; calendar_id?: string; domain?: string };
    appId = parsed.app_id ?? '';
    calendarId = parsed.calendar_id || 'primary';
    domain = parsed.domain === 'lark' ? 'lark' : 'feishu';
  } catch {
    appId = '';
  }
  return {
    status: 'not_configured',
    connected: false,
    app_id: appId,
    calendar_id: calendarId,
    domain,
    has_app_secret: false,
    redirect_uri: 'http://127.0.0.1:43821/callback',
    last_error: 'Feishu Calendar sync is available in the desktop app only.',
  };
}

function publicWecomConfig(config?: string): string | undefined {
  if (!config) return config;
  try {
    const parsed = JSON.parse(config) as {
      corp_id?: string;
      agent_id?: string;
      organizer_userid?: string;
      calendar_id?: string;
    };
    return JSON.stringify({
      corp_id: parsed.corp_id ?? '',
      agent_id: parsed.agent_id ?? '',
      organizer_userid: parsed.organizer_userid ?? '',
      calendar_id: parsed.calendar_id || 'primary',
    });
  } catch {
    return undefined;
  }
}

function readLocalWecomConnection(): WecomConnection {
  const account = readSyncAccounts().find((item) => item.provider === 'wecom');
  let corpId = '';
  let agentId = '';
  let organizerUserId = '';
  let calendarId = 'primary';
  try {
    const parsed = JSON.parse(account?.config || '{}') as {
      corp_id?: string;
      agent_id?: string;
      organizer_userid?: string;
      calendar_id?: string;
    };
    corpId = parsed.corp_id ?? '';
    agentId = parsed.agent_id ?? '';
    organizerUserId = parsed.organizer_userid ?? '';
    calendarId = parsed.calendar_id || 'primary';
  } catch {
    corpId = '';
  }
  return {
    status: 'not_configured',
    connected: false,
    corp_id: corpId,
    agent_id: agentId,
    organizer_userid: organizerUserId,
    calendar_id: calendarId,
    has_secret: false,
    last_error: 'WeCom Calendar sync is available in the desktop app only.',
  };
}

function readLocalMacosConnection(): MacosCalendarConnection {
  const account = readSyncAccounts().find((item) => item.provider === 'macos');
  let calendarId = '';
  let calendarName = 'Calendar';
  try {
    const parsed = JSON.parse(account?.config || '{}') as { calendar_id?: string; calendar_name?: string };
    calendarId = parsed.calendar_id ?? '';
    calendarName = parsed.calendar_name || 'Calendar';
  } catch {
    calendarId = '';
  }
  return {
    supported: false,
    status: 'unsupported',
    connected: false,
    permission: 'unsupported',
    calendar_id: calendarId,
    calendar_name: calendarName,
    calendars: [],
    last_error: 'macOS Calendar sync is only available on macOS.',
  };
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
  if (provider === 'macos') {
    return {
      provider,
      status: 'unsupported',
      message: 'macOS Calendar sync is only available on macOS.',
      affected_tasks: 0,
    };
  }
  if (provider === 'feishu') {
    return {
      provider,
      status: 'error',
      message: 'Feishu Calendar sync is available in the desktop app only.',
      affected_tasks: 0,
    };
  }
  if (provider === 'wecom') {
    return {
      provider,
      status: 'error',
      message: 'WeCom Calendar sync is available in the desktop app only.',
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
    note: null,
    task_id: null,
  };
  localStorage.setItem(pomodoroKey, JSON.stringify([session, ...readPomodoroSessions()]));
  return session;
}

function updateLocalPomodoroSessionRecord(input: { id: string; note?: string | null; task_id?: string | null }): PomodoroSession | null {
  const sessions = readPomodoroSessions();
  const session = sessions.find((item) => item.id === input.id);
  if (!session) return null;
  const updated: PomodoroSession = {
    ...session,
    note: normalizeOptionalText(input.note),
    task_id: normalizeOptionalText(input.task_id),
  };
  localStorage.setItem(pomodoroKey, JSON.stringify(sessions.map((item) => item.id === input.id ? updated : item)));
  return updated;
}

function startLocalPomodoroTimer(): PomodoroTimerSnapshot {
  const synced = syncLocalPomodoroTimer();
  if (synced.completion) return startLocalPomodoroTimer();
  const timer = synced.timer;
  const now = nowSeconds();
  const startedAt = now - (timer.duration - timer.remaining);
  return writeLocalPomodoroTimer({
    duration: timer.duration,
    remaining: timer.remaining,
    running: true,
    started_at: startedAt,
    updated_at: now,
  });
}

function pauseLocalPomodoroTimer(): PomodoroTimerSnapshot {
  const synced = syncLocalPomodoroTimer();
  if (synced.completion) return synced;
  const now = nowSeconds();
  return writeLocalPomodoroTimer({
    ...synced.timer,
    remaining: synced.timer.remaining,
    running: false,
    started_at: null,
    updated_at: now,
  });
}

function resetLocalPomodoroTimer(): PomodoroTimerSnapshot {
  const timer = readLocalPomodoroTimer();
  return writeLocalPomodoroTimer({
    duration: timer.duration,
    remaining: timer.duration,
    running: false,
    started_at: null,
    updated_at: nowSeconds(),
  });
}

function setLocalPomodoroTimerDuration(minutes: number): PomodoroTimerSnapshot {
  if (!Number.isFinite(minutes) || minutes < 5 || minutes > 120) {
    throw new Error('pomodoro timer minutes must be between 5 and 120');
  }
  const duration = Math.round(minutes) * 60;
  return writeLocalPomodoroTimer({
    duration,
    remaining: duration,
    running: false,
    started_at: null,
    updated_at: nowSeconds(),
  });
}

function completeLocalPomodoroTimer(): PomodoroTimerSnapshot {
  const timer = readLocalPomodoroTimer();
  const now = nowSeconds();
  const session = createLocalPomodoroSession({
    started_at: timer.started_at ?? now,
    duration: timer.duration,
    completed: true,
  });
  const snapshot = writeLocalPomodoroTimer({
    duration: timer.duration,
    remaining: timer.duration,
    running: false,
    started_at: null,
    updated_at: now,
  });
  return {
    ...snapshot,
    completion: {
      session,
      completed_at: now,
      duration: timer.duration,
    },
  };
}

function readLocalPomodoroTimer(): PomodoroTimerSnapshot['timer'] {
  return normalizeLocalPomodoroTimer(readJson<Partial<PomodoroTimerSnapshot['timer']>>(pomodoroTimerKey, {}));
}

function writeLocalPomodoroTimer(timer: PomodoroTimerSnapshot['timer']): PomodoroTimerSnapshot {
  const normalized = normalizeLocalPomodoroTimer(timer);
  localStorage.setItem(pomodoroTimerKey, JSON.stringify(normalized));
  return { timer: normalized };
}

function normalizeLocalPomodoroTimer(timer: Partial<PomodoroTimerSnapshot['timer']>): PomodoroTimerSnapshot['timer'] {
  const duration = Number.isFinite(timer.duration) && Number(timer.duration) > 0
    ? Math.round(Number(timer.duration))
    : 25 * 60;
  const running = Boolean(timer.running);
  const startedAt = Number.isFinite(timer.started_at) ? Number(timer.started_at) : null;
  const remaining = running && startedAt !== null
    ? Math.max(0, Math.min(duration, duration - (nowSeconds() - startedAt)))
    : Math.max(0, Math.min(duration, Math.round(Number(timer.remaining ?? duration))));
  return {
    duration,
    remaining,
    running: running && remaining > 0,
    started_at: running && startedAt !== null ? startedAt : null,
    updated_at: Number.isFinite(timer.updated_at) ? Number(timer.updated_at) : nowSeconds(),
  };
}

function getLocalTimerRemaining(timer: PomodoroTimerSnapshot['timer']): number {
  if (!timer.running || timer.started_at === null || timer.started_at === undefined) {
    return timer.remaining;
  }
  return Math.max(0, Math.min(timer.duration, timer.duration - (nowSeconds() - timer.started_at)));
}

function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

function normalizeOptionalText(value: string | null | undefined): string | null {
  const trimmed = value?.trim();
  return trimmed ? trimmed : null;
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
