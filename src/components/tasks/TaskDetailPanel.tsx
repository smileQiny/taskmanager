import { FormEvent, useEffect, useMemo, useState } from 'react';
import dayjs from 'dayjs';
import { CreateTaskInput, SyncProvider, Task, TaskSyncState } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';
import { syncService } from '../../services/taskService';

interface TaskDetailPanelProps {
  task?: Task | null;
  defaults?: Partial<CreateTaskInput> | null;
}

const statusLabels = {
  todo: '待办',
  in_progress: '进行中',
  done: '已完成',
};

const priorityLabels = {
  low: '低',
  medium: '中',
  high: '高',
};

const syncProviders: Array<{ provider: SyncProvider; label: string }> = [
  { provider: 'feishu', label: '飞书' },
  { provider: 'macos', label: 'macOS' },
  { provider: 'wecom', label: '企业微信' },
  { provider: 'google', label: 'Google' },
];

export function TaskDetailPanel({ task, defaults }: TaskDetailPanelProps) {
  const { createTask, updateTask, deleteTask, closePanel } = useTaskStore();
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [status, setStatus] = useState<Task['status']>('todo');
  const [priority, setPriority] = useState<Task['priority']>('medium');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [allDay, setAllDay] = useState(false);
  const [tags, setTags] = useState('');
  const [syncStates, setSyncStates] = useState<TaskSyncState[]>([]);
  const [syncError, setSyncError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const isOpen = Boolean(task || defaults);
  const panelTitle = task ? '任务详情' : '新建任务';

  useEffect(() => {
    setConfirmDelete(false);
    if (task) {
      setTitle(task.title);
      setDescription(task.description ?? '');
      setStatus(task.status);
      setPriority(task.priority);
      setAllDay(task.all_day);
      setStartTime(toLocalInput(task.start_time));
      setEndTime(toLocalInput(task.end_time));
      setTags(task.tags ?? '');
      return;
    }

    setTitle(defaults?.title ?? '');
    setDescription(defaults?.description ?? '');
    setStatus(defaults?.status ?? 'todo');
    setPriority(defaults?.priority ?? 'medium');
    setAllDay(defaults?.all_day ?? false);
    setStartTime(toLocalInput(defaults?.start_time));
    setEndTime(toLocalInput(defaults?.end_time));
    setTags(defaults?.tags ?? '');
  }, [task, defaults]);

  useEffect(() => {
    if (!task) {
      setSyncStates([]);
      return;
    }
    syncService
      .listTaskStates(task.id)
      .then((states) => {
        setSyncStates(states);
        setSyncError(null);
      })
      .catch((error) => setSyncError(String(error)));
  }, [task]);

  const canSubmit = useMemo(() => title.trim().length > 0, [title]);

  if (!isOpen) {
    return (
      <aside className="hidden md:flex w-80 shrink-0 border-l border-slate-200 bg-slate-50/70 px-5 py-6">
        <div className="self-center text-sm text-slate-400">
          选择任务或新建任务后，详情会显示在这里。
        </div>
      </aside>
    );
  }

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    const start = fromLocalInput(startTime);
    const end = fromLocalInput(endTime);

    if (task) {
      await updateTask({
        id: task.id,
        title,
        description: description.trim() || null,
        status,
        priority,
        start_time: start,
        end_time: end,
        all_day: allDay,
        tags: tags.trim() || null,
      });
    } else {
      await createTask({
        title,
        description: description.trim() || undefined,
        status,
        priority,
        start_time: start ?? undefined,
        end_time: end ?? undefined,
        all_day: allDay,
        tags: tags.trim() || undefined,
      });
    }
  };

  return (
    <aside className="w-full md:w-96 shrink-0 border-l border-slate-200 bg-white">
      <form onSubmit={handleSubmit} className="flex h-full flex-col">
        <div className="flex items-center justify-between border-b border-slate-200 px-5 py-4">
          <div>
            <p className="text-xs font-medium uppercase tracking-wide text-slate-400">{panelTitle}</p>
            <h2 className="text-lg font-semibold text-slate-950">{task ? task.title : '安排一件事'}</h2>
          </div>
          <button type="button" className="btn btn-ghost btn-sm" onClick={closePanel}>关闭</button>
        </div>

        <div className="flex-1 space-y-4 overflow-y-auto px-5 py-5">
          <label className="form-control">
            <span className="label-text mb-1 text-slate-500">标题</span>
            <input
              className="input input-bordered bg-white text-base font-medium"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="输入任务标题"
              required
            />
          </label>

          <label className="form-control">
            <span className="label-text mb-1 text-slate-500">描述</span>
            <textarea
              className="textarea textarea-bordered min-h-28 bg-white"
              value={description}
              onChange={(event) => setDescription(event.target.value)}
              placeholder="补充背景、链接或验收标准"
            />
          </label>

          <div className="grid grid-cols-2 gap-3">
            <label className="form-control">
              <span className="label-text mb-1 text-slate-500">状态</span>
              <select className="select select-bordered bg-white" value={status} onChange={(event) => setStatus(event.target.value as Task['status'])}>
                {Object.entries(statusLabels).map(([value, label]) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </label>
            <label className="form-control">
              <span className="label-text mb-1 text-slate-500">优先级</span>
              <select className="select select-bordered bg-white" value={priority} onChange={(event) => setPriority(event.target.value as Task['priority'])}>
                {Object.entries(priorityLabels).map(([value, label]) => (
                  <option key={value} value={value}>{label}</option>
                ))}
              </select>
            </label>
          </div>

          <label className="flex items-center gap-3 rounded-md border border-slate-200 px-3 py-2">
            <input type="checkbox" className="toggle toggle-sm toggle-primary" checked={allDay} onChange={(event) => setAllDay(event.target.checked)} />
            <span className="text-sm text-slate-600">全天任务</span>
          </label>

          <div className="grid grid-cols-1 gap-3">
            <label className="form-control">
              <span className="label-text mb-1 text-slate-500">开始时间</span>
              <input className="input input-bordered bg-white" type="datetime-local" value={startTime} onChange={(event) => setStartTime(event.target.value)} />
            </label>
            <label className="form-control">
              <span className="label-text mb-1 text-slate-500">结束时间</span>
              <input className="input input-bordered bg-white" type="datetime-local" value={endTime} onChange={(event) => setEndTime(event.target.value)} />
            </label>
          </div>

          <label className="form-control">
            <span className="label-text mb-1 text-slate-500">标签</span>
            <input
              className="input input-bordered bg-white"
              value={tags}
              onChange={(event) => setTags(event.target.value)}
              placeholder="work, home"
            />
          </label>

          {task && (
            <section className="rounded-md border border-slate-200 p-3">
              <div className="mb-2 flex items-center justify-between">
                <span className="text-sm font-medium text-slate-700">远端同步</span>
                {syncError && <span className="text-xs text-rose-600">{syncError}</span>}
              </div>
              <div className="grid grid-cols-2 gap-2">
                {syncProviders.map(({ provider, label }) => {
                  const state = syncStates.find((item) => item.provider === provider);
                  return (
                    <label key={provider} className="flex items-center justify-between rounded bg-slate-50 px-2 py-2 text-sm">
                      <span>
                        <span className="block text-slate-700">{label}</span>
                        <span className="text-xs text-slate-400">{state ? syncStatusLabel(state.sync_status) : '未选择'}</span>
                      </span>
                      <input
                        type="checkbox"
                        className="toggle toggle-xs toggle-primary"
                        checked={Boolean(state)}
                        onChange={(event) => void setTaskSync(task.id, provider, event.target.checked, setSyncStates, setSyncError)}
                      />
                    </label>
                  );
                })}
              </div>
            </section>
          )}
        </div>

        <div className="border-t border-slate-200 px-5 py-4">
          {task && (
            <div className="mb-3">
              {confirmDelete ? (
                <div className="flex items-center gap-2">
                  <button type="button" className="btn btn-error btn-sm" onClick={() => void deleteTask(task.id)}>
                    确认删除
                  </button>
                  <button type="button" className="btn btn-ghost btn-sm" onClick={() => setConfirmDelete(false)}>
                    取消
                  </button>
                </div>
              ) : (
                <button type="button" className="btn btn-ghost btn-sm text-error" onClick={() => setConfirmDelete(true)}>
                  删除任务
                </button>
              )}
            </div>
          )}
          <button type="submit" className="btn btn-primary w-full" disabled={!canSubmit}>
            {task ? '保存更改' : '创建任务'}
          </button>
        </div>
      </form>
    </aside>
  );
}

async function setTaskSync(
  taskId: string,
  provider: SyncProvider,
  enabled: boolean,
  setSyncStates: (states: TaskSyncState[]) => void,
  setSyncError: (error: string | null) => void,
) {
  try {
    const states = await syncService.setTaskSync({ task_id: taskId, provider, enabled });
    setSyncStates(states);
    setSyncError(null);
  } catch (error) {
    setSyncError(String(error));
  }
}

function syncStatusLabel(status: string): string {
  return {
    pending: '待同步',
    needs_auth: '需授权',
    synced: '已同步',
    conflict: '冲突',
    error: '错误',
  }[status] ?? status;
}

function toLocalInput(timestamp?: number | null): string {
  if (!timestamp) return '';
  return dayjs.unix(timestamp).format('YYYY-MM-DDTHH:mm');
}

function fromLocalInput(value: string): number | null {
  if (!value) return null;
  return Math.floor(new Date(value).getTime() / 1000);
}
