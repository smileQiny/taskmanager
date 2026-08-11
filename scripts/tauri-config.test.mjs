import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('tauri window configuration', () => {
  it('does not create the transparent cockpit window during app startup', () => {
    const config = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8'));
    const labels = config.app.windows.map((window) => window.label);

    expect(labels).toEqual(['main']);
  });

  it('keeps permissions for the dynamically-created cockpit window', () => {
    const capabilities = JSON.parse(readFileSync('src-tauri/capabilities/default.json', 'utf8'));

    expect(capabilities.windows).toContain('main');
    expect(capabilities.windows).toContain('cockpit');
  });
});
