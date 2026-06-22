import { ReleaseAsset, AppUpdateInfo, InstallUpdateResult } from '../types/update';

const githubOwner = 'smileQiny';
const githubRepo = 'taskmanager';
const githubLatestReleaseUrl = `https://github.com/${githubOwner}/${githubRepo}/releases/latest`;
const githubLatestJsonUrl = `https://github.com/${githubOwner}/${githubRepo}/releases/latest/download/latest.json`;

interface GitHubReleaseAsset {
  name: string;
  browser_download_url: string;
  size: number;
  content_type?: string;
}

interface GitHubRelease {
  tag_name: string;
  name?: string | null;
  html_url: string;
  published_at?: string | null;
  assets?: GitHubReleaseAsset[];
}

interface TauriUpdaterPlatform {
  signature?: string;
  url: string;
}

interface TauriUpdaterManifest {
  version: string;
  notes?: string | null;
  pub_date?: string | null;
  platforms?: Record<string, TauriUpdaterPlatform>;
}

interface CheckForUpdatesOptions {
  currentVersion?: string;
  apiUrl?: string;
}

interface ParsedVersion {
  numbers: number[];
  prerelease: string | null;
}

export const appVersion = __APP_VERSION__;
export const releasesUrl = githubLatestReleaseUrl;
export const latestJsonUrl = githubLatestJsonUrl;

export async function checkForUpdates(options: CheckForUpdatesOptions = {}): Promise<AppUpdateInfo> {
  const currentVersion = normalizeVersion(options.currentVersion ?? appVersion);
  if (hasTauriRuntime() && !options.apiUrl) {
    return checkNativeUpdater(currentVersion);
  }
  if (!hasTauriRuntime() && !options.apiUrl) {
    return {
      currentVersion,
      latestVersion: currentVersion,
      releaseName: `TaskManager v${currentVersion}`,
      releaseUrl: githubLatestReleaseUrl,
      publishedAt: null,
      assets: [],
      preferredAsset: null,
      isUpdateAvailable: false,
      checkedAt: new Date().toISOString(),
    };
  }

  const response = await fetch(options.apiUrl ?? githubLatestJsonUrl, {
    headers: {
      Accept: 'application/json',
    },
  });

  if (response.status === 404) {
    throw new Error('没有找到更新元数据。请确认 GitHub Release 已上传 latest.json。');
  }

  if (!response.ok) {
    throw new Error(`GitHub 更新检查失败：HTTP ${response.status}`);
  }

  const payload = await response.json() as GitHubRelease | TauriUpdaterManifest;
  const release = isTauriUpdaterManifest(payload)
    ? mapUpdaterManifestToRelease(payload)
    : mapGitHubRelease(payload);
  const latestVersion = normalizeVersion(release.tagName);

  return {
    currentVersion,
    latestVersion,
    releaseName: release.name || `TaskManager v${latestVersion}`,
    releaseUrl: release.htmlUrl,
    publishedAt: release.publishedAt,
    assets: release.assets,
    preferredAsset: getPreferredInstallAsset(release.assets),
    isUpdateAvailable: compareVersions(latestVersion, currentVersion) > 0,
    checkedAt: new Date().toISOString(),
  };
}

async function checkNativeUpdater(currentVersion: string): Promise<AppUpdateInfo> {
  const { check } = await import('@tauri-apps/plugin-updater');
  const update = await check();
  const latestVersion = normalizeVersion(update?.version ?? currentVersion);
  const assets = mapNativeUpdateAssets(update?.rawJson);

  return {
    currentVersion,
    latestVersion,
    releaseName: update ? `TaskManager v${latestVersion}` : `TaskManager v${currentVersion}`,
    releaseUrl: getReleaseUrlFromAssets(assets) ?? `${githubLatestReleaseUrl.replace('/latest', `/tag/v${latestVersion}`)}`,
    publishedAt: update?.date ?? null,
    assets,
    preferredAsset: getPreferredInstallAsset(assets),
    isUpdateAvailable: Boolean(update) && compareVersions(latestVersion, currentVersion) > 0,
    checkedAt: new Date().toISOString(),
  };
}

function mapGitHubRelease(release: GitHubRelease) {
  const assets = (release.assets ?? [])
    .map((asset) => ({
      name: asset.name,
      downloadUrl: asset.browser_download_url,
      size: asset.size,
      contentType: asset.content_type,
    }))
    .filter(isInstallerAsset);

  return {
    tagName: release.tag_name,
    name: release.name || release.tag_name,
    htmlUrl: release.html_url,
    publishedAt: release.published_at ?? null,
    assets,
  };
}

function mapUpdaterManifestToRelease(manifest: TauriUpdaterManifest) {
  const assets = mapUpdaterPlatforms(manifest.platforms)
    .filter(isInstallerAsset);
  const tagName = `v${normalizeVersion(manifest.version)}`;

  return {
    tagName,
    name: `TaskManager ${tagName}`,
    htmlUrl: getReleaseUrlFromAssets(assets) ?? `${githubLatestReleaseUrl.replace('/latest', `/tag/${tagName}`)}`,
    publishedAt: manifest.pub_date ?? null,
    assets,
  };
}

function mapNativeUpdateAssets(rawJson: Record<string, unknown> | undefined): ReleaseAsset[] {
  const platforms = rawJson?.platforms;
  return mapUpdaterPlatforms(isUpdaterPlatforms(platforms) ? platforms : undefined)
    .filter(isInstallerAsset);
}

function mapUpdaterPlatforms(platforms: Record<string, TauriUpdaterPlatform> | undefined): ReleaseAsset[] {
  return Object.values(platforms ?? {}).reduce<ReleaseAsset[]>((items, platform) => {
    if (!platform.url || items.some((asset) => asset.downloadUrl === platform.url)) {
      return items;
    }
    items.push({
      name: getFileNameFromUrl(platform.url),
      downloadUrl: platform.url,
      size: 0,
    });
    return items;
  }, []);
}

function isUpdaterPlatforms(value: unknown): value is Record<string, TauriUpdaterPlatform> {
  return value !== null
    && typeof value === 'object'
    && Object.values(value).every((platform) => (
      Boolean(platform)
      && typeof platform === 'object'
      && typeof (platform as TauriUpdaterPlatform).url === 'string'
    ));
}

function isTauriUpdaterManifest(payload: GitHubRelease | TauriUpdaterManifest): payload is TauriUpdaterManifest {
  return typeof (payload as TauriUpdaterManifest).version === 'string'
    && typeof (payload as GitHubRelease).tag_name !== 'string';
}

export async function openExternalUrl(url: string): Promise<void> {
  if (hasTauriRuntime()) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
      return;
    } catch {
      // Fall through to the browser fallback so the action still works in previews.
    }
  }

  const opener = typeof window !== 'undefined'
    ? window.open.bind(window)
    : (globalThis as typeof globalThis & { open?: Window['open'] }).open;

  if (!opener) {
    throw new Error('当前环境无法打开外部链接。');
  }

  opener(url, '_blank', 'noopener,noreferrer');
}

export async function installUpdateFromRelease(release: AppUpdateInfo | null): Promise<InstallUpdateResult> {
  if (!release) {
    throw new Error('请先检查更新。');
  }

  if (hasTauriRuntime()) {
    const { check } = await import('@tauri-apps/plugin-updater');
    const { relaunch } = await import('@tauri-apps/plugin-process');
    const update = await check();

    if (!update) {
      return {
        action: 'up_to_date',
        message: '当前已经是最新版本。',
      };
    }

    await update.downloadAndInstall();
    await relaunch();

    return {
      action: 'installed',
      message: '更新已安装，应用正在重启。',
    };
  }

  await openExternalUrl(release.releaseUrl);
  return {
    action: 'opened',
    message: '已打开 GitHub Release 页面。',
  };
}

export function getPreferredInstallAsset(assets: ReleaseAsset[]): ReleaseAsset | null {
  if (assets.length === 0) return null;

  const platform = detectPlatform();
  const preferredExtensions = {
    mac: ['.dmg', '.app.tar.gz', '.zip'],
    windows: ['.msi', '.exe'],
    linux: ['.AppImage', '.deb', '.rpm'],
    unknown: ['.dmg', '.msi', '.exe', '.AppImage', '.deb', '.rpm', '.zip'],
  }[platform];

  for (const extension of preferredExtensions) {
    const asset = assets.find((item) => item.name.endsWith(extension));
    if (asset) return asset;
  }

  return assets[0];
}

export function compareVersions(left: string, right: string): number {
  const a = parseVersion(left);
  const b = parseVersion(right);

  for (let index = 0; index < Math.max(a.numbers.length, b.numbers.length); index += 1) {
    const diff = (a.numbers[index] ?? 0) - (b.numbers[index] ?? 0);
    if (diff !== 0) return diff > 0 ? 1 : -1;
  }

  if (a.prerelease === b.prerelease) return 0;
  if (a.prerelease === null) return 1;
  if (b.prerelease === null) return -1;
  return a.prerelease.localeCompare(b.prerelease);
}

export function normalizeVersion(value: string): string {
  const match = value.trim().match(/\d+(?:\.\d+){0,2}(?:[-+][0-9A-Za-z.-]+)?/);
  const normalized = match?.[0] ?? value.trim().replace(/^v/i, '');
  const [version] = normalized.split('+');
  return version;
}

function parseVersion(value: string): ParsedVersion {
  const [main, prerelease = null] = normalizeVersion(value).split('-', 2);
  return {
    numbers: main.split('.').map((part) => {
      const valuePart = Number.parseInt(part, 10);
      return Number.isFinite(valuePart) ? valuePart : 0;
    }),
    prerelease,
  };
}

function detectPlatform(): 'mac' | 'windows' | 'linux' | 'unknown' {
  if (typeof navigator === 'undefined') return 'unknown';
  const platform = `${navigator.platform} ${navigator.userAgent}`.toLowerCase();
  if (platform.includes('mac')) return 'mac';
  if (platform.includes('win')) return 'windows';
  if (platform.includes('linux')) return 'linux';
  return 'unknown';
}

function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined'
    && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

function isInstallerAsset(asset: ReleaseAsset): boolean {
  const name = asset.name.toLowerCase();
  return !name.endsWith('.sig') && !name.endsWith('.json') && !name.endsWith('.sha256');
}

function getFileNameFromUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const parts = parsed.pathname.split('/').filter(Boolean);
    const pathname = parts[parts.length - 1];
    return pathname ? decodeURIComponent(pathname) : url;
  } catch {
    const parts = url.split('/').filter(Boolean);
    return parts[parts.length - 1] ?? url;
  }
}

function getReleaseUrlFromAssets(assets: ReleaseAsset[]): string | null {
  for (const asset of assets) {
    const match = asset.downloadUrl.match(/github\.com\/([^/]+)\/([^/]+)\/releases\/download\/([^/]+)/);
    if (!match) continue;
    const [, owner, repo, tag] = match;
    return `https://github.com/${owner}/${repo}/releases/tag/${tag}`;
  }
  return null;
}
