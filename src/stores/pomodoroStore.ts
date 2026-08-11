import { create } from 'zustand';
import { PomodoroSession } from '../types/task';
import { pomodoroService } from '../services/taskService';
import { notifyPomodoroComplete, preparePomodoroNotifications } from '../services/notificationService';

export interface PomodoroCompletion {
  session: PomodoroSession;
  completedAt: number;
  duration: number;
}

interface PomodoroStore {
  duration: number;
  remaining: number;
  running: boolean;
  completing: boolean;
  startedAt: number | null;
  sessions: PomodoroSession[];
  lastCompletion: PomodoroCompletion | null;
  error: string | null;
  setDurationMinutes: (minutes: number) => void;
  start: () => void;
  pause: () => void;
  reset: () => void;
  tick: () => void;
  complete: () => Promise<PomodoroCompletion | null>;
  clearLastCompletion: (sessionId?: string) => void;
  fetchSessions: () => Promise<void>;
}

export const usePomodoroStore = create<PomodoroStore>((set, get) => ({
  duration: 25 * 60,
  remaining: 25 * 60,
  running: false,
  completing: false,
  startedAt: null,
  sessions: [],
  lastCompletion: null,
  error: null,

  setDurationMinutes: (minutes) => {
    const duration = minutes * 60;
    set({ duration, remaining: duration, running: false, completing: false, startedAt: null });
  },

  start: () => {
    set((s) => ({ running: true, startedAt: s.startedAt ?? Math.floor(Date.now() / 1000) }));
    void preparePomodoroNotifications();
  },
  pause: () => set({ running: false }),
  reset: () => set((s) => ({ running: false, remaining: s.duration, startedAt: null })),
  tick: () => {
    const { running, completing, remaining, complete } = get();
    if (!running || completing) return;
    if (remaining <= 1) {
      set({ running: false, remaining: 0 });
      void complete();
      return;
    }
    set({ remaining: remaining - 1 });
  },

  complete: async () => {
    const { completing, startedAt, duration } = get();
    if (completing) return null;
    const completedAt = Math.floor(Date.now() / 1000);
    set({ completing: true, running: false, remaining: 0, error: null });
    try {
      const session = await pomodoroService.createSession({
        started_at: startedAt ?? Math.floor(Date.now() / 1000),
        duration,
        completed: true,
      });
      const completion = { session, completedAt, duration };
      set((s) => ({
        completing: false,
        running: false,
        remaining: s.duration,
        startedAt: null,
        sessions: [session, ...s.sessions].slice(0, 10),
        lastCompletion: completion,
        error: null,
      }));
      void notifyPomodoroComplete();
      return completion;
    } catch (e) {
      set((s) => ({ error: String(e), completing: false, running: false, remaining: s.duration, startedAt: null }));
      throw e;
    }
  },

  clearLastCompletion: (sessionId) => {
    set((s) => {
      if (!s.lastCompletion) return s;
      if (sessionId && s.lastCompletion.session.id !== sessionId) return s;
      return { lastCompletion: null };
    });
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
