# TaskManager Implementation Plan (Phase 1 & 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working Tauri 2 desktop task manager with SQLite-backed CRUD, today view, task list, and month/week/day calendar views.

**Architecture:** Single Tauri 2 process — React 19 frontend communicates with Rust backend via IPC commands. Rust handles all SQLite operations via rusqlite. Frontend state managed by Zustand stores.

**Tech Stack:** Tauri 2, React 19, TypeScript, Vite, Tailwind CSS, DaisyUI, Zustand, dayjs, react-router-dom, Rust, rusqlite, serde, uuid, chrono, anyhow

---

## File Map

**Rust backend (`src-tauri/src/`):**
- `main.rs` — Tauri entry point
- `lib.rs` — register commands, init DB on startup, AppState
- `models/mod.rs` — module declarations
- `models/task.rs` — Task, CreateTaskInput, UpdateTaskInput structs
- `modules/mod.rs` — module declarations
- `modules/db.rs` — SQLite init, migrations, connection
- `modules/task.rs` — Task CRUD business logic
- `commands/mod.rs` — module declarations
- `commands/task.rs` — Tauri IPC handlers for task operations

**Frontend (`src/`):**
- `main.tsx` — React entry point
- `App.tsx` — Router setup + layout
- `index.css` — Tailwind imports
- `types/task.ts` — TypeScript Task type
- `services/taskService.ts` — invoke() wrappers
- `stores/taskStore.ts` — Zustand task store
- `stores/calendarStore.ts` — Zustand calendar store
- `components/layout/Sidebar.tsx` — navigation sidebar
- `components/layout/TopBar.tsx` — page header
- `components/tasks/TaskCard.tsx` — single task display
- `components/tasks/TaskForm.tsx` — create/edit task modal
- `components/tasks/TaskList.tsx` — filtered/sorted task list
- `components/calendar/CalendarView.tsx` — view switcher container
- `components/calendar/MonthView.tsx` — month grid
- `components/calendar/WeekView.tsx` — week time columns
- `components/calendar/DayView.tsx` — day timeline
- `pages/TodayPage.tsx` — today's tasks
- `pages/TasksPage.tsx` — all tasks with filters
- `pages/CalendarPage.tsx` — calendar route
- `pages/SettingsPage.tsx` — placeholder settings

---

## Task 1: Scaffold Tauri 2 Project

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `index.html`
- Create: `src/main.tsx`, `src/App.tsx`, `src/index.css`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `tailwind.config.js`, `postcss.config.js`

- [ ] **Step 1: Create Tauri 2 project with React TypeScript template**

```bash
cd /mnt/mac/Users/qiny/codespace/taskmanager
npm create tauri-app@latest . -- --template react-ts --manager npm --force
```

Expected: project scaffold with `src/`, `src-tauri/`, `package.json`, `vite.config.ts`

- [ ] **Step 2: Install additional frontend dependencies**

```bash
npm install zustand@5 react-router-dom@6 dayjs
npm install -D tailwindcss@3 postcss autoprefixer daisyui@4
npx tailwindcss init -p
```

- [ ] **Step 3: Configure Tailwind**

`tailwind.config.js`:
```js
/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: { extend: {} },
  plugins: [require("daisyui")],
}
```

Replace `src/index.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

- [ ] **Step 4: Add Rust dependencies to `src-tauri/Cargo.toml`**

Add under `[dependencies]`:
```toml
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
tauri-plugin-notification = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
```

- [ ] **Step 5: Verify the app builds and opens**

```bash
npm run tauri dev
```

Expected: Tauri window opens with default React content. No compile errors.

- [ ] **Step 6: Commit**

```bash
git init && git add . && git commit -m "feat: scaffold Tauri 2 + React 19 + Tailwind project"
```

---

## Task 2: Rust Data Models

**Files:**
- Create: `src-tauri/src/models/mod.rs`
- Create: `src-tauri/src/models/task.rs`

- [ ] **Step 1: Create models module**

`src-tauri/src/models/mod.rs`:
```rust
pub mod task;
```

- [ ] **Step 2: Write Task and input structs**

`src-tauri/src/models/task.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub all_day: bool,
    pub recurrence: Option<String>,
    pub tags: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskInput {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub all_day: Option<bool>,
    pub recurrence: Option<String>,
    pub tags: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub all_day: Option<bool>,
    pub recurrence: Option<String>,
    pub tags: Option<String>,
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd src-tauri && cargo check
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models/ && git commit -m "feat: add Rust data models for Task"
```

---

## Task 3: SQLite Database Module

**Files:**
- Create: `src-tauri/src/modules/mod.rs`
- Create: `src-tauri/src/modules/db.rs`

- [ ] **Step 1: Create modules directory and mod.rs**

`src-tauri/src/modules/mod.rs`:
```rust
pub mod db;
pub mod task;
```

- [ ] **Step 2: Write db.rs with init and migrations**

`src-tauri/src/modules/db.rs`:
```rust
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub fn init(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tasks (
            id          TEXT PRIMARY KEY,
            title       TEXT NOT NULL,
            description TEXT,
            status      TEXT NOT NULL DEFAULT 'todo',
            priority    TEXT NOT NULL DEFAULT 'medium',
            start_time  INTEGER,
            end_time    INTEGER,
            all_day     INTEGER NOT NULL DEFAULT 0,
            recurrence  TEXT,
            tags        TEXT,
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS sync_mappings (
            id          TEXT PRIMARY KEY,
            task_id     TEXT NOT NULL,
            provider    TEXT NOT NULL,
            external_id TEXT NOT NULL,
            last_synced INTEGER NOT NULL,
            sync_status TEXT NOT NULL DEFAULT 'pending',
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS sync_accounts (
            id         TEXT PRIMARY KEY,
            provider   TEXT NOT NULL UNIQUE,
            expires_at INTEGER,
            config     TEXT
        );
        CREATE TABLE IF NOT EXISTS pomodoro_sessions (
            id         TEXT PRIMARY KEY,
            started_at INTEGER NOT NULL,
            duration   INTEGER NOT NULL DEFAULT 1500,
            completed  INTEGER NOT NULL DEFAULT 0
        );"
    )?;
    Ok(())
}
```

- [ ] **Step 3: Verify compilation**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/modules/ && git commit -m "feat: SQLite init with WAL mode and schema migrations"
```

---

## Task 4: Task CRUD Business Logic

**Files:**
- Create: `src-tauri/src/modules/task.rs`

- [ ] **Step 1: Write task module with all CRUD operations**

`src-tauri/src/modules/task.rs`:
```rust
use anyhow::Result;
use chrono::Utc;
use rusqlite::{Connection, params};
use uuid::Uuid;
use crate::models::task::{Task, CreateTaskInput, UpdateTaskInput};

pub fn get_all(conn: &Connection) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, start_time, end_time,
                all_day, recurrence, tags, created_at, updated_at
         FROM tasks ORDER BY created_at DESC"
    )?;
    let tasks = stmt.query_map([], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            all_day: row.get::<_, i64>(7)? != 0,
            recurrence: row.get(8)?,
            tags: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tasks)
}

pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, start_time, end_time,
                all_day, recurrence, tags, created_at, updated_at
         FROM tasks WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            all_day: row.get::<_, i64>(7)? != 0,
            recurrence: row.get(8)?,
            tags: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn create(conn: &Connection, input: CreateTaskInput) -> Result<Task> {
    let now = Utc::now().timestamp();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title: input.title,
        description: input.description,
        status: input.status.unwrap_or_else(|| "todo".to_string()),
        priority: input.priority.unwrap_or_else(|| "medium".to_string()),
        start_time: input.start_time,
        end_time: input.end_time,
        all_day: input.all_day.unwrap_or(false),
        recurrence: input.recurrence,
        tags: input.tags,
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO tasks (id, title, description, status, priority, start_time,
                            end_time, all_day, recurrence, tags, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            task.id, task.title, task.description, task.status, task.priority,
            task.start_time, task.end_time, task.all_day as i64,
            task.recurrence, task.tags, task.created_at, task.updated_at
        ],
    )?;
    Ok(task)
}

pub fn update(conn: &Connection, input: UpdateTaskInput) -> Result<Option<Task>> {
    let now = Utc::now().timestamp();
    let rows = conn.execute(
        "UPDATE tasks SET
            title = COALESCE(?2, title),
            description = COALESCE(?3, description),
            status = COALESCE(?4, status),
            priority = COALESCE(?5, priority),
            start_time = COALESCE(?6, start_time),
            end_time = COALESCE(?7, end_time),
            all_day = COALESCE(?8, all_day),
            recurrence = COALESCE(?9, recurrence),
            tags = COALESCE(?10, tags),
            updated_at = ?11
         WHERE id = ?1",
        params![
            input.id, input.title, input.description, input.status, input.priority,
            input.start_time, input.end_time, input.all_day.map(|b| b as i64),
            input.recurrence, input.tags, now
        ],
    )?;
    if rows == 0 { return Ok(None); }
    get_by_id(conn, &input.id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
    let rows = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

pub fn get_by_date_range(conn: &Connection, from: i64, to: i64) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, description, status, priority, start_time, end_time,
                all_day, recurrence, tags, created_at, updated_at
         FROM tasks
         WHERE (start_time >= ?1 AND start_time <= ?2)
            OR (end_time >= ?1 AND end_time <= ?2)
            OR (start_time <= ?1 AND end_time >= ?2)
         ORDER BY start_time ASC"
    )?;
    let tasks = stmt.query_map(params![from, to], |row| {
        Ok(Task {
            id: row.get(0)?,
            title: row.get(1)?,
            description: row.get(2)?,
            status: row.get(3)?,
            priority: row.get(4)?,
            start_time: row.get(5)?,
            end_time: row.get(6)?,
            all_day: row.get::<_, i64>(7)? != 0,
            recurrence: row.get(8)?,
            tags: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    })?.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tasks)
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd src-tauri && cargo check
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/modules/task.rs && git commit -m "feat: task CRUD business logic"
```

---

## Task 5: Tauri IPC Command Handlers + AppState

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/task.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create commands module**

`src-tauri/src/commands/mod.rs`:
```rust
pub mod task;
```

- [ ] **Step 2: Write Tauri command handlers**

`src-tauri/src/commands/task.rs`:
```rust
use tauri::State;
use crate::AppState;
use crate::models::task::{Task, CreateTaskInput, UpdateTaskInput};
use crate::modules::task as task_module;

#[tauri::command]
pub fn get_tasks(state: State<AppState>) -> Result<Vec<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::get_all(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_task(state: State<AppState>, id: String) -> Result<Option<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::get_by_id(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_task(state: State<AppState>, input: CreateTaskInput) -> Result<Task, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::create(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task(state: State<AppState>, input: UpdateTaskInput) -> Result<Option<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::update(&conn, input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task(state: State<AppState>, id: String) -> Result<bool, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::delete(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_tasks_by_range(state: State<AppState>, from: i64, to: i64) -> Result<Vec<Task>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    task_module::get_by_date_range(&conn, from, to).map_err(|e| e.to_string())
}
```

- [ ] **Step 3: Wire up lib.rs with AppState and command registration**

`src-tauri/src/lib.rs`:
```rust
mod commands;
mod models;
mod modules;

use std::sync::Mutex;
use rusqlite::Connection;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("taskmanager.db");
            let conn = modules::db::init(&db_path)
                .expect("Failed to initialize database");
            app.manage(AppState { db: Mutex::new(conn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::task::get_tasks,
            commands::task::get_task,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::delete_task,
            commands::task::get_tasks_by_range,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Build full project**

```bash
cd src-tauri && cargo build
```

Expected: compiles with no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ && git commit -m "feat: Tauri IPC commands and AppState wiring"
```

---

## Task 6: Frontend Types and Service Layer

**Files:**
- Create: `src/types/task.ts`
- Create: `src/services/taskService.ts`

- [ ] **Step 1: Write TypeScript types**

`src/types/task.ts`:
```typescript
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
```

- [ ] **Step 2: Write task service**

`src/services/taskService.ts`:
```typescript
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
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add src/types/ src/services/ && git commit -m "feat: TypeScript task types and service layer"
```

---

## Task 7: Zustand Stores

**Files:**
- Create: `src/stores/taskStore.ts`
- Create: `src/stores/calendarStore.ts`

- [ ] **Step 1: Write task store**

`src/stores/taskStore.ts`:
```typescript
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
```

- [ ] **Step 2: Write calendar store**

`src/stores/calendarStore.ts`:
```typescript
import { create } from 'zustand';
import dayjs, { Dayjs } from 'dayjs';

type ViewType = 'month' | 'week' | 'day';

interface CalendarStore {
  viewType: ViewType;
  selectedDate: Dayjs;
  setViewType: (v: ViewType) => void;
  setSelectedDate: (d: Dayjs) => void;
  goToToday: () => void;
  goToPrev: () => void;
  goToNext: () => void;
}

export const useCalendarStore = create<CalendarStore>((set, get) => ({
  viewType: 'month',
  selectedDate: dayjs(),

  setViewType: (viewType) => set({ viewType }),
  setSelectedDate: (selectedDate) => set({ selectedDate }),
  goToToday: () => set({ selectedDate: dayjs() }),

  goToPrev: () => {
    const { viewType, selectedDate } = get();
    set({ selectedDate: selectedDate.subtract(1, viewType) });
  },

  goToNext: () => {
    const { viewType, selectedDate } = get();
    set({ selectedDate: selectedDate.add(1, viewType) });
  },
}));
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 4: Commit**

```bash
git add src/stores/ && git commit -m "feat: Zustand stores for tasks and calendar"
```

---

## Task 8: Layout Components and Router

**Files:**
- Create: `src/components/layout/Sidebar.tsx`
- Create: `src/components/layout/TopBar.tsx`
- Modify: `src/App.tsx`
- Create: `src/pages/TodayPage.tsx`
- Create: `src/pages/TasksPage.tsx`
- Create: `src/pages/CalendarPage.tsx`
- Create: `src/pages/SettingsPage.tsx`

- [ ] **Step 1: Write Sidebar**

`src/components/layout/Sidebar.tsx`:
```tsx
import { NavLink } from 'react-router-dom';

const links = [
  { to: '/', label: '今日', icon: '☀' },
  { to: '/calendar', label: '日历', icon: '📅' },
  { to: '/tasks', label: '任务', icon: '✓' },
  { to: '/settings', label: '设置', icon: '⚙' },
];

export function Sidebar() {
  return (
    <aside className="w-16 bg-base-200 flex flex-col items-center py-4 gap-4 h-screen">
      {links.map(({ to, label, icon }) => (
        <NavLink
          key={to}
          to={to}
          end={to === '/'}
          className={({ isActive }) =>
            `flex flex-col items-center gap-1 p-2 rounded-lg w-12 text-xs
             ${isActive ? 'bg-primary text-primary-content' : 'hover:bg-base-300'}`
          }
        >
          <span className="text-lg">{icon}</span>
          <span>{label}</span>
        </NavLink>
      ))}
    </aside>
  );
}
```

- [ ] **Step 2: Write TopBar**

`src/components/layout/TopBar.tsx`:
```tsx
interface TopBarProps {
  title: string;
  children?: React.ReactNode;
}

export function TopBar({ title, children }: TopBarProps) {
  return (
    <header className="h-12 bg-base-100 border-b border-base-300 flex items-center justify-between px-4">
      <h1 className="text-lg font-semibold">{title}</h1>
      <div className="flex items-center gap-2">{children}</div>
    </header>
  );
}
```

- [ ] **Step 3: Write placeholder pages**

`src/pages/TodayPage.tsx`:
```tsx
import { TopBar } from '../components/layout/TopBar';

export function TodayPage() {
  return (
    <div className="flex flex-col flex-1">
      <TopBar title="今日" />
      <div className="flex-1 p-4">
        <p className="text-base-content/60">今日任务将在这里显示</p>
      </div>
    </div>
  );
}
```

`src/pages/TasksPage.tsx`:
```tsx
import { TopBar } from '../components/layout/TopBar';

export function TasksPage() {
  return (
    <div className="flex flex-col flex-1">
      <TopBar title="任务" />
      <div className="flex-1 p-4">
        <p className="text-base-content/60">任务列表将在这里显示</p>
      </div>
    </div>
  );
}
```

`src/pages/CalendarPage.tsx`:
```tsx
import { TopBar } from '../components/layout/TopBar';

export function CalendarPage() {
  return (
    <div className="flex flex-col flex-1">
      <TopBar title="日历" />
      <div className="flex-1 p-4">
        <p className="text-base-content/60">日历视图将在这里显示</p>
      </div>
    </div>
  );
}
```

`src/pages/SettingsPage.tsx`:
```tsx
import { TopBar } from '../components/layout/TopBar';

export function SettingsPage() {
  return (
    <div className="flex flex-col flex-1">
      <TopBar title="设置" />
      <div className="flex-1 p-4">
        <p className="text-base-content/60">设置页面</p>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Wire up App.tsx**

`src/App.tsx`:
```tsx
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { Sidebar } from './components/layout/Sidebar';
import { TodayPage } from './pages/TodayPage';
import { CalendarPage } from './pages/CalendarPage';
import { TasksPage } from './pages/TasksPage';
import { SettingsPage } from './pages/SettingsPage';

export default function App() {
  return (
    <BrowserRouter>
      <div className="flex h-screen bg-base-100">
        <Sidebar />
        <main className="flex-1 flex flex-col overflow-hidden">
          <Routes>
            <Route path="/" element={<TodayPage />} />
            <Route path="/calendar" element={<CalendarPage />} />
            <Route path="/tasks" element={<TasksPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
```

- [ ] **Step 5: Run dev and verify navigation works**

```bash
npm run tauri dev
```

Expected: app opens, sidebar navigation switches between pages.

- [ ] **Step 6: Commit**

```bash
git add src/ && git commit -m "feat: layout components, router, and placeholder pages"
```

---

## Task 9: Task Components (TaskCard, TaskForm, TaskList)

**Files:**
- Create: `src/components/tasks/TaskCard.tsx`
- Create: `src/components/tasks/TaskForm.tsx`
- Create: `src/components/tasks/TaskList.tsx`

- [ ] **Step 1: Write TaskCard**

`src/components/tasks/TaskCard.tsx`:
```tsx
import { Task } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';

const priorityColors = {
  low: 'border-l-info',
  medium: 'border-l-warning',
  high: 'border-l-error',
};

const statusLabels = {
  todo: '待办',
  in_progress: '进行中',
  done: '已完成',
};

interface TaskCardProps {
  task: Task;
  onEdit: (task: Task) => void;
}

export function TaskCard({ task, onEdit }: TaskCardProps) {
  const { deleteTask, updateTask } = useTaskStore();

  const toggleStatus = () => {
    const next = task.status === 'done' ? 'todo' : 'done';
    updateTask({ id: task.id, status: next });
  };

  return (
    <div className={`card bg-base-100 shadow-sm border-l-4 ${priorityColors[task.priority]}`}>
      <div className="card-body p-3 flex-row items-center gap-3">
        <input
          type="checkbox"
          className="checkbox checkbox-sm"
          checked={task.status === 'done'}
          onChange={toggleStatus}
        />
        <div className="flex-1 min-w-0">
          <p className={`font-medium truncate ${task.status === 'done' ? 'line-through opacity-50' : ''}`}>
            {task.title}
          </p>
          {task.start_time && (
            <p className="text-xs text-base-content/60">
              {new Date(task.start_time * 1000).toLocaleString('zh-CN', {
                month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit'
              })}
            </p>
          )}
        </div>
        <div className="flex gap-1">
          <button className="btn btn-ghost btn-xs" onClick={() => onEdit(task)}>编辑</button>
          <button className="btn btn-ghost btn-xs text-error" onClick={() => deleteTask(task.id)}>删除</button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Write TaskForm**

`src/components/tasks/TaskForm.tsx`:
```tsx
import { useState, useEffect } from 'react';
import { Task, CreateTaskInput, UpdateTaskInput } from '../../types/task';
import { useTaskStore } from '../../stores/taskStore';

interface TaskFormProps {
  task?: Task | null;
  onClose: () => void;
}

export function TaskForm({ task, onClose }: TaskFormProps) {
  const { createTask, updateTask } = useTaskStore();
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [priority, setPriority] = useState<'low' | 'medium' | 'high'>('medium');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [allDay, setAllDay] = useState(false);

  useEffect(() => {
    if (task) {
      setTitle(task.title);
      setDescription(task.description || '');
      setPriority(task.priority as 'low' | 'medium' | 'high');
      setAllDay(task.all_day);
      if (task.start_time) {
        setStartTime(new Date(task.start_time * 1000).toISOString().slice(0, 16));
      }
      if (task.end_time) {
        setEndTime(new Date(task.end_time * 1000).toISOString().slice(0, 16));
      }
    }
  }, [task]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const start = startTime ? Math.floor(new Date(startTime).getTime() / 1000) : undefined;
    const end = endTime ? Math.floor(new Date(endTime).getTime() / 1000) : undefined;

    if (task) {
      await updateTask({
        id: task.id, title, description: description || undefined,
        priority, start_time: start, end_time: end, all_day: allDay,
      });
    } else {
      await createTask({
        title, description: description || undefined,
        priority, start_time: start, end_time: end, all_day: allDay,
      });
    }
    onClose();
  };

  return (
    <dialog className="modal modal-open">
      <div className="modal-box">
        <h3 className="font-bold text-lg">{task ? '编辑任务' : '新建任务'}</h3>
        <form onSubmit={handleSubmit} className="flex flex-col gap-3 mt-4">
          <input
            type="text" placeholder="任务标题" value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="input input-bordered w-full" required
          />
          <textarea
            placeholder="描述（可选）" value={description}
            onChange={(e) => setDescription(e.target.value)}
            className="textarea textarea-bordered w-full"
          />
          <select
            value={priority} onChange={(e) => setPriority(e.target.value as any)}
            className="select select-bordered w-full"
          >
            <option value="low">低优先级</option>
            <option value="medium">中优先级</option>
            <option value="high">高优先级</option>
          </select>
          <label className="label cursor-pointer justify-start gap-2">
            <input type="checkbox" className="checkbox checkbox-sm" checked={allDay}
              onChange={(e) => setAllDay(e.target.checked)} />
            <span className="label-text">全天</span>
          </label>
          {!allDay && (
            <>
              <input type="datetime-local" value={startTime}
                onChange={(e) => setStartTime(e.target.value)}
                className="input input-bordered w-full" />
              <input type="datetime-local" value={endTime}
                onChange={(e) => setEndTime(e.target.value)}
                className="input input-bordered w-full" />
            </>
          )}
          <div className="modal-action">
            <button type="button" className="btn" onClick={onClose}>取消</button>
            <button type="submit" className="btn btn-primary">
              {task ? '保存' : '创建'}
            </button>
          </div>
        </form>
      </div>
      <form method="dialog" className="modal-backdrop">
        <button onClick={onClose}>close</button>
      </form>
    </dialog>
  );
}
```

- [ ] **Step 3: Write TaskList**

`src/components/tasks/TaskList.tsx`:
```tsx
import { Task } from '../../types/task';
import { TaskCard } from './TaskCard';

interface TaskListProps {
  tasks: Task[];
  onEdit: (task: Task) => void;
}

export function TaskList({ tasks, onEdit }: TaskListProps) {
  if (tasks.length === 0) {
    return <p className="text-base-content/50 text-center py-8">暂无任务</p>;
  }
  return (
    <div className="flex flex-col gap-2">
      {tasks.map((task) => (
        <TaskCard key={task.id} task={task} onEdit={onEdit} />
      ))}
    </div>
  );
}
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add src/components/tasks/ && git commit -m "feat: TaskCard, TaskForm, and TaskList components"
```

---

## Task 10: Today Page and Tasks Page (Full Implementation)

**Files:**
- Modify: `src/pages/TodayPage.tsx`
- Modify: `src/pages/TasksPage.tsx`

- [ ] **Step 1: Implement TodayPage**

`src/pages/TodayPage.tsx`:
```tsx
import { useEffect, useState } from 'react';
import dayjs from 'dayjs';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskForm } from '../components/tasks/TaskForm';
import { useTaskStore } from '../stores/taskStore';
import { Task } from '../types/task';

export function TodayPage() {
  const { tasks, fetchTasks } = useTaskStore();
  const [editing, setEditing] = useState<Task | null>(null);
  const [showForm, setShowForm] = useState(false);

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const todayStart = dayjs().startOf('day').unix();
  const todayEnd = dayjs().endOf('day').unix();

  const todayTasks = tasks.filter((t) => {
    if (!t.start_time) return false;
    return t.start_time >= todayStart && t.start_time <= todayEnd;
  });

  const noDateTasks = tasks.filter((t) => !t.start_time && t.status !== 'done');

  return (
    <div className="flex flex-col flex-1">
      <TopBar title={`今日 · ${dayjs().format('M月D日 ddd')}`}>
        <button className="btn btn-primary btn-sm" onClick={() => setShowForm(true)}>
          + 新任务
        </button>
      </TopBar>
      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        <section>
          <h2 className="text-sm font-semibold text-base-content/70 mb-2">今日任务</h2>
          <TaskList tasks={todayTasks} onEdit={setEditing} />
        </section>
        {noDateTasks.length > 0 && (
          <section>
            <h2 className="text-sm font-semibold text-base-content/70 mb-2">未安排</h2>
            <TaskList tasks={noDateTasks} onEdit={setEditing} />
          </section>
        )}
      </div>
      {(showForm || editing) && (
        <TaskForm task={editing} onClose={() => { setShowForm(false); setEditing(null); }} />
      )}
    </div>
  );
}
```

- [ ] **Step 2: Implement TasksPage with filters**

`src/pages/TasksPage.tsx`:
```tsx
import { useEffect, useState } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { TaskList } from '../components/tasks/TaskList';
import { TaskForm } from '../components/tasks/TaskForm';
import { useTaskStore } from '../stores/taskStore';
import { Task } from '../types/task';

export function TasksPage() {
  const { tasks, fetchTasks } = useTaskStore();
  const [editing, setEditing] = useState<Task | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [filter, setFilter] = useState<'all' | 'todo' | 'in_progress' | 'done'>('all');

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const filtered = filter === 'all' ? tasks : tasks.filter((t) => t.status === filter);

  return (
    <div className="flex flex-col flex-1">
      <TopBar title="任务">
        <div className="join">
          {(['all', 'todo', 'in_progress', 'done'] as const).map((f) => (
            <button key={f}
              className={`btn btn-xs join-item ${filter === f ? 'btn-active' : ''}`}
              onClick={() => setFilter(f)}
            >
              {{ all: '全部', todo: '待办', in_progress: '进行中', done: '已完成' }[f]}
            </button>
          ))}
        </div>
        <button className="btn btn-primary btn-sm" onClick={() => setShowForm(true)}>
          + 新任务
        </button>
      </TopBar>
      <div className="flex-1 overflow-y-auto p-4">
        <TaskList tasks={filtered} onEdit={setEditing} />
      </div>
      {(showForm || editing) && (
        <TaskForm task={editing} onClose={() => { setShowForm(false); setEditing(null); }} />
      )}
    </div>
  );
}
```

- [ ] **Step 3: Run dev and verify task CRUD works end-to-end**

```bash
npm run tauri dev
```

Expected: can create, edit, delete tasks. Today page shows today's tasks. Tasks page shows all with filters.

- [ ] **Step 4: Commit**

```bash
git add src/pages/ && git commit -m "feat: TodayPage and TasksPage with full CRUD"
```

---

## Task 11: Calendar — MonthView

**Files:**
- Create: `src/components/calendar/MonthView.tsx`

- [ ] **Step 1: Write MonthView component**

`src/components/calendar/MonthView.tsx`:
```tsx
import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface MonthViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
}

export function MonthView({ selectedDate, tasks, onDateClick }: MonthViewProps) {
  const startOfMonth = selectedDate.startOf('month');
  const startDay = startOfMonth.startOf('week');
  const weeks: Dayjs[][] = [];

  let current = startDay;
  for (let w = 0; w < 6; w++) {
    const week: Dayjs[] = [];
    for (let d = 0; d < 7; d++) {
      week.push(current);
      current = current.add(1, 'day');
    }
    weeks.push(week);
  }

  const getTasksForDay = (date: Dayjs) => {
    const dayStart = date.startOf('day').unix();
    const dayEnd = date.endOf('day').unix();
    return tasks.filter((t) => {
      if (!t.start_time) return false;
      return t.start_time >= dayStart && t.start_time <= dayEnd;
    });
  };

  const isToday = (date: Dayjs) => date.isSame(dayjs(), 'day');
  const isCurrentMonth = (date: Dayjs) => date.month() === selectedDate.month();

  return (
    <div className="grid grid-cols-7 gap-px bg-base-300 rounded-lg overflow-hidden">
      {['日', '一', '二', '三', '四', '五', '六'].map((d) => (
        <div key={d} className="bg-base-200 text-center text-xs font-medium py-2">{d}</div>
      ))}
      {weeks.flat().map((date, i) => {
        const dayTasks = getTasksForDay(date);
        return (
          <div
            key={i}
            onClick={() => onDateClick(date)}
            className={`bg-base-100 min-h-[80px] p-1 cursor-pointer hover:bg-base-200 transition
              ${!isCurrentMonth(date) ? 'opacity-40' : ''}`}
          >
            <span className={`text-xs inline-block w-6 h-6 text-center leading-6 rounded-full
              ${isToday(date) ? 'bg-primary text-primary-content' : ''}`}>
              {date.date()}
            </span>
            <div className="mt-1 space-y-0.5">
              {dayTasks.slice(0, 3).map((t) => (
                <div key={t.id} className="text-[10px] truncate px-1 rounded bg-primary/10 text-primary">
                  {t.title}
                </div>
              ))}
              {dayTasks.length > 3 && (
                <div className="text-[10px] text-base-content/50">+{dayTasks.length - 3} 更多</div>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/components/calendar/MonthView.tsx && git commit -m "feat: MonthView calendar component"
```

---

## Task 12: Calendar — WeekView

**Files:**
- Create: `src/components/calendar/WeekView.tsx`

- [ ] **Step 1: Write WeekView component**

`src/components/calendar/WeekView.tsx`:
```tsx
import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface WeekViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
}

const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function WeekView({ selectedDate, tasks, onDateClick }: WeekViewProps) {
  const startOfWeek = selectedDate.startOf('week');
  const days = Array.from({ length: 7 }, (_, i) => startOfWeek.add(i, 'day'));

  const getTasksForDayHour = (date: Dayjs, hour: number) => {
    const slotStart = date.hour(hour).minute(0).unix();
    const slotEnd = date.hour(hour).minute(59).unix();
    return tasks.filter((t) => {
      if (!t.start_time) return false;
      return t.start_time >= slotStart && t.start_time <= slotEnd;
    });
  };

  return (
    <div className="flex flex-col h-full overflow-auto">
      <div className="grid grid-cols-[60px_repeat(7,1fr)] sticky top-0 bg-base-100 z-10 border-b">
        <div className="p-2" />
        {days.map((d, i) => (
          <div key={i} className="text-center py-2 border-l border-base-300 cursor-pointer hover:bg-base-200"
            onClick={() => onDateClick(d)}>
            <div className="text-xs text-base-content/60">{['日','一','二','三','四','五','六'][d.day()]}</div>
            <div className={`text-sm font-medium w-7 h-7 mx-auto flex items-center justify-center rounded-full
              ${d.isSame(dayjs(), 'day') ? 'bg-primary text-primary-content' : ''}`}>
              {d.date()}
            </div>
          </div>
        ))}
      </div>
      <div className="grid grid-cols-[60px_repeat(7,1fr)] flex-1">
        {HOURS.map((hour) => (
          <div key={hour} className="contents">
            <div className="text-[10px] text-base-content/50 text-right pr-2 pt-1 h-12 border-t border-base-200">
              {hour.toString().padStart(2, '0')}:00
            </div>
            {days.map((d, di) => {
              const slotTasks = getTasksForDayHour(d, hour);
              return (
                <div key={di} className="border-l border-t border-base-200 h-12 p-0.5 relative">
                  {slotTasks.map((t) => (
                    <div key={t.id} className="text-[10px] truncate px-1 rounded bg-primary/20 text-primary mb-0.5">
                      {t.title}
                    </div>
                  ))}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/components/calendar/WeekView.tsx && git commit -m "feat: WeekView calendar component"
```

---

## Task 13: Calendar — DayView

**Files:**
- Create: `src/components/calendar/DayView.tsx`

- [ ] **Step 1: Write DayView component**

`src/components/calendar/DayView.tsx`:
```tsx
import dayjs, { Dayjs } from 'dayjs';
import { Task } from '../../types/task';

interface DayViewProps {
  selectedDate: Dayjs;
  tasks: Task[];
}

const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function DayView({ selectedDate, tasks }: DayViewProps) {
  const getTasksForHour = (hour: number) => {
    const slotStart = selectedDate.hour(hour).minute(0).unix();
    const slotEnd = selectedDate.hour(hour).minute(59).unix();
    return tasks.filter((t) => {
      if (!t.start_time) return false;
      return t.start_time >= slotStart && t.start_time <= slotEnd;
    });
  };

  const allDayTasks = tasks.filter((t) => t.all_day && t.start_time &&
    dayjs.unix(t.start_time).isSame(selectedDate, 'day'));

  return (
    <div className="flex flex-col h-full overflow-auto">
      {allDayTasks.length > 0 && (
        <div className="border-b border-base-300 p-2 bg-base-200">
          <span className="text-xs text-base-content/60 mr-2">全天</span>
          {allDayTasks.map((t) => (
            <span key={t.id} className="badge badge-primary badge-sm mr-1">{t.title}</span>
          ))}
        </div>
      )}
      <div className="flex-1">
        {HOURS.map((hour) => {
          const hourTasks = getTasksForHour(hour);
          return (
            <div key={hour} className="flex border-b border-base-200 min-h-[48px]">
              <div className="w-16 text-right pr-3 pt-1 text-xs text-base-content/50 shrink-0">
                {hour.toString().padStart(2, '0')}:00
              </div>
              <div className="flex-1 p-1 border-l border-base-200">
                {hourTasks.map((t) => (
                  <div key={t.id} className="text-sm px-2 py-1 rounded bg-primary/10 text-primary mb-1">
                    {t.title}
                  </div>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/components/calendar/DayView.tsx && git commit -m "feat: DayView calendar component"
```

---

## Task 14: CalendarView Switcher and CalendarPage Integration

**Files:**
- Create: `src/components/calendar/CalendarView.tsx`
- Modify: `src/pages/CalendarPage.tsx`

- [ ] **Step 1: Write CalendarView switcher**

`src/components/calendar/CalendarView.tsx`:
```tsx
import { Dayjs } from 'dayjs';
import { Task } from '../../types/task';
import { MonthView } from './MonthView';
import { WeekView } from './WeekView';
import { DayView } from './DayView';

interface CalendarViewProps {
  viewType: 'month' | 'week' | 'day';
  selectedDate: Dayjs;
  tasks: Task[];
  onDateClick: (date: Dayjs) => void;
}

export function CalendarView({ viewType, selectedDate, tasks, onDateClick }: CalendarViewProps) {
  switch (viewType) {
    case 'month':
      return <MonthView selectedDate={selectedDate} tasks={tasks} onDateClick={onDateClick} />;
    case 'week':
      return <WeekView selectedDate={selectedDate} tasks={tasks} onDateClick={onDateClick} />;
    case 'day':
      return <DayView selectedDate={selectedDate} tasks={tasks} />;
  }
}
```

- [ ] **Step 2: Implement CalendarPage with view switching and navigation**

`src/pages/CalendarPage.tsx`:
```tsx
import { useEffect } from 'react';
import { TopBar } from '../components/layout/TopBar';
import { CalendarView } from '../components/calendar/CalendarView';
import { useCalendarStore } from '../stores/calendarStore';
import { useTaskStore } from '../stores/taskStore';

export function CalendarPage() {
  const { viewType, selectedDate, setViewType, goToToday, goToPrev, goToNext, setSelectedDate } = useCalendarStore();
  const { tasks, fetchTasks } = useTaskStore();

  useEffect(() => { fetchTasks(); }, [fetchTasks]);

  const title = (() => {
    switch (viewType) {
      case 'month': return selectedDate.format('YYYY年M月');
      case 'week': return `${selectedDate.startOf('week').format('M/D')} - ${selectedDate.endOf('week').format('M/D')}`;
      case 'day': return selectedDate.format('YYYY年M月D日 ddd');
    }
  })();

  return (
    <div className="flex flex-col flex-1">
      <TopBar title={title}>
        <button className="btn btn-ghost btn-xs" onClick={goToPrev}>◀</button>
        <button className="btn btn-ghost btn-xs" onClick={goToToday}>今天</button>
        <button className="btn btn-ghost btn-xs" onClick={goToNext}>▶</button>
        <div className="join ml-2">
          {(['month', 'week', 'day'] as const).map((v) => (
            <button key={v}
              className={`btn btn-xs join-item ${viewType === v ? 'btn-active' : ''}`}
              onClick={() => setViewType(v)}
            >
              {{ month: '月', week: '周', day: '日' }[v]}
            </button>
          ))}
        </div>
      </TopBar>
      <div className="flex-1 overflow-hidden p-4">
        <CalendarView
          viewType={viewType}
          selectedDate={selectedDate}
          tasks={tasks}
          onDateClick={(date) => {
            setSelectedDate(date);
            setViewType('day');
          }}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Run dev and verify calendar views work**

```bash
npm run tauri dev
```

Expected: calendar page shows month view by default. Can switch between month/week/day. Clicking a date in month view drills into day view. Navigation arrows move forward/back.

- [ ] **Step 4: Commit**

```bash
git add src/components/calendar/ src/pages/CalendarPage.tsx && git commit -m "feat: calendar views with month/week/day switching"
```

---

## Task 15: Final Integration Test

**Files:** None (verification only)

- [ ] **Step 1: Full end-to-end test**

```bash
npm run tauri dev
```

Verify:
1. App opens with sidebar navigation
2. Today page: create a task with a time set to today — it appears in "今日任务"
3. Tasks page: all tasks visible, filter buttons work
4. Calendar page: month view shows task dots, click date → day view, week view shows time slots
5. Edit a task — changes reflect immediately
6. Delete a task — removed from all views

- [ ] **Step 2: TypeScript check**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Rust check**

```bash
cd src-tauri && cargo clippy
```

- [ ] **Step 4: Final commit**

```bash
git add -A && git commit -m "chore: Phase 1 & 2 complete — task CRUD + calendar views"
```

---

## Follow-on Plans

The following phases will be planned separately after Phase 1 & 2 are verified working:

- **Phase 3:** 飞书同步 — `docs/superpowers/plans/2026-XX-XX-taskmanager-feishu-sync.md`
- **Phase 4:** macOS 同步 — `docs/superpowers/plans/2026-XX-XX-taskmanager-macos-sync.md`
- **Phase 5:** 番茄钟 — `docs/superpowers/plans/2026-XX-XX-taskmanager-pomodoro.md`
- **Phase 6:** 企业微信同步
- **Phase 7:** Google Calendar 同步
