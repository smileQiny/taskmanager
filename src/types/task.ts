export interface Task {
  id: string;
  title: string;
  description?: string;
  status: 'todo' | 'in_progress' | 'done';
  priority: 'low' | 'medium' | 'high';
  start_time?: number;
  end_time?: number;
  all_day: boolean;
  recurrence?: string;
  tags?: string;
  created_at: number;
  updated_at: number;
}

export interface CreateTaskInput {
  title: string;
  description?: string;
  status?: Task['status'];
  priority?: Task['priority'];
  start_time?: number;
  end_time?: number;
  all_day?: boolean;
  recurrence?: string;
  tags?: string;
}

export interface UpdateTaskInput {
  id: string;
  title?: string;
  description?: string;
  status?: Task['status'];
  priority?: Task['priority'];
  start_time?: number;
  end_time?: number;
  all_day?: boolean;
  recurrence?: string;
  tags?: string;
}
