import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AppSettings } from '../types/task';
import {
  broadcastSettingsUpdated,
  broadcastTasksChanged,
  listenForSettingsUpdates,
  listenForTasksChanged,
  settingsUpdatedEvent,
  tasksChangedEvent,
} from './appEvents';

vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn(),
  listen: vi.fn(),
}));

describe('app events', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('does not emit updates outside Tauri', async () => {
    const { emit } = await import('@tauri-apps/api/event');

    await broadcastSettingsUpdated(makeSettings({ cockpit_opacity: 70 }));
    await broadcastTasksChanged();

    expect(emit).not.toHaveBeenCalled();
  });

  it('broadcasts settings updates inside Tauri', async () => {
    const { emit } = await import('@tauri-apps/api/event');
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
    const settings = makeSettings({ cockpit_opacity: 70 });

    await broadcastSettingsUpdated(settings);

    expect(emit).toHaveBeenCalledWith(settingsUpdatedEvent, settings);
  });

  it('broadcasts task changes inside Tauri', async () => {
    const { emit } = await import('@tauri-apps/api/event');
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });

    await broadcastTasksChanged();

    expect(emit).toHaveBeenCalledWith(tasksChangedEvent);
  });

  it('listens for settings updates inside Tauri', async () => {
    const { listen } = await import('@tauri-apps/api/event');
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
    const settings = makeSettings({ cockpit_opacity: 64 });
    const unlisten = vi.fn();
    vi.mocked(listen).mockResolvedValue(unlisten);
    const handler = vi.fn();

    const cleanup = await listenForSettingsUpdates(handler);
    const listener = vi.mocked(listen).mock.calls[0][1];
    listener({ event: settingsUpdatedEvent, id: 1, payload: settings });
    cleanup();

    expect(listen).toHaveBeenCalledWith(settingsUpdatedEvent, expect.any(Function));
    expect(handler).toHaveBeenCalledWith(settings);
    expect(unlisten).toHaveBeenCalled();
  });

  it('listens for task changes inside Tauri', async () => {
    const { listen } = await import('@tauri-apps/api/event');
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });
    const unlisten = vi.fn();
    vi.mocked(listen).mockResolvedValue(unlisten);
    const handler = vi.fn();

    const cleanup = await listenForTasksChanged(handler);
    const listener = vi.mocked(listen).mock.calls[0][1];
    listener({ event: tasksChangedEvent, id: 1, payload: null });
    cleanup();

    expect(listen).toHaveBeenCalledWith(tasksChangedEvent, expect.any(Function));
    expect(handler).toHaveBeenCalled();
    expect(unlisten).toHaveBeenCalled();
  });

  it('uses browser fallback events for settings updates outside Tauri', async () => {
    const windowTarget = createBrowserWindow();
    vi.stubGlobal('window', windowTarget);
    const settings = makeSettings({ cockpit_opacity: 35 });
    const handler = vi.fn();

    const cleanup = await listenForSettingsUpdates(handler);
    await broadcastSettingsUpdated(settings);
    cleanup();

    expect(handler).toHaveBeenCalledWith(settings);
  });

  it('uses browser fallback events for task changes outside Tauri', async () => {
    const windowTarget = createBrowserWindow();
    vi.stubGlobal('window', windowTarget);
    const handler = vi.fn();

    const cleanup = await listenForTasksChanged(handler);
    await broadcastTasksChanged();
    cleanup();

    expect(handler).toHaveBeenCalled();
  });
});

function makeSettings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    theme: 'light',
    default_calendar_view: 'month',
    pomodoro_minutes: 25,
    cockpit_opacity: 92,
    ...overrides,
  };
}

function createBrowserWindow(): Window {
  const target = new EventTarget() as Window;
  Object.defineProperty(target, 'localStorage', {
    configurable: true,
    value: createMemoryStorage(),
  });
  return target;
}

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (key) => data.get(key) ?? null,
    key: (index) => Array.from(data.keys())[index] ?? null,
    removeItem: (key) => data.delete(key),
    setItem: (key, value) => data.set(key, value),
  };
}
