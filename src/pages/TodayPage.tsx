import { useEffect, useState } from 'react';
import dayjs from 'dayjs';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskForm } from '../components/tasks/TaskForm';
import { useTaskStore } from '../stores/taskStore';
import { Task } from '../types/task';

export function TodayPage() {
  const { tasks, fetchTasks } = useTaskStore();
  const [editing, setEditing] = useState<Task | null>(null);
  const [showForm, setShowForm] = useState(false);

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const todayStart = dayjs().startOf('day').unix();
  const todayEnd = dayjs().endOf('day').unix();

  const todayTasks = tasks.filter((t) => {
    if (!t.start_time) return false;
    return t.start_time >= todayStart && t.start_time <= todayEnd;
  });

  const noDateTasks = tasks.filter((t) => !t.start_time && t.status !== 'done');

  return (
    <div className="flex flex-col flex-1">
      <TopBar title={`今日 · ${dayjs().format('M月D日 ddd')}`}>
        <button className="btn btn-primary btn-sm" onClick={() => setShowForm(true)}>
          + 新任务
        </button>
      </TopBar>
      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        <section>
          <h2 className="text-sm font-semibold text-base-content/70 mb-2">今日任务</h2>
          <TaskList tasks={todayTasks} onEdit={setEditing} />
        </section>
        {noDateTasks.length > 0 && (
          <section>
            <h2 className="text-sm font-semibold text-base-content/70 mb-2">未安排</h2>
            <TaskList tasks={noDateTasks} onEdit={setEditing} />
          </section>
        )}
      </div>
      {(showForm || editing) && (
        <TaskForm task={editing} onClose={() => { setShowForm(false); setEditing(null); }} />
      )}
    </div>
  );
}
