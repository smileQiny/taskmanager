import { emit, listen, type Event as TauriEvent } from '@tauri-apps/api/event';
import { AppSettings } from '../types/task';

export const settingsUpdatedEvent = 'settings-updated';
export const tasksChangedEvent = 'tasks-changed';

type Unlisten = () => void;
const settingsStorageKey = 'taskmanager.event.settings-updated';
const tasksStorageKey = 'taskmanager.event.tasks-changed';

const hasTauri = () => (
  typeof window !== 'undefined'
  && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
);

export async function broadcastSettingsUpdated(settings: AppSettings): Promise<void> {
  if (hasTauri()) {
    await emit(settingsUpdatedEvent, settings);
    return;
  }
  dispatchBrowserEvent(settingsUpdatedEvent, settings);
  writeStorageEvent(settingsStorageKey, settings);
}

export async function listenForSettingsUpdates(
  handler: (settings: AppSettings) => void,
): Promise<Unlisten> {
  if (!hasTauri()) {
    return listenForBrowserEvent<AppSettings>(settingsUpdatedEvent, settingsStorageKey, handler);
  }
  return listen<AppSettings>(settingsUpdatedEvent, (event: TauriEvent<AppSettings>) => {
    handler(event.payload);
  });
}

export async function broadcastTasksChanged(): Promise<void> {
  if (hasTauri()) {
    await emit(tasksChangedEvent);
    return;
  }
  dispatchBrowserEvent(tasksChangedEvent);
  writeStorageEvent(tasksStorageKey, null);
}

export async function listenForTasksChanged(handler: () => void): Promise<Unlisten> {
  if (!hasTauri()) {
    return listenForBrowserEvent(tasksChangedEvent, tasksStorageKey, handler);
  }
  return listen(tasksChangedEvent, () => {
    handler();
  });
}

function dispatchBrowserEvent<T>(eventName: string, payload?: T) {
  if (typeof window === 'undefined') return;
  window.dispatchEvent(new CustomEvent(eventName, { detail: payload }));
}

function writeStorageEvent<T>(key: string, payload: T) {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(key, JSON.stringify({
      id: crypto.randomUUID(),
      payload,
    }));
  } catch {
    // Storage is best-effort in browser preview.
  }
}

function listenForBrowserEvent<T>(
  eventName: string,
  storageKey: string,
  handler: (payload: T) => void,
): Unlisten {
  if (typeof window === 'undefined') return noop;

  const eventHandler = (event: Event) => {
    handler((event as CustomEvent<T>).detail);
  };
  const storageHandler = (event: StorageEvent) => {
    if (event.key !== storageKey || !event.newValue) return;
    try {
      handler(JSON.parse(event.newValue).payload as T);
    } catch {
      // Ignore malformed preview events from external tooling.
    }
  };

  window.addEventListener(eventName, eventHandler);
  window.addEventListener('storage', storageHandler);
  return () => {
    window.removeEventListener(eventName, eventHandler);
    window.removeEventListener('storage', storageHandler);
  };
}

function noop() {
  // No global event target exists during server-side tests.
}
