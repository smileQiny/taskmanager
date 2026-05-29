import { useEffect } from 'react';
import dayjs from 'dayjs';
import { TopBar } from '../components/layout/TopBar';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useSettingsStore } from '../stores/settingsStore';

export function PomodoroPage() {
  const {
    duration,
    remaining,
    running,
    sessions,
    error,
    setDurationMinutes,
    start,
    pause,
    reset,
    tick,
    complete,
    fetchSessions,
  } = usePomodoroStore();
  const { settings, fetchSettings } = useSettingsStore();

  useEffect(() => {
    void fetchSettings();
    void fetchSessions();
  }, [fetchSettings, fetchSessions]);

  useEffect(() => {
    setDurationMinutes(settings.pomodoro_minutes);
  }, [settings.pomodoro_minutes, setDurationMinutes]);

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(tick, 1000);
    return () => window.clearInterval(id);
  }, [running, tick]);

  const progress = Math.round(((duration - remaining) / duration) * 100);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TopBar title="专注" subtitle="本地番茄钟和完成记录" />
      <div className="grid flex-1 gap-5 overflow-y-auto p-5 lg:grid-cols-[minmax(0,1fr)_360px]">
        <section className="flex min-h-[420px] items-center justify-center rounded-md border border-slate-200 bg-white">
          <div className="w-full max-w-md px-8 text-center">
            <p className="text-xs font-medium uppercase tracking-wide text-slate-400">Focus timer</p>
            <div className="relative mx-auto mt-8 flex h-64 w-64 items-center justify-center rounded-full border border-slate-200 bg-slate-50">
              <div className="radial-progress text-teal-500" style={{ '--value': progress, '--size': '15rem', '--thickness': '8px' } as React.CSSProperties}>
                <span className="text-5xl font-semibold tabular-nums text-slate-950">{formatRemaining(remaining)}</span>
              </div>
            </div>
            {error && <div className="mt-5 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>}
            <div className="mt-8 flex justify-center gap-2">
              {running ? (
                <button className="btn btn-primary" onClick={pause}>暂停</button>
              ) : (
                <button className="btn btn-primary" onClick={start}>开始</button>
              )}
              <button className="btn" onClick={reset}>重置</button>
              <button className="btn btn-ghost" onClick={() => void complete()}>完成并记录</button>
            </div>
          </div>
        </section>

        <aside className="rounded-md border border-slate-200 bg-white p-5">
          <h2 className="text-sm font-semibold text-slate-950">最近记录</h2>
          <div className="mt-4 space-y-3">
            {sessions.length === 0 ? (
              <p className="text-sm text-slate-400">暂无完成记录</p>
            ) : (
              sessions.map((session) => (
                <div key={session.id} className="rounded-md bg-slate-50 px-3 py-2">
                  <p className="text-sm font-medium text-slate-800">{Math.round(session.duration / 60)} 分钟</p>
                  <p className="text-xs text-slate-500">{dayjs.unix(session.started_at).format('M月D日 HH:mm')}</p>
                </div>
              ))
            )}
          </div>
        </aside>
      </div>
    </div>
  );
}

function formatRemaining(seconds: number): string {
  const minutes = Math.floor(seconds / 60).toString().padStart(2, '0');
  const secs = (seconds % 60).toString().padStart(2, '0');
  return `${minutes}:${secs}`;
}
