import { FormEvent, useCallback, useEffect, useState } from 'react';
import dayjs from 'dayjs';
import { TopBar } from '../components/layout/TopBar';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useTaskStore } from '../stores/taskStore';
import { PomodoroSession } from '../types/task';
import { buildPomodoroTaskRecord } from '../utils/cockpitTaskRecords';

export function PomodoroPage() {
  const {
    duration,
    remaining,
    running,
    sessions,
    error,
    lastCompletion,
    setDurationMinutes,
    start,
    pause,
    reset,
    complete,
    updateSessionRecord,
    clearLastCompletion,
    fetchSessions,
  } = usePomodoroStore();
  const { createTask, updateTask } = useTaskStore();
  const [recordTitle, setRecordTitle] = useState('');
  const [isRecordDialogOpen, setIsRecordDialogOpen] = useState(false);
  const [editingSession, setEditingSession] = useState<PomodoroSession | null>(null);
  const [durationInput, setDurationInput] = useState(String(Math.round(duration / 60)));

  useEffect(() => {
    void fetchSessions();
  }, [fetchSessions]);

  useEffect(() => {
    setDurationInput(String(Math.round(duration / 60)));
  }, [duration]);

  const openRecordDialog = useCallback((session: PomodoroSession) => {
    setEditingSession(session);
    setRecordTitle(session.note ?? '');
    setIsRecordDialogOpen(true);
  }, []);

  const completeAndAskRecord = useCallback(async () => {
    await complete();
  }, [complete]);

  useEffect(() => {
    if (!lastCompletion) return;
    openRecordDialog(lastCompletion.session);
    clearLastCompletion(lastCompletion.session.id);
  }, [clearLastCompletion, lastCompletion, openRecordDialog]);

  const progress = Math.round(((duration - remaining) / duration) * 100);

  const closeRecordDialog = () => {
    setIsRecordDialogOpen(false);
    setRecordTitle('');
    setEditingSession(null);
  };

  const submitDuration = async (event: FormEvent) => {
    event.preventDefault();
    await applyDurationInput();
  };

  const adjustDuration = (delta: number) => {
    if (running) return;
    const next = normalizeDurationMinutes(Number(durationInput), Math.round(duration / 60)) + delta;
    const clamped = normalizeDurationMinutes(next, Math.round(duration / 60));
    setDurationInput(String(clamped));
    void setDurationMinutes(clamped);
  };

  const applyDurationInput = async () => {
    if (running) {
      setDurationInput(String(Math.round(duration / 60)));
      return;
    }
    const next = normalizeDurationMinutes(Number(durationInput), Math.round(duration / 60));
    setDurationInput(String(next));
    void setDurationMinutes(next);
  };

  const submitPomodoroRecord = async (event: FormEvent) => {
    event.preventDefault();
    const title = recordTitle.trim();
    if (title && editingSession) {
      const completedAt = editingSession.started_at + editingSession.duration;
      const taskRecord = buildPomodoroTaskRecord(title, completedAt, editingSession.duration, 'pomodoro');
      let taskId = editingSession.task_id ?? null;
      if (taskId) {
        const updated = await updateTask({
          id: taskId,
          title: taskRecord.title,
          description: taskRecord.description ?? null,
          status: 'done',
          priority: 'medium',
          start_time: taskRecord.start_time ?? null,
          end_time: taskRecord.end_time ?? null,
          all_day: false,
          tags: taskRecord.tags ?? null,
        });
        if (!updated) {
          const created = await createTask(taskRecord);
          taskId = created.id;
        }
      } else {
        const created = await createTask(taskRecord);
        taskId = created.id;
      }
      await updateSessionRecord({
        id: editingSession.id,
        note: title,
        task_id: taskId,
      });
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
            <form className="mx-auto mt-6 flex w-full max-w-xs items-center justify-center gap-2" onSubmit={submitDuration}>
              <button
                aria-label="减少专注分钟"
                className="btn btn-sm h-10 min-h-10 w-10 rounded-xl border-slate-200 bg-white/90 p-0 text-lg text-slate-600"
                disabled={running}
                onClick={() => adjustDuration(-5)}
                type="button"
              >
                -
              </button>
              <label className="relative min-w-0 flex-1">
                <input
                  aria-label="专注分钟"
                  className="input input-bordered h-10 w-full border-[#cbd8e8] bg-white/90 pr-14 text-center text-base font-semibold tabular-nums text-slate-900"
                  max={120}
                  min={5}
                  step={1}
                  type="number"
                  value={durationInput}
                  disabled={running}
                  onBlur={() => void applyDurationInput()}
                  onChange={(event) => setDurationInput(event.target.value)}
                />
                <span className="pointer-events-none absolute right-4 top-1/2 -translate-y-1/2 text-xs font-semibold text-slate-400">
                  min
                </span>
              </label>
              <button
                aria-label="增加专注分钟"
                className="btn btn-sm h-10 min-h-10 w-10 rounded-xl border-slate-200 bg-white/90 p-0 text-lg text-slate-600"
                disabled={running}
                onClick={() => adjustDuration(5)}
                type="button"
              >
                +
              </button>
            </form>
            <div className="mx-auto mt-2 grid max-w-xs grid-cols-4 gap-1 rounded-xl border border-[#dbe5f1] bg-white/70 p-1">
              {[15, 25, 45, 60].map((minutes) => (
                <button
                  key={minutes}
                  className={`h-8 rounded-lg px-3 text-xs font-semibold transition ${
                    Math.round(duration / 60) === minutes
                      ? 'bg-blue-600 text-white shadow-sm'
                      : running
                        ? 'cursor-not-allowed text-slate-300'
                        : 'text-slate-500 hover:bg-slate-100 hover:text-slate-900'
                  }`}
                  disabled={running}
                  onClick={() => void setDurationMinutes(minutes)}
                  type="button"
                >
                  {minutes}
                </button>
              ))}
            </div>
            {error && <div className="mt-5 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>}
            <div className="mt-8 flex justify-center gap-2">
              {running ? (
                <button className="btn btn-primary" onClick={() => void pause()}>暂停</button>
              ) : (
                <button className="btn btn-primary" onClick={() => void start()}>开始</button>
              )}
              <button className="btn" onClick={() => void reset()}>重置</button>
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
                <button
                  key={session.id}
                  className="w-full rounded-lg bg-slate-50/90 px-3 py-2 text-left transition hover:bg-white hover:shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-200"
                  onClick={() => openRecordDialog(session)}
                  type="button"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-slate-800">{Math.round(session.duration / 60)} 分钟</p>
                      <p className="text-xs text-slate-500">{dayjs.unix(session.started_at).format('M月D日 HH:mm')}</p>
                    </div>
                    <span className="rounded-full bg-white px-2 py-0.5 text-[11px] font-semibold text-blue-600 ring-1 ring-blue-100">
                      编辑
                    </span>
                  </div>
                  <p className={`mt-2 line-clamp-2 text-xs ${session.note ? 'text-slate-700' : 'text-slate-400'}`}>
                    {session.note || '未记录本轮完成内容'}
                  </p>
                </button>
              ))
            )}
          </div>
        </aside>
      </div>
      {isRecordDialogOpen && (
        <PomodoroRecordDialog
          title={recordTitle}
          session={editingSession}
          onChangeTitle={setRecordTitle}
          onClose={closeRecordDialog}
          onSubmit={submitPomodoroRecord}
        />
      )}
    </div>
  );
}

function PomodoroRecordDialog({
  session,
  title,
  onChangeTitle,
  onClose,
  onSubmit,
}: {
  session: PomodoroSession | null;
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
            <h2 className="text-base font-semibold text-slate-900">编辑番茄记录</h2>
            <p className="mt-1 text-sm text-slate-500">
              {session ? `${dayjs.unix(session.started_at).format('M月D日 HH:mm')} · ${Math.round(session.duration / 60)} 分钟` : '补写本轮完成内容'}
            </p>
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

function normalizeDurationMinutes(value: number, fallback: number): number {
  const minutes = Number.isFinite(value) ? value : fallback;
  return Math.min(120, Math.max(5, Math.round(minutes)));
}
