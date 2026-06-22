import { beforeEach, describe, expect, it, vi } from 'vitest';
import { settingsService, taskService } from './taskService';

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

  it('persists cockpit opacity in browser fallback settings', async () => {
    expect(await settingsService.get()).toMatchObject({
      cockpit_opacity: 92,
    });

    const settings = await settingsService.update({ cockpit_opacity: 76 });

    expect(settings.cockpit_opacity).toBe(76);
    expect((await settingsService.get()).cockpit_opacity).toBe(76);
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
