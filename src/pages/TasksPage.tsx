import { FormEvent, useEffect, useState } from 'react';
import dayjs from 'dayjs';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskDetailPanel } from '../components/tasks/TaskDetailPanel';
import { TaskFilters } from '../components/tasks/TaskFilters';
import { useTaskStore } from '../stores/taskStore';
import { buildCompletedTaskRecord } from '../utils/cockpitTaskRecords';

export function TasksPage() {
  const {
    tasks,
    fetchTasks,
    selectedTaskId,
    draftDefaults,
    getVisibleTasks,
    startNewTask,
    createTask,
    error,
  } = useTaskStore();
  const [completedTitle, setCompletedTitle] = useState('');
  const [isRecordDialogOpen, setIsRecordDialogOpen] = useState(false);

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const visibleTasks = getVisibleTasks();
  const selectedTask = tasks.find((task) => task.id === selectedTaskId) ?? null;

  const closeRecordDialog = () => {
    setIsRecordDialogOpen(false);
    setCompletedTitle('');
  };

  const submitCompletedTask = async (event: FormEvent) => {
    event.preventDefault();
    const title = completedTitle.trim();
    if (!title) return;
    await createTask(buildCompletedTaskRecord(title, dayjs().unix(), 'task'));
    closeRecordDialog();
  };

  return (
    <div className="flex min-h-0 flex-1">
      <section className="flex min-w-0 flex-1 flex-col">
        <TopBar title="任务" subtitle={`${visibleTasks.length} / ${tasks.length} 个任务`}>
          <button className="btn btn-outline btn-sm border-[#cbd8e8] text-slate-600 hover:border-[#8fb7e8] hover:bg-[#eef5ff] hover:text-[#1f5f9f]" onClick={() => setIsRecordDialogOpen(true)}>
            临时任务完成记录
          </button>
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
      {isRecordDialogOpen && (
        <CompletedTaskDialog
          title={completedTitle}
          onChangeTitle={setCompletedTitle}
          onClose={closeRecordDialog}
          onSubmit={submitCompletedTask}
        />
      )}
    </div>
  );
}

function CompletedTaskDialog({
  title,
  onChangeTitle,
  onClose,
  onSubmit,
}: {
  title: string;
  onChangeTitle: (title: string) => void;
  onClose: () => void;
  onSubmit: (event: FormEvent) => void;
}) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/24 px-5 backdrop-blur-sm">
      <form className="w-full max-w-md rounded-2xl border border-[#dbe5f1] bg-white p-5 shadow-[0_24px_70px_rgba(30,41,59,0.18)]" onSubmit={onSubmit}>
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-base font-semibold text-slate-900">临时任务完成记录</h2>
            <p className="mt-1 text-sm text-slate-500">登记时间会作为开始和结束时间。</p>
          </div>
          <button className="btn btn-ghost btn-sm h-8 min-h-8 w-8 rounded-full p-0 text-slate-400 hover:bg-slate-100 hover:text-slate-700" onClick={onClose} type="button">
            ×
          </button>
        </div>
        <textarea
          autoFocus
          className="textarea textarea-bordered mt-5 min-h-28 w-full resize-none border-[#cbd8e8] bg-[#f8fbff] text-sm text-slate-800 outline-none focus:border-[#8fb7e8]"
          placeholder="刚完成的临时任务"
          value={title}
          onChange={(event) => onChangeTitle(event.target.value)}
        />
        <div className="mt-4 flex justify-end gap-2">
          <button className="btn btn-ghost btn-sm" onClick={onClose} type="button">
            取消
          </button>
          <button className="btn btn-primary btn-sm" disabled={!title.trim()} type="submit">
            记录完成
          </button>
        </div>
      </form>
    </div>
  );
}
