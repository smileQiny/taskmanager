import { useEffect } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { useSettingsStore } from '../stores/settingsStore';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useUpdateStore } from '../stores/updateStore';
import { AppUpdateInfo, UpdateStatus } from '../types/update';
import { SyncAccount, SyncProvider } from '../types/task';
import { minCockpitOpacity, maxCockpitOpacity } from '../utils/cockpitTransparency';

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
  const {
    currentVersion,
    release,
    installResult,
    status: updateStatus,
    error: updateError,
    check: checkForUpdates,
    installLatestUpdate,
    openReleasePage,
  } = useUpdateStore();

  useEffect(() => { fetchSettings(); }, [fetchSettings]);

  const updatePomodoroMinutes = async (minutes: number) => {
    await updateSettings({ pomodoro_minutes: minutes });
    setDurationMinutes(minutes);
  };

  const updateCockpitOpacity = async (opacity: number) => {
    await updateSettings({ cockpit_opacity: opacity });
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TopBar title="设置" subtitle="本地偏好和数据位置" />
      <div className="flex-1 overflow-y-auto p-5">
        {error && <div className="mb-4 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>}
        <div className="max-w-3xl space-y-5">
          <section className="rounded-xl border border-[#dbe5f1] bg-white/86 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-slate-900">偏好</h2>
            <div className="mt-4 grid gap-4 md:grid-cols-3">
              <label className="form-control">
                <span className="label-text mb-1 text-slate-500">主题</span>
                <select className="select select-bordered border-slate-200 bg-white/90" value={settings.theme} onChange={(event) => void updateSettings({ theme: event.target.value as 'light' | 'dark' })}>
                  <option value="light">浅色</option>
                  <option value="dark">深色</option>
                </select>
              </label>
              <label className="form-control">
                <span className="label-text mb-1 text-slate-500">默认日历</span>
                <select className="select select-bordered border-slate-200 bg-white/90" value={settings.default_calendar_view} onChange={(event) => void updateSettings({ default_calendar_view: event.target.value as 'month' | 'week' | 'day' })}>
                  <option value="month">月</option>
                  <option value="week">周</option>
                  <option value="day">日</option>
                </select>
              </label>
              <label className="form-control">
                <span className="label-text mb-1 text-slate-500">番茄钟分钟</span>
                <input className="input input-bordered border-slate-200 bg-white/90" type="number" min={5} max={120} value={settings.pomodoro_minutes} onChange={(event) => void updatePomodoroMinutes(Number(event.target.value))} />
              </label>
              <label className="form-control md:col-span-3">
                <div className="mb-2 flex items-center justify-between gap-3">
                  <span className="label-text text-slate-500">驾驶舱背景不透明度</span>
                  <span className="rounded-full border border-blue-100 bg-blue-50 px-2.5 py-1 text-xs font-semibold tabular-nums text-blue-700">
                    {settings.cockpit_opacity}%
                  </span>
                </div>
                <div className="rounded-xl border border-slate-200 bg-[#f6f8fc] px-4 py-3">
                  <input
                    aria-label="驾驶舱透明度"
                    className="range range-primary range-sm"
                    type="range"
                    min={minCockpitOpacity}
                    max={maxCockpitOpacity}
                    step={1}
                    value={settings.cockpit_opacity}
                    onChange={(event) => void updateCockpitOpacity(Number(event.target.value))}
                  />
                  <div className="mt-2 flex justify-between text-[11px] font-medium text-slate-400">
                    <span>更透明</span>
                    <span>更清晰</span>
                  </div>
                </div>
              </label>
            </div>
          </section>

          <section className="rounded-xl border border-[#dbe5f1] bg-white/86 p-5 shadow-sm">
            <h2 className="text-sm font-semibold text-slate-900">本地数据</h2>
            <div className="mt-4 space-y-3 text-sm">
              <PathRow label="数据目录" value={metadata?.data_dir ?? (loading ? '加载中...' : '未读取')} />
              <PathRow label="数据库" value={metadata?.db_path ?? (loading ? '加载中...' : '未读取')} />
            </div>
          </section>

          <UpdateSection
            currentVersion={currentVersion}
            release={release}
            installResult={installResult}
            status={updateStatus}
            error={updateError}
            onCheck={() => void checkForUpdates('manual')}
            onOpenRelease={() => void openReleasePage()}
            onInstall={() => void installLatestUpdate()}
          />

          <section className="rounded-xl border border-[#dbe5f1] bg-white/86 p-5 shadow-sm">
            <div className="flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-slate-900">远端同步</h2>
                <p className="mt-1 text-sm text-slate-500">先配置同步边界；真实远端同步需要各平台授权凭证。</p>
              </div>
              {lastSyncResult && (
                <span className="rounded bg-slate-100/90 px-2 py-1 text-xs text-slate-600">
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

function UpdateSection({
  currentVersion,
  release,
  installResult,
  status,
  error,
  onCheck,
  onOpenRelease,
  onInstall,
}: {
  currentVersion: string;
  release: AppUpdateInfo | null;
  installResult: { message: string } | null;
  status: UpdateStatus;
  error: string | null;
  onCheck: () => void;
  onOpenRelease: () => void;
  onInstall: () => void;
}) {
  const isChecking = status === 'checking';
  const isInstalling = status === 'installing';
  const canInstall = status === 'update_available' || status === 'update_handled';

  return (
    <section className="overflow-hidden rounded-xl border border-[#dbe5f1] bg-white/86 shadow-sm">
      <div className="border-b border-slate-100 p-5">
        <div className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="text-sm font-semibold text-slate-900">版本与更新</h2>
              <span className={updateBadgeClass(status)}>{updateStatusLabel(status)}</span>
            </div>
            <p className="mt-1 text-sm text-slate-500">
              启动时自动检查 GitHub 最新版本；点击后会下载、安装并重启应用。
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <button className="btn btn-sm border-slate-200 bg-white/90" onClick={onCheck} disabled={isChecking || isInstalling}>
              {isChecking ? '检查中...' : '检查更新'}
            </button>
            <button className="btn btn-primary btn-sm" onClick={onInstall} disabled={!canInstall || isInstalling}>
              {isInstalling ? '下载中...' : '下载并安装'}
            </button>
          </div>
        </div>
      </div>

      <div className="grid gap-0 md:grid-cols-2">
        <VersionMetric label="当前版本" value={`v${currentVersion}`} />
        <VersionMetric
          label="最新版本"
          value={release ? `v${release.latestVersion}` : (isChecking ? '检查中...' : '未检查')}
          accent={canInstall}
        />
      </div>

      {release && (
        <div className="border-t border-slate-100 px-5 py-4">
          <div className="grid gap-3 text-sm md:grid-cols-[1fr_auto] md:items-center">
            <div className="min-w-0">
              <p className="truncate font-medium text-slate-900">{release.releaseName}</p>
              <p className="mt-1 text-slate-500">
                {release.publishedAt ? `发布于 ${formatDateTime(release.publishedAt)}` : 'GitHub 未提供发布时间'}
                {release.preferredAsset ? ` · ${release.preferredAsset.name}` : ' · 暂无可下载安装包'}
              </p>
            </div>
            <button className="btn btn-sm border-slate-200 bg-white/90" onClick={onOpenRelease}>
              查看 Release
            </button>
          </div>
        </div>
      )}

      {error && (
        <div className="border-t border-rose-100 bg-rose-50 px-5 py-3 text-sm text-rose-700">
          {error}
        </div>
      )}

      {status === 'update_handled' && installResult && (
        <div className="border-t border-emerald-100 bg-emerald-50 px-5 py-3 text-sm text-emerald-800">
          <p>{installResult.message}</p>
        </div>
      )}
    </section>
  );
}

function VersionMetric({
  label,
  value,
  accent = false,
}: {
  label: string;
  value: string;
  accent?: boolean;
}) {
  return (
    <div className="border-t border-slate-100 px-5 py-4 md:border-t-0 md:first:border-r">
      <p className="text-xs text-slate-500">{label}</p>
      <p className={`mt-1 text-lg font-semibold ${accent ? 'text-emerald-700' : 'text-slate-900'}`}>{value}</p>
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
            <span className="rounded bg-slate-100/90 px-2 py-0.5 text-xs text-slate-500">{statusLabel(account.status)}</span>
          </div>
          <p className="mt-1 text-sm text-slate-500">{providerDescriptions[account.provider]}</p>
        </div>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">非密钥配置 JSON</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
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
      <code className="block overflow-x-auto rounded-lg bg-slate-100/90 px-3 py-2 text-xs text-slate-600">{value}</code>
    </div>
  );
}

function updateStatusLabel(status: UpdateStatus): string {
  return {
    idle: '未检查',
    checking: '检查中',
    update_available: '有新版本',
    up_to_date: '已是最新',
    installing: '下载中',
    update_handled: '更新已处理',
    error: '检查失败',
  }[status];
}

function updateBadgeClass(status: UpdateStatus): string {
  const base = 'rounded px-2 py-0.5 text-xs';
  if (status === 'update_available') return `${base} bg-emerald-50 text-emerald-700`;
  if (status === 'update_handled') return `${base} bg-emerald-50 text-emerald-700`;
  if (status === 'error') return `${base} bg-rose-50 text-rose-700`;
  if (status === 'checking') return `${base} bg-sky-50 text-sky-700`;
  if (status === 'installing') return `${base} bg-sky-50 text-sky-700`;
  return `${base} bg-slate-100/90 text-slate-600`;
}

function formatDateTime(value: string): string {
  return new Intl.DateTimeFormat('zh-CN', {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value));
}
