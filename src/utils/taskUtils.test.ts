import { describe, expect, it } from 'vitest';
import { Task } from '../types/task';
import {
  filterTasks,
  getTaskTags,
  groupTasksForToday,
  sortTasks,
} from './taskUtils';

const task = (overrides: Partial<Task>): Task => ({
  id: overrides.id ?? 'task',
  title: overrides.title ?? 'Task',
  description: overrides.description,
  status: overrides.status ?? 'todo',
  priority: overrides.priority ?? 'medium',
  start_time: overrides.start_time,
  end_time: overrides.end_time,
  all_day: overrides.all_day ?? false,
  recurrence: overrides.recurrence,
  tags: overrides.tags,
  created_at: overrides.created_at ?? 100,
  updated_at: overrides.updated_at ?? 100,
});

describe('task utilities', () => {
  it('parses comma separated tags and trims empty values', () => {
    expect(getTaskTags(' work,  urgent ,, home ')).toEqual(['work', 'urgent', 'home']);
  });

  it('filters by query, status, and priority', () => {
    const tasks = [
      task({ id: '1', title: 'Write report', status: 'todo', priority: 'high', tags: 'work' }),
      task({ id: '2', title: 'Buy milk', status: 'done', priority: 'low', tags: 'home' }),
      task({ id: '3', title: 'Plan sprint', status: 'in_progress', priority: 'high', description: 'team' }),
    ];

    expect(filterTasks(tasks, { query: 'work', status: 'all', priority: 'all' }).map((t) => t.id)).toEqual(['1']);
    expect(filterTasks(tasks, { query: '', status: 'done', priority: 'all' }).map((t) => t.id)).toEqual(['2']);
    expect(filterTasks(tasks, { query: '', status: 'all', priority: 'high' }).map((t) => t.id)).toEqual(['1', '3']);
  });

  it('sorts scheduled tasks before unscheduled tasks by time', () => {
    const tasks = [
      task({ id: 'late', start_time: 300 }),
      task({ id: 'none' }),
      task({ id: 'early', start_time: 100 }),
    ];

    expect(sortTasks(tasks, 'schedule').map((t) => t.id)).toEqual(['early', 'late', 'none']);
  });

  it('groups active tasks into overdue, today, and unscheduled buckets', () => {
    const dayStart = 1_000;
    const dayEnd = 1_999;
    const tasks = [
      task({ id: 'overdue', start_time: 900, status: 'todo' }),
      task({ id: 'today', start_time: 1_200, status: 'in_progress' }),
      task({ id: 'unscheduled', status: 'todo' }),
      task({ id: 'done', start_time: 1_300, status: 'done' }),
      task({ id: 'future', start_time: 2_100, status: 'todo' }),
    ];

    const grouped = groupTasksForToday(tasks, dayStart, dayEnd);

    expect(grouped.overdue.map((t) => t.id)).toEqual(['overdue']);
    expect(grouped.today.map((t) => t.id)).toEqual(['today']);
    expect(grouped.unscheduled.map((t) => t.id)).toEqual(['unscheduled']);
  });
});
