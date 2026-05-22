import { invoke } from '@tauri-apps/api/core';
import { Task, CreateTaskInput, UpdateTaskInput } from '../types/task';

export const taskService = {
  getAll: () => invoke<Task[]>('get_tasks'),
  getById: (id: string) => invoke<Task | null>('get_task', { id }),
  create: (input: CreateTaskInput) => invoke<Task>('create_task', { input }),
  update: (input: UpdateTaskInput) => invoke<Task | null>('update_task', { input }),
  delete: (id: string) => invoke<boolean>('delete_task', { id }),
  getByRange: (from: number, to: number) =>
    invoke<Task[]>('get_tasks_by_range', { from, to }),
};
