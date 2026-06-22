import { FormEvent, useCallback, useEffect, useState } from 'react';
import dayjs from 'dayjs';
import { TopBar } from '../components/layout/TopBar';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useTaskStore } from '../stores/taskStore';
import { buildPomodoroTaskRecord } from '../utils/cockpitTaskRecords';

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
  const { createTask } = useTaskStore();
  const [recordTitle, setRecordTitle] = useState('');
  const [isRecordDialogOpen, setIsRecordDialogOpen] = useState(false);
  const [lastCompletedAt, setLastCompletedAt] = useState<number | null>(null);
  const [lastDuration, setLastDuration] = useState(duration);

  useEffect(() => {
    void fetchSettings();
    void fetchSessions();
  }, [fetchSettings, fetchSessions]);

  useEffect(() => {
    setDurationMinutes(settings.pomodoro_minutes);
  }, [settings.pomodoro_minutes, setDurationMinutes]);

  const openRecordDialog = useCallback((completedAt: number, completedDuration: number) => {
    setLastCompletedAt(completedAt);
    setLastDuration(completedDuration);
    setRecordTitle('');
    setIsRecordDialogOpen(true);
  }, []);

  const completeAndAskRecord = useCallback(async () => {
    const completedAt = dayjs().unix();
    const completedDuration = duration;
    await complete();
    openRecordDialog(completedAt, completedDuration);
  }, [complete, duration, openRecordDialog]);

  useEffect(() => {
    if (!running) return;
    const id = window.setInterval(() => {
      if (remaining <= 1) {
        void completeAndAskRecord();
        return;
      }
      tick();
    }, 1000);
    return () => window.clearInterval(id);
  }, [completeAndAskRecord, remaining, running, tick]);

  const progress = Math.round(((duration - remaining) / duration) * 100);

  const closeRecordDialog = () => {
    setIsRecordDialogOpen(false);
    setRecordTitle('');
  };

  const submitPomodoroRecord = async (event: FormEvent) => {
    event.preventDefault();
    const title = recordTitle.trim();
    if (title && lastCompletedAt !== null) {
      await createTask(buildPomodoroTaskRecord(title, lastCompletedAt, lastDuration, 'pomodoro'));
    }
    closeRecordDialog();
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <TopBar title="专注" subtitle="本地番茄钟和完成记录" />
      <div className="grid flex-1 gap-5 overflow-y-auto p-5 lg:grid-cols-[minmax(0,1fr)_360px]">
        <section className="flex min-h-[420px] items-center justify-center rounded-xl border border-[#dbe5f1] bg-white/86 shadow-sm">
          <div className="w-full max-w-md px-8 text-center">
            <p className="text-xs font-medium uppercase tracking-wide text-slate-400">Focus timer</p>
            <div className="relative mx-auto mt-8 flex h-64 w-64 items-center justify-center rounded-full border border-[#dbe5f1] bg-[#f6f8fc] shadow-inner">
              <div className="radial-progress text-blue-600" style={{ '--value': progress, '--size': '15rem', '--thickness': '8px' } as React.CSSProperties}>
                <span className="text-5xl font-semibold tabular-nums text-slate-900">{formatRemaining(remaining)}</span>
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
              <button className="btn btn-ghost" onClick={() => void completeAndAskRecord()}>完成并记录</button>
            </div>
          </div>
        </section>

        <aside className="rounded-xl border border-[#dbe5f1] bg-white/86 p-5 shadow-sm">
          <h2 className="text-sm font-semibold text-slate-900">最近记录</h2>
          <div className="mt-4 space-y-3">
            {sessions.length === 0 ? (
              <p className="text-sm text-slate-400">暂无完成记录</p>
            ) : (
              sessions.map((session) => (
                <div key={session.id} className="rounded-lg bg-slate-50/90 px-3 py-2">
                  <p className="text-sm font-medium text-slate-800">{Math.round(session.duration / 60)} 分钟</p>
                  <p className="text-xs text-slate-500">{dayjs.unix(session.started_at).format('M月D日 HH:mm')}</p>
                </div>
              ))
            )}
          </div>
        </aside>
      </div>
      {isRecordDialogOpen && (
        <PomodoroRecordDialog
          title={recordTitle}
          onChangeTitle={setRecordTitle}
          onClose={closeRecordDialog}
          onSubmit={submitPomodoroRecord}
        />
      )}
    </div>
  );
}

function PomodoroRecordDialog({
  title,
  onChangeTitle,
  onClose,
  onSubmit,
}: {
  title: string;
  onChangeTitle: (title: string) => void;
  onClose: () => void;
  onSubmit: (event: FormEvent) => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/24 px-5 backdrop-blur-sm">
      <form className="w-full max-w-md rounded-2xl border border-[#dbe5f1] bg-white p-5 shadow-[0_24px_70px_rgba(30,41,59,0.18)]" onSubmit={onSubmit}>
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold text-slate-900">记录本轮番茄任务</h2>
            <p className="mt-1 text-sm text-slate-500">不填写也可以跳过，只保留番茄钟记录。</p>
          </div>
          <button className="btn btn-ghost btn-sm h-8 min-h-8 w-8 rounded-full p-0 text-slate-400 hover:bg-slate-100 hover:text-slate-700" onClick={onClose} type="button">
            ×
          </button>
        </div>
        <textarea
          autoFocus
          className="textarea textarea-bordered mt-5 min-h-28 w-full resize-none border-[#cbd8e8] bg-[#f8fbff] text-sm text-slate-800 outline-none focus:border-[#8fb7e8]"
          placeholder="本次番茄钟完成了什么任务"
          value={title}
          onChange={(event) => onChangeTitle(event.target.value)}
        />
        <div className="mt-4 flex justify-end gap-2">
          <button className="btn btn-ghost btn-sm" onClick={onClose} type="button">
            跳过
          </button>
          <button className="btn btn-primary btn-sm" type="submit">
            保存记录
          </button>
        </div>
      </form>
    </div>
  );
}

function formatRemaining(seconds: number): string {
  const minutes = Math.floor(seconds / 60).toString().padStart(2, '0');
  const secs = (seconds % 60).toString().padStart(2, '0');
  return `${minutes}:${secs}`;
}
