import { FormEvent, MouseEvent, useEffect, useMemo, useState } from 'react';
import dayjs from 'dayjs';
import { usePomodoroStore } from '../stores/pomodoroStore';
import { useSettingsStore } from '../stores/settingsStore';
import { useTaskStore } from '../stores/taskStore';
import { Task } from '../types/task';
import { formatTaskTime, groupTasksForToday } from '../utils/taskUtils';
import { listenForSettingsUpdates } from '../services/appEvents';
import { getCockpitTransparency } from '../utils/cockpitTransparency';
import { buildCompletedTaskRecord, buildPomodoroTaskRecord } from '../utils/cockpitTaskRecords';

const pageCount = 3;

const priorityTone: Record<Task['priority'], string> = {
  high: 'border-rose-400/20 bg-rose-400/12 text-rose-200',
  medium: 'border-sky-300/20 bg-sky-300/12 text-sky-100',
  low: 'border-zinc-300/15 bg-zinc-300/10 text-zinc-300',
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
    lastCompletion,
    clearLastCompletion,
    setDurationMinutes,
  } = usePomodoroStore();
  const {
    settings,
    applySettings,
    fetchSettings,
  } = useSettingsStore();
  const [title, setTitle] = useState('');
  const [completedTitle, setCompletedTitle] = useState('');
  const [pomodoroTitle, setPomodoroTitle] = useState('');
  const [isPinned, setIsPinned] = useState(true);
  const [pageIndex, setPageIndex] = useState(0);
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
        const currentWindow = getCurrentWindow();
        await currentWindow.setShadow(false);
        const alwaysOnTop = await currentWindow.isAlwaysOnTop();
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

  useEffect(() => {
    if (!lastCompletion) return;
    const trimmed = pomodoroTitle.trim();
    clearLastCompletion(lastCompletion.session.id);
    if (!trimmed) return;
    void createTask(buildPomodoroTaskRecord(trimmed, lastCompletion.completedAt, lastCompletion.duration))
      .then(() => setPomodoroTitle(''));
  }, [clearLastCompletion, createTask, lastCompletion, pomodoroTitle]);

  const grouped = useMemo(() => groupTasksForToday(tasks), [tasks]);
  const cockpitTasks = useMemo(
    () => [...grouped.overdue, ...grouped.today, ...grouped.unscheduled].slice(0, 5),
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
  const pageLabel = ['任务', '专注', '记录'][pageIndex];

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

  const submitCompletedTask = async (event: FormEvent) => {
    event.preventDefault();
    const trimmed = completedTitle.trim();
    if (!trimmed) return;
    await createTask(buildCompletedTaskRecord(trimmed, dayjs().unix()));
    setCompletedTitle('');
  };

  const completePomodoro = async () => {
    await complete();
  };

  const goPreviousPage = () => {
    setPageIndex((current) => (current + pageCount - 1) % pageCount);
  };

  const goNextPage = () => {
    setPageIndex((current) => (current + 1) % pageCount);
  };

  const togglePin = async () => {
    const next = !isPinned;
    setWindowError(null);
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().setAlwaysOnTop(next);
      setIsPinned(next);
    } catch {
      setWindowError('无法切换置顶状态');
    }
  };

  const closeWindow = async () => {
    setWindowError(null);
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
    } catch {
      window.close();
      setWindowError('无法关闭窗口');
    }
  };

  const startDragging = async (event: MouseEvent<HTMLElement>) => {
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
    <main className="h-screen overflow-hidden bg-transparent text-slate-100">
      <section
        className="relative flex h-full cursor-move flex-col overflow-hidden rounded-[18px] border border-white/[0.08] transition-colors duration-200"
        style={{ backgroundColor: transparency.shellBackground }}
        data-tauri-drag-region
        onMouseDown={(event) => void startDragging(event)}
      >
        <div className="pointer-events-none absolute inset-0" style={{ background: transparency.glowBackground }} />
        <div className="pointer-events-none absolute inset-x-0 top-0 h-px bg-white/[0.16]" />

        <header
          className="relative flex h-8 shrink-0 select-none items-center justify-between px-2.5"
          style={{ backgroundColor: transparency.headerBackground }}
          data-tauri-drag-region
        >
          <div className="z-10 grid h-5 w-5 place-items-center rounded-full border border-white/[0.16] bg-white/[0.08] text-[10px] font-bold text-slate-50">
            ✓
          </div>
          <div className="pointer-events-none absolute inset-x-16 top-0 flex h-8 items-center justify-center">
            <p className="truncate text-[13px] font-semibold leading-none text-slate-50">今日驾驶舱</p>
          </div>
          <div className="z-20 flex items-center gap-1">
            <IconButton
              active={isPinned}
              label={isPinned ? '取消置顶' : '置顶窗口'}
              onClick={() => void togglePin()}
              title={isPinned ? '取消置顶' : '置顶窗口'}
            >
              ⌖
            </IconButton>
            <IconButton label="关闭" onClick={() => void closeWindow()} title="关闭">
              ×
            </IconButton>
          </div>
        </header>

        <div className="relative flex h-8 shrink-0 items-center gap-1.5 px-2 pt-1.5">
          <button
            className="h-6 rounded-md border border-white/[0.08] bg-white/[0.06] px-2 text-[11px] font-semibold text-slate-100 transition hover:bg-white/[0.1]"
            data-no-window-drag
            onClick={() => setPageIndex((current) => (current + 1) % pageCount)}
            title="切换页面"
            type="button"
          >
            {pageLabel}
          </button>
          <IconButton label="上一页" onClick={goPreviousPage} title="上一页">
            &lt;
          </IconButton>
          <span className="min-w-10 text-center text-[12px] font-semibold tabular-nums text-slate-50">
            {pageIndex + 1} / {pageCount}
          </span>
          <IconButton label="下一页" onClick={goNextPage} title="下一页">
            &gt;
          </IconButton>
          <span className="ml-auto rounded-md bg-white/[0.07] px-2 py-1 text-[11px] font-semibold text-slate-50">
            当前
          </span>
        </div>

        <div className="relative flex min-h-0 flex-1 flex-col px-1.5 pb-1 pt-1.5">
          {pageIndex === 0 && (
            <TaskCockpitPage
              activeCount={activeCount}
              doneToday={doneToday}
              error={error}
              tasks={cockpitTasks}
              title={title}
              windowError={windowError}
              onTitleChange={setTitle}
              onSubmitTask={submitTask}
              onComplete={(task) => void updateTask({ id: task.id, status: task.status === 'done' ? 'todo' : 'done' })}
              onFocus={(task) => void updateTask({ id: task.id, status: 'in_progress' })}
              onDelete={(task) => void deleteTask(task.id)}
            />
          )}
          {pageIndex === 1 && (
            <PomodoroCockpitPage
              duration={duration}
              pomodoroTitle={pomodoroTitle}
              progress={progress}
              remaining={remaining}
              running={running}
              onPomodoroTitleChange={setPomodoroTitle}
              onComplete={() => void completePomodoro()}
              onPause={pause}
              onReset={reset}
              onSetDurationMinutes={setDurationMinutes}
              onStart={start}
            />
          )}
          {pageIndex === 2 && (
            <CompletedRecordPage
              doneToday={doneToday}
              title={completedTitle}
              windowError={windowError}
              error={error}
              onTitleChange={setCompletedTitle}
              onSubmit={submitCompletedTask}
            />
          )}
        </div>
      </section>
    </main>
  );
}

function TaskCockpitPage({
  activeCount,
  doneToday,
  error,
  tasks,
  title,
  windowError,
  onTitleChange,
  onSubmitTask,
  onComplete,
  onFocus,
  onDelete,
}: {
  activeCount: number;
  doneToday: number;
  error: string | null;
  tasks: Task[];
  title: string;
  windowError: string | null;
  onTitleChange: (title: string) => void;
  onSubmitTask: (event: FormEvent) => void;
  onComplete: (task: Task) => void;
  onFocus: (task: Task) => void;
  onDelete: (task: Task) => void;
}) {
  return (
    <>
      <div className="mb-1.5 flex h-8 items-center justify-between border-b border-white/[0.045] px-1">
        <div className="min-w-0">
          <p className="truncate text-[11px] font-medium uppercase text-slate-300">
            {dayjs().format('YYYY.MM.DD')}
          </p>
          <p className="truncate text-[17px] font-semibold leading-none text-slate-50">{activeCount} 个待处理</p>
        </div>
        <div className="text-right">
          <p className="text-[10px] font-medium uppercase text-slate-400">DONE</p>
          <p className="text-[16px] font-semibold leading-none tabular-nums text-sky-100">{doneToday}</p>
        </div>
      </div>

      {(error || windowError) && (
        <div className="mb-1.5 rounded-lg border border-amber-200/20 bg-amber-300/12 px-2 py-1 text-[10px] font-semibold text-amber-100">
          {error ?? windowError}
        </div>
      )}

      <form className="mb-1.5 flex h-8 shrink-0 items-center gap-1.5 px-0.5" onSubmit={onSubmitTask}>
        <input
          className="h-8 min-w-0 flex-1 rounded-[10px] border border-white/[0.07] bg-white/[0.055] px-2.5 text-[12px] font-medium text-slate-100 outline-none transition placeholder:text-slate-400 focus:border-sky-200/[0.2] focus:bg-white/[0.085]"
          data-no-window-drag
          placeholder="快速添加任务"
          value={title}
          onChange={(event) => onTitleChange(event.target.value)}
        />
        <button
          className="grid h-8 w-8 shrink-0 place-items-center rounded-[10px] border border-white/[0.1] bg-white/[0.065] text-[17px] font-semibold leading-none text-slate-50 transition hover:border-sky-200/[0.22] hover:bg-sky-300/[0.14]"
          data-no-window-drag
          title="添加任务"
          type="submit"
        >
          +
        </button>
      </form>

      <section className="flex min-h-0 flex-1 flex-col overflow-hidden rounded-[14px] border border-white/[0.065] bg-[#171d21]/60">
        <div className="flex h-6 items-center justify-between border-b border-white/[0.045] px-2">
          <p className="text-[11px] font-semibold text-slate-100">任务队列</p>
          <span className="rounded-md bg-white/[0.08] px-1.5 py-0.5 text-[10px] font-semibold text-slate-300">
            {tasks.length} / 5
          </span>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          {tasks.length === 0 ? (
            <div className="flex h-full items-center justify-center px-4 text-center text-[12px] font-medium text-slate-400">
              暂无今日任务
            </div>
          ) : (
            tasks.map((task) => (
              <CockpitTaskRow
                key={task.id}
                task={task}
                onComplete={() => onComplete(task)}
                onFocus={() => onFocus(task)}
                onDelete={() => onDelete(task)}
              />
            ))
          )}
        </div>
      </section>
    </>
  );
}

function PomodoroCockpitPage({
  duration,
  pomodoroTitle,
  progress,
  remaining,
  running,
  onPomodoroTitleChange,
  onComplete,
  onPause,
  onReset,
  onSetDurationMinutes,
  onStart,
}: {
  duration: number;
  pomodoroTitle: string;
  progress: number;
  remaining: number;
  running: boolean;
  onPomodoroTitleChange: (title: string) => void;
  onComplete: () => void;
  onPause: () => void;
  onReset: () => void;
  onSetDurationMinutes: (minutes: number) => void;
  onStart: () => void;
}) {
  return (
    <>
      <section className="flex min-h-0 flex-1 flex-col rounded-xl border border-white/8 bg-black/18 px-3 py-3 shadow-inner">
        <div className="flex items-start justify-between">
          <div>
            <p className="text-[11px] font-medium uppercase text-slate-400">FOCUS</p>
            <p className="mt-1 text-[14px] font-semibold text-slate-50">番茄时钟</p>
          </div>
          <span className="rounded-md bg-white/10 px-2 py-1 text-[11px] font-semibold tabular-nums text-sky-100">
            {Math.round(duration / 60)} min
          </span>
        </div>

        <div className="mt-4 flex items-center gap-4">
          <div
            className="grid h-[92px] w-[92px] shrink-0 place-items-center rounded-full shadow-[inset_0_0_0_8px_rgba(255,255,255,0.04)]"
            style={{
              background: `conic-gradient(#d7ecff ${progress * 3.6}deg, rgba(255,255,255,0.08) 0deg)`,
            }}
          >
            <div className="grid h-[72px] w-[72px] place-items-center rounded-full bg-[#11171a]/90 text-[18px] font-semibold tabular-nums text-slate-50 shadow-inner">
              {formatRemaining(remaining)}
            </div>
          </div>
          <div className="min-w-0 flex-1">
            <div className="h-1.5 overflow-hidden rounded-full bg-white/8">
              <div className="h-full rounded-full bg-sky-100 transition-all" style={{ width: `${progress}%` }} />
            </div>
            <p className="mt-2 text-[11px] font-medium text-slate-400">本轮进度 {progress}%</p>
            <input
              className="mt-2 h-7 w-full rounded-lg border border-white/10 bg-black/55 px-2 text-[11px] font-medium text-slate-100 outline-none transition placeholder:text-slate-400 focus:border-sky-200/30 focus:bg-black/70"
              data-no-window-drag
              placeholder="本轮完成了什么"
              value={pomodoroTitle}
              onChange={(event) => onPomodoroTitleChange(event.target.value)}
            />
            <div className="mt-3 grid grid-cols-3 gap-1">
              {[15, 25, 45].map((minutes) => (
                <button
                  key={minutes}
                  className={`rounded-md border px-2 py-1 text-[11px] font-semibold transition ${Math.round(duration / 60) === minutes ? 'border-sky-200/35 bg-sky-200/18 text-sky-50' : 'border-white/8 bg-white/7 text-slate-300 hover:bg-white/12'}`}
                  data-no-window-drag
                  onClick={() => onSetDurationMinutes(minutes)}
                  type="button"
                >
                  {minutes}
                </button>
              ))}
            </div>
          </div>
        </div>
      </section>

      <div className="mt-2 grid h-8 grid-cols-[1fr_58px_58px] gap-1.5">
        <button
          className="rounded-lg border border-white/10 bg-zinc-50 text-[13px] font-semibold text-zinc-950 transition hover:bg-white"
          data-no-window-drag
          onClick={running ? onPause : onStart}
          type="button"
        >
          {running ? '暂停' : '开始'}
        </button>
        <button
          className="rounded-lg border border-white/8 bg-white/8 text-[12px] font-semibold text-slate-200 transition hover:bg-white/12"
          data-no-window-drag
          onClick={onReset}
          type="button"
        >
          重置
        </button>
        <button
          className="rounded-lg border border-emerald-200/22 bg-emerald-300/12 text-[12px] font-semibold text-emerald-100 transition hover:bg-emerald-300/20"
          data-no-window-drag
          onClick={onComplete}
          type="button"
        >
          完成
        </button>
      </div>
    </>
  );
}

function CompletedRecordPage({
  doneToday,
  error,
  title,
  windowError,
  onTitleChange,
  onSubmit,
}: {
  doneToday: number;
  error: string | null;
  title: string;
  windowError: string | null;
  onTitleChange: (title: string) => void;
  onSubmit: (event: FormEvent) => void;
}) {
  return (
    <>
      <section className="flex min-h-0 flex-1 flex-col rounded-xl border border-white/8 bg-black/40 px-3 py-3 shadow-inner">
        <div className="flex items-start justify-between border-b border-white/8 pb-2">
          <div className="min-w-0">
            <p className="text-[11px] font-medium uppercase text-slate-400">
              LOG
            </p>
            <p className="mt-1 truncate text-[15px] font-semibold text-slate-50">快速完成记录</p>
          </div>
          <div className="rounded-lg border border-sky-100/14 bg-sky-100/10 px-2 py-1 text-right">
            <p className="text-[10px] font-medium uppercase text-sky-100/75">DONE</p>
            <p className="text-[16px] font-semibold leading-none tabular-nums text-sky-50">{doneToday}</p>
          </div>
        </div>

        {(error || windowError) && (
          <div className="mt-2 rounded-lg border border-amber-200/20 bg-amber-300/12 px-2 py-1 text-[10px] font-semibold text-amber-100">
            {error ?? windowError}
          </div>
        )}

        <form className="mt-3 flex flex-1 flex-col" onSubmit={onSubmit}>
          <textarea
            className="min-h-0 flex-1 resize-none rounded-xl border border-white/10 bg-black/58 px-3 py-2 text-[13px] font-medium leading-relaxed text-slate-100 outline-none transition placeholder:text-slate-400 focus:border-sky-200/30 focus:bg-black/72"
            data-no-window-drag
            placeholder="刚完成的临时任务"
            value={title}
            onChange={(event) => onTitleChange(event.target.value)}
          />
          <button
            className="mt-2 h-9 rounded-lg border border-sky-100/22 bg-sky-100/16 text-[13px] font-semibold text-sky-50 transition hover:bg-sky-100/24"
            data-no-window-drag
            type="submit"
          >
            登记完成
          </button>
        </form>
      </section>
    </>
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
    <div className={`group grid grid-cols-[auto_1fr_auto] items-center gap-2 border-b border-white/[0.04] px-2 py-1.5 last:border-b-0 ${isFocused ? 'bg-sky-200/[0.05]' : 'hover:bg-white/[0.025]'}`}>
      <button
        aria-label={isDone ? '标记未完成' : '完成任务'}
        className={`grid h-5 w-5 shrink-0 place-items-center rounded-md border text-[10px] font-semibold transition ${isDone ? 'border-sky-100/30 bg-sky-100/18 text-sky-50' : 'border-white/[0.1] bg-white/[0.025] text-transparent hover:border-sky-100/24 hover:text-sky-100'}`}
        data-no-window-drag
        onClick={onComplete}
        title={isDone ? '标记未完成' : '完成任务'}
      >
        ✓
      </button>
      <div className="min-w-0">
        <div className="flex min-w-0 items-center gap-1.5">
          <p className={`truncate text-[13px] font-semibold leading-tight ${isDone ? 'text-slate-500 line-through' : 'text-slate-50'}`}>
            {task.title}
          </p>
          <span className={`shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold ${priorityTone[task.priority]}`}>
            {priorityLabel(task.priority)}
          </span>
        </div>
        <p className="mt-0.5 truncate text-[10px] font-medium text-slate-400">
          {isFocused ? '专注中 · ' : ''}{formatTaskTime(task)}
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <button
          aria-label="设为专注任务"
          className={`grid h-6 w-6 place-items-center rounded-md border text-[11px] transition ${isFocused ? 'border-sky-100/28 bg-sky-100/14 text-sky-50' : 'border-white/[0.08] bg-white/[0.045] text-slate-300 hover:bg-white/[0.08]'}`}
          data-no-window-drag
          onClick={onFocus}
          title="设为专注任务"
        >
          ◷
        </button>
        <button
          aria-label="删除任务"
          className="grid h-6 w-6 place-items-center rounded-md border border-white/[0.08] bg-white/[0.045] text-[14px] leading-none text-slate-400 transition hover:border-rose-200/22 hover:bg-rose-300/10 hover:text-rose-100"
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

function IconButton({
  active = false,
  children,
  label,
  onClick,
  title,
}: {
  active?: boolean;
  children: string;
  label: string;
  onClick: () => void;
  title: string;
}) {
  return (
    <button
      aria-label={label}
      className={`grid h-5 w-5 place-items-center rounded-full border text-[12px] font-semibold leading-none transition ${active ? 'border-sky-100/24 bg-sky-100/12 text-sky-50' : 'border-white/[0.08] bg-white/[0.055] text-slate-300 hover:bg-white/[0.09] hover:text-white'}`}
      data-no-window-drag
      onMouseDown={(event) => event.stopPropagation()}
      onClick={onClick}
      title={title}
      type="button"
    >
      {children}
    </button>
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
