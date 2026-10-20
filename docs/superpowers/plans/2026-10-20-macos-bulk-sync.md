# macOS Calendar Bulk Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** macOS Sync now enrolls every eligible timed task and imports the selected calendar’s in-window events, with a direction setting and four conflict resolutions.

**Architecture:** Keep the Google/Feishu/WeCom per-task outcome engine. macOS gets its own planner in `macos_calendar` that ensures mappings, adopts events by embedded task id, pushes and/or imports based on `sync_direction`, and resolves conflicts with `local`, `remote`, `keep_both_rename`, or `keep_both_newest`. EventKit gains `list_events` for one calendar. The task detail macOS cell becomes read-only status plus the four buttons.

**Tech Stack:** Rust (Tauri, rusqlite, EventKit), React, Vitest.

## Global Constraints

- Direction values are exactly `bidirectional` (default), `local_to_remote`, `remote_to_local`.
- Import window is past 90 days through future 180 days on the selected calendar only.
- Eligible tasks have both `start_time` and `end_time`, blank recurrence, any status including `done`.
- Feishu, Google, and WeCom stay per-task opt-in with two conflict buttons.
- Recurrence stays unsupported. Do not bump the release version.
- macOS conflict actions are exactly: 保留本地 (`local`), 采用日历 (`remote`), 保留全部（重命名） (`keep_both_rename`), 保留全部（留最新） (`keep_both_newest`).
- `keep_both_rename` suffixes the local task with ` (本地)`, leaves the original event body untouched, and inserts a second event for the renamed task.
- `keep_both_newest` puts the newer side on the primary mapping (tie = local newer) and stores the older side as a separate task titled with ` (较早)` plus its own new event.
- Spec: `docs/superpowers/specs/2026-10-20-macos-bulk-sync-design.md`.

---

### Task 1: Direction config

**Files:**
- Modify: `src-tauri/src/models/sync.rs`
- Modify: `src-tauri/src/modules/macos_calendar.rs`
- Modify: `src-tauri/src/commands/sync.rs` (struct literals and upsert)

- [ ] **Step 1:** Failing test that missing/unknown `sync_direction` becomes `bidirectional`, a valid value round-trips, and `save_settings` with `sync_direction: None` keeps the stored value.
- [ ] **Step 2:** `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib macos_calendar::tests::sync_direction -- --test-threads=8`
- [ ] **Step 3:** Add the field to config parse/serialize, `MacosCalendarConnection`, and `SaveMacosCalendarSettingsInput` (`#[serde(default)]`).
- [ ] **Step 4:** Re-run the test. Expected: PASS.

### Task 2: Eligibility, auto-map, title suffix

**Files:**
- Modify: `src-tauri/src/modules/macos_calendar.rs`

- [ ] **Step 1:** Tests for `task_is_sync_eligible`, `ensure_timed_mappings` (done included, untimed and recurrence skipped, no duplicate, other providers untouched), `import_window`, and `disambiguated_title`.
- [ ] **Step 2:** Run those tests. Expected: FAIL.
- [ ] **Step 3:** Implement the helpers.
- [ ] **Step 4:** Re-run. Expected: PASS.

### Task 3: Planner, import, four-way resolve

**Files:**
- Modify: `src-tauri/src/modules/google_calendar.rs` (default `list_events`)
- Modify: `src-tauri/src/modules/macos_calendar.rs`
- Modify: `src-tauri/src/commands/sync.rs` (`sync_macos`, `resolve_macos`)

- [ ] **Step 1:** Tests: bidirectional push + import and list bounds; `local_to_remote` does not import a bare event; `remote_to_local` does not insert for a local-only task; adoption does not insert a second event; bilateral edit stays `conflict`; `local`, `remote`, `keep_both_rename`, `keep_both_newest` (local newer, calendar newer, equal timestamps) and missing-event error.
- [ ] **Step 2:** Run `macos_calendar::tests`. Expected: FAIL on new tests.
- [ ] **Step 3:** Implement `run_macos_sync` and `resolve_macos_conflict`. Wire the Tauri commands. Default `CalendarApi::list_events` returns an empty vec.
- [ ] **Step 4:** Re-run macos tests plus `google_calendar`, `feishu_calendar`, `wecom_calendar`. Expected: PASS.

### Task 4: EventKit list

**Files:**
- Modify: `src-tauri/src/modules/macos_eventkit.rs`

- [ ] **Step 1:** Implement `list_events` with `predicateForEventsWithStartDate_endDate_calendars` scoped to the resolved calendar. Dedupe by event identifier. This file is macOS-only; Linux tests cover the planner against a fake.

### Task 5: Settings, task detail, checklist

**Files:**
- Modify: `src/utils/macosSync.ts`, `src/utils/macosSync.test.ts`
- Modify: `src/types/task.ts`, `src/services/taskService.ts`, `src/stores/settingsStore.ts`
- Modify: `src/pages/SettingsPage.tsx`, `src/components/tasks/TaskDetailPanel.tsx`
- Modify: `docs/sync-checklist.md`

- [ ] **Step 1:** Vitest for `parseMacosSyncDirection` and the four `MACOS_CONFLICT_ACTIONS` labels.
- [ ] **Step 2:** `npm test`. Expected: FAIL until helpers exist, then PASS.
- [ ] **Step 3:** Direction select and copy in Settings. Remove the macOS opt-in checkbox. Show read-only status and four conflict buttons. Refresh tasks after every non-`local` resolution. Update the macOS checklist section.
- [ ] **Step 4:** `npm test` and `npx tsc --noEmit`. Expected: PASS.

---
