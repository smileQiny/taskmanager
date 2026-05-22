import { useState, useEffect } from 'react';
import { Task } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';

interface TaskFormProps {
  task?: Task | null;
  onClose: () => void;
}

export function TaskForm({ task, onClose }: TaskFormProps) {
  const { createTask, updateTask } = useTaskStore();
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState<'low' | 'medium' | 'high'>('medium');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [allDay, setAllDay] = useState(false);

  useEffect(() => {
    if (task) {
      setTitle(task.title);
      setDescription(task.description || '');
      setPriority(task.priority);
      setAllDay(task.all_day);
      if (task.start_time) {
        setStartTime(new Date(task.start_time * 1000).toISOString().slice(0, 16));
      }
      if (task.end_time) {
        setEndTime(new Date(task.end_time * 1000).toISOString().slice(0, 16));
      }
    }
  }, [task]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const start = startTime ? Math.floor(new Date(startTime).getTime() / 1000) : undefined;
    const end = endTime ? Math.floor(new Date(endTime).getTime() / 1000) : undefined;

    if (task) {
      await updateTask({
        id: task.id, title, description: description || undefined,
        priority, start_time: start, end_time: end, all_day: allDay,
      });
    } else {
      await createTask({
        title, description: description || undefined,
        priority, start_time: start, end_time: end, all_day: allDay,
      });
    }
    onClose();
  };

  return (
    <dialog className="modal modal-open">
      <div className="modal-box">
        <h3 className="font-bold text-lg">{task ? '编辑任务' : '新建任务'}</h3>
        <form onSubmit={handleSubmit} className="flex flex-col gap-3 mt-4">
          <input
            type="text" placeholder="任务标题" value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="input input-bordered w-full" required
          />
          <textarea
            placeholder="描述（可选）" value={description}
            onChange={(e) => setDescription(e.target.value)}
            className="textarea textarea-bordered w-full"
          />
          <select
            value={priority} onChange={(e) => setPriority(e.target.value as 'low' | 'medium' | 'high')}
            className="select select-bordered w-full"
          >
            <option value="low">低优先级</option>
            <option value="medium">中优先级</option>
            <option value="high">高优先级</option>
          </select>
          <label className="label cursor-pointer justify-start gap-2">
            <input type="checkbox" className="checkbox checkbox-sm" checked={allDay}
              onChange={(e) => setAllDay(e.target.checked)} />
            <span className="label-text">全天</span>
          </label>
          {!allDay && (
            <>
              <input type="datetime-local" value={startTime}
                onChange={(e) => setStartTime(e.target.value)}
                className="input input-bordered w-full" />
              <input type="datetime-local" value={endTime}
                onChange={(e) => setEndTime(e.target.value)}
                className="input input-bordered w-full" />
            </>
          )}
          <div className="modal-action">
            <button type="button" className="btn" onClick={onClose}>取消</button>
            <button type="submit" className="btn btn-primary">
              {task ? '保存' : '创建'}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>
  );
}
