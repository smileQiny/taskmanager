import { FormEvent, useEffect, useMemo, useState } from 'react';
import dayjs from 'dayjs';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useTaskStore } from '../stores/taskStore';
import { Task } from '../types/task';
import { formatTaskTime, groupTasksForToday } from '../utils/taskUtils';
import { listenForSettingsUpdates } from '../services/appEvents';
import { getCockpitTransparency } from '../utils/cockpitTransparency';

const priorityTone: Record<Task['priority'], string> = {
  high: 'border-rose-200 bg-rose-50 text-rose-600',
  medium: 'border-blue-100 bg-blue-50 text-blue-700',
  low: 'border-slate-200 bg-slate-50 text-slate-600',
};

export function CockpitPage() {
  const {
    tasks,
    error,
    fetchTasks,
    createTask,
    updateTask,
    deleteTask,
  } = useTaskStore();
  const {
    duration,
    remaining,
    running,
    start,
    pause,
    reset,
    tick,
    complete,
    setDurationMinutes,
  } = usePomodoroStore();
  const {
    settings,
    applySettings,
    fetchSettings,
  } = useSettingsStore();
  const [title, setTitle] = useState('');
  const [isPinned, setIsPinned] = useState(true);
  const [windowError, setWindowError] = useState<string | null>(null);
  const transparency = useMemo(
    () => getCockpitTransparency(settings.cockpit_opacity),
    [settings.cockpit_opacity],
  );

  useEffect(() => {
    void fetchTasks();
    void fetchSettings();
  }, [fetchSettings, fetchTasks]);

  useEffect(() => {
    document.documentElement.classList.add('cockpit-window');
    document.body.classList.add('cockpit-window');
    return () => {
      document.documentElement.classList.remove('cockpit-window');
      document.body.classList.remove('cockpit-window');
    };
  }, []);

  useEffect(() => {
    let cleanup: (() => void) | null = null;
    let cancelled = false;
    void listenForSettingsUpdates((nextSettings) => {
      applySettings(nextSettings);
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
  }, [applySettings]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const alwaysOnTop = await getCurrentWindow().isAlwaysOnTop();
        if (!cancelled) setIsPinned(alwaysOnTop);
      } catch {
        // Browser preview has no native window state to read.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(tick, 1000);
    return () => window.clearInterval(timer);
  }, [running, tick]);

  const grouped = useMemo(() => groupTasksForToday(tasks), [tasks]);
  const cockpitTasks = useMemo(
    () => [...grouped.overdue, ...grouped.today, ...grouped.unscheduled].slice(0, 7),
    [grouped],
  );
  const doneToday = tasks.filter((task) => (
    task.status === 'done'
    && dayjs.unix(task.updated_at).isSame(dayjs(), 'day')
  )).length;
  const activeCount = grouped.overdue.length + grouped.today.length + grouped.unscheduled.length;
  const progress = duration > 0
    ? Math.min(100, Math.max(0, Math.round(((duration - remaining) / duration) * 100)))
    : 0;

  const submitTask = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = title.trim();
    if (!trimmed) return;
    await createTask({
      title: trimmed,
      status: 'todo',
      priority: 'medium',
      start_time: dayjs().unix(),
      all_day: false,
      tags: 'cockpit',
    });
    setTitle('');
  };

  const togglePin = async () => {
    const next = !isPinned;
    setWindowError(null);
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().setAlwaysOnTop(next);
      setIsPinned(next);
    } catch {
      setWindowError('无法切换置顶状态，请确认应用窗口权限已启用。');
    }
  };

  const closeWindow = async () => {
    setWindowError(null);
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch {
      window.close();
      setWindowError('无法关闭窗口，请从系统窗口控制关闭。');
    }
  };

  const startDragging = async (event: React.MouseEvent<HTMLElement>) => {
    if (event.button !== 0 || shouldSkipWindowDrag(event.target)) return;
    setWindowError(null);
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().startDragging();
    } catch {
      // Browser preview cannot start native window dragging.
    }
  };

  return (
    <main className="h-screen overflow-hidden bg-transparent p-2 text-slate-900">
      <section
        className="relative flex h-full cursor-move flex-col overflow-hidden rounded-[24px] border border-white/45 shadow-[0_18px_48px_rgba(71,85,105,0.18)] transition-colors duration-200"
        style={{ backgroundColor: transparency.shellBackground }}
        data-tauri-drag-region
        onMouseDown={(event) => void startDragging(event)}
      >
        <div className="pointer-events-none absolute inset-0" style={{ background: transparency.glowBackground }} />

        <header
          className="relative h-11 shrink-0 select-none border-b border-slate-200/80"
          style={{ backgroundColor: transparency.headerBackground }}
          data-tauri-drag-region
        >
          <div className="absolute left-3 top-2 z-10 flex items-center gap-1.5">
            <span className="grid h-7 w-7 place-items-center rounded-xl bg-[#2474d6] text-[15px] font-black text-white shadow-sm shadow-blue-200">
              ✓
            </span>
          </div>
          <div className="pointer-events-none absolute inset-x-14 top-0 flex h-11 items-center justify-center">
            <p className="truncate text-[13px] font-semibold text-slate-700">今日驾驶舱</p>
          </div>
          <div className="absolute right-2.5 top-2 z-20 flex items-center gap-1">
            <button
              aria-label={isPinned ? '取消置顶' : '置顶窗口'}
              className={`grid h-7 w-7 place-items-center rounded-full border text-[14px] leading-none shadow-sm transition ${isPinned ? 'border-blue-200 bg-blue-50 text-blue-700' : 'border-slate-200 bg-white text-slate-500 hover:border-blue-200 hover:text-blue-700'}`}
              data-no-window-drag
              onMouseDown={(event) => event.stopPropagation()}
              onClick={() => void togglePin()}
              title={isPinned ? '取消置顶' : '置顶窗口'}
            >
              ⌖
            </button>
            <button
              aria-label="关闭"
              className="grid h-7 w-7 place-items-center rounded-full border border-slate-200 bg-white text-[18px] leading-none text-slate-500 shadow-sm transition hover:border-rose-200 hover:bg-rose-50 hover:text-rose-600"
              data-no-window-drag
              onMouseDown={(event) => event.stopPropagation()}
              onClick={() => void closeWindow()}
              title="关闭"
            >
              ×
            </button>
          </div>
        </header>

        <div className="relative flex min-h-0 flex-1 flex-col gap-2 p-2.5">
          <div className="grid grid-cols-[1fr_82px] gap-2">
            <div className="min-w-0 rounded-2xl border border-white/55 px-3 py-2 shadow-sm" style={{ backgroundColor: transparency.panelBackground }}>
              <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-blue-600">{dayjs().format('YYYY.MM.DD')}</p>
              <p className="mt-0.5 truncate text-base font-bold text-slate-900">{activeCount} 个待处理</p>
            </div>
            <div className="rounded-2xl border border-white/50 px-3 py-2 text-right shadow-sm" style={{ backgroundColor: transparency.panelSoftBackground }}>
              <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-slate-500">完成</p>
              <p className="mt-0.5 text-base font-bold text-blue-700">{doneToday}</p>
            </div>
          </div>

          <form className="flex h-10 gap-1.5" onSubmit={(event) => void submitTask(event)}>
            <input
              className="min-w-0 flex-1 rounded-full border border-slate-200 bg-white px-3 text-[13px] text-slate-800 shadow-sm outline-none transition placeholder:text-slate-400 focus:border-blue-300 focus:ring-2 focus:ring-blue-100"
              style={{ backgroundColor: transparency.controlBackground }}
              data-no-window-drag
              placeholder="快速添加今天的任务"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
            />
            <button
              className="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-[#2474d6] text-xl font-semibold leading-none text-white shadow-sm shadow-blue-200 transition hover:bg-[#1e63bd]"
              data-no-window-drag
              title="添加任务"
            >
              +
            </button>
          </form>

          {error && (
            <div className="rounded-xl border border-rose-200 bg-rose-50 px-2.5 py-1.5 text-xs text-rose-700">
              {error}
            </div>
          )}

          {windowError && (
            <div className="rounded-xl border border-amber-200 bg-amber-50 px-2.5 py-1.5 text-xs text-amber-800">
              {windowError}
            </div>
          )}

          <section className="min-h-0 flex-1 overflow-hidden rounded-2xl border border-white/55 shadow-sm" style={{ backgroundColor: transparency.panelBackground }}>
            <div className="flex h-8 items-center justify-between border-b border-slate-100 px-3">
              <p className="text-xs font-bold text-slate-700">任务队列</p>
              <span className="rounded-full border border-blue-100 bg-blue-50 px-2 py-0.5 text-[10px] font-semibold text-blue-700">当前</span>
            </div>
            <div className="h-[calc(100%-2rem)] overflow-y-auto">
              {cockpitTasks.length === 0 ? (
                <div className="flex h-full items-center justify-center px-5 text-center text-xs font-medium text-slate-400">
                  今天没有待处理任务
                </div>
              ) : (
                cockpitTasks.map((task) => (
                  <CockpitTaskRow
                    key={task.id}
                    task={task}
                    onComplete={() => void updateTask({ id: task.id, status: task.status === 'done' ? 'todo' : 'done' })}
                    onFocus={() => void updateTask({ id: task.id, status: 'in_progress' })}
                    onDelete={() => void deleteTask(task.id)}
                  />
                ))
              )}
            </div>
          </section>

          <section className="rounded-2xl border border-white/55 px-3 py-2.5 shadow-sm" style={{ backgroundColor: transparency.panelBackground }}>
            <div className="flex items-center gap-3">
              <div
                className="grid h-16 w-16 shrink-0 place-items-center rounded-full shadow-inner"
                style={{
                  background: `conic-gradient(#2f78d0 ${progress * 3.6}deg, ${transparency.progressTrack} 0deg)`,
                }}
              >
                <div className="grid h-[52px] w-[52px] place-items-center rounded-full text-[13px] font-black tabular-nums text-slate-800 shadow-sm" style={{ backgroundColor: transparency.controlBackground }}>
                  {formatRemaining(remaining)}
                </div>
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center justify-between">
                  <p className="text-xs font-bold text-slate-800">专注倒计时</p>
                  <span className="text-[10px] font-semibold text-blue-700">{Math.round(duration / 60)} min</span>
                </div>
                <div className="mt-2 flex gap-1">
                  {[15, 25, 45].map((minutes) => (
                    <button
                      key={minutes}
                      className="rounded-full border border-blue-100 bg-blue-50 px-2 py-1 text-[10px] font-semibold text-blue-700 transition hover:border-blue-200 hover:bg-blue-100"
                      data-no-window-drag
                      onClick={() => setDurationMinutes(minutes)}
                    >
                      {minutes}
                    </button>
                  ))}
                </div>
                <div className="mt-2 flex gap-1.5">
                  <button className="rounded-full bg-slate-900 px-3 py-1 text-xs font-semibold text-white transition hover:bg-slate-700" data-no-window-drag onClick={running ? pause : start}>
                    {running ? '暂停' : '开始'}
                  </button>
                  <button className="rounded-full border border-slate-200 px-3 py-1 text-xs font-semibold text-slate-600 transition hover:border-slate-300 hover:text-slate-900" style={{ backgroundColor: transparency.controlBackground }} data-no-window-drag onClick={reset}>
                    重置
                  </button>
                  <button className="rounded-full border border-emerald-200 bg-emerald-50 px-3 py-1 text-xs font-semibold text-emerald-700 transition hover:bg-emerald-100" data-no-window-drag onClick={() => void complete()}>
                    完成
                  </button>
                </div>
              </div>
            </div>
          </section>
        </div>
      </section>
    </main>
  );
}

function CockpitTaskRow({
  task,
  onComplete,
  onFocus,
  onDelete,
}: {
  task: Task;
  onComplete: () => void;
  onFocus: () => void;
  onDelete: () => void;
}) {
  const isDone = task.status === 'done';
  const isFocused = task.status === 'in_progress';

  return (
    <div className={`group grid grid-cols-[auto_1fr_auto] items-center gap-2 border-b border-slate-100 px-2.5 py-2 last:border-b-0 ${isFocused ? 'bg-blue-50/60' : ''}`}>
      <button
        aria-label={isDone ? '标记未完成' : '完成任务'}
        className={`grid h-6 w-6 shrink-0 place-items-center rounded-full border text-[12px] font-bold transition ${isDone ? 'border-blue-500 bg-blue-500 text-white' : 'border-slate-200 bg-white text-transparent hover:border-blue-300 hover:text-blue-500'}`}
        data-no-window-drag
        onClick={onComplete}
        title={isDone ? '标记未完成' : '完成任务'}
      >
        ✓
      </button>
      <div className="min-w-0">
        <div className="flex min-w-0 items-center gap-1.5">
          <p className={`truncate text-[13px] font-semibold ${isDone ? 'text-slate-400 line-through' : 'text-slate-800'}`}>{task.title}</p>
          <span className={`shrink-0 rounded-full border px-1.5 py-0.5 text-[9px] font-bold ${priorityTone[task.priority]}`}>{priorityLabel(task.priority)}</span>
        </div>
        <p className="mt-0.5 truncate text-[10px] font-medium text-slate-400">{isFocused ? '专注中 · ' : ''}{formatTaskTime(task)}</p>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <button
          aria-label="设为专注任务"
          className={`grid h-7 w-7 place-items-center rounded-full border text-[13px] transition ${isFocused ? 'border-blue-200 bg-blue-50 text-blue-700' : 'border-slate-200 bg-white text-slate-500 hover:border-blue-200 hover:text-blue-700'}`}
          data-no-window-drag
          onClick={onFocus}
          title="设为专注任务"
        >
          ◷
        </button>
        <button
          aria-label="删除任务"
          className="grid h-7 w-7 place-items-center rounded-full border border-slate-200 bg-white text-[17px] leading-none text-slate-400 transition hover:border-rose-200 hover:bg-rose-50 hover:text-rose-600"
          data-no-window-drag
          onClick={onDelete}
          title="删除"
        >
          ×
        </button>
      </div>
    </div>
  );
}

function priorityLabel(priority: Task['priority']) {
  return {
    high: '高',
    medium: '中',
    low: '低',
  }[priority];
}

function formatRemaining(seconds: number): string {
  const minutes = Math.floor(seconds / 60).toString().padStart(2, '0');
  const secs = (seconds % 60).toString().padStart(2, '0');
  return `${minutes}:${secs}`;
}

function shouldSkipWindowDrag(target: EventTarget): boolean {
  return target instanceof Element
    && Boolean(target.closest('button, input, textarea, select, a, [data-no-window-drag]'));
}
