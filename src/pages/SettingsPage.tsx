import { useEffect } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { useSettingsStore } from '../stores/settingsStore';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { SyncAccount, SyncProvider } from '../types/task';

const providerLabels: Record<SyncProvider, string> = {
  feishu: '飞书日历',
  macos: 'macOS 日历',
  wecom: '企业微信',
  google: 'Google Calendar',
};

const providerDescriptions: Record<SyncProvider, string> = {
  feishu: '优先级最高。需要 OAuth 应用凭证后启用真实同步。',
  macos: '本机日历桥接。需要系统日历授权后启用真实同步。',
  wecom: '需要企业微信 CorpID 和 OAuth 配置。',
  google: '后续通过 Google Calendar OAuth 接入。',
};

export function SettingsPage() {
  const {
    settings,
    metadata,
    syncAccounts,
    lastSyncResult,
    loading,
    error,
    fetchSettings,
    updateSettings,
    toggleSyncAccount,
    saveSyncAccountConfig,
    syncNow,
  } = useSettingsStore();
  const { setDurationMinutes } = usePomodoroStore();
  useEffect(() => { fetchSettings(); }, [fetchSettings]);

  const updatePomodoroMinutes = async (minutes: number) => {
    await updateSettings({ pomodoro_minutes: minutes });
    setDurationMinutes(minutes);
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TopBar title="设置" subtitle="本地偏好和数据位置" />
      <div className="flex-1 overflow-y-auto p-5">
        {error && <div className="mb-4 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>}
        <div className="max-w-3xl space-y-5">
          <section className="rounded-md border border-slate-200 bg-white p-5">
            <h2 className="text-sm font-semibold text-slate-950">偏好</h2>
            <div className="mt-4 grid gap-4 md:grid-cols-3">
              <label className="form-control">
                <span className="label-text mb-1 text-slate-500">主题</span>
                <select className="select select-bordered bg-white" value={settings.theme} onChange={(event) => void updateSettings({ theme: event.target.value as 'light' | 'dark' })}>
                  <option value="light">浅色</option>
                  <option value="dark">深色</option>
                </select>
              </label>
              <label className="form-control">
                <span className="label-text mb-1 text-slate-500">默认日历</span>
                <select className="select select-bordered bg-white" value={settings.default_calendar_view} onChange={(event) => void updateSettings({ default_calendar_view: event.target.value as 'month' | 'week' | 'day' })}>
                  <option value="month">月</option>
                  <option value="week">周</option>
                  <option value="day">日</option>
                </select>
              </label>
              <label className="form-control">
                <span className="label-text mb-1 text-slate-500">番茄钟分钟</span>
                <input className="input input-bordered bg-white" type="number" min={5} max={120} value={settings.pomodoro_minutes} onChange={(event) => void updatePomodoroMinutes(Number(event.target.value))} />
              </label>
            </div>
          </section>

          <section className="rounded-md border border-slate-200 bg-white p-5">
            <h2 className="text-sm font-semibold text-slate-950">本地数据</h2>
            <div className="mt-4 space-y-3 text-sm">
              <PathRow label="数据目录" value={metadata?.data_dir ?? (loading ? '加载中...' : '未读取')} />
              <PathRow label="数据库" value={metadata?.db_path ?? (loading ? '加载中...' : '未读取')} />
            </div>
          </section>

          <section className="rounded-md border border-slate-200 bg-white p-5">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-slate-950">远端同步</h2>
                <p className="mt-1 text-sm text-slate-500">先配置同步边界；真实远端同步需要各平台授权凭证。</p>
              </div>
              {lastSyncResult && (
                <span className="rounded bg-slate-100 px-2 py-1 text-xs text-slate-600">
                  {providerLabels[lastSyncResult.provider]} · {lastSyncResult.status}
                </span>
              )}
            </div>
            {lastSyncResult && (
              <div className="mt-4 rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800">
                {lastSyncResult.message}
                <span className="ml-2 text-amber-700">
                  影响任务：{lastSyncResult.affected_tasks}
                </span>
              </div>
            )}
            <div className="mt-4 divide-y divide-slate-100">
              {syncAccounts.map((account) => (
                <SyncProviderRow
                  key={account.provider}
                  account={account}
                  onToggle={(enabled) => void toggleSyncAccount(account.provider, enabled)}
                  onConfigSave={(config) => void saveSyncAccountConfig(account.provider, config)}
                  onSync={() => void syncNow(account.provider)}
                />
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function SyncProviderRow({
  account,
  onToggle,
  onConfigSave,
  onSync,
}: {
  account: SyncAccount;
  onToggle: (enabled: boolean) => void;
  onConfigSave: (config: string) => void;
  onSync: () => void;
}) {
  return (
    <div className="grid gap-3 py-4 lg:grid-cols-[minmax(0,1fr)_220px]">
      <div className="min-w-0 space-y-3">
        <div>
          <div className="flex items-center gap-2">
            <p className="font-medium text-slate-900">{providerLabels[account.provider]}</p>
            <span className="rounded bg-slate-100 px-2 py-0.5 text-xs text-slate-500">{statusLabel(account.status)}</span>
          </div>
          <p className="mt-1 text-sm text-slate-500">{providerDescriptions[account.provider]}</p>
        </div>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">非密钥配置 JSON</span>
          <input
            className="input input-bordered input-sm bg-white font-mono text-xs"
            defaultValue={account.config ?? defaultProviderConfig(account.provider)}
            onBlur={(event) => onConfigSave(event.target.value)}
            placeholder={defaultProviderConfig(account.provider)}
          />
        </label>
      </div>
      <div className="flex shrink-0 items-center justify-end gap-2">
        <input
          type="checkbox"
          className="toggle toggle-primary"
          checked={account.enabled}
          onChange={(event) => onToggle(event.target.checked)}
          aria-label={`${providerLabels[account.provider]} 启用`}
        />
        <button className="btn btn-sm" onClick={onSync}>立即同步</button>
      </div>
    </div>
  );
}

function defaultProviderConfig(provider: SyncProvider): string {
  return {
    feishu: '{"calendar_id":"primary"}',
    macos: '{"calendar_name":"Calendar"}',
    wecom: '{"corp_id":"","calendar_id":"primary"}',
    google: '{"calendar_id":"primary"}',
  }[provider];
}

function statusLabel(status: string): string {
  return {
    not_configured: '未配置',
    pending_auth: '等待授权',
    needs_auth: '需要授权',
    synced: '已同步',
    error: '错误',
  }[status] ?? status;
}

function PathRow({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="mb-1 text-xs text-slate-500">{label}</p>
      <code className="block overflow-x-auto rounded bg-slate-100 px-3 py-2 text-xs text-slate-600">{value}</code>
    </div>
  );
}
