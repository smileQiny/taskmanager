const cockpitWindowLabel = 'cockpit';

export async function openCockpitWindow(): Promise<'created' | 'already_open' | 'unsupported'> {
  if (!hasTauriRuntime()) return 'unsupported';

  const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
  const existing = await WebviewWindow.getByLabel(cockpitWindowLabel);
  if (existing) return 'already_open';

  const cockpit = new WebviewWindow(cockpitWindowLabel, {
    title: '今日驾驶舱',
    url: '/?window=cockpit',
    width: 286,
    height: 342,
    minWidth: 286,
    minHeight: 342,
    resizable: false,
    fullscreen: false,
    decorations: false,
    transparent: true,
    shadow: false,
    alwaysOnTop: true,
    center: true,
  });

  await new Promise<void>((resolve, reject) => {
    void cockpit.once('tauri://created', () => resolve());
    void cockpit.once('tauri://error', (event) => reject(event.payload));
  });

  return 'created';
}

function hasTauriRuntime(): boolean {
  return typeof window !== 'undefined'
    && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}
