export type MacosBusy = 'idle' | 'requesting' | 'syncing';

export type MacosPhase =
  | 'macos_only'
  | 'not_configured'
  | 'needs_auth'
  | 'requesting'
  | 'authenticated'
  | 'syncing'
  | 'error';

export interface MacosPhaseInput {
  supported: boolean;
  enabled: boolean;
  permission: string;
  status: string;
  busy: MacosBusy;
}

export function macosConnectionPhase(input: MacosPhaseInput): MacosPhase {
  if (!input.supported) return 'macos_only';
  if (input.busy === 'requesting') return 'requesting';
  if (input.busy === 'syncing') return 'syncing';
  if (!input.enabled) return 'not_configured';
  if (input.permission !== 'full_access') return 'needs_auth';
  if (input.status === 'error') return 'error';
  return 'authenticated';
}

export function macosActions(phase: MacosPhase): { requestAccess: boolean; sync: boolean } {
  return {
    requestAccess: phase === 'needs_auth',
    sync: phase === 'authenticated' || phase === 'error',
  };
}

export function macosPermissionHint(permission: string): string | null {
  switch (permission) {
    case 'denied':
      return '日历访问已被拒绝。请在「系统设置 → 隐私与安全性 → 日历」中允许 Task Manager。';
    case 'restricted':
      return '此 Mac 限制了日历访问，无法同步。';
    case 'write_only':
      return '当前只有写入权限。双向同步需要完整日历访问。';
    case 'not_determined':
      return '尚未授予日历访问。请请求完整访问权限。';
    case 'unsupported':
      return 'macOS 日历同步只能在 macOS 上使用。当前系统不会执行同步。';
    default:
      return null;
  }
}
