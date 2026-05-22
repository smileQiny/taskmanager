# TaskManager 设计规格文档

**日期：** 2026-05-21  
**状态：** 已确认  
**技术栈：** Tauri 2 + React 19 + TypeScript + Rust + SQLite

---

## 1. 项目概述

一个桌面端任务管理应用，核心价值是将飞书、macOS 日历、企业微信、Google Calendar 统一管理。数据本地优先（SQLite），同步只是推送/拉取到三方。附带极简番茄钟（计时器 + 系统通知）。

**同步优先级：** 飞书 → macOS → 企业微信 → Google

---

## 2. 整体架构

**方案：单体 Tauri 应用**

```
taskmanager/
├── src/                          # React 前端
│   ├── components/
│   │   ├── calendar/             # 月/周/日视图组件
│   │   ├── tasks/                # 任务卡片、表单、列表
│   │   ├── pomodoro/             # 番茄钟计时器
│   │   └── layout/               # 侧边栏、顶栏
│   ├── pages/                    # 路由页面
│   ├── stores/                   # Zustand 状态
│   ├── services/                 # Tauri command 调用封装
│   └── types/                    # TypeScript 类型定义
├── src-tauri/
│   └── src/
│       ├── commands/             # Tauri IPC 入口
│       │   ├── task.rs
│       │   ├── calendar.rs
│       │   └── sync.rs
│       ├── modules/
│       │   ├── db.rs             # SQLite 初始化 + migration
│       │   ├── task.rs           # 任务 CRUD 业务逻辑
│       │   ├── pomodoro.rs       # 番茄钟计时逻辑
│       │   └── sync/
│       │       ├── feishu.rs     # 飞书同步
│       │       ├── macos.rs      # macOS EventKit
│       │       ├── wecom.rs      # 企业微信
│       │       └── google.rs     # Google Calendar
│       └── models/               # 数据结构定义
```

**数据流：** React UI → Tauri IPC command → Rust 业务模块 → SQLite  
**同步流：** Rust sync 模块定时拉取三方 API → 写入本地 SQLite → 通知前端刷新

---

## 3. 数据模型

```sql
-- 任务主表
tasks (
  id          TEXT PRIMARY KEY,  -- UUID
  title       TEXT NOT NULL,
  description TEXT,
  status      TEXT,              -- todo | in_progress | done
  priority    TEXT,              -- low | medium | high
  start_time  INTEGER,           -- Unix timestamp
  end_time    INTEGER,
  all_day     BOOLEAN,
  recurrence  TEXT,              -- JSON: 重复规则 (daily/weekly/monthly)
  tags        TEXT,              -- JSON array
  created_at  INTEGER,
  updated_at  INTEGER
)

-- 三方同步映射表
sync_mappings (
  id            TEXT PRIMARY KEY,
  task_id       TEXT,            -- 关联本地任务
  provider      TEXT,            -- feishu | macos | wecom | google
  external_id   TEXT,            -- 三方平台的事件 ID
  last_synced   INTEGER,
  sync_status   TEXT             -- synced | pending | conflict
)

-- 三方账号凭证表（token 不存此处，走系统 keyring）
sync_accounts (
  id         TEXT PRIMARY KEY,
  provider   TEXT,
  expires_at INTEGER,
  config     TEXT               -- JSON: 额外配置（如企业微信 CorpID）
)

-- 番茄钟记录表
pomodoro_sessions (
  id         TEXT PRIMARY KEY,
  started_at INTEGER,
  duration   INTEGER,            -- 秒，默认 1500 (25分钟)
  completed  BOOLEAN
)
```

**关键决策：**
- `sync_mappings` 解耦本地任务与三方 ID，一个任务可同步到多个平台
- token 用 `keyring` crate 存系统钥匙串，不明文写 SQLite
- `recurrence` 用 JSON 存储，避免过度范式化

---

## 4. 三方同步机制

### 统一接口

```rust
trait CalendarProvider {
    async fn fetch_events(&self, from: i64, to: i64) -> Result<Vec<ExternalEvent>>;
    async fn push_event(&self, task: &Task) -> Result<String>;
    async fn update_event(&self, external_id: &str, task: &Task) -> Result<()>;
    async fn delete_event(&self, external_id: &str) -> Result<()>;
}
```

### 各平台实现

| 平台 | 认证方式 | 同步方向 | 关键 API |
|------|---------|---------|---------|
| 飞书 | OAuth 2.0 | 双向 | 飞书日历 API v4 |
| macOS | 无需认证（osascript） | 双向 | `osascript` 调用 EventKit，需在 `Info.plist` 声明 `NSCalendarsUsageDescription` |
| 企业微信 | OAuth 2.0 + CorpID | 双向 | 企业微信日历 API |
| Google | OAuth 2.0 | 双向（后期） | Google Calendar API v3 |

### 同步策略

- 应用启动时全量拉取一次
- 之后每 15 分钟增量同步（`tokio::spawn` 后台任务）
- 冲突处理：本地优先，三方变更若与本地未同步修改冲突，保留本地版本并标记 `conflict`
- 前端可在任务详情页手动触发同步

---

## 5. 前端设计

### 路由

```
/              → 今日视图（当天任务 + 番茄钟入口）
/calendar      → 日历视图（月/周/日切换）
/tasks         → 任务列表（全部任务，筛选/排序）
/settings      → 设置（三方账号绑定、同步配置）
```

### 关键组件

| 组件 | 功能 |
|------|------|
| `CalendarView` | 月/周/日三视图切换，任务以色块显示在对应时间格 |
| `TaskForm` | 新建/编辑任务，支持标题、时间、优先级、重复规则、标签 |
| `TaskCard` | 任务卡片，显示状态、优先级色标、同步状态图标 |
| `PomodoroTimer` | 悬浮小组件，25分钟倒计时，完成时系统通知 |
| `SyncStatus` | 顶栏同步状态指示器，显示最后同步时间和错误 |

### Zustand Stores

```
useTaskStore     — 任务列表、CRUD 操作
useCalendarStore — 当前视图类型、选中日期
usePomodoroStore — 计时状态、剩余时间
useSyncStore     — 各平台同步状态、账号信息
```

### 番茄钟交互

- 固定在右下角的悬浮按钮，点击展开计时器
- 开始/暂停/重置三个操作
- 完成时调用 Tauri 系统通知 API

---

## 6. 依赖清单

### Rust (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = ["notification"] }
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
tauri-plugin-notification = "2"
tauri-plugin-shell = "2"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }
keyring = "2"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
```

### 前端 (package.json)

```json
{
  "react": "^19",
  "typescript": "^5",
  "vite": "^6",
  "tailwindcss": "^3",
  "daisyui": "^4",
  "zustand": "^5",
  "@tauri-apps/api": "^2",
  "react-router-dom": "^6",
  "dayjs": "^1"
}
```

---

## 7. 实现优先级

1. **Phase 1：核心任务管理** — SQLite 初始化、任务 CRUD、今日视图、任务列表
2. **Phase 2：日历视图** — 月/周/日三视图，任务时间可视化
3. **Phase 3：飞书同步** — OAuth 认证、双向同步
4. **Phase 4：macOS 同步** — EventKit via osascript
5. **Phase 5：番茄钟** — 极简计时器 + 系统通知
6. **Phase 6：企业微信同步**
7. **Phase 7：Google Calendar 同步**
