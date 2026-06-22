import { describe, expect, it } from 'vitest';
import { buildUpdaterManifest } from './generate-updater-manifest.mjs';

const assets = [
  asset('Task.Manager_aarch64.app.tar.gz'),
  asset('Task.Manager_aarch64.app.tar.gz.sig'),
  asset('Task.Manager_x64.app.tar.gz'),
  asset('Task.Manager_x64.app.tar.gz.sig'),
  asset('Task.Manager_0.2.8_x64-setup.exe'),
  asset('Task.Manager_0.2.8_x64-setup.exe.sig'),
  asset('Task.Manager_0.2.8_amd64.AppImage'),
  asset('Task.Manager_0.2.8_amd64.AppImage.sig'),
];

describe('generate updater manifest', () => {
  it('keeps both macOS architecture platform keys in latest.json', async () => {
    const manifest = await buildUpdaterManifest({
      assets,
      notes: 'Release notes',
      pubDate: '2026-06-22T10:00:00.000Z',
      readSignature: async (name) => `signature:${name}`,
      version: '0.2.8',
    });

    expect(manifest.version).toBe('0.2.8');
    expect(manifest.platforms['darwin-aarch64']).toMatchObject({
      signature: 'signature:Task.Manager_aarch64.app.tar.gz.sig',
      url: 'https://github.com/smileQiny/taskmanager/releases/download/v0.2.8/Task.Manager_aarch64.app.tar.gz',
    });
    expect(manifest.platforms['darwin-aarch64-app']).toEqual(manifest.platforms['darwin-aarch64']);
    expect(manifest.platforms['darwin-x86_64']).toMatchObject({
      signature: 'signature:Task.Manager_x64.app.tar.gz.sig',
    });
    expect(manifest.platforms['darwin-x86_64-app']).toEqual(manifest.platforms['darwin-x86_64']);
  });

  it('fails when a required updater signature is missing', async () => {
    await expect(buildUpdaterManifest({
      assets: assets.filter((item) => item.name !== 'Task.Manager_aarch64.app.tar.gz.sig'),
      notes: 'Release notes',
      pubDate: '2026-06-22T10:00:00.000Z',
      readSignature: async (name) => `signature:${name}`,
      version: '0.2.8',
    })).rejects.toThrow('Missing signature asset for Task.Manager_aarch64.app.tar.gz');
  });
});

function asset(name) {
  return {
    name,
    browser_download_url: `https://github.com/smileQiny/taskmanager/releases/download/v0.2.8/${name}`,
    size: 100,
  };
}
