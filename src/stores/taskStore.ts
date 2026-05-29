import { create } from 'zustand';
import {
  CreateTaskInput,
  Task,
  TaskFilters,
  TaskPriorityFilter,
  TaskSortKey,
  TaskStatusFilter,
  UpdateTaskInput,
} from '../types/task';
import { taskService } from '../services/taskService';
import { filterTasks, sortTasks } from '../utils/taskUtils';

interface TaskStore {
  tasks: Task[];
  loading: boolean;
  error: string | null;
  selectedTaskId: string | null;
  draftDefaults: Partial<CreateTaskInput> | null;
  filters: TaskFilters;
  sortKey: TaskSortKey;
  getVisibleTasks: () => Task[];
  fetchTasks: () => Promise<void>;
  createTask: (input: CreateTaskInput) => Promise<Task>;
  updateTask: (input: UpdateTaskInput) => Promise<Task | null>;
  deleteTask: (id: string) => Promise<void>;
  selectTask: (task: Task | null) => void;
  startNewTask: (defaults?: Partial<CreateTaskInput>) => void;
  closePanel: () => void;
  setQuery: (query: string) => void;
  setStatusFilter: (status: TaskStatusFilter) => void;
  setPriorityFilter: (priority: TaskPriorityFilter) => void;
  setSortKey: (sortKey: TaskSortKey) => void;
  clearError: () => void;
}

export const useTaskStore = create<TaskStore>((set, get) => ({
  tasks: [],
  loading: false,
  error: null,
  selectedTaskId: null,
  draftDefaults: null,
  filters: {
    query: '',
    status: 'all',
    priority: 'all',
  },
  sortKey: 'schedule',

  getVisibleTasks: () => sortTasks(filterTasks(get().tasks, get().filters), get().sortKey),

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
    try {
      const task = await taskService.create(input);
      set((s) => ({ tasks: [task, ...s.tasks], selectedTaskId: task.id, draftDefaults: null, error: null }));
      return task;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateTask: async (input) => {
    try {
      const updated = await taskService.update(input);
      if (!updated) return null;
      set((s) => ({
        tasks: s.tasks.map((t) => (t.id === updated.id ? updated : t)),
        error: null,
      }));
      return updated;
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteTask: async (id) => {
    try {
      await taskService.delete(id);
      set((s) => ({
        tasks: s.tasks.filter((t) => t.id !== id),
        selectedTaskId: s.selectedTaskId === id ? null : s.selectedTaskId,
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  selectTask: (task) => set({ selectedTaskId: task?.id ?? null, draftDefaults: null }),
  startNewTask: (defaults = {}) => set({ selectedTaskId: null, draftDefaults: defaults }),
  closePanel: () => set({ selectedTaskId: null, draftDefaults: null }),
  setQuery: (query) => set((s) => ({ filters: { ...s.filters, query } })),
  setStatusFilter: (status) => set((s) => ({ filters: { ...s.filters, status } })),
  setPriorityFilter: (priority) => set((s) => ({ filters: { ...s.filters, priority } })),
  setSortKey: (sortKey) => set({ sortKey }),
  clearError: () => set({ error: null }),
}));
