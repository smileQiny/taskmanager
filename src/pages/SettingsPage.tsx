import { useEffect, useState } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { useSettingsStore } from '../stores/settingsStore';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useUpdateStore } from '../stores/updateStore';
import { AppUpdateInfo, InstallUpdateResult, UpdateStatus } from '../types/update';
import { SyncAccount, SyncProvider } from '../types/task';
import { minCockpitOpacity, maxCockpitOpacity } from '../utils/cockpitTransparency';
import { googleActions, googleConnectionPhase, isDesktopRuntime } from '../utils/googleSync';
import { feishuActions, feishuConnectionPhase, FEISHU_REDIRECT_URI } from '../utils/feishuSync';
import { MACOS_SYNC_DIRECTIONS, macosActions, macosConnectionPhase, macosPermissionHint, parseMacosSyncDirection } from '../utils/macosSync';
import { wecomActions, wecomConnectionPhase } from '../utils/wecomSync';
import { FeishuConnection, MacosCalendarConnection, WecomConnection } from '../types/task';

const providerLabels: Record<SyncProvider, string> = {
  feishu: '飞书日历',
  macos: 'macOS 日历',
  wecom: '企业微信',
  google: 'Google Calendar',
};

const providerDescriptions: Record<SyncProvider, string> = {
  feishu: '桌面端通过 PKCE 和固定回环地址授权后，按任务勾选与飞书或 Lark 日历双向同步。',
  macos: '仅 macOS。开启后自动同步所有带开始和结束时间的本地任务（含已完成），以及所选日历在过去 90 天到未来 180 天内的事件。',
  wecom: '桌面端使用企业自建应用凭证同步已勾选任务。企业微信没有用户 OAuth 日历范围，日程写到本应用创建的日历。',
  google: '桌面端通过 PKCE 授权后，按任务勾选与 Google Calendar 双向同步。',
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
    saveGoogleSettings,
    connectGoogle,
    disconnectGoogle,
    syncNow,
    googleBusy,
    macosConnection,
    macosBusy,
    saveMacosCalendar,
    requestMacosAccess,
    feishuConnection,
    feishuBusy,
    saveFeishuSettings,
    connectFeishu,
    disconnectFeishu,
    wecomConnection,
    wecomBusy,
    saveWecomSettings,
    connectWecom,
    disconnectWecom,
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
    await setDurationMinutes(minutes);
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
              {syncAccounts.map((account) => account.provider === 'feishu' ? (
                <FeishuSyncRow
                  key={account.provider}
                  account={account}
                  connection={feishuConnection}
                  busy={feishuBusy}
                  onToggle={(enabled) => void toggleSyncAccount(account.provider, enabled)}
                  onSave={(input) => void saveFeishuSettings(input).catch(() => undefined)}
                  onConnect={(input) => void connectFeishu(input).catch(() => undefined)}
                  onDisconnect={() => void disconnectFeishu().catch(() => undefined)}
                  onSync={() => void syncNow(account.provider)}
                />
              ) : account.provider === 'google' ? (
                <GoogleSyncRow
                  key={account.provider}
                  account={account}
                  busy={googleBusy}
                  onToggle={(enabled) => void toggleSyncAccount(account.provider, enabled)}
                  onSave={(input) => void saveGoogleSettings(input).catch(() => undefined)}
                  onConnect={() => void connectGoogle().catch(() => undefined)}
                  onDisconnect={() => void disconnectGoogle().catch(() => undefined)}
                  onSync={() => void syncNow(account.provider)}
                />
              ) : account.provider === 'wecom' ? (
                <WecomSyncRow
                  key={account.provider}
                  account={account}
                  connection={wecomConnection}
                  busy={wecomBusy}
                  onToggle={(enabled) => void toggleSyncAccount(account.provider, enabled)}
                  onSave={(input) => void saveWecomSettings(input).catch(() => undefined)}
                  onConnect={(input) => void connectWecom(input).catch(() => undefined)}
                  onDisconnect={() => void disconnectWecom().catch(() => undefined)}
                  onSync={() => void syncNow(account.provider)}
                />
              ) : account.provider === 'macos' ? (
                <MacosSyncRow
                  key={account.provider}
                  account={account}
                  connection={macosConnection}
                  busy={macosBusy}
                  onToggle={(enabled) => void toggleSyncAccount(account.provider, enabled)}
                  onSelectCalendar={(calendarId, calendarName) => void saveMacosCalendar({
                    calendarId,
                    calendarName,
                    enabled: account.enabled,
                  }).catch(() => undefined)}
                  onDirection={(syncDirection) => void saveMacosCalendar({
                    calendarId: macosConnection?.calendar_id,
                    calendarName: macosConnection?.calendar_name,
                    enabled: account.enabled,
                    syncDirection,
                  }).catch(() => undefined)}
                  onRequestAccess={() => void requestMacosAccess().catch(() => undefined)}
                  onSync={() => void syncNow(account.provider)}
                />
              ) : (
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
  installResult: InstallUpdateResult | null;
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
              启动时自动检查 GitHub 最新版本；点击后自动下载安装并重启。
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <button className="btn btn-sm border-slate-200 bg-white/90" onClick={onCheck} disabled={isChecking || isInstalling}>
              {isChecking ? '检查中...' : '检查更新'}
            </button>
            <button className="btn btn-primary btn-sm" onClick={onInstall} disabled={!canInstall || isInstalling}>
              {isInstalling ? '安装中...' : '安装更新'}
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
          {installResult.manualCommand && (
            <code className="mt-2 block overflow-x-auto rounded-lg border border-emerald-200 bg-white/80 px-3 py-2 font-mono text-xs text-emerald-950">
              {installResult.manualCommand}
            </code>
          )}
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

function FeishuSyncRow({
  account,
  connection,
  busy,
  onToggle,
  onSave,
  onConnect,
  onDisconnect,
  onSync,
}: {
  account: SyncAccount;
  connection: FeishuConnection | null;
  busy: 'idle' | 'connecting' | 'syncing';
  onToggle: (enabled: boolean) => void;
  onSave: (input: { appId: string; calendarId: string; domain: string; appSecret?: string }) => void;
  onConnect: (input: { appId: string; calendarId: string; domain: string; appSecret?: string }) => void;
  onDisconnect: () => void;
  onSync: () => void;
}) {
  const desktop = isDesktopRuntime();
  const parsed = parseFeishuConfig(connection, account.config);
  const [appId, setAppId] = useState(parsed.appId);
  const [calendarId, setCalendarId] = useState(parsed.calendarId);
  const [domain, setDomain] = useState(parsed.domain);
  const [secretDraft, setSecretDraft] = useState('');
  useEffect(() => {
    setAppId(parsed.appId);
    setCalendarId(parsed.calendarId);
    setDomain(parsed.domain);
  }, [parsed.appId, parsed.calendarId, parsed.domain]);
  const phase = feishuConnectionPhase({
    desktop,
    appId,
    hasAppSecret: Boolean(connection?.has_app_secret) || secretDraft.trim().length > 0,
    status: connection?.status ?? account.status,
    busy,
  });
  const actions = feishuActions(phase);
  const redirectUri = connection?.redirect_uri || FEISHU_REDIRECT_URI;
  const save = (secret?: string) => onSave({
    appId,
    calendarId,
    domain,
    appSecret: secret,
  });

  return (
    <div className="grid gap-3 py-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <p className="font-medium text-slate-900">{providerLabels.feishu}</p>
            <span className="rounded bg-slate-100/90 px-2 py-0.5 text-xs text-slate-500">
              {statusLabel(connection?.status ?? account.status)}
            </span>
          </div>
          <p className="mt-1 text-sm text-slate-500">{providerDescriptions.feishu}</p>
        </div>
        <input
          type="checkbox"
          className="toggle toggle-primary"
          checked={account.enabled}
          onChange={(event) => onToggle(event.target.checked)}
          aria-label="飞书日历启用"
        />
      </div>
      {phase === 'desktop_only' && (
        <div className="rounded-md bg-slate-100/90 px-3 py-2 text-sm text-slate-600">
          浏览器预览不能连接飞书。请在桌面版中授权。当前环境不会把同步显示为成功。
        </div>
      )}
      {connection?.last_error && phase !== 'desktop_only' && (
        <div className="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{connection.last_error}</div>
      )}
      <div className="grid gap-3 md:grid-cols-2">
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">App ID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={appId}
            onChange={(event) => setAppId(event.target.value)}
            onBlur={() => save()}
            placeholder="cli_xxx"
          />
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">日历 ID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={calendarId}
            onChange={(event) => setCalendarId(event.target.value)}
            onBlur={() => save()}
            placeholder="primary"
          />
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">开放平台</span>
          <select
            className="select select-bordered select-sm border-slate-200 bg-white/90"
            value={domain}
            onChange={(event) => {
              const next = event.target.value;
              setDomain(next);
              onSave({ appId, calendarId, domain: next });
            }}
          >
            <option value="feishu">飞书（open.feishu.cn）</option>
            <option value="lark">Lark（open.larksuite.com）</option>
          </select>
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">
            App Secret（只写入本机，不会回显）{connection?.has_app_secret ? ' · 已保存' : ''}
          </span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            type="password"
            value={secretDraft}
            onChange={(event) => setSecretDraft(event.target.value)}
            placeholder="留空表示不修改"
            onBlur={() => {
              const secret = secretDraft.trim();
              if (!secret) return;
              save(secret);
              setSecretDraft('');
            }}
          />
        </label>
      </div>
      <div className="rounded-md bg-slate-100/90 px-3 py-2 text-xs text-slate-600">
        在开放平台「安全设置 → 重定向 URL」加入
        <code className="mx-1 font-mono">{redirectUri}</code>
      </div>
      <div className="flex flex-wrap justify-end gap-2">
        <button
          className="btn btn-sm"
          onClick={() => onConnect({
            appId,
            calendarId,
            domain,
            appSecret: secretDraft.trim() || undefined,
          })}
          disabled={!actions.connect || busy !== 'idle'}
        >
          {busy === 'connecting' ? '连接中...' : '连接飞书'}
        </button>
        <button className="btn btn-sm" onClick={onDisconnect} disabled={!actions.disconnect || busy !== 'idle'}>断开</button>
        <button className="btn btn-sm btn-primary" onClick={onSync} disabled={!actions.sync || busy !== 'idle'}>
          {busy === 'syncing' ? '同步中...' : '立即同步'}
        </button>
      </div>
    </div>
  );
}

function parseFeishuConfig(connection: FeishuConnection | null, config?: string): { appId: string; calendarId: string; domain: string } {
  if (connection) {
    return {
      appId: connection.app_id,
      calendarId: connection.calendar_id || 'primary',
      domain: connection.domain === 'lark' ? 'lark' : 'feishu',
    };
  }
  try {
    const parsed = JSON.parse(config || '{}') as { app_id?: string; calendar_id?: string; domain?: string };
    return {
      appId: parsed.app_id ?? '',
      calendarId: parsed.calendar_id || 'primary',
      domain: parsed.domain === 'lark' ? 'lark' : 'feishu',
    };
  } catch {
    return { appId: '', calendarId: 'primary', domain: 'feishu' };
  }
}

function GoogleSyncRow({
  account,
  busy,
  onToggle,
  onSave,
  onConnect,
  onDisconnect,
  onSync,
}: {
  account: SyncAccount;
  busy: 'idle' | 'connecting' | 'syncing';
  onToggle: (enabled: boolean) => void;
  onSave: (input: { clientId: string; calendarId: string; clientSecret?: string }) => void;
  onConnect: () => void;
  onDisconnect: () => void;
  onSync: () => void;
}) {
  const desktop = isDesktopRuntime();
  const parsed = parseGoogleConfig(account.config);
  const [clientId, setClientId] = useState(parsed.clientId);
  const [calendarId, setCalendarId] = useState(parsed.calendarId);
  useEffect(() => {
    setClientId(parsed.clientId);
    setCalendarId(parsed.calendarId);
  }, [parsed.clientId, parsed.calendarId]);
  const phase = googleConnectionPhase({
    desktop,
    clientId,
    status: account.status,
    busy,
  });
  const actions = googleActions(phase);
  const save = (secret?: string) => onSave({
    clientId,
    calendarId,
    clientSecret: secret,
  });

  return (
    <div className="grid gap-3 py-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <p className="font-medium text-slate-900">{providerLabels.google}</p>
            <span className="rounded bg-slate-100/90 px-2 py-0.5 text-xs text-slate-500">{statusLabel(account.status)}</span>
          </div>
          <p className="mt-1 text-sm text-slate-500">{providerDescriptions.google}</p>
        </div>
        <input
          type="checkbox"
          className="toggle toggle-primary"
          checked={account.enabled}
          onChange={(event) => onToggle(event.target.checked)}
          aria-label="Google Calendar 启用"
        />
      </div>
      {phase === 'desktop_only' && (
        <div className="rounded-md bg-slate-100/90 px-3 py-2 text-sm text-slate-600">
          浏览器预览不能连接 Google。请在桌面版中授权。
        </div>
      )}
      {account.last_error && (
        <div className="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{account.last_error}</div>
      )}
      <div className="grid gap-3 md:grid-cols-2">
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">OAuth Client ID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={clientId}
            onChange={(event) => setClientId(event.target.value)}
            onBlur={() => save()}
            placeholder="apps.googleusercontent.com"
          />
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">日历 ID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={calendarId}
            onChange={(event) => setCalendarId(event.target.value)}
            onBlur={() => save()}
            placeholder="primary"
          />
        </label>
        <label className="form-control md:col-span-2">
          <span className="label-text mb-1 text-slate-500">OAuth Client Secret（可选，只写入本机，不会回显）</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            type="password"
            defaultValue=""
            placeholder="留空表示不修改"
            onBlur={(event) => {
              const secret = event.target.value.trim();
              if (!secret) return;
              save(secret);
              event.target.value = '';
            }}
          />
        </label>
      </div>
      <div className="flex flex-wrap justify-end gap-2">
        <button className="btn btn-sm" onClick={onConnect} disabled={!actions.connect || busy !== 'idle'}>
          {busy === 'connecting' ? '连接中...' : '连接 Google'}
        </button>
        <button className="btn btn-sm" onClick={onDisconnect} disabled={!actions.disconnect || busy !== 'idle'}>断开</button>
        <button className="btn btn-sm btn-primary" onClick={onSync} disabled={!actions.sync || busy !== 'idle'}>
          {busy === 'syncing' ? '同步中...' : '立即同步'}
        </button>
      </div>
    </div>
  );
}

function parseGoogleConfig(config?: string): { clientId: string; calendarId: string } {
  try {
    const parsed = JSON.parse(config || '{}') as { client_id?: string; calendar_id?: string };
    return {
      clientId: parsed.client_id ?? '',
      calendarId: parsed.calendar_id || 'primary',
    };
  } catch {
    return { clientId: '', calendarId: 'primary' };
  }
}

function MacosSyncRow({
  account,
  connection,
  busy,
  onToggle,
  onSelectCalendar,
  onDirection,
  onRequestAccess,
  onSync,
}: {
  account: SyncAccount;
  connection: MacosCalendarConnection | null;
  busy: 'idle' | 'requesting' | 'syncing';
  onToggle: (enabled: boolean) => void;
  onSelectCalendar: (calendarId: string, calendarName: string) => void;
  onDirection: (direction: 'bidirectional' | 'local_to_remote' | 'remote_to_local') => void;
  onRequestAccess: () => void;
  onSync: () => void;
}) {
  const supported = connection?.supported ?? false;
  const permission = connection?.permission ?? 'unsupported';
  const phase = macosConnectionPhase({
    supported,
    enabled: account.enabled,
    permission,
    status: connection?.status ?? account.status,
    busy,
  });
  const actions = macosActions(phase);
  const calendars = connection?.calendars ?? [];
  const calendarId = connection?.calendar_id ?? '';
  const hint = macosPermissionHint(permission);
  const selectedName = calendars.find((calendar) => calendar.id === calendarId)?.name
    ?? connection?.calendar_name
    ?? 'Calendar';

  return (
    <div className="grid gap-3 py-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <p className="font-medium text-slate-900">{providerLabels.macos}</p>
            <span className="rounded bg-slate-100/90 px-2 py-0.5 text-xs text-slate-500">
              {statusLabel(connection?.status ?? account.status)}
            </span>
          </div>
          <p className="mt-1 text-sm text-slate-500">{providerDescriptions.macos}</p>
        </div>
        <input
          type="checkbox"
          className="toggle toggle-primary"
          checked={account.enabled}
          onChange={(event) => onToggle(event.target.checked)}
          aria-label="macOS 日历启用"
        />
      </div>
      {phase === 'macos_only' && (
        <div className="rounded-md bg-slate-100/90 px-3 py-2 text-sm text-slate-600">
          macOS 日历同步只能在 macOS 上使用。当前系统不会执行同步，也不会显示为已成功。
        </div>
      )}
      {phase !== 'macos_only' && hint && phase === 'needs_auth' && (
        <div className="rounded-md bg-amber-50 px-3 py-2 text-sm text-amber-800">{hint}</div>
      )}
      {connection?.last_error && phase !== 'macos_only' && phase !== 'needs_auth' && phase !== 'authenticated' && (
        <div className="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{connection.last_error}</div>
      )}
      <label className="form-control">
        <span className="label-text mb-1 text-slate-500">日历</span>
        <select
          className="select select-bordered select-sm border-slate-200 bg-white/90"
          value={calendarId}
          disabled={!supported || permission !== 'full_access' || calendars.length === 0}
          onChange={(event) => {
            const nextId = event.target.value;
            const nextName = calendars.find((calendar) => calendar.id === nextId)?.name ?? selectedName;
            onSelectCalendar(nextId, nextName);
          }}
        >
          {calendars.length === 0 && (
            <option value={calendarId}>{supported ? '授权后可选择日历' : selectedName}</option>
          )}
          {calendars.length > 0 && calendarId === '' && <option value="">默认日历</option>}
          {calendars.map((calendar) => (
            <option key={calendar.id} value={calendar.id}>{calendar.name}</option>
          ))}
        </select>
      </label>
      <label className="form-control">
        <span className="label-text mb-1 text-slate-500">同步方向</span>
        <select
          className="select select-bordered select-sm border-slate-200 bg-white/90"
          value={parseMacosSyncDirection(connection?.sync_direction)}
          onChange={(event) => onDirection(parseMacosSyncDirection(event.target.value))}
          aria-label="macOS 同步方向"
        >
          {MACOS_SYNC_DIRECTIONS.map((direction) => (
            <option key={direction.value} value={direction.value}>{direction.label}</option>
          ))}
        </select>
      </label>
      <p className="text-xs text-slate-500">
        双向会同时推送和导入。仅本地 → 日历不会把未映射的日历事件变成任务。仅日历 → 本地不会把仅存在于本地的任务推到日历。
      </p>
      <div className="flex flex-wrap justify-end gap-2">
        <button className="btn btn-sm" onClick={onRequestAccess} disabled={!actions.requestAccess || busy !== 'idle'}>
          {busy === 'requesting' ? '请求中...' : '请求日历权限'}
        </button>
        <button className="btn btn-sm btn-primary" onClick={onSync} disabled={!actions.sync || busy !== 'idle'}>
          {busy === 'syncing' ? '同步中...' : '立即同步'}
        </button>
      </div>
    </div>
  );
}

function WecomSyncRow({
  account,
  connection,
  busy,
  onToggle,
  onSave,
  onConnect,
  onDisconnect,
  onSync,
}: {
  account: SyncAccount;
  connection: WecomConnection | null;
  busy: 'idle' | 'connecting' | 'syncing';
  onToggle: (enabled: boolean) => void;
  onSave: (input: { corpId: string; agentId: string; organizerUserId: string; calendarId: string; secret?: string }) => void;
  onConnect: (input: { corpId: string; agentId: string; organizerUserId: string; calendarId: string; secret?: string }) => void;
  onDisconnect: () => void;
  onSync: () => void;
}) {
  const desktop = isDesktopRuntime();
  const parsed = parseWecomConfig(connection, account.config);
  const [corpId, setCorpId] = useState(parsed.corpId);
  const [agentId, setAgentId] = useState(parsed.agentId);
  const [organizerUserId, setOrganizerUserId] = useState(parsed.organizerUserId);
  const [calendarId, setCalendarId] = useState(parsed.calendarId);
  const [secretDraft, setSecretDraft] = useState('');
  useEffect(() => {
    setCorpId(parsed.corpId);
    setAgentId(parsed.agentId);
    setOrganizerUserId(parsed.organizerUserId);
    setCalendarId(parsed.calendarId);
  }, [parsed.corpId, parsed.agentId, parsed.organizerUserId, parsed.calendarId]);
  const phase = wecomConnectionPhase({
    desktop,
    corpId,
    agentId,
    organizerUserId,
    hasSecret: Boolean(connection?.has_secret) || secretDraft.trim().length > 0,
    status: connection?.status ?? account.status,
    busy,
  });
  const actions = wecomActions(phase);
  const save = (secret?: string) => onSave({
    corpId,
    agentId,
    organizerUserId,
    calendarId,
    secret,
  });

  return (
    <div className="grid gap-3 py-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <p className="font-medium text-slate-900">{providerLabels.wecom}</p>
            <span className="rounded bg-slate-100/90 px-2 py-0.5 text-xs text-slate-500">
              {statusLabel(connection?.status ?? account.status)}
            </span>
          </div>
          <p className="mt-1 text-sm text-slate-500">{providerDescriptions.wecom}</p>
        </div>
        <input
          type="checkbox"
          className="toggle toggle-primary"
          checked={account.enabled}
          onChange={(event) => onToggle(event.target.checked)}
          aria-label="企业微信启用"
        />
      </div>
      {phase === 'desktop_only' && (
        <div className="rounded-md bg-slate-100/90 px-3 py-2 text-sm text-slate-600">
          浏览器预览不能连接企业微信。请在桌面版中配置企业自建应用。当前环境不会把同步显示为成功。
        </div>
      )}
      {connection?.last_error && phase !== 'desktop_only' && (
        <div className="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{connection.last_error}</div>
      )}
      <div className="grid gap-3 md:grid-cols-2">
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">CorpID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={corpId}
            onChange={(event) => setCorpId(event.target.value)}
            onBlur={() => save()}
            placeholder="wwxxxxxxxx"
          />
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">AgentId</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={agentId}
            onChange={(event) => setAgentId(event.target.value)}
            onBlur={() => save()}
            placeholder="1000002"
          />
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">成员 UserID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={organizerUserId}
            onChange={(event) => setOrganizerUserId(event.target.value)}
            onBlur={() => save()}
            placeholder="zhangsan"
          />
        </label>
        <label className="form-control">
          <span className="label-text mb-1 text-slate-500">日历 ID</span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            value={calendarId}
            onChange={(event) => setCalendarId(event.target.value)}
            onBlur={() => save()}
            placeholder="primary"
          />
        </label>
        <label className="form-control md:col-span-2">
          <span className="label-text mb-1 text-slate-500">
            Secret（只写入本机，不会回显）{connection?.has_secret ? ' · 已保存' : ''}
          </span>
          <input
            className="input input-bordered input-sm border-slate-200 bg-white/90 font-mono text-xs"
            type="password"
            value={secretDraft}
            onChange={(event) => setSecretDraft(event.target.value)}
            placeholder="留空表示不修改"
            onBlur={() => {
              const secret = secretDraft.trim();
              if (!secret) return;
              save(secret);
              setSecretDraft('');
            }}
          />
        </label>
      </div>
      <div className="rounded-md bg-slate-100/90 px-3 py-2 text-xs text-slate-600">
        在管理后台把应用加入「日程 → 可调用接口的应用」，并把这台电脑的公网 IP 配到应用的企业可信 IP。日历 ID 留空或填 primary 时，连接会创建 Task Manager 日历并共享给上面的成员。企业微信日程接口不使用 OAuth 回跳地址。
      </div>
      <div className="flex flex-wrap justify-end gap-2">
        <button
          className="btn btn-sm"
          onClick={() => onConnect({
            corpId,
            agentId,
            organizerUserId,
            calendarId,
            secret: secretDraft.trim() || undefined,
          })}
          disabled={!actions.connect || busy !== 'idle'}
        >
          {busy === 'connecting' ? '连接中...' : '连接企业微信'}
        </button>
        <button className="btn btn-sm" onClick={onDisconnect} disabled={!actions.disconnect || busy !== 'idle'}>断开</button>
        <button className="btn btn-sm btn-primary" onClick={onSync} disabled={!actions.sync || busy !== 'idle'}>
          {busy === 'syncing' ? '同步中...' : '立即同步'}
        </button>
      </div>
    </div>
  );
}

function parseWecomConfig(connection: WecomConnection | null, config?: string): {
  corpId: string;
  agentId: string;
  organizerUserId: string;
  calendarId: string;
} {
  if (connection) {
    return {
      corpId: connection.corp_id,
      agentId: connection.agent_id,
      organizerUserId: connection.organizer_userid,
      calendarId: connection.calendar_id || 'primary',
    };
  }
  try {
    const parsed = JSON.parse(config || '{}') as {
      corp_id?: string;
      agent_id?: string;
      organizer_userid?: string;
      calendar_id?: string;
    };
    return {
      corpId: parsed.corp_id ?? '',
      agentId: parsed.agent_id ?? '',
      organizerUserId: parsed.organizer_userid ?? '',
      calendarId: parsed.calendar_id || 'primary',
    };
  } catch {
    return { corpId: '', agentId: '', organizerUserId: '', calendarId: 'primary' };
  }
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
    authenticated: '已连接',
    synced: '已同步',
    error: '错误',
    unsupported: '仅 macOS',
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
