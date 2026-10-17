import { useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Sidebar } from './components/layout/Sidebar';
import { TodayPage } from './pages/TodayPage';
import { CalendarPage } from './pages/CalendarPage';
import { TasksPage } from './pages/TasksPage';
import { SettingsPage } from './pages/SettingsPage';
import { PomodoroPage } from './pages/PomodoroPage';
import { CockpitPage } from './pages/CockpitPage';
import { useUpdateStore } from './stores/updateStore';
import { useTaskStore } from './stores/taskStore';
import { usePomodoroStore } from './stores/pomodoroStore';
import { useSettingsStore } from './stores/settingsStore';
import { listenForTasksChanged } from './services/appEvents';
import { applyDocumentTheme } from './utils/appearance';

export default function App() {
  const checkForUpdates = useUpdateStore((state) => state.check);
  const fetchTasks = useTaskStore((state) => state.fetchTasks);
  const refreshPomodoroTimer = usePomodoroStore((state) => state.refreshTimer);
  const tickPomodoroTimer = usePomodoroStore((state) => state.tick);
  const fetchSettings = useSettingsStore((state) => state.fetchSettings);
  const isCockpit = typeof window !== 'undefined'
    && (window.location.search.includes('window=cockpit') || window.location.pathname === '/cockpit');

  useEffect(() => {
    applyDocumentTheme(useSettingsStore.getState().settings.theme);
    void fetchSettings();
  }, [fetchSettings]);

  useEffect(() => {
    if (!isCockpit) void checkForUpdates('startup');
  }, [checkForUpdates, isCockpit]);

  useEffect(() => {
    void refreshPomodoroTimer();
    const timer = window.setInterval(() => {
      void tickPomodoroTimer();
    }, 1000);
    return () => window.clearInterval(timer);
  }, [refreshPomodoroTimer, tickPomodoroTimer]);

  useEffect(() => {
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    void listenForTasksChanged(() => {
      void fetchTasks();
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      cleanup = unlisten;
    });
    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, [fetchTasks]);

  if (isCockpit) {
    return <CockpitPage />;
  }

  return (
    <BrowserRouter>
      <div className="flex h-screen bg-base-200 text-base-content">
        <Sidebar />
        <main className="app-canvas flex-1 flex flex-col overflow-hidden">
          <Routes>
            <Route path="/" element={<TodayPage />} />
            <Route path="/calendar" element={<CalendarPage />} />
            <Route path="/tasks" element={<TasksPage />} />
            <Route path="/pomodoro" element={<PomodoroPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/cockpit" element={<CockpitPage />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
