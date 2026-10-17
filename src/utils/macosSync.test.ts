import { describe, expect, it } from 'vitest';
import { macosActions, macosConnectionPhase, macosPermissionHint } from './macosSync';

describe('macOS calendar connection state', () => {
  it('stays macOS-only when the host cannot use EventKit', () => {
    const phase = macosConnectionPhase({
      supported: false,
      enabled: true,
      permission: 'unsupported',
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('macos_only');
    expect(macosActions(phase)).toEqual({ requestAccess: false, sync: false });
    expect(macosPermissionHint('unsupported')).toMatch(/macOS/);
  });

  it('asks for full calendar access when permission is denied, restricted, or write-only', () => {
    for (const permission of ['not_determined', 'denied', 'restricted', 'write_only']) {
      const phase = macosConnectionPhase({
        supported: true,
        enabled: true,
        permission,
        status: 'needs_auth',
        busy: 'idle',
      });
      expect(phase).toBe('needs_auth');
      expect(macosActions(phase).requestAccess).toBe(true);
      expect(macosActions(phase).sync).toBe(false);
      expect(macosPermissionHint(permission)).toBeTruthy();
    }
  });

  it('syncs only after full access, including a retry after an error', () => {
    const authenticated = macosConnectionPhase({
      supported: true,
      enabled: true,
      permission: 'full_access',
      status: 'authenticated',
      busy: 'idle',
    });
    expect(authenticated).toBe('authenticated');
    expect(macosActions(authenticated).sync).toBe(true);

    const failed = macosConnectionPhase({
      supported: true,
      enabled: true,
      permission: 'full_access',
      status: 'error',
      busy: 'idle',
    });
    expect(failed).toBe('error');
    expect(macosActions(failed).sync).toBe(true);
    expect(macosActions(failed).requestAccess).toBe(false);

    expect(macosConnectionPhase({
      supported: true,
      enabled: true,
      permission: 'full_access',
      status: 'authenticated',
      busy: 'syncing',
    })).toBe('syncing');
    expect(macosConnectionPhase({
      supported: true,
      enabled: false,
      permission: 'full_access',
      status: 'not_configured',
      busy: 'idle',
    })).toBe('not_configured');
  });
});
