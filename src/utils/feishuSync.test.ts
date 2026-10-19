import { describe, expect, it } from 'vitest';
import { FEISHU_REDIRECT_URI, feishuActions, feishuConnectionPhase } from './feishuSync';

describe('feishu connection state machine', () => {
  it('stays desktop-only in the browser preview and does not offer sync', () => {
    const phase = feishuConnectionPhase({
      desktop: false,
      appId: 'cli_test',
      hasAppSecret: true,
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('desktop_only');
    expect(feishuActions(phase)).toEqual({ connect: false, disconnect: false, sync: false });
    expect(FEISHU_REDIRECT_URI).toBe('http://127.0.0.1:43821/callback');
  });

  it('asks for an app id and app secret before authorization', () => {
    expect(feishuConnectionPhase({
      desktop: true,
      appId: '  ',
      hasAppSecret: true,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
    expect(feishuConnectionPhase({
      desktop: true,
      appId: 'cli_test',
      hasAppSecret: false,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
  });

  it('moves from needs auth through connecting to authenticated', () => {
    expect(feishuConnectionPhase({
      desktop: true,
      appId: 'cli_test',
      hasAppSecret: true,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('needs_auth');
    expect(feishuActions(feishuConnectionPhase({
      desktop: true,
      appId: 'cli_test',
      hasAppSecret: true,
      status: 'needs_auth',
      busy: 'connecting',
    })).connect).toBe(false);
    const phase = feishuConnectionPhase({
      desktop: true,
      appId: 'cli_test',
      hasAppSecret: true,
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('authenticated');
    expect(feishuActions(phase).sync).toBe(true);
    expect(feishuActions(phase).disconnect).toBe(true);
  });

  it('surfaces sync and authorization errors without reporting success', () => {
    const phase = feishuConnectionPhase({
      desktop: true,
      appId: 'cli_test',
      hasAppSecret: true,
      status: 'error',
      busy: 'idle',
    });
    expect(phase).toBe('error');
    expect(feishuActions(phase).connect).toBe(true);
    expect(feishuActions(phase).sync).toBe(false);
    expect(feishuConnectionPhase({
      desktop: true,
      appId: 'cli_test',
      hasAppSecret: true,
      status: 'authenticated',
      busy: 'syncing',
    })).toBe('syncing');
  });
});
