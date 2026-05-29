import { beforeEach, describe, expect, it, vi } from 'vitest';
import { taskService } from './taskService';

describe('browser task service fallback', () => {
  beforeEach(() => {
    vi.stubGlobal('localStorage', createMemoryStorage());
  });

  it('rejects tasks whose end time is before the start time', async () => {
    await expect(taskService.create({
      title: 'Bad time range',
      start_time: 200,
      end_time: 100,
    })).rejects.toThrow('end_time');
  });

  it('rejects updates that would clear the task title', async () => {
    const task = await taskService.create({ title: 'Existing task' });

    await expect(taskService.update({
      id: task.id,
      title: '   ',
    })).rejects.toThrow('title');
  });
});

function createMemoryStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (key) => data.get(key) ?? null,
    key: (index) => Array.from(data.keys())[index] ?? null,
    removeItem: (key) => data.delete(key),
    setItem: (key, value) => data.set(key, value),
  };
}
