export type WecomBusy = 'idle' | 'connecting' | 'syncing';

export type WecomPhase =
  | 'desktop_only'
  | 'not_configured'
  | 'needs_auth'
  | 'connecting'
  | 'authenticated'
  | 'syncing'
  | 'error';

export interface WecomPhaseInput {
  desktop: boolean;
  corpId: string;
  agentId: string;
  organizerUserId: string;
  hasSecret: boolean;
  status: string;
  busy: WecomBusy;
}

export function wecomAgentIdReady(agentId: string): boolean {
  return /^[1-9]\d*$/.test(agentId.trim());
}

export function wecomConnectionPhase(input: WecomPhaseInput): WecomPhase {
  if (!input.desktop) return 'desktop_only';
  if (input.busy === 'connecting') return 'connecting';
  if (input.busy === 'syncing') return 'syncing';
  if (!input.corpId.trim() || !wecomAgentIdReady(input.agentId) || !input.organizerUserId.trim() || !input.hasSecret) {
    return 'not_configured';
  }
  if (input.status === 'error') return 'error';
  if (input.status === 'authenticated' || input.status === 'synced') return 'authenticated';
  return 'needs_auth';
}

export function wecomActions(phase: WecomPhase): { connect: boolean; disconnect: boolean; sync: boolean } {
  return {
    connect: phase === 'needs_auth' || phase === 'error',
    disconnect: phase === 'authenticated' || phase === 'error',
    sync: phase === 'authenticated',
  };
}
