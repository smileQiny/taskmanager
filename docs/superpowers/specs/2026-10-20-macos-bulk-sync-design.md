# macOS Calendar account-level bulk sync

Status: approved (Approach B), including the later conflict-resolution amendment.
Date: 2026-10-20

macOS Calendar sync is account-level. With the account enabled, a selected calendar, and Full Access, **Sync now** pushes every eligible local task and, when the direction includes calendar → local, imports events from that calendar. Tasks no longer need a per-task macOS opt-in. Feishu, Google, and WeCom stay on per-task opt-in.

## Problem

Sync now only walks `sync_mappings`. The macOS account can be enabled, with a calendar selected and Full Access granted, while `sync_mappings` is empty. Sync then reports 0 affected tasks. Events that live on a different calendar are not a substitute: import reads only the selected calendar, inside a fixed time window.

## Goals

- Settings keep the enable toggle, calendar picker, Request permission, Sync now, and auth status.
- Settings add a macOS-only sync direction: `bidirectional` (default), `local_to_remote`, or `remote_to_local`.
- Eligible timed tasks are enrolled automatically. No per-task macOS checkbox.
- Local → calendar creates or updates EventKit events on the selected calendar.
- Calendar → local lists events on the selected calendar from 90 days in the past through 180 days in the future and creates or updates local tasks.
- Bidirectional runs both directions. When both sides of a mapped pair changed since the last sync, the task stays in `conflict` until the user picks one of four resolutions.
- Tasks without both start and end times do not participate. Recurrence stays unsupported.

## Non-goals

- Do not switch Feishu, Google, or WeCom to bulk mode.
- Do not add recurrence create, edit, or sync.
- Do not bump the release version.
- Do not delete EventKit events when a local task is deleted or loses its times. That behavior is unchanged.
- Do not import events from calendars other than the selected one.

## Settings (macOS only)

Keep the current controls: enable toggle, calendar picker, **请求日历权限**, **立即同步**, and the auth / permission status.

Add a three-way **同步方向** control:

| Value | Label | Default |
| --- | --- | --- |
| `bidirectional` | 双向 | yes |
| `local_to_remote` | 仅本地 → 日历 | |
| `remote_to_local` | 仅日历 → 本地 | |

The macOS provider description, shown while the row is enabled or disabled, is:

> 仅 macOS。开启后自动同步所有带开始和结束时间的本地任务（含已完成），以及所选日历在过去 90 天到未来 180 天内的事件。

Under the direction control, explain the one-way limits:

> 双向会同时推送和导入。仅本地 → 日历不会把未映射的日历事件变成任务。仅日历 → 本地不会把仅存在于本地的任务推到日历。

An omitted or unknown `sync_direction` is `bidirectional`. Saving the calendar or the enable toggle does not clear a direction the user already chose. A missing direction on the save input means “leave the stored value”, and a missing stored value means `bidirectional`.

## Task detail

Feishu, Google, and WeCom keep the per-task checkbox and their existing two conflict buttons (**保留本地** / **采用 Google|飞书|企业微信**).

macOS no longer has an opt-in checkbox. The macOS cell is read-only status:

- No mapping row: **未同步**.
- Mapping row: the existing status words (`待同步`, `需授权`, `已同步`, `冲突`, `错误`).
- Task is missing start or end: **缺少起止时间，不会同步**, and no checkbox.
- `needs_auth` and `error` keep the current short hints.
- `conflict` shows four actions, documented below. The macOS card spans the sync grid so the four buttons fit.

The user cannot manually enroll or unenroll a single task for macOS. Enrollment is the eligibility rule during sync.

## Eligibility

A task is eligible when all of these are true:

- `start_time` is not null
- `end_time` is not null
- `recurrence` is null or only whitespace
- status may be `todo`, `in_progress`, or `done` (completed tasks are included)
- `all_day` may be true or false

Ineligible tasks are not auto-mapped, not pushed, and not pulled. An older macOS mapping on an ineligible task is left in place and skipped. Recurrence is still not written to or from EventKit.

Completion is not stored on the EventKit event. A done task with both times is still created or updated as a normal event.

## Data

`sync_accounts.config` for provider `macos` gains `sync_direction`:

```json
{
  "calendar_id": "cal-work",
  "calendar_name": "工作",
  "sync_direction": "bidirectional"
}
```

`sanitize_public_config` keeps `calendar_id`, `calendar_name`, and `sync_direction`, and still drops unrelated keys.

`MacosCalendarConnection` includes `sync_direction` so Settings can render the saved choice.

`SaveMacosCalendarSettingsInput.sync_direction` is optional. `None` leaves the stored direction unchanged.

Before the push/import pass, sync inserts a `sync_mappings` row for every eligible task that does not already have a macOS row:

- `provider = 'macos'`
- `external_id = ''`
- `last_synced = 0`
- `sync_status = 'pending'`

Existing rows are not reset. Other providers’ rows are not touched. This runs for every macOS sync direction so eligible tasks show a status without a manual checkbox. `remote_to_local` still does not push those empty rows.

## Sync entry

The behavior applies to the existing macOS sync entry (`sync_provider_now` → `sync_macos`) and to conflict resolution (`resolve_task_sync_conflict` for provider `macos`). Disabled, unsupported, and needs-auth gates stay as they are and run before any EventKit write or import.

The selected calendar is `calendar_id` when it is non-empty, otherwise the same name / default-calendar fallback `insert` already uses. Listing never spans every calendar on the Mac.

Time window, measured from the sync clock in unix seconds:

- start = now − 90 × 86400
- end = now + 180 × 86400

EventKit’s predicate returns events that overlap that range on the selected calendar. The planner asks for exactly those bounds. Occurrences that share one `eventIdentifier` produce one mapping.

## Direction behavior

`CalendarApi` gains `list_events(calendar_id, start_unix, end_unix)`. The default returns an empty list so Google, Feishu, and WeCom are unchanged. EventKit implements it with `predicateForEventsWithStartDate:endDate:calendars:` and `eventsMatchingPredicate:`, scoped to the resolved calendar.

### Shared pre-pass

1. List events in the window on the selected calendar.
2. Auto-ensure mappings for eligible tasks.
3. Adopt an existing app event before creating a second one. If a listed event’s notes contain `taskmanager-task-id:<id>`, that task exists, and its macOS mapping has an empty `external_id`, set `external_id` to that event id. If the task already has a different non-empty `external_id`, leave the extra event on the calendar and do not create another local task for it. Adoption is read-and-link only. It is how a wiped mapping avoids duplicating an event this app already wrote.

### `local_to_remote`

- Every eligible task is created or updated on the selected calendar.
- Empty `external_id` inserts, unless adoption filled it, in which case the adopted event is updated.
- A remote edit does not pull and does not become a conflict. The local task overwrites the calendar event.
- Listed events that do not adopt onto an eligible task do not create local tasks.

### `remote_to_local`

- Do not insert or update EventKit events for local tasks, including eligible tasks whose mapping `external_id` is empty.
- For each listed event whose id is already a macOS `external_id`, overwrite that local task from the event (title, description, start, end, all-day). Do not mark a conflict when the local task was also dirty.
- Mapped events outside the window are left unchanged and are not recreated.
- A listed event with no mapping and no adopted task creates a new local task (see Import).
- A missing remote event does not cause a new insert.

### `bidirectional`

- Run the existing per-mapping outcome engine on eligible snapshots after adoption:
  - no remote event → insert
  - only local changed → update the event
  - only remote changed → pull into the task
  - both changed since `last_synced` → `conflict`, and do not overwrite either side
  - neither changed → already synced
- Then import listed events whose ids are still not mapped and whose embedded task id did not adopt onto an existing task.
- Conflicts use the four resolutions below. Other providers keep the two-button resolver.

`affected_tasks` counts pushed, pulled, conflicted, failed, and newly imported tasks. A skipped untimed local task does not increment the count.

## Import

A new local task from an unmapped event:

| Field | Value |
| --- | --- |
| title | event summary, trimmed; `未命名日程` when empty |
| description | notes with the `taskmanager-task-id:` marker removed |
| status | `todo` |
| priority | `medium` |
| start / end / all-day | the event’s bounds |
| recurrence | null |
| tags | null |

Skip the event when start or end is missing. Insert a macOS mapping with that event id, `last_synced` at least the event’s updated time, and `sync_status = 'synced'`. Do not modify the calendar event during a pure import.

## Conflict resolution (macOS only)

These four strategies are the only macOS conflict actions. They replace a two-button or three-button macOS resolver. Google, Feishu, and WeCom do not gain them and reject the new strategy names.

The buttons:

1. **保留本地** — `local`
2. **采用日历** — `remote`
3. **保留全部（重命名）** — `keep_both_rename`
4. **保留全部（留最新）** — `keep_both_newest`

They stay available on an existing macOS `conflict` row even if the account direction was changed later. `local` and `remote` keep today’s resolver: `local` writes the task over the calendar event (or inserts if the event is gone); `remote` writes the calendar event over the task and errors with `macOS Calendar event no longer exists` when the event is gone.

Title suffixes use an ASCII space and parentheses. `disambiguated_title(title, marker)`:

- `Meeting` + `本地` → `Meeting (本地)`
- `Meeting (本地)` → `Meeting (本地 2)`
- `Meeting (本地 2)` → `Meeting (本地 3)`
- the same rules with marker `较早`

### 保留全部（重命名）

Keep both versions as separate items. The calendar event’s content is not overwritten.

1. Rename the existing local task with marker `本地`. Leave its description, status, priority, times, and all-day flag as they are.
2. Create a new local task from the calendar event (import field rules) and map it to the original event id. Do not write that event.
3. Insert a new EventKit event on the selected calendar from the renamed local task, and point the original task’s mapping at that new event id.
4. Mark both mappings `synced` with `last_synced` high enough that the next bidirectional sync does not immediately conflict.

Afterward the calendar has the original event plus a new event titled with the `(本地)` suffix. The task list has the renamed local task and a task that matches the calendar event.

### 保留全部（留最新）

Keep both versions. The primary mapping (the original task + the original event id) takes the newer content. The older content is kept as a separate renamed copy, on the task list and as its own new calendar event. Nothing is dropped.

Compare `task.updated_at` with `event.updated_unix`. Equal timestamps count as local newer.

- When local is newer: leave the primary task’s content as-is and update the original calendar event from it. The archived copy is the previous calendar content.
- When the calendar is newer: pull the event onto the primary task. The archived copy is the previous local content (status, priority, tags, description, and times included).

The archived task title is `disambiguated_title(older_title, "较早")`. Empty calendar summaries use `未命名日程` before the suffix. Insert a new EventKit event for the archived task and map it as `synced`. The primary mapping stays on the original event id and is `synced`.

`keep_both_rename` always suffixes the local task and always leaves the original event body untouched. `keep_both_newest` does not suffix the primary title; only the older copy gets `(较早)`, and the original event is updated so it matches the newer primary content.

If the remote event is already gone, both keep-all strategies return `macOS Calendar event no longer exists` and do not rename or insert.

## Error handling

- Account disabled, platform unsupported, or calendar access short of Full Access: the existing blocked result. No list, insert, or import.
- `list_events` failure fails the sync before applying outcomes.
- One task’s push/pull error becomes `SyncOutcome::Failed` and does not abort the rest, matching the current engine. Auth-shaped failures still abort.
- Unknown macOS strategy: error `conflict strategy must be local, remote, keep_both_rename, or keep_both_newest`.

## Testing

Rust tests in the macOS sync module, using an in-memory database and a fake calendar:

- Direction parse/save default, unknown value, and “omit means keep”.
- Auto-map eligible tasks, including `done`; skip missing times, skip recurrence, do not duplicate rows, do not touch other providers.
- `disambiguated_title` suffix increments.
- `bidirectional` with no prior mapping pushes a timed task and imports an in-window unmapped event. The list call receives the selected calendar id and the 90/180 window.
- `local_to_remote` pushes an unmapped timed task and does not create a task for a bare calendar event.
- `remote_to_local` imports an unmapped event and does not insert an event for a local-only timed task.
- Adoption links an event whose notes carry the task id instead of inserting a second event.
- Bilateral edits in `bidirectional` still end as `conflict` with neither side overwritten.
- `local`, `remote`, `keep_both_rename`, and `keep_both_newest` (local newer and calendar newer, plus the equal-timestamp tie) follow the rules above.
- Existing Google / Feishu / WeCom outcome tests stay green.

Frontend unit tests cover direction parsing and the four macOS conflict action labels. The checklist’s macOS section matches this behavior.

## Files

- `src-tauri/src/modules/macos_calendar.rs` — direction, eligibility, ensure-mappings, planner, import, four-way resolve.
- `src-tauri/src/modules/macos_eventkit.rs` — `list_events` for the selected calendar.
- `src-tauri/src/modules/google_calendar.rs` — default `list_events` only. No bulk behavior.
- `src-tauri/src/models/sync.rs`, `src-tauri/src/commands/sync.rs` — connection field, save field, call the macOS planner and resolver.
- `src/types/task.ts`, `src/utils/macosSync.ts`, `src/pages/SettingsPage.tsx`, `src/components/tasks/TaskDetailPanel.tsx`, `src/stores/settingsStore.ts`, `src/services/taskService.ts` — direction control, read-only macOS status, four buttons.
- `docs/sync-checklist.md` — macOS section.

## Acceptance

With macOS enabled, Full Access, and a calendar selected, Sync now in `bidirectional` pushes every eligible timed task and imports in-window events from that calendar, with no per-task checkbox. `local_to_remote` does not turn bare calendar events into tasks. `remote_to_local` does not push local-only tasks. A bidirectional conflict offers 保留本地, 采用日历, 保留全部（重命名）, and 保留全部（留最新）, and each one does what this spec says.
