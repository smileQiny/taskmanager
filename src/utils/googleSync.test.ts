import { describe, expect, it } from 'vitest';
import { googleActions, googleConnectionPhase } from './googleSync';

describe('google connection state machine', () => {
  it('stays desktop-only in the browser preview', () => {
    const phase = googleConnectionPhase({
      desktop: false,
      clientId: 'client',
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('desktop_only');
    expect(googleActions(phase)).toEqual({ connect: false, disconnect: false, sync: false });
  });

  it('asks for a client id before authorization', () => {
    expect(googleConnectionPhase({
      desktop: true,
      clientId: '  ',
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('not_configured');
  });

  it('moves from needs auth through connecting to authenticated', () => {
    expect(googleConnectionPhase({
      desktop: true,
      clientId: 'client',
      status: 'needs_auth',
      busy: 'idle',
    })).toBe('needs_auth');
    expect(googleConnectionPhase({
      desktop: true,
      clientId: 'client',
      status: 'needs_auth',
      busy: 'connecting',
    })).toBe('connecting');
    const phase = googleConnectionPhase({
      desktop: true,
      clientId: 'client',
      status: 'authenticated',
      busy: 'idle',
    });
    expect(phase).toBe('authenticated');
    expect(googleActions(phase).sync).toBe(true);
    expect(googleActions(phase).disconnect).toBe(true);
  });

  it('surfaces sync and authorization errors without reporting success', () => {
    const phase = googleConnectionPhase({
      desktop: true,
      clientId: 'client',
      status: 'error',
      busy: 'idle',
    });
    expect(phase).toBe('error');
    expect(googleActions(phase).connect).toBe(true);
    expect(googleActions(phase).sync).toBe(false);
    expect(googleConnectionPhase({
      desktop: true,
      clientId: 'client',
      status: 'authenticated',
      busy: 'syncing',
    })).toBe('syncing');
  });
});
