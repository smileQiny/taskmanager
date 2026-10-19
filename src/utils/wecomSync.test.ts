import { describe, expect, it } from 'vitest';
import { wecomActions, wecomConnectionPhase } from './wecomSync';

describe('wecom connection state machine', () => {
  it('stays desktop-only in the browser preview and does not offer sync', () => {
    const phase = wecomConnectionPhase({
      desktop: false,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('desktop_only');
    expect(wecomActions(phase)).toEqual({ connect: false, disconnect: false, sync: false });
  });

  it('asks for corp id, agent id, organizer, and secret before connect', () => {
    expect(wecomConnectionPhase({
      desktop: true,
      corpId: '  ',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
    expect(wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: 'abc',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
    expect(wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: ' ',
      hasSecret: true,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
    expect(wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: false,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
  });

  it('moves from needs auth through connecting to authenticated', () => {
    expect(wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('needs_auth');
    expect(wecomActions(wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'needs_auth',
      busy: 'connecting',
    })).connect).toBe(false);
    const phase = wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('authenticated');
    expect(wecomActions(phase).sync).toBe(true);
    expect(wecomActions(phase).disconnect).toBe(true);
  });

  it('surfaces sync and authorization errors without reporting success', () => {
    const phase = wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'error',
      busy: 'idle',
    });
    expect(phase).toBe('error');
    expect(wecomActions(phase).connect).toBe(true);
    expect(wecomActions(phase).sync).toBe(false);
    expect(wecomConnectionPhase({
      desktop: true,
      corpId: 'ww123',
      agentId: '1000002',
      organizerUserId: 'zhangsan',
      hasSecret: true,
      status: 'authenticated',
      busy: 'syncing',
    })).toBe('syncing');
  });
});
