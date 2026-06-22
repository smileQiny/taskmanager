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
import { listenForTasksChanged } from './services/appEvents';

export default function App() {
  const checkForUpdates = useUpdateStore((state) => state.check);
  const fetchTasks = useTaskStore((state) => state.fetchTasks);
  const isCockpit = typeof window !== 'undefined'
    && (window.location.search.includes('window=cockpit') || window.location.pathname === '/cockpit');

  useEffect(() => {
    if (!isCockpit) void checkForUpdates('startup');
  }, [checkForUpdates, isCockpit]);

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
      <div data-theme="taskmanager" className="flex h-screen bg-[#eef2f7] text-slate-900">
        <Sidebar />
        <main className="flex-1 flex flex-col overflow-hidden bg-[radial-gradient(circle_at_12%_0%,rgba(218,226,242,0.82),transparent_34%),radial-gradient(circle_at_92%_4%,rgba(214,235,229,0.55),transparent_32%)]">
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
