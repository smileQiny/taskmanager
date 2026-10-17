import { create } from 'zustand';
import {
  AppMetadata,
  AppSettings,
  SyncAccount,
  SyncProvider,
  SyncRunResult,
} from '../types/task';
import { appService, settingsService, syncService } from '../services/taskService';
import { broadcastSettingsUpdated } from '../services/appEvents';
import { applyDocumentTheme } from '../utils/appearance';

interface SettingsStore {
  settings: AppSettings;
  metadata: AppMetadata | null;
  syncAccounts: SyncAccount[];
  lastSyncResult: SyncRunResult | null;
  loading: boolean;
  hydrated: boolean;
  googleBusy: 'idle' | 'connecting' | 'syncing';
  syncRevision: number;
  error: string | null;
  fetchSettings: () => Promise<void>;
  applySettings: (settings: AppSettings) => void;
  updateSettings: (input: Partial<AppSettings>) => Promise<void>;
  toggleSyncAccount: (provider: SyncProvider, enabled: boolean) => Promise<void>;
  saveSyncAccountConfig: (provider: SyncProvider, config: string) => Promise<void>;
  saveGoogleSettings: (input: { clientId: string; calendarId: string; clientSecret?: string; enabled?: boolean }) => Promise<void>;
  connectGoogle: () => Promise<void>;
  disconnectGoogle: () => Promise<void>;
  resolveGoogleConflict: (taskId: string, strategy: 'local' | 'remote') => Promise<void>;
  syncNow: (provider: SyncProvider) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: {
    theme: 'light',
    default_calendar_view: 'month',
    pomodoro_minutes: 25,
    cockpit_opacity: 92,
  },
  metadata: null,
  syncAccounts: [],
  lastSyncResult: null,
  loading: false,
  hydrated: false,
  googleBusy: 'idle',
  syncRevision: 0,
  error: null,

  applySettings: (settings) => {
    set({ settings, error: null });
    applyDocumentTheme(settings.theme);
  },

  fetchSettings: async () => {
    set({ loading: true, error: null });
    try {
      const [settings, metadata, syncAccounts] = await Promise.all([
        settingsService.get(),
        appService.getMetadata(),
        syncService.listAccounts(),
      ]);
      set({ settings, metadata, syncAccounts, loading: false, hydrated: true });
      applyDocumentTheme(settings.theme);
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  updateSettings: async (input) => {
    try {
      const settings = await settingsService.update(input);
      useSettingsStore.getState().applySettings(settings);
      await broadcastSettingsUpdated(settings);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  toggleSyncAccount: async (provider, enabled) => {
    try {
      const current = useSettingsStore.getState().syncAccounts.find((item) => item.provider === provider);
      const account = await syncService.upsertAccount({ provider, enabled, config: current?.config });
      set((s) => ({
        syncAccounts: s.syncAccounts.map((item) => item.provider === provider ? account : item),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  saveSyncAccountConfig: async (provider, config) => {
    try {
      const current = useSettingsStore.getState().syncAccounts.find((item) => item.provider === provider);
      const account = await syncService.upsertAccount({
        provider,
        enabled: current?.enabled ?? false,
        config: config.trim() || undefined,
      });
      set((s) => ({
        syncAccounts: s.syncAccounts.map((item) => item.provider === provider ? account : item),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  saveGoogleSettings: async ({ clientId, calendarId, clientSecret, enabled }) => {
    try {
      await syncService.saveGoogleSettings({
        client_id: clientId,
        calendar_id: calendarId,
        client_secret: clientSecret,
        enabled,
      });
      const syncAccounts = await syncService.listAccounts();
      set({ syncAccounts, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  connectGoogle: async () => {
    set({ googleBusy: 'connecting', error: null });
    try {
      await syncService.connectGoogle();
      const syncAccounts = await syncService.listAccounts();
      set({ syncAccounts, googleBusy: 'idle', error: null });
    } catch (e) {
      set({ googleBusy: 'idle', error: String(e) });
      throw e;
    }
  },

  disconnectGoogle: async () => {
    try {
      await syncService.disconnectGoogle();
      const syncAccounts = await syncService.listAccounts();
      set((state) => ({ syncAccounts, error: null, syncRevision: state.syncRevision + 1 }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  resolveGoogleConflict: async (taskId, strategy) => {
    try {
      const result = await syncService.resolveConflict({ task_id: taskId, provider: 'google', strategy });
      const syncAccounts = await syncService.listAccounts();
      set((state) => ({
        syncAccounts,
        lastSyncResult: result,
        error: null,
        syncRevision: state.syncRevision + 1,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  syncNow: async (provider) => {
    if (provider === 'google') set({ googleBusy: 'syncing', error: null });
    try {
      const result = await syncService.syncNow(provider);
      const syncAccounts = provider === 'google' ? await syncService.listAccounts() : useSettingsStore.getState().syncAccounts;
      set((state) => ({
        syncAccounts,
        lastSyncResult: result,
        googleBusy: 'idle',
        error: result.status === 'error' ? result.message : null,
        syncRevision: state.syncRevision + 1,
      }));
    } catch (e) {
      set({ googleBusy: 'idle', error: String(e) });
      throw e;
    }
  },
}));
