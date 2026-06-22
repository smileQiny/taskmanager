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
    <div className="overflow-hidden rounded-xl border border-[#dbe5f1] bg-white/88 shadow-sm">
      {tasks.map((task) => (
        <TaskCard key={task.id} task={task} selected={selectedTaskId === task.id} />
      ))}
    </div>
  );
}
