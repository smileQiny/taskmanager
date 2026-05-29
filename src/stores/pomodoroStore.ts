import { create } from 'zustand';
import { PomodoroSession } from '../types/task';
import { pomodoroService } from '../services/taskService';

interface PomodoroStore {
  duration: number;
  remaining: number;
  running: boolean;
  startedAt: number | null;
  sessions: PomodoroSession[];
  error: string | null;
  setDurationMinutes: (minutes: number) => void;
  start: () => void;
  pause: () => void;
  reset: () => void;
  tick: () => void;
  complete: () => Promise<void>;
  fetchSessions: () => Promise<void>;
}

export const usePomodoroStore = create<PomodoroStore>((set, get) => ({
  duration: 25 * 60,
  remaining: 25 * 60,
  running: false,
  startedAt: null,
  sessions: [],
  error: null,

  setDurationMinutes: (minutes) => {
    const duration = minutes * 60;
    set({ duration, remaining: duration, running: false, startedAt: null });
  },

  start: () => set((s) => ({ running: true, startedAt: s.startedAt ?? Math.floor(Date.now() / 1000) })),
  pause: () => set({ running: false }),
  reset: () => set((s) => ({ running: false, remaining: s.duration, startedAt: null })),
  tick: () => {
    const { running, remaining, complete } = get();
    if (!running) return;
    if (remaining <= 1) {
      void complete();
      return;
    }
    set({ remaining: remaining - 1 });
  },

  complete: async () => {
    const { startedAt, duration } = get();
    try {
      const session = await pomodoroService.createSession({
        started_at: startedAt ?? Math.floor(Date.now() / 1000),
        duration,
        completed: true,
      });
      set((s) => ({
        running: false,
        remaining: s.duration,
        startedAt: null,
        sessions: [session, ...s.sessions].slice(0, 10),
        error: null,
      }));
      if ('Notification' in window && Notification.permission === 'granted') {
        new Notification('番茄钟完成', { body: '一轮专注已记录。' });
      }
    } catch (e) {
      set({ error: String(e), running: false });
      throw e;
    }
  },

  fetchSessions: async () => {
    try {
      const sessions = await pomodoroService.listSessions(10);
      set({ sessions, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
