import { CreateTaskInput } from '../types/task';

export function buildCompletedTaskRecord(
  title: string,
  registeredAt: number,
  source = 'cockpit',
): CreateTaskInput {
  return {
    title: title.trim(),
    status: 'done',
    priority: 'medium',
    start_time: registeredAt,
    end_time: registeredAt,
    all_day: false,
    tags: `${source},completed`,
  };
}

export function buildPomodoroTaskRecord(
  title: string,
  completedAt: number,
  durationSeconds: number,
  source = 'cockpit',
): CreateTaskInput {
  const duration = Math.max(0, Math.round(durationSeconds));
  const minutes = Math.round(duration / 60);
  return {
    title: title.trim(),
    status: 'done',
    priority: 'medium',
    start_time: completedAt - duration,
    end_time: completedAt,
    all_day: false,
    tags: source === 'pomodoro' ? 'pomodoro' : `${source},pomodoro`,
    description: `番茄钟 ${minutes} 分钟`,
  };
}
