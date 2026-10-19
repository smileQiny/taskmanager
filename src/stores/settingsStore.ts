import { create } from 'zustand';
import {
  AppMetadata,
  AppSettings,
  FeishuConnection,
  MacosCalendarConnection,
  SyncAccount,
  WecomConnection,
  SyncProvider,
  SyncRunResult,
} from '../types/task';
import { useTaskStore } from './taskStore';
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
  macosConnection: MacosCalendarConnection | null;
  macosBusy: 'idle' | 'requesting' | 'syncing';
  feishuConnection: FeishuConnection | null;
  feishuBusy: 'idle' | 'connecting' | 'syncing';
  wecomConnection: WecomConnection | null;
  wecomBusy: 'idle' | 'connecting' | 'syncing';
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
  resolveSyncConflict: (provider: SyncProvider, taskId: string, strategy: 'local' | 'remote') => Promise<void>;
  refreshMacosConnection: () => Promise<void>;
  saveMacosCalendar: (input: { calendarId?: string; calendarName?: string; enabled?: boolean }) => Promise<void>;
  requestMacosAccess: () => Promise<void>;
  saveFeishuSettings: (input: { appId: string; calendarId: string; domain: string; appSecret?: string; enabled?: boolean }) => Promise<void>;
  connectFeishu: (input?: { appId: string; calendarId: string; domain: string; appSecret?: string }) => Promise<void>;
  disconnectFeishu: () => Promise<void>;
  saveWecomSettings: (input: { corpId: string; agentId: string; organizerUserId: string; calendarId: string; secret?: string; enabled?: boolean }) => Promise<void>;
  connectWecom: (input?: { corpId: string; agentId: string; organizerUserId: string; calendarId: string; secret?: string }) => Promise<void>;
  disconnectWecom: () => Promise<void>;
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
  macosConnection: null,
  macosBusy: 'idle',
  feishuConnection: null,
  feishuBusy: 'idle',
  wecomConnection: null,
  wecomBusy: 'idle',
  syncRevision: 0,
  error: null,

  applySettings: (settings) => {
    set({ settings, error: null });
    applyDocumentTheme(settings.theme);
  },

  fetchSettings: async () => {
    set({ loading: true, error: null });
    try {
      const [settings, metadata, syncAccounts, macosConnection, feishuConnection, wecomConnection] = await Promise.all([
        settingsService.get(),
        appService.getMetadata(),
        syncService.listAccounts(),
        syncService.macosStatus(),
        syncService.feishuStatus(),
        syncService.wecomStatus(),
      ]);
      set({ settings, metadata, syncAccounts, macosConnection, feishuConnection, wecomConnection, loading: false, hydrated: true });
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
      const wecomConnection = provider === 'wecom'
        ? await syncService.wecomStatus()
        : useSettingsStore.getState().wecomConnection;
      set((s) => ({
        syncAccounts: s.syncAccounts.map((item) => item.provider === provider ? account : item),
        wecomConnection,
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
    await useSettingsStore.getState().resolveSyncConflict('google', taskId, strategy);
  },

  resolveSyncConflict: async (provider, taskId, strategy) => {
    try {
      const result = await syncService.resolveConflict({ task_id: taskId, provider, strategy });
      const [syncAccounts, macosConnection, feishuConnection, wecomConnection] = await Promise.all([
        syncService.listAccounts(),
        provider === 'macos' ? syncService.macosStatus() : Promise.resolve(useSettingsStore.getState().macosConnection),
        provider === 'feishu' ? syncService.feishuStatus() : Promise.resolve(useSettingsStore.getState().feishuConnection),
        provider === 'wecom' ? syncService.wecomStatus() : Promise.resolve(useSettingsStore.getState().wecomConnection),
      ]);
      if (strategy === 'remote') {
        await useTaskStore.getState().fetchTasks();
      }
      set((state) => ({
        syncAccounts,
        macosConnection,
        feishuConnection,
        wecomConnection,
        lastSyncResult: result,
        error: null,
        syncRevision: state.syncRevision + 1,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  refreshMacosConnection: async () => {
    try {
      const macosConnection = await syncService.macosStatus();
      set({ macosConnection, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  saveMacosCalendar: async ({ calendarId, calendarName, enabled }) => {
    try {
      const macosConnection = await syncService.saveMacosSettings({
        calendar_id: calendarId,
        calendar_name: calendarName,
        enabled,
      });
      const syncAccounts = await syncService.listAccounts();
      set({ macosConnection, syncAccounts, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  requestMacosAccess: async () => {
    set({ macosBusy: 'requesting', error: null });
    try {
      const macosConnection = await syncService.requestMacosAccess();
      const syncAccounts = await syncService.listAccounts();
      set({ macosConnection, syncAccounts, macosBusy: 'idle', error: null });
    } catch (e) {
      set({ macosBusy: 'idle', error: String(e) });
      throw e;
    }
  },

  saveFeishuSettings: async ({ appId, calendarId, domain, appSecret, enabled }) => {
    try {
      const feishuConnection = await syncService.saveFeishuSettings({
        app_id: appId,
        calendar_id: calendarId,
        domain,
        app_secret: appSecret,
        enabled,
      });
      const syncAccounts = await syncService.listAccounts();
      set({ feishuConnection, syncAccounts, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  connectFeishu: async (input) => {
    set({ feishuBusy: 'connecting', error: null });
    try {
      if (input) {
        await syncService.saveFeishuSettings({
          app_id: input.appId,
          calendar_id: input.calendarId,
          domain: input.domain,
          app_secret: input.appSecret,
        });
      }
      const feishuConnection = await syncService.connectFeishu();
      const syncAccounts = await syncService.listAccounts();
      set({ feishuConnection, syncAccounts, feishuBusy: 'idle', error: null });
    } catch (e) {
      set({ feishuBusy: 'idle', error: String(e) });
      throw e;
    }
  },

  disconnectFeishu: async () => {
    try {
      const feishuConnection = await syncService.disconnectFeishu();
      const syncAccounts = await syncService.listAccounts();
      set((state) => ({
        feishuConnection,
        syncAccounts,
        error: null,
        syncRevision: state.syncRevision + 1,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  saveWecomSettings: async ({ corpId, agentId, organizerUserId, calendarId, secret, enabled }) => {
    try {
      const wecomConnection = await syncService.saveWecomSettings({
        corp_id: corpId,
        agent_id: agentId,
        organizer_userid: organizerUserId,
        calendar_id: calendarId,
        secret,
        enabled,
      });
      const syncAccounts = await syncService.listAccounts();
      set({ wecomConnection, syncAccounts, error: null });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  connectWecom: async (input) => {
    set({ wecomBusy: 'connecting', error: null });
    try {
      if (input) {
        await syncService.saveWecomSettings({
          corp_id: input.corpId,
          agent_id: input.agentId,
          organizer_userid: input.organizerUserId,
          calendar_id: input.calendarId,
          secret: input.secret,
        });
      }
      const wecomConnection = await syncService.connectWecom();
      const syncAccounts = await syncService.listAccounts();
      set({ wecomConnection, syncAccounts, wecomBusy: 'idle', error: null });
    } catch (e) {
      set({ wecomBusy: 'idle', error: String(e) });
      throw e;
    }
  },

  disconnectWecom: async () => {
    try {
      const wecomConnection = await syncService.disconnectWecom();
      const syncAccounts = await syncService.listAccounts();
      set((state) => ({
        wecomConnection,
        syncAccounts,
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
    if (provider === 'macos') set({ macosBusy: 'syncing', error: null });
    if (provider === 'feishu') set({ feishuBusy: 'syncing', error: null });
    if (provider === 'wecom') set({ wecomBusy: 'syncing', error: null });
    try {
      const result = await syncService.syncNow(provider);
      const refreshAccount = provider === 'google' || provider === 'macos' || provider === 'feishu' || provider === 'wecom';
      const syncAccounts = refreshAccount ? await syncService.listAccounts() : useSettingsStore.getState().syncAccounts;
      const macosConnection = provider === 'macos'
        ? await syncService.macosStatus()
        : useSettingsStore.getState().macosConnection;
      const feishuConnection = provider === 'feishu'
        ? await syncService.feishuStatus()
        : useSettingsStore.getState().feishuConnection;
      const wecomConnection = provider === 'wecom'
        ? await syncService.wecomStatus()
        : useSettingsStore.getState().wecomConnection;
      if (result.status === 'synced') {
        await useTaskStore.getState().fetchTasks();
      }
      set((state) => ({
        syncAccounts,
        macosConnection,
        lastSyncResult: result,
        googleBusy: 'idle',
        macosBusy: 'idle',
        feishuBusy: 'idle',
        feishuConnection,
        wecomBusy: 'idle',
        wecomConnection,
        error: result.status === 'error' ? result.message : null,
        syncRevision: state.syncRevision + 1,
      }));
    } catch (e) {
      set({ googleBusy: 'idle', macosBusy: 'idle', feishuBusy: 'idle', wecomBusy: 'idle', error: String(e) });
      throw e;
    }
  },
}));
