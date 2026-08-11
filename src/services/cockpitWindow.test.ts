import { beforeEach, describe, expect, it, vi } from 'vitest';
import { openCockpitWindow } from './cockpitWindow';

vi.mock('@tauri-apps/api/webviewWindow', () => {
  class MockWebviewWindow {
    static existing: MockWebviewWindow | null = null;
    static createdLabels: string[] = [];
    static createdOptions: unknown[] = [];

    label: string;

    constructor(label: string, options: unknown) {
      this.label = label;
      MockWebviewWindow.createdLabels.push(label);
      MockWebviewWindow.createdOptions.push(options);
    }

    static async getByLabel() {
      return MockWebviewWindow.existing;
    }

    async once(eventName: string, handler: () => void) {
      if (eventName === 'tauri://created') queueMicrotask(handler);
      return vi.fn();
    }
  }

  return { WebviewWindow: MockWebviewWindow };
});

describe('cockpit window service', () => {
  beforeEach(async () => {
    vi.unstubAllGlobals();
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const mockClass = WebviewWindow as unknown as {
      existing: unknown;
      createdLabels: string[];
      createdOptions: unknown[];
    };
    mockClass.existing = null;
    mockClass.createdLabels = [];
    mockClass.createdOptions = [];
  });

  it('does nothing when cockpit window is already open', async () => {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const mockClass = WebviewWindow as unknown as {
      existing: unknown;
      createdLabels: string[];
    };
    mockClass.existing = { label: 'cockpit' };
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });

    const result = await openCockpitWindow();

    expect(result).toBe('already_open');
    expect(mockClass.createdLabels).toEqual([]);
  });

  it('creates the cockpit window when it was closed', async () => {
    const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
    const mockClass = WebviewWindow as unknown as {
      createdLabels: string[];
      createdOptions: Array<{ shadow?: boolean }>;
    };
    vi.stubGlobal('window', { __TAURI_INTERNALS__: {} });

    const result = await openCockpitWindow();

    expect(result).toBe('created');
    expect(mockClass.createdLabels).toEqual(['cockpit']);
    expect(mockClass.createdOptions[0]).toMatchObject({ shadow: false });
  });
});
