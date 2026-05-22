import { create } from 'zustand';
import { Task, CreateTaskInput, UpdateTaskInput } from '../types/task';
import { taskService } from '../services/taskService';

interface TaskStore {
  tasks: Task[];
  loading: boolean;
  error: string | null;
  fetchTasks: () => Promise<void>;
  createTask: (input: CreateTaskInput) => Promise<Task>;
  updateTask: (input: UpdateTaskInput) => Promise<void>;
  deleteTask: (id: string) => Promise<void>;
}

export const useTaskStore = create<TaskStore>((set) => ({
  tasks: [],
  loading: false,
  error: null,

  fetchTasks: async () => {
    set({ loading: true, error: null });
    try {
      const tasks = await taskService.getAll();
      set({ tasks, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createTask: async (input) => {
    const task = await taskService.create(input);
    set((s) => ({ tasks: [task, ...s.tasks] }));
    return task;
  },

  updateTask: async (input) => {
    const updated = await taskService.update(input);
    if (!updated) return;
    set((s) => ({
      tasks: s.tasks.map((t) => (t.id === updated.id ? updated : t)),
    }));
  },

  deleteTask: async (id) => {
    await taskService.delete(id);
    set((s) => ({ tasks: s.tasks.filter((t) => t.id !== id) }));
  },
}));
