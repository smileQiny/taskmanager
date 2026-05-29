import { useEffect } from 'react';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Sidebar } from './components/layout/Sidebar';
import { TodayPage } from './pages/TodayPage';
import { CalendarPage } from './pages/CalendarPage';
import { TasksPage } from './pages/TasksPage';
import { SettingsPage } from './pages/SettingsPage';
import { PomodoroPage } from './pages/PomodoroPage';
import { useUpdateStore } from './stores/updateStore';

export default function App() {
  const checkForUpdates = useUpdateStore((state) => state.check);

  useEffect(() => {
    void checkForUpdates('startup');
  }, [checkForUpdates]);

  return (
    <BrowserRouter>
      <div className="flex h-screen bg-slate-100 text-slate-900">
        <Sidebar />
        <main className="flex-1 flex flex-col overflow-hidden">
          <Routes>
            <Route path="/" element={<TodayPage />} />
            <Route path="/calendar" element={<CalendarPage />} />
            <Route path="/tasks" element={<TasksPage />} />
            <Route path="/pomodoro" element={<PomodoroPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
