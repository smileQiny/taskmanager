import { useEffect, useState } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskForm } from '../components/tasks/TaskForm';
import { useTaskStore } from '../stores/taskStore';
import { Task } from '../types/task';

export function TasksPage() {
  const { tasks, fetchTasks } = useTaskStore();
  const [editing, setEditing] = useState<Task | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [filter, setFilter] = useState<'all' | 'todo' | 'in_progress' | 'done'>('all');

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const filtered = filter === 'all' ? tasks : tasks.filter((t) => t.status === filter);

  return (
    <div className="flex flex-col flex-1">
      <TopBar title="任务">
        <div className="join">
          {(['all', 'todo', 'in_progress', 'done'] as const).map((f) => (
            <button key={f}
              className={`btn btn-xs join-item ${filter === f ? 'btn-active' : ''}`}
              onClick={() => setFilter(f)}
            >
              {{ all: '全部', todo: '待办', in_progress: '进行中', done: '已完成' }[f]}
            </button>
          ))}
        </div>
        <button className="btn btn-primary btn-sm" onClick={() => setShowForm(true)}>
          + 新任务
        </button>
      </TopBar>
      <div className="flex-1 overflow-y-auto p-4">
        <TaskList tasks={filtered} onEdit={setEditing} />
      </div>
      {(showForm || editing) && (
        <TaskForm task={editing} onClose={() => { setShowForm(false); setEditing(null); }} />
      )}
    </div>
  );
}
