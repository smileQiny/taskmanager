const pomodoroCompleteTitle = '番茄钟完成';
const pomodoroCompleteBody = '一轮专注已记录。';

export async function notifyPomodoroComplete(): Promise<boolean> {
  return notifySystem(pomodoroCompleteTitle, pomodoroCompleteBody);
}

export async function preparePomodoroNotifications(): Promise<boolean> {
  if (typeof window === 'undefined') return false;

  if (isTauriRuntime()) {
    const granted = await ensureTauriNotificationPermission();
    if (granted) return true;
  }

  return ensureBrowserNotificationPermission();
}

async function notifySystem(title: string, body: string): Promise<boolean> {
  if (typeof window === 'undefined') return false;

  if (isTauriRuntime()) {
    const notified = await notifyWithTauri(title, body);
    if (notified) return true;
  }

  return notifyWithBrowser(title, body);
}

function isTauriRuntime(): boolean {
  return Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

async function notifyWithTauri(title: string, body: string): Promise<boolean> {
  try {
    const { sendNotification } = await import('@tauri-apps/plugin-notification');
    const granted = await ensureTauriNotificationPermission();
    if (!granted) return false;
    sendNotification({ title, body });
    return true;
  } catch {
    return false;
  }
}

async function notifyWithBrowser(title: string, body: string): Promise<boolean> {
  if (!('Notification' in window)) return false;

  try {
    const granted = await ensureBrowserNotificationPermission();
    if (!granted) return false;
    new Notification(title, { body });
    return true;
  } catch {
    return false;
  }
}

async function ensureTauriNotificationPermission(): Promise<boolean> {
  try {
    const { isPermissionGranted, requestPermission } = await import('@tauri-apps/plugin-notification');
    const granted = await isPermissionGranted();
    if (granted) return true;
    return await requestPermission() === 'granted';
  } catch {
    return false;
  }
}

async function ensureBrowserNotificationPermission(): Promise<boolean> {
  if (!('Notification' in window)) return false;
  if (Notification.permission === 'granted') return true;
  if (Notification.permission === 'denied') return false;
  return await Notification.requestPermission() === 'granted';
}
