import { create } from 'zustand';
import { PomodoroSession, PomodoroTimerSnapshot } from '../types/task';
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
  refreshTimer: () => Promise<void>;
  setDurationMinutes: (minutes: number) => Promise<void>;
  start: () => Promise<void>;
  pause: () => Promise<void>;
  reset: () => Promise<void>;
  tick: () => Promise<void>;
  complete: () => Promise<PomodoroCompletion | null>;
  updateSessionRecord: (input: { id: string; note?: string | null; task_id?: string | null }) => Promise<PomodoroSession | null>;
  clearLastCompletion: (sessionId?: string) => void;
  fetchSessions: () => Promise<void>;
}

const defaultDuration = 25 * 60;

export const usePomodoroStore = create<PomodoroStore>((set, get) => ({
  duration: defaultDuration,
  remaining: defaultDuration,
  running: false,
  completing: false,
  startedAt: null,
  sessions: [],
  lastCompletion: null,
  error: null,

  refreshTimer: async () => {
    if (get().completing) return;
    try {
      await applyTimerSnapshot(await pomodoroService.getTimer(), set);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setDurationMinutes: async (minutes) => {
    try {
      await applyTimerSnapshot(await pomodoroService.setTimerDuration(minutes), set);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  start: async () => {
    try {
      await applyTimerSnapshot(await pomodoroService.startTimer(), set);
      void preparePomodoroNotifications();
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  pause: async () => {
    try {
      await applyTimerSnapshot(await pomodoroService.pauseTimer(), set);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  reset: async () => {
    try {
      await applyTimerSnapshot(await pomodoroService.resetTimer(), set);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  tick: async () => {
    if (get().completing) return;
    try {
      await applyTimerSnapshot(await pomodoroService.getTimer(), set);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  complete: async () => {
    if (get().completing) return null;
    set({ completing: true, error: null });
    try {
      return await applyTimerSnapshot(await pomodoroService.completeTimer(), set);
    } catch (e) {
      set({ error: String(e), completing: false });
      throw e;
    }
  },

  updateSessionRecord: async (input) => {
    try {
      const session = await pomodoroService.updateSessionRecord(input);
      if (!session) return null;
      set((s) => ({
        sessions: s.sessions.map((item) => item.id === session.id ? session : item),
        lastCompletion: s.lastCompletion?.session.id === session.id
          ? { ...s.lastCompletion, session }
          : s.lastCompletion,
        error: null,
      }));
      return session;
    } catch (e) {
      set({ error: String(e) });
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

async function applyTimerSnapshot(
  snapshot: PomodoroTimerSnapshot,
  set: (partial: Partial<PomodoroStore> | ((state: PomodoroStore) => Partial<PomodoroStore>)) => void,
): Promise<PomodoroCompletion | null> {
  const completion = snapshot.completion
    ? {
        session: snapshot.completion.session,
        completedAt: snapshot.completion.completed_at,
        duration: snapshot.completion.duration,
      }
    : null;

  set((state) => ({
    duration: snapshot.timer.duration,
    remaining: snapshot.timer.remaining,
    running: snapshot.timer.running,
    completing: false,
    startedAt: snapshot.timer.started_at ?? null,
    sessions: completion ? [completion.session, ...state.sessions].slice(0, 10) : state.sessions,
    lastCompletion: completion ?? state.lastCompletion,
    error: null,
  }));

  if (completion) {
    await notifyPomodoroComplete();
  }
  return completion;
}
