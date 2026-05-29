import { useEffect } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskDetailPanel } from '../components/tasks/TaskDetailPanel';
import { TaskFilters } from '../components/tasks/TaskFilters';
import { useTaskStore } from '../stores/taskStore';

export function TasksPage() {
  const { tasks, fetchTasks, selectedTaskId, draftDefaults, getVisibleTasks, startNewTask, error } = useTaskStore();

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const visibleTasks = getVisibleTasks();
  const selectedTask = tasks.find((task) => task.id === selectedTaskId) ?? null;

  return (
    <div className="flex min-h-0 flex-1">
      <section className="flex min-w-0 flex-1 flex-col">
        <TopBar title="任务" subtitle={`${visibleTasks.length} / ${tasks.length} 个任务`}>
          <button className="btn btn-primary btn-sm" onClick={() => startNewTask()}>
            新任务
          </button>
        </TopBar>
        <TaskFilters />
        {error && <div className="mx-5 mt-4 rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700">{error}</div>}
        <div className="flex-1 overflow-y-auto p-5">
          <TaskList tasks={visibleTasks} selectedTaskId={selectedTaskId} emptyText="没有匹配的任务" />
        </div>
      </section>
      <TaskDetailPanel task={selectedTask} defaults={draftDefaults} />
    </div>
  );
}
