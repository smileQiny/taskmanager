import { beforeEach, describe, expect, it, vi } from 'vitest';
import { notifyPomodoroComplete } from '../services/notificationService';
import { pomodoroService } from '../services/taskService';
import { PomodoroTimerSnapshot } from '../types/task';
import { usePomodoroStore } from './pomodoroStore';

vi.mock('../services/notificationService', () => ({
  notifyPomodoroComplete: vi.fn().mockResolvedValue(true),
  preparePomodoroNotifications: vi.fn().mockResolvedValue(true),
}));

vi.mock('../services/taskService', () => ({
  pomodoroService: {
    createSession: vi.fn(),
    listSessions: vi.fn(),
    getTimer: vi.fn(),
    startTimer: vi.fn(),
    pauseTimer: vi.fn(),
    resetTimer: vi.fn(),
    setTimerDuration: vi.fn(),
    completeTimer: vi.fn(),
    updateSessionRecord: vi.fn(),
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
    vi.mocked(pomodoroService.getTimer).mockResolvedValue(makeSnapshot());
  });

  it('applies a backend completion once when repeated ticks refresh the timer', async () => {
    const session = {
      id: 'session-1',
      started_at: 1_800_000_000,
      duration: 2,
      completed: true,
    };
    vi.mocked(pomodoroService.getTimer)
      .mockResolvedValueOnce(makeSnapshot({
        duration: 2,
        remaining: 2,
        running: false,
        completion: {
          session,
          completed_at: 1_800_000_002,
          duration: 2,
        },
      }))
      .mockResolvedValueOnce(makeSnapshot({ duration: 2, remaining: 2, running: false }));

    await usePomodoroStore.getState().tick();
    await usePomodoroStore.getState().tick();

    expect(notifyPomodoroComplete).toHaveBeenCalledTimes(1);
    expect(usePomodoroStore.getState()).toMatchObject({
      completing: false,
      lastCompletion: {
        session,
        completedAt: 1_800_000_002,
        duration: 2,
      },
      remaining: 2,
      running: false,
      sessions: [session],
    });
  });

  it('uses the backend timer duration selected by either window', async () => {
    vi.mocked(pomodoroService.setTimerDuration).mockResolvedValue(makeSnapshot({
      duration: 15 * 60,
      remaining: 15 * 60,
      running: false,
    }));

    await usePomodoroStore.getState().setDurationMinutes(15);

    expect(pomodoroService.setTimerDuration).toHaveBeenCalledWith(15);
    expect(usePomodoroStore.getState()).toMatchObject({
      duration: 15 * 60,
      remaining: 15 * 60,
      running: false,
    });
  });
});

function makeSnapshot(overrides: {
  duration?: number;
  remaining?: number;
  running?: boolean;
  completion?: PomodoroTimerSnapshot['completion'];
} = {}): PomodoroTimerSnapshot {
  const duration = overrides.duration ?? 25 * 60;
  return {
    timer: {
      duration,
      remaining: overrides.remaining ?? duration,
      running: overrides.running ?? false,
      started_at: null,
      updated_at: 1_800_000_000,
    },
    completion: overrides.completion ?? null,
  };
}
