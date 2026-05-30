# 📊 Graph Analysis Report

**Root:** `.`

## Summary

| Metric | Value |
|--------|-------|
| Nodes | 409 |
| Edges | 868 |
| Communities | 55 |
| Hyperedges | 0 |

### Confidence Breakdown

| Level | Count | Percentage |
|-------|-------|------------|
| EXTRACTED | 355 | 40.9% |
| INFERRED | 513 | 59.1% |
| AMBIGUOUS | 0 | 0.0% |

## 🌟 God Nodes (Most Connected)

| Node | Degree | Community |
|------|--------|-----------|
| taskService | 37 | 0 |
| create() | 34 | 1 |
| update() | 33 | 1 |
| memory_conn() | 30 | 1 |
| update_can_clear_nullable_fields() | 30 | 1 |
| normalize_required_title() | 29 | 1 |
| validate_status() | 29 | 1 |
| create_rejects_blank_title() | 29 | 1 |
| create_rejects_end_time_before_start_time() | 29 | 1 |
| validate_time_range() | 29 | 1 |

## 🔮 Surprising Connections

- **src_services_taskservice_ts** → **src_services_taskservice_ts_readtasks** (defines)
- **src_services_taskservice_ts** → **src_services_taskservice_ts_writetasks** (defines)
- **src_services_taskservice_ts** → **src_services_taskservice_ts_createlocaltask** (defines)
- **src_services_taskservice_ts** → **src_services_taskservice_ts_updatelocaltask** (defines)
- **src_services_taskservice_ts** → **src_services_taskservice_ts_normalizerequiredtitle** (defines)

## 🏘️ Communities

### Community 0 — writeTaskSyncStates() (29 nodes, cohesion: 0.10)

- taskService
- createLocalPomodoroSession()
- @tauri-apps/api/core/invoke
- ../types/task/AppMetadata
- ../types/task/AppSettings
- ../types/task/CreateTaskInput
- ../types/task/PomodoroSession
- ../types/task/SetTaskSyncInput
- ../types/task/SyncAccount
- ../types/task/SyncProvider
- ../types/task/SyncRunResult
- ../types/task/Task
- ../types/task/TaskSyncState
- ../types/task/UpdateTaskInput
- ../types/task/UpsertSyncAccountInput
- local()
- providerLabel()
- readAllTaskSyncStates()
- readJson()
- readPomodoroSessions()
- _…and 9 more_

### Community 1 — validate_time_range() (22 nodes, cohesion: 0.95)

- task
- create()
- create_rejects_blank_title()
- create_rejects_end_time_before_start_time()
- delete()
- get_all()
- get_by_date_range()
- get_by_id()
- anyhow::{bail, Result}
- chrono::Utc
- crate::models::task::{CreateTaskInput, Task, UpdateTaskInput}
- crate::modules::db
- rusqlite::{params, Connection}
- super::*
- uuid::Uuid
- memory_conn()
- normalize_required_title()
- update()
- update_can_clear_nullable_fields()
- validate_priority()
- _…and 2 more_

### Community 2 — validate_provider() (21 nodes, cohesion: 0.80)

- sync
- ensure_default_accounts()
- get_account()
- anyhow::{bail, Result}
- crate::models::sync::{
    SetTaskSyncInput, SyncAccount, SyncRunResult, TaskSyncState, UpsertSyncAccountInput,
}
- crate::models::task::CreateTaskInput
- crate::modules::db
- crate::modules::task as task_module
- rusqlite::{params, Connection}
- super::*
- uuid::Uuid
- list_accounts()
- list_task_sync_states()
- lists_default_provider_accounts()
- mark_provider_mappings()
- set_task_sync()
- sync_now()
- task_sync_mapping_can_be_enabled_and_marked_by_sync_run()
- upsert_account()
- upserts_provider_account_and_reports_auth_needed()
- _…and 1 more_

### Community 3 — sortTasks() (15 nodes, cohesion: 0.18)

- taskUtils
- compareOptionalNumber()
- filterTasks()
- formatTaskTime()
- getTaskTags()
- groupTasksForToday()
- dayjs/dayjs
- ../types/task/Task
- ../types/task/TaskFilters
- ../types/task/TaskPriorityFilter
- ../types/task/TaskSortKey
- ../types/task/TaskStatusFilter
- matchesPriorityFilter()
- matchesStatusFilter()
- sortTasks()

### Community 4 — validate_theme() (14 nodes, cohesion: 0.85)

- settings
- get()
- get_value()
- anyhow::{bail, Result}
- crate::models::settings::{AppSettings, UpdateSettingsInput}
- crate::modules::db
- rusqlite::{params, Connection}
- super::*
- set_value()
- settings_have_defaults_and_can_update()
- settings_reject_invalid_duration()
- update()
- validate_calendar_view()
- validate_theme()

### Community 5 — App() (13 nodes, cohesion: 0.15)

- App
- App()
- ./components/layout/Sidebar/Sidebar
- ./pages/CalendarPage/CalendarPage
- ./pages/PomodoroPage/PomodoroPage
- ./pages/SettingsPage/SettingsPage
- ./pages/TasksPage/TasksPage
- ./pages/TodayPage/TodayPage
- react-router-dom/BrowserRouter
- react-router-dom/Route
- react-router-dom/Routes
- react/useEffect
- ./stores/updateStore/useUpdateStore

### Community 6 — handleSubmit() (13 nodes, cohesion: 0.15)

- TaskDetailPanel
- handleSubmit()
- dayjs/dayjs
- react/FormEvent
- react/useEffect
- react/useMemo
- react/useState
- ../../services/taskService/syncService
- ../../stores/taskStore/useTaskStore
- ../../types/task/CreateTaskInput
- ../../types/task/SyncProvider
- ../../types/task/Task
- ../../types/task/TaskSyncState

### Community 7 — updatePomodoroMinutes() (12 nodes, cohesion: 0.17)

- SettingsPage
- ../components/layout/TopBar/TopBar
- react/useEffect
- ../stores/pomodoroStore/usePomodoroStore
- ../stores/settingsStore/useSettingsStore
- ../stores/updateStore/useUpdateStore
- ../types/task/SyncAccount
- ../types/task/SyncProvider
- ../types/update/AppUpdateInfo
- ../types/update/UpdateStatus
- updateBadgeClass()
- updatePomodoroMinutes()

### Community 8 — zustand/create (8) (12 nodes, cohesion: 0.17)

- taskStore
- ../services/taskService/taskService
- ../types/task/CreateTaskInput
- ../types/task/Task
- ../types/task/TaskFilters
- ../types/task/TaskPriorityFilter
- ../types/task/TaskSortKey
- ../types/task/TaskStatusFilter
- ../types/task/UpdateTaskInput
- ../utils/taskUtils/filterTasks
- ../utils/taskUtils/sortTasks
- zustand/create

### Community 9 — TodayPage() (11 nodes, cohesion: 0.18)

- TodayPage
- ../components/layout/TopBar/TopBar
- ../components/tasks/TaskDetailPanel/TaskDetailPanel
- ../components/tasks/TaskList/TaskList
- dayjs/dayjs
- react/useEffect
- ../stores/taskStore/useTaskStore
- ../utils/taskUtils/groupTasksForToday
- Summary()
- TaskSection()
- TodayPage()

### Community 10 — update_task() (11 nodes, cohesion: 0.73)

- task
- create_task()
- delete_task()
- get_task()
- get_tasks()
- get_tasks_by_range()
- crate::AppState
- crate::models::task::{CreateTaskInput, Task, UpdateTaskInput}
- crate::modules::task as task_module
- tauri::State
- update_task()

### Community 11 — parseVersion() (11 nodes, cohesion: 0.29)

- updateService
- checkForUpdates()
- compareVersions()
- detectPlatform()
- getPreferredInstallAsset()
- ../types/update/AppUpdateInfo
- ../types/update/ReleaseAsset
- isInstallerAsset()
- normalizeVersion()
- openExternalUrl()
- parseVersion()

### Community 12 — vitest/vi (11 nodes, cohesion: 0.18)

- updateService.test
- ../types/update/ReleaseAsset
- ./updateService/checkForUpdates
- ./updateService/compareVersions
- ./updateService/getPreferredInstallAsset
- ./updateService/normalizeVersion
- vitest/beforeEach
- vitest/describe
- vitest/expect
- vitest/it
- vitest/vi

### Community 13 — task() (10 nodes, cohesion: 0.20)

- taskUtils.test
- ./taskUtils/filterTasks
- ./taskUtils/getTaskTags
- ./taskUtils/groupTasksForToday
- ./taskUtils/sortTasks
- ../types/task/Task
- vitest/describe
- vitest/expect
- vitest/it
- task()

### Community 14 — zustand/create (14) (10 nodes, cohesion: 0.20)

- settingsStore
- ../services/taskService/appService
- ../services/taskService/settingsService
- ../services/taskService/syncService
- ../types/task/AppMetadata
- ../types/task/AppSettings
- ../types/task/SyncAccount
- ../types/task/SyncProvider
- ../types/task/SyncRunResult
- zustand/create

### Community 15 — upsert_sync_account() (10 nodes, cohesion: 0.64)

- sync
- crate::AppState
- crate::models::sync::{
    SetTaskSyncInput, SyncAccount, SyncRunResult, TaskSyncState, UpsertSyncAccountInput,
}
- crate::modules::sync as sync_module
- tauri::State
- list_sync_accounts()
- list_task_sync_states()
- set_task_sync()
- sync_provider_now()
- upsert_sync_account()

### Community 16 — migration_adds_sync_account_columns_to_existing_database() (10 nodes, cohesion: 0.29)

- db
- add_column_if_missing()
- anyhow::Result
- rusqlite::Connection
- std::path::Path
- super::*
- init()
- init_in_memory()
- migrate()
- migration_adds_sync_account_columns_to_existing_database()

### Community 17 — list_recent() (10 nodes, cohesion: 0.38)

- pomodoro
- create()
- creates_and_lists_recent_sessions()
- anyhow::{bail, Result}
- crate::models::pomodoro::{CreatePomodoroSessionInput, PomodoroSession}
- crate::modules::db
- rusqlite::{params, Connection}
- super::*
- uuid::Uuid
- list_recent()

### Community 18 — writeTasks() (9 nodes, cohesion: 0.39)

- createLocalTask()
- deleteLocalTask()
- normalizeRequiredTitle()
- readTasks()
- updateLocalTask()
- validatePriority()
- validateStatus()
- validateTimeRange()
- writeTasks()

### Community 19 — zustand/create (19) (9 nodes, cohesion: 0.22)

- updateStore
- ../services/updateService/appVersion
- ../services/updateService/checkForUpdates
- ../services/updateService/openExternalUrl
- ../services/updateService/releasesUrl
- ../types/update/AppUpdateInfo
- ../types/update/UpdateCheckSource
- ../types/update/UpdateStatus
- zustand/create

### Community 20 — TasksPage() (8 nodes, cohesion: 0.25)

- TasksPage
- ../components/layout/TopBar/TopBar
- ../components/tasks/TaskDetailPanel/TaskDetailPanel
- ../components/tasks/TaskFilters/TaskFilters
- ../components/tasks/TaskList/TaskList
- react/useEffect
- ../stores/taskStore/useTaskStore
- TasksPage()

### Community 21 — createMemoryStorage() (8 nodes, cohesion: 0.25)

- taskService.test
- createMemoryStorage()
- ./taskService/taskService
- vitest/beforeEach
- vitest/describe
- vitest/expect
- vitest/it
- vitest/vi

### Community 22 — list_pomodoro_sessions() (7 nodes, cohesion: 0.38)

- pomodoro
- create_pomodoro_session()
- crate::AppState
- crate::models::pomodoro::{CreatePomodoroSessionInput, PomodoroSession}
- crate::modules::pomodoro as pomodoro_module
- tauri::State
- list_pomodoro_sessions()

### Community 23 — UpsertSyncAccountInput (7 nodes, cohesion: 0.29)

- sync
- serde::{Deserialize, Serialize}
- SetTaskSyncInput
- SyncAccount
- SyncRunResult
- TaskSyncState
- UpsertSyncAccountInput

### Community 24 — ../stores/taskStore/useTaskStore (7 nodes, cohesion: 0.29)

- CalendarPage
- ../components/calendar/CalendarView/CalendarView
- ../components/layout/TopBar/TopBar
- ../components/tasks/TaskDetailPanel/TaskDetailPanel
- react/useEffect
- ../stores/calendarStore/useCalendarStore
- ../stores/taskStore/useTaskStore

### Community 25 — update_settings() (7 nodes, cohesion: 0.38)

- settings
- get_settings()
- crate::AppState
- crate::models::settings::{AppSettings, UpdateSettingsInput}
- crate::modules::settings as settings_module
- tauri::State
- update_settings()

### Community 26 — CalendarView() (7 nodes, cohesion: 0.29)

- CalendarView
- CalendarView()
- dayjs/Dayjs
- ./DayView/DayView
- ./MonthView/MonthView
- ../../types/task/Task
- ./WeekView/WeekView

### Community 27 — run() (7 nodes, cohesion: 0.29)

- lib
- AppState
- rusqlite::Connection
- std::path::PathBuf
- std::sync::Mutex
- tauri::Manager
- run()

### Community 28 — ../stores/settingsStore/useSettingsStore (6 nodes, cohesion: 0.33)

- PomodoroPage
- ../components/layout/TopBar/TopBar
- dayjs/dayjs
- react/useEffect
- ../stores/pomodoroStore/usePomodoroStore
- ../stores/settingsStore/useSettingsStore

### Community 29 — isToday() (6 nodes, cohesion: 0.33)

- MonthView
- getTasksForDay()
- dayjs/dayjs
- ../../types/task/Task
- isCurrentMonth()
- isToday()

### Community 30 — TaskFilters() (6 nodes, cohesion: 0.33)

- TaskFilters
- ../../stores/taskStore/useTaskStore
- ../../types/task/TaskPriorityFilter
- ../../types/task/TaskSortKey
- ../../types/task/TaskStatusFilter
- TaskFilters()

### Community 31 — TaskCard() (6 nodes, cohesion: 0.33)

- TaskCard
- ../../stores/taskStore/useTaskStore
- ../../types/task/Task
- ../../utils/taskUtils/formatTaskTime
- ../../utils/taskUtils/getTaskTags
- TaskCard()

### Community 32 — react/React (5 nodes, cohesion: 0.40)

- main
- ./App/App
- ./index.css
- react-dom/client/ReactDOM
- react/React

### Community 33 — UpdateTaskInput (5 nodes, cohesion: 0.40)

- task
- CreateTaskInput
- serde::{Deserialize, Serialize}
- Task
- UpdateTaskInput

### Community 34 — UpdateSettingsInput (5 nodes, cohesion: 0.40)

- settings
- AppMetadata
- AppSettings
- serde::{Deserialize, Serialize}
- UpdateSettingsInput

### Community 35 — get_app_metadata() (5 nodes, cohesion: 0.40)

- app
- get_app_metadata()
- crate::AppState
- crate::models::settings::AppMetadata
- tauri::State

### Community 36 — @vitejs/plugin-react/react (4 nodes, cohesion: 0.50)

- vite.config
- node:fs/readFileSync
- vite/defineConfig
- @vitejs/plugin-react/react

### Community 37 — getTasksForHour() (4 nodes, cohesion: 0.50)

- DayView
- getTasksForHour()
- dayjs/dayjs
- ../../types/task/Task

### Community 38 — TaskList() (4 nodes, cohesion: 0.50)

- TaskList
- ./TaskCard/TaskCard
- ../../types/task/Task
- TaskList()

### Community 39 — PomodoroSession (4 nodes, cohesion: 0.50)

- pomodoro
- CreatePomodoroSessionInput
- serde::{Deserialize, Serialize}
- PomodoroSession

### Community 40 — zustand/create (4 nodes, cohesion: 0.50)

- pomodoroStore
- ../services/taskService/pomodoroService
- ../types/task/PomodoroSession
- zustand/create

### Community 41 — getTasksForDayHour() (4 nodes, cohesion: 0.50)

- WeekView
- getTasksForDayHour()
- dayjs/dayjs
- ../../types/task/Task

### Community 42 — Sidebar() (3 nodes, cohesion: 0.67)

- Sidebar
- react-router-dom/NavLink
- Sidebar()

### Community 43 — zustand/create (43) (3 nodes, cohesion: 0.67)

- calendarStore
- dayjs/dayjs
- zustand/create

### Community 44 — main() (2 nodes, cohesion: 1.00)

- build
- main()

### Community 45 — main() (45) (2 nodes, cohesion: 1.00)

- main
- main()

### Community 46 — TopBar() (2 nodes, cohesion: 1.00)

- TopBar
- TopBar()

### Community 47 — vite-env.d (1 nodes, cohesion: 1.00)

- vite-env.d

### Community 48 — update (1 nodes, cohesion: 1.00)

- update

### Community 49 — mod (49) (1 nodes, cohesion: 1.00)

- mod

### Community 50 — tailwind.config (1 nodes, cohesion: 1.00)

- tailwind.config

### Community 51 — mod (1 nodes, cohesion: 1.00)

- mod

### Community 52 — postcss.config (1 nodes, cohesion: 1.00)

- postcss.config

### Community 53 — mod (53) (1 nodes, cohesion: 1.00)

- mod

### Community 54 — task (1 nodes, cohesion: 1.00)

- task

## 🕳️ Knowledge Gaps

**Isolated nodes** (8):
- tailwind.config
- mod
- mod
- mod
- postcss.config
- task
- update
- vite-env.d

**Thin communities** (< 3 nodes): 11 communities

## 💰 Token Cost

| File | Tokens |
|------|--------|
| output | 0 |
| input | 0 |
| **Total** | **0** |

## ❓ Suggested Questions

1. Can you verify the inferred relationships of 'create()' (degree 34)?
1. Can you verify the inferred relationships of 'update()' (degree 33)?
1. Can you verify the inferred relationships of 'memory_conn()' (degree 30)?
1. Can you verify the inferred relationships of 'update_can_clear_nullable_fields()' (degree 30)?
1. What role does 'mod' play? It has no connections in the graph.
1. What role does 'update' play? It has no connections in the graph.
1. What role does 'vite-env.d' play? It has no connections in the graph.

---
_Generated by graphify-rs_
