import { create } from 'zustand';
import {
  AppMetadata,
  AppSettings,
  SyncAccount,
  SyncProvider,
  SyncRunResult,
} from '../types/task';
import { appService, settingsService, syncService } from '../services/taskService';

interface SettingsStore {
  settings: AppSettings;
  metadata: AppMetadata | null;
  syncAccounts: SyncAccount[];
  lastSyncResult: SyncRunResult | null;
  loading: boolean;
  error: string | null;
  fetchSettings: () => Promise<void>;
  updateSettings: (input: Partial<AppSettings>) => Promise<void>;
  toggleSyncAccount: (provider: SyncProvider, enabled: boolean) => Promise<void>;
  saveSyncAccountConfig: (provider: SyncProvider, config: string) => Promise<void>;
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
  error: null,

  fetchSettings: async () => {
    set({ loading: true, error: null });
    try {
      const [settings, metadata, syncAccounts] = await Promise.all([
        settingsService.get(),
        appService.getMetadata(),
        syncService.listAccounts(),
      ]);
      set({ settings, metadata, syncAccounts, loading: false });
      document.documentElement.setAttribute('data-theme', settings.theme);
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  updateSettings: async (input) => {
    try {
      const settings = await settingsService.update(input);
      set({ settings, error: null });
      document.documentElement.setAttribute('data-theme', settings.theme);
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

  syncNow: async (provider) => {
    try {
      const result = await syncService.syncNow(provider);
      set({ lastSyncResult: result, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },
}));
