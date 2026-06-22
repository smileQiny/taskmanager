import dayjs from 'dayjs';
import { useEffect } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskDetailPanel } from '../components/tasks/TaskDetailPanel';
import { useTaskStore } from '../stores/taskStore';
import { groupTasksForToday } from '../utils/taskUtils';

export function TodayPage() {
  const { tasks, fetchTasks, selectedTaskId, draftDefaults, startNewTask, error } = useTaskStore();

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const grouped = groupTasksForToday(tasks);
  const selectedTask = tasks.find((task) => task.id === selectedTaskId) ?? null;
  const activeCount = grouped.overdue.length + grouped.today.length + grouped.unscheduled.length;

  return (
    <div className="flex min-h-0 flex-1">
      <section className="flex min-w-0 flex-1 flex-col">
        <TopBar title={`今日 · ${dayjs().format('M月D日')}`} subtitle={`${activeCount} 个本地任务待处理`}>
          <button className="btn btn-primary btn-sm" onClick={() => startNewTask({ start_time: dayjs().hour(9).minute(0).second(0).unix() })}>
            新任务
          </button>
        </TopBar>
        {error && <div className="mx-5 mt-4 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>}
        <div className="flex-1 overflow-y-auto p-5">
          <div className="mb-6 grid grid-cols-3 gap-3">
            <Summary label="逾期" value={grouped.overdue.length} tone="text-rose-600" />
            <Summary label="今日" value={grouped.today.length} tone="text-blue-700" />
            <Summary label="未安排" value={grouped.unscheduled.length} tone="text-slate-600" />
          </div>
          <TaskSection title="逾期" tasks={grouped.overdue} selectedTaskId={selectedTaskId} emptyText="没有逾期任务" />
          <TaskSection title="今日任务" tasks={grouped.today} selectedTaskId={selectedTaskId} emptyText="今天还没有安排任务" />
          <TaskSection title="未安排" tasks={grouped.unscheduled} selectedTaskId={selectedTaskId} emptyText="没有未安排任务" />
        </div>
      </section>
      <TaskDetailPanel task={selectedTask} defaults={draftDefaults} />
    </div>
  );
}

function Summary({ label, value, tone }: { label: string; value: number; tone: string }) {
  return (
    <div className="rounded-xl border border-[#dbe5f1] bg-white/84 px-4 py-3 shadow-sm">
      <p className="text-xs text-slate-500">{label}</p>
      <p className={`mt-1 text-2xl font-semibold ${tone}`}>{value}</p>
    </div>
  );
}

function TaskSection({ title, tasks, selectedTaskId, emptyText }: { title: string; tasks: ReturnType<typeof groupTasksForToday>['today']; selectedTaskId: string | null; emptyText: string }) {
  return (
    <section className="mb-6">
      <div className="mb-2 flex items-center justify-between">
        <h2 className="text-sm font-semibold text-slate-700">{title}</h2>
        <span className="text-xs text-slate-400">{tasks.length}</span>
      </div>
      <TaskList tasks={tasks} selectedTaskId={selectedTaskId} emptyText={emptyText} />
    </section>
  );
}
