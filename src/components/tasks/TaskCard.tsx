import { Task } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';

const priorityColors = {
  low: 'border-l-info',
  medium: 'border-l-warning',
  high: 'border-l-error',
};

interface TaskCardProps {
  task: Task;
  onEdit: (task: Task) => void;
}

export function TaskCard({ task, onEdit }: TaskCardProps) {
  const { deleteTask, updateTask } = useTaskStore();

  const toggleStatus = () => {
    const next = task.status === 'done' ? 'todo' : 'done';
    updateTask({ id: task.id, status: next });
  };

  return (
    <div className={`card bg-base-100 shadow-sm border-l-4 ${priorityColors[task.priority]}`}>
      <div className="card-body p-3 flex-row items-center gap-3">
        <input
          type="checkbox"
          className="checkbox checkbox-sm"
          checked={task.status === 'done'}
          onChange={toggleStatus}
        />
        <div className="flex-1 min-w-0">
          <p className={`font-medium truncate ${task.status === 'done' ? 'line-through opacity-50' : ''}`}>
            {task.title}
          </p>
          {task.start_time && (
            <p className="text-xs text-base-content/60">
              {new Date(task.start_time * 1000).toLocaleString('zh-CN', {
                month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
              })}
            </p>
          )}
        </div>
        <div className="flex gap-1">
          <button className="btn btn-ghost btn-xs" onClick={() => onEdit(task)}>编辑</button>
          <button className="btn btn-ghost btn-xs text-error" onClick={() => deleteTask(task.id)}>删除</button>
        </div>
      </div>
    </div>
  );
}
