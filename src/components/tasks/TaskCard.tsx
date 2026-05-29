import { Task } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';
import { formatTaskTime, getTaskTags } from '../../utils/taskUtils';

const priorityColors = {
  low: 'bg-sky-400',
  medium: 'bg-amber-400',
  high: 'bg-rose-500',
};

interface TaskCardProps {
  task: Task;
  selected?: boolean;
}

export function TaskCard({ task, selected = false }: TaskCardProps) {
  const { updateTask, selectTask } = useTaskStore();

  const toggleStatus = () => {
    const next = task.status === 'done' ? 'todo' : 'done';
    updateTask({ id: task.id, status: next });
  };

  return (
    <div
      className={`group flex cursor-pointer items-center gap-3 border-b border-slate-100 px-4 py-3 transition hover:bg-slate-50 ${selected ? 'bg-teal-50/80' : 'bg-white'}`}
      onClick={() => selectTask(task)}
    >
      <span className={`h-10 w-1 rounded-full ${priorityColors[task.priority]}`} />
      <div onClick={(event) => event.stopPropagation()}>
        <input
          type="checkbox"
          className="checkbox checkbox-sm checkbox-primary"
          checked={task.status === 'done'}
          onChange={toggleStatus}
        />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className={`truncate font-medium text-slate-950 ${task.status === 'done' ? 'line-through opacity-45' : ''}`}>
            {task.title}
          </p>
          {task.status === 'in_progress' && <span className="badge badge-primary badge-sm">进行中</span>}
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-slate-500">
          <span>{formatTaskTime(task)}</span>
          {getTaskTags(task.tags).map((tag) => (
            <span key={tag} className="rounded bg-slate-100 px-1.5 py-0.5 text-slate-500">{tag}</span>
          ))}
        </div>
      </div>
      <button className="btn btn-ghost btn-xs opacity-0 transition group-hover:opacity-100" onClick={(event) => { event.stopPropagation(); selectTask(task); }}>
        编辑
      </button>
    </div>
  );
}
