import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  checkForUpdates,
  compareVersions,
  getPreferredInstallAsset,
  installUpdateFromRelease,
  normalizeVersion,
} from './updateService';
import { AppUpdateInfo, ReleaseAsset } from '../types/update';

vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}));

describe('update service', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    if (typeof window !== 'undefined') {
      delete (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    }
  });

  it('compares semantic versions numerically', () => {
    expect(compareVersions('0.10.0', '0.2.0')).toBe(1);
    expect(compareVersions('v1.0.0', '1.0.0')).toBe(0);
    expect(compareVersions('1.0.0-beta.1', '1.0.0')).toBe(-1);
  });

  it('normalizes release tags to app versions', () => {
    expect(normalizeVersion('v0.2.0')).toBe('0.2.0');
    expect(normalizeVersion('taskmanager-v1.4.3')).toBe('1.4.3');
  });

  it('maps GitHub latest release responses to update info', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => ({
      ok: true,
      status: 200,
      json: async () => ({
        tag_name: 'v0.2.0',
        name: 'TaskManager v0.2.0',
        html_url: 'https://github.com/smileQiny/taskmanager/releases/tag/v0.2.0',
        published_at: '2026-05-29T00:00:00Z',
        assets: [
          {
            name: 'TaskManager_0.2.0_aarch64.dmg',
            browser_download_url: 'https://example.com/app.dmg',
            size: 1024,
            content_type: 'application/octet-stream',
          },
          {
            name: 'latest.json',
            browser_download_url: 'https://example.com/latest.json',
            size: 200,
            content_type: 'application/json',
          },
        ],
      }),
    } as Response)));

    const info = await checkForUpdates({
      currentVersion: '0.1.0',
      apiUrl: 'https://example.com/latest-release',
    });

    expect(info.isUpdateAvailable).toBe(true);
    expect(info.latestVersion).toBe('0.2.0');
    expect(info.assets).toHaveLength(1);
    expect(info.preferredAsset?.downloadUrl).toBe('https://example.com/app.dmg');
  });

  it('selects a platform installer when possible', () => {
    vi.stubGlobal('navigator', {
      platform: 'MacIntel',
      userAgent: 'Mac OS',
    });

    const assets: ReleaseAsset[] = [
      { name: 'TaskManager_0.2.0_x64.msi', downloadUrl: 'https://example.com/app.msi', size: 1 },
      { name: 'TaskManager_0.2.0_aarch64.dmg', downloadUrl: 'https://example.com/app.dmg', size: 1 },
    ];

    expect(getPreferredInstallAsset(assets)?.name).toBe('TaskManager_0.2.0_aarch64.dmg');
  });

  it('uses Tauri signed updater inside Tauri', async () => {
    const { check } = await import('@tauri-apps/plugin-updater');
    const { relaunch } = await import('@tauri-apps/plugin-process');
    const downloadAndInstall = vi.fn().mockResolvedValue(undefined);
    vi.mocked(check).mockResolvedValue({
      downloadAndInstall,
    } as unknown as Awaited<ReturnType<typeof check>>);
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });

    const release = makeRelease();
    const result = await installUpdateFromRelease(release);

    expect(check).toHaveBeenCalled();
    expect(downloadAndInstall).toHaveBeenCalled();
    expect(relaunch).toHaveBeenCalled();
    expect(result.action).toBe('installed');
  });

  it('opens the release page in browser previews', async () => {
    const { check } = await import('@tauri-apps/plugin-updater');
    const open = vi.fn();
    vi.stubGlobal('open', open);
    vi.stubGlobal('window', undefined);

    const result = await installUpdateFromRelease(makeRelease());

    expect(check).not.toHaveBeenCalled();
    expect(open).toHaveBeenCalledWith(
      'https://github.com/smileQiny/taskmanager/releases/tag/v0.2.2',
      '_blank',
      'noopener,noreferrer',
    );
    expect(result.action).toBe('opened');
  });
});

function makeRelease(): AppUpdateInfo {
  return {
    currentVersion: '0.2.1',
    latestVersion: '0.2.2',
    releaseName: 'TaskManager v0.2.2',
    releaseUrl: 'https://github.com/smileQiny/taskmanager/releases/tag/v0.2.2',
    publishedAt: '2026-06-22T05:47:00Z',
    assets: [
      {
        name: 'Task.Manager_0.2.2_aarch64.dmg',
        downloadUrl: 'https://github.com/smileQiny/taskmanager/releases/download/v0.2.2/Task.Manager_0.2.2_aarch64.dmg',
        size: 1,
      },
    ],
    preferredAsset: {
      name: 'Task.Manager_0.2.2_aarch64.dmg',
      downloadUrl: 'https://github.com/smileQiny/taskmanager/releases/download/v0.2.2/Task.Manager_0.2.2_aarch64.dmg',
      size: 1,
    },
    isUpdateAvailable: true,
    checkedAt: '2026-06-22T05:48:00Z',
  };
}
