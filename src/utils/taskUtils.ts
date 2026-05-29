import dayjs from 'dayjs';
import {
  Task,
  TaskFilters,
  TaskPriorityFilter,
  TaskSortKey,
  TaskStatusFilter,
} from '../types/task';

const priorityRank: Record<Task['priority'], number> = {
  high: 0,
  medium: 1,
  low: 2,
};

export function getTaskTags(tags?: string): string[] {
  if (!tags) return [];
  return tags
    .split(',')
    .map((tag) => tag.trim())
    .filter(Boolean);
}

export function filterTasks(tasks: Task[], filters: TaskFilters): Task[] {
  const query = filters.query.trim().toLowerCase();
  return tasks.filter((task) => {
    const matchesStatus = matchesStatusFilter(task, filters.status);
    const matchesPriority = matchesPriorityFilter(task, filters.priority);
    const searchable = [
      task.title,
      task.description ?? '',
      getTaskTags(task.tags).join(' '),
    ].join(' ').toLowerCase();
    const matchesQuery = !query || searchable.includes(query);

    return matchesStatus && matchesPriority && matchesQuery;
  });
}

export function sortTasks(tasks: Task[], sortKey: TaskSortKey): Task[] {
  return [...tasks].sort((a, b) => {
    if (sortKey === 'schedule') {
      return compareOptionalNumber(a.start_time, b.start_time) || b.created_at - a.created_at;
    }
    if (sortKey === 'created') {
      return b.created_at - a.created_at;
    }
    if (sortKey === 'priority') {
      return priorityRank[a.priority] - priorityRank[b.priority] || b.created_at - a.created_at;
    }
    return a.title.localeCompare(b.title, 'zh-CN');
  });
}

export function groupTasksForToday(tasks: Task[], dayStart = dayjs().startOf('day').unix(), dayEnd = dayjs().endOf('day').unix()) {
  const active = tasks.filter((task) => task.status !== 'done');
  return {
    overdue: sortTasks(
      active.filter((task) => Boolean(task.start_time && task.start_time < dayStart)),
      'schedule',
    ),
    today: sortTasks(
      active.filter((task) => Boolean(task.start_time && task.start_time >= dayStart && task.start_time <= dayEnd)),
      'schedule',
    ),
    unscheduled: sortTasks(
      active.filter((task) => !task.start_time),
      'created',
    ),
  };
}

export function formatTaskTime(task: Task): string {
  if (!task.start_time) return '未安排';
  const start = dayjs.unix(task.start_time);
  if (task.all_day) return start.format('M月D日 全天');
  if (!task.end_time) return start.format('M月D日 HH:mm');
  const end = dayjs.unix(task.end_time);
  return `${start.format('M月D日 HH:mm')} - ${end.format(end.isSame(start, 'day') ? 'HH:mm' : 'M月D日 HH:mm')}`;
}

function matchesStatusFilter(task: Task, status: TaskStatusFilter): boolean {
  return status === 'all' || task.status === status;
}

function matchesPriorityFilter(task: Task, priority: TaskPriorityFilter): boolean {
  return priority === 'all' || task.priority === priority;
}

function compareOptionalNumber(a?: number, b?: number): number {
  if (a === undefined && b === undefined) return 0;
  if (a === undefined) return 1;
  if (b === undefined) return -1;
  return a - b;
}
