import { ReleaseAsset, AppUpdateInfo } from '../types/update';

const githubOwner = 'smileQiny';
const githubRepo = 'taskmanager';
const githubApiUrl = `https://api.github.com/repos/${githubOwner}/${githubRepo}/releases/latest`;
const githubLatestReleaseUrl = `https://github.com/${githubOwner}/${githubRepo}/releases/latest`;
const hasTauri = typeof window !== 'undefined' && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

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

export async function checkForUpdates(options: CheckForUpdatesOptions = {}): Promise<AppUpdateInfo> {
  const currentVersion = normalizeVersion(options.currentVersion ?? appVersion);
  const response = await fetch(options.apiUrl ?? githubApiUrl, {
    headers: {
      Accept: 'application/vnd.github+json',
    },
  });

  if (response.status === 404) {
    throw new Error('没有找到 GitHub Release。请先发布一个非草稿版本。');
  }

  if (!response.ok) {
    throw new Error(`GitHub 更新检查失败：HTTP ${response.status}`);
  }

  const release = await response.json() as GitHubRelease;
  const latestVersion = normalizeVersion(release.tag_name);
  const assets = (release.assets ?? [])
    .map((asset) => ({
      name: asset.name,
      downloadUrl: asset.browser_download_url,
      size: asset.size,
      contentType: asset.content_type,
    }))
    .filter(isInstallerAsset);

  return {
    currentVersion,
    latestVersion,
    releaseName: release.name || release.tag_name,
    releaseUrl: release.html_url,
    publishedAt: release.published_at ?? null,
    assets,
    preferredAsset: getPreferredInstallAsset(assets),
    isUpdateAvailable: compareVersions(latestVersion, currentVersion) > 0,
    checkedAt: new Date().toISOString(),
  };
}

export async function openExternalUrl(url: string): Promise<void> {
  if (hasTauri) {
    try {
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
      return;
    } catch {
      // Fall through to the browser fallback so the action still works in previews.
    }
  }

  window.open(url, '_blank', 'noopener,noreferrer');
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

function isInstallerAsset(asset: ReleaseAsset): boolean {
  const name = asset.name.toLowerCase();
  return !name.endsWith('.sig') && !name.endsWith('.json') && !name.endsWith('.sha256');
}
