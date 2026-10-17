export type GoogleBusy = 'idle' | 'connecting' | 'syncing';

export type GooglePhase =
  | 'desktop_only'
  | 'not_configured'
  | 'needs_auth'
  | 'connecting'
  | 'authenticated'
  | 'syncing'
  | 'error';

export interface GooglePhaseInput {
  desktop: boolean;
  clientId: string;
  status: string;
  busy: GoogleBusy;
}

export function isDesktopRuntime(): boolean {
  return typeof window !== 'undefined'
    && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

export function googleConnectionPhase(input: GooglePhaseInput): GooglePhase {
  if (!input.desktop) return 'desktop_only';
  if (input.busy === 'connecting') return 'connecting';
  if (input.busy === 'syncing') return 'syncing';
  if (!input.clientId.trim()) return 'not_configured';
  if (input.status === 'error') return 'error';
  if (input.status === 'authenticated' || input.status === 'synced') return 'authenticated';
  return 'needs_auth';
}

export function googleActions(phase: GooglePhase): { connect: boolean; disconnect: boolean; sync: boolean } {
  return {
    connect: phase === 'needs_auth' || phase === 'error',
    disconnect: phase === 'authenticated' || phase === 'error',
    sync: phase === 'authenticated',
  };
}
