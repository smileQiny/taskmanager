import { beforeEach, describe, expect, it, vi } from 'vitest';
import { notifyPomodoroComplete } from '../services/notificationService';
import { pomodoroService } from '../services/taskService';
import { PomodoroSession } from '../types/task';
import { usePomodoroStore } from './pomodoroStore';

vi.mock('../services/notificationService', () => ({
  notifyPomodoroComplete: vi.fn().mockResolvedValue(true),
  preparePomodoroNotifications: vi.fn().mockResolvedValue(true),
}));

vi.mock('../services/taskService', () => ({
  pomodoroService: {
    createSession: vi.fn(),
    listSessions: vi.fn(),
  },
}));

describe('pomodoro store', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    usePomodoroStore.setState({
      duration: 25 * 60,
      remaining: 25 * 60,
      running: false,
      completing: false,
      startedAt: null,
      sessions: [],
      lastCompletion: null,
      error: null,
    });
  });

  it('records a completed timer once when repeated ticks happen at zero', async () => {
    const completion = deferred<PomodoroSession>();
    vi.mocked(pomodoroService.createSession).mockReturnValue(completion.promise);
    usePomodoroStore.setState({
      duration: 2,
      remaining: 1,
      running: true,
      startedAt: 1_800_000_000,
    });

    usePomodoroStore.getState().tick();
    usePomodoroStore.getState().tick();

    expect(pomodoroService.createSession).toHaveBeenCalledTimes(1);
    expect(usePomodoroStore.getState()).toMatchObject({
      completing: true,
      remaining: 0,
      running: false,
    });

    const session = {
      id: 'session-1',
      started_at: 1_800_000_000,
      duration: 2,
      completed: true,
    };
    completion.resolve(session);
    await completion.promise;
    await Promise.resolve();

    expect(notifyPomodoroComplete).toHaveBeenCalledTimes(1);
    expect(usePomodoroStore.getState()).toMatchObject({
      completing: false,
      lastCompletion: {
        session,
        duration: 2,
      },
      remaining: 2,
      running: false,
      sessions: [session],
    });
  });
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
}
