export const FEISHU_REDIRECT_URI = 'http://127.0.0.1:43821/callback';

export type FeishuBusy = 'idle' | 'connecting' | 'syncing';

export type FeishuPhase =
  | 'desktop_only'
  | 'not_configured'
  | 'needs_auth'
  | 'connecting'
  | 'authenticated'
  | 'syncing'
  | 'error';

export interface FeishuPhaseInput {
  desktop: boolean;
  appId: string;
  hasAppSecret: boolean;
  status: string;
  busy: FeishuBusy;
}

export function feishuConnectionPhase(input: FeishuPhaseInput): FeishuPhase {
  if (!input.desktop) return 'desktop_only';
  if (input.busy === 'connecting') return 'connecting';
  if (input.busy === 'syncing') return 'syncing';
  if (!input.appId.trim() || !input.hasAppSecret) return 'not_configured';
  if (input.status === 'error') return 'error';
  if (input.status === 'authenticated' || input.status === 'synced') return 'authenticated';
  return 'needs_auth';
}

export function feishuActions(phase: FeishuPhase): { connect: boolean; disconnect: boolean; sync: boolean } {
  return {
    connect: phase === 'needs_auth' || phase === 'error',
    disconnect: phase === 'authenticated' || phase === 'error',
    sync: phase === 'authenticated',
  };
}
