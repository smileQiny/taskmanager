import { describe, expect, it } from 'vitest';
import { buildCompletedTaskRecord, buildPomodoroTaskRecord } from './cockpitTaskRecords';

describe('cockpit task records', () => {
  it('records quick completed tasks at the registration time', () => {
    const task = buildCompletedTaskRecord('临时处理客户消息', 1_800_000_000);

    expect(task).toMatchObject({
      title: '临时处理客户消息',
      status: 'done',
      priority: 'medium',
      start_time: 1_800_000_000,
      end_time: 1_800_000_000,
      all_day: false,
      tags: 'cockpit,completed',
    });
  });

  it('records pomodoro tasks with timer duration when a title is provided', () => {
    const task = buildPomodoroTaskRecord('整理发布说明', 1_800_000_000, 25 * 60);

    expect(task).toMatchObject({
      title: '整理发布说明',
      status: 'done',
      priority: 'medium',
      start_time: 1_799_998_500,
      end_time: 1_800_000_000,
      all_day: false,
      tags: 'cockpit,pomodoro',
      description: '番茄钟 25 分钟',
    });
  });

  it('can label completed tasks from the main tasks page', () => {
    const task = buildCompletedTaskRecord('临时线上支持', 1_800_000_000, 'task');

    expect(task.tags).toBe('task,completed');
  });

  it('can label pomodoro task records outside the cockpit', () => {
    const task = buildPomodoroTaskRecord('复盘会议纪要', 1_800_000_000, 15 * 60, 'pomodoro');

    expect(task.tags).toBe('pomodoro');
  });
});
