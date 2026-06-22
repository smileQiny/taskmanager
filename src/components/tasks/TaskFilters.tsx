import { TaskPriorityFilter, TaskSortKey, TaskStatusFilter } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';

const statusOptions: Array<{ value: TaskStatusFilter; label: string }> = [
  { value: 'all', label: '全部' },
  { value: 'todo', label: '待办' },
  { value: 'in_progress', label: '进行中' },
  { value: 'done', label: '已完成' },
];

const priorityOptions: Array<{ value: TaskPriorityFilter; label: string }> = [
  { value: 'all', label: '全部优先级' },
  { value: 'high', label: '高优先级' },
  { value: 'medium', label: '中优先级' },
  { value: 'low', label: '低优先级' },
];

const sortOptions: Array<{ value: TaskSortKey; label: string }> = [
  { value: 'schedule', label: '按时间' },
  { value: 'created', label: '按创建' },
  { value: 'priority', label: '按优先级' },
  { value: 'title', label: '按标题' },
];

export function TaskFilters() {
  const {
    filters,
    sortKey,
    setQuery,
    setStatusFilter,
    setPriorityFilter,
    setSortKey,
  } = useTaskStore();

  return (
    <div className="grid gap-3 border-b border-[#dbe5f1] bg-white/62 px-4 py-3 backdrop-blur lg:grid-cols-[1fr_150px_150px_130px]">
      <input
        className="input input-bordered input-sm border-slate-200 bg-white/88"
        value={filters.query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="搜索标题、描述或标签"
      />
      <select className="select select-bordered select-sm border-slate-200 bg-white/88" value={filters.status} onChange={(event) => setStatusFilter(event.target.value as TaskStatusFilter)}>
        {statusOptions.map((option) => (
          <option key={option.value} value={option.value}>{option.label}</option>
        ))}
      </select>
      <select className="select select-bordered select-sm border-slate-200 bg-white/88" value={filters.priority} onChange={(event) => setPriorityFilter(event.target.value as TaskPriorityFilter)}>
        {priorityOptions.map((option) => (
          <option key={option.value} value={option.value}>{option.label}</option>
        ))}
      </select>
      <select className="select select-bordered select-sm border-slate-200 bg-white/88" value={sortKey} onChange={(event) => setSortKey(event.target.value as TaskSortKey)}>
        {sortOptions.map((option) => (
          <option key={option.value} value={option.value}>{option.label}</option>
        ))}
      </select>
    </div>
  );
}
