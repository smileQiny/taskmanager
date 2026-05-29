import { Task } from '../../types/task';
import { TaskCard } from './TaskCard';

interface TaskListProps {
  tasks: Task[];
  selectedTaskId?: string | null;
  emptyText?: string;
}

export function TaskList({ tasks, selectedTaskId, emptyText = '暂无任务' }: TaskListProps) {
  if (tasks.length === 0) {
    return <p className="py-8 text-center text-sm text-slate-400">{emptyText}</p>;
  }
  return (
    <div className="overflow-hidden rounded-md border border-slate-200 bg-white">
      {tasks.map((task) => (
        <TaskCard key={task.id} task={task} selected={selectedTaskId === task.id} />
      ))}
    </div>
  );
}
