//! EventKit bridge. This module is compiled only for macOS.
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use block2::RcBlock;
use chrono::{Local, NaiveDate, TimeZone};
use dispatch2::DispatchQueue;
use objc2::rc::Retained;
use objc2::runtime::{Bool, NSObjectProtocol};
use objc2::sel;
use objc2_event_kit::{EKEntityType, EKEvent, EKEventStore, EKSpan};
use objc2_foundation::{NSArray, NSDate, NSError, NSPredicate, NSString, NSURL};
use std::collections::HashSet;
use std::time::Duration;

use crate::modules::google_calendar::{CalendarApi, EventDraft, EventMoment, RemoteEvent};
use crate::modules::macos_calendar::{
    access_from_code, decode_notes, encode_notes, task_url, CalendarAccess,
};

pub struct EventKitApi {
    calendar_id: String,
    calendar_name: String,
}

impl EventKitApi {
    pub fn new(calendar_id: impl Into<String>, calendar_name: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            calendar_name: calendar_name.into(),
        }
    }
}

pub fn current_access() -> CalendarAccess {
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
    access_from_code(status.0 as i64)
}

pub async fn request_full_access() -> Result<CalendarAccess> {
    let (tx, rx) = std::sync::mpsc::channel();
    // The permission prompt has to be created on the main queue, and EventKit
    // invokes the completion later. The store and block are forgotten so they
    // stay alive until that completion runs.
    on_main(Box::new(move || unsafe {
        let store = EKEventStore::new();
        let block: RcBlock<dyn Fn(Bool, *mut NSError)> =
            RcBlock::new(move |granted: Bool, error: *mut NSError| {
                let _ = (granted.as_bool(), error_text(error));
                let _ = tx.send(());
            });
        let completion = RcBlock::as_ptr(&block);
        if store.respondsToSelector(sel!(requestFullAccessToEventsWithCompletion:)) {
            store.requestFullAccessToEventsWithCompletion(completion);
        } else {
            #[allow(deprecated)]
            store.requestAccessToEntityType_completion(EKEntityType::Event, completion);
        }
        std::mem::forget(block);
        std::mem::forget(store);
    }));
    // Waiting on this thread would deadlock when it is the main thread: the
    // completion is queued there and cannot run until this wait returns.
    tokio::task::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(600)))
        .await
        .map_err(|err| anyhow!("Calendar permission task failed: {err}"))?
        .map_err(|_| anyhow!("Timed out waiting for Calendar permission"))?;
    Ok(current_access())
}

pub fn list_writable_calendars() -> Result<Vec<crate::models::sync::MacosCalendarInfo>> {
    if current_access() != CalendarAccess::FullAccess {
        return Ok(Vec::new());
    }
    with_store(|store| {
        let mut calendars = Vec::new();
        unsafe {
            let found = store.calendarsForEntityType(EKEntityType::Event);
            for index in 0..found.count() {
                let calendar = found.objectAtIndex(index);
                if !calendar.allowsContentModifications() {
                    continue;
                }
                calendars.push(crate::models::sync::MacosCalendarInfo {
                    id: calendar.calendarIdentifier().to_string(),
                    name: calendar.title().to_string(),
                });
            }
        }
        calendars.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(calendars)
    })
}

#[async_trait]
impl CalendarApi for EventKitApi {
    async fn get_event(&self, _calendar_id: &str, event_id: &str) -> Result<Option<RemoteEvent>> {
        let event_id = event_id.to_string();
        if event_id.is_empty() {
            return Ok(None);
        }
        tokio::task::spawn_blocking(move || {
            with_store(|store| unsafe {
                match store.eventWithIdentifier(&NSString::from_str(&event_id)) {
                    Some(event) => remote_from_event(&event).map(Some),
                    None => Ok(None),
                }
            })
        })
        .await
        .map_err(|err| anyhow!(err))?
    }

    async fn insert_event(&self, calendar_id: &str, draft: &EventDraft) -> Result<RemoteEvent> {
        let calendar_id = chosen_calendar_id(calendar_id, &self.calendar_id);
        let calendar_name = self.calendar_name.clone();
        let draft = draft.clone();
        tokio::task::spawn_blocking(move || {
            with_store(|store| unsafe {
                let event = EKEvent::eventWithEventStore(store);
                apply_draft(store, &event, &draft, &calendar_id, &calendar_name, true)?;
                store
                    .saveEvent_span_commit_error(&event, EKSpan::ThisEvent, true)
                    .map_err(|err| anyhow!(err.localizedDescription().to_string()))?;
                remote_from_event(&event)
            })
        })
        .await
        .map_err(|err| anyhow!(err))?
    }

    async fn update_event(
        &self,
        calendar_id: &str,
        event_id: &str,
        draft: &EventDraft,
    ) -> Result<RemoteEvent> {
        let calendar_id = chosen_calendar_id(calendar_id, &self.calendar_id);
        let calendar_name = self.calendar_name.clone();
        let event_id = event_id.to_string();
        let draft = draft.clone();
        tokio::task::spawn_blocking(move || {
            with_store(|store| unsafe {
                let Some(event) = store.eventWithIdentifier(&NSString::from_str(&event_id)) else {
                    return Err(anyhow!("macOS Calendar event no longer exists"));
                };
                apply_draft(store, &event, &draft, &calendar_id, &calendar_name, false)?;
                store
                    .saveEvent_span_commit_error(&event, EKSpan::ThisEvent, true)
                    .map_err(|err| anyhow!(err.localizedDescription().to_string()))?;
                remote_from_event(&event)
            })
        })
        .await
        .map_err(|err| anyhow!(err))?
    }

    async fn list_events(
        &self,
        calendar_id: &str,
        start_unix: i64,
        end_unix: i64,
    ) -> Result<Vec<RemoteEvent>> {
        let calendar_id = chosen_calendar_id(calendar_id, &self.calendar_id);
        let calendar_name = self.calendar_name.clone();
        tokio::task::spawn_blocking(move || {
            with_store(|store| unsafe {
                let calendar = resolve_calendar(store, &calendar_id, &calendar_name)?;
                let start = NSDate::dateWithTimeIntervalSince1970(start_unix as f64);
                let end = NSDate::dateWithTimeIntervalSince1970(end_unix as f64);
                let calendars = NSArray::arrayWithObject(calendar.as_ref());
                let predicate: Retained<NSPredicate> = store
                    .predicateForEventsWithStartDate_endDate_calendars(
                        &start,
                        &end,
                        Some(&calendars),
                    );
                let found = store.eventsMatchingPredicate(&predicate);
                let mut events = Vec::new();
                let mut seen = HashSet::new();
                for index in 0..found.count() {
                    let event = found.objectAtIndex(index);
                    let Some(identifier) = event.eventIdentifier() else {
                        continue;
                    };
                    if identifier.to_string().is_empty() {
                        continue;
                    }
                    let remote = remote_from_event(&event)?;
                    if seen.insert(remote.id.clone()) {
                        events.push(remote);
                    }
                }
                Ok(events)
            })
        })
        .await
        .map_err(|err| anyhow!(err))?
    }
}

fn chosen_calendar_id(requested: &str, fallback: &str) -> String {
    if requested.is_empty() {
        fallback.to_string()
    } else {
        requested.to_string()
    }
}

fn with_store<T>(work: impl FnOnce(&EKEventStore) -> Result<T>) -> Result<T> {
    let store = unsafe { EKEventStore::new() };
    work(&store)
}

fn on_main(work: Box<dyn FnOnce() + Send>) {
    // `dispatch_get_main_queue` is an inline in the Apple SDK, so it is not a
    // linker symbol. `DispatchQueue::main` uses the exported `_dispatch_main_q`.
    DispatchQueue::main().exec_async(move || work());
}

unsafe fn apply_draft(
    store: &EKEventStore,
    event: &EKEvent,
    draft: &EventDraft,
    calendar_id: &str,
    calendar_name: &str,
    assign_calendar: bool,
) -> Result<()> {
    event.setTitle(Some(&NSString::from_str(&draft.summary)));
    event.setNotes(Some(&NSString::from_str(&encode_notes(
        draft.description.as_deref(),
        &draft.task_id,
    ))));
    if let Some(url) = NSURL::URLWithString(&NSString::from_str(&task_url(&draft.task_id))) {
        event.setURL(Some(&url));
    }
    if assign_calendar {
        let calendar = resolve_calendar(store, calendar_id, calendar_name)?;
        event.setCalendar(Some(&calendar));
    }
    match (&draft.start, &draft.end) {
        (EventMoment::Date(start), EventMoment::Date(end)) => {
            event.setAllDay(true);
            let start_date = local_midnight(start)?;
            let end_date = local_midnight(end)?;
            event.setStartDate(Some(&start_date));
            event.setEndDate(Some(&end_date));
        }
        (EventMoment::DateTime(start), EventMoment::DateTime(end)) => {
            event.setAllDay(false);
            let start_date = unix_date(parse_unix(start)?)?;
            let end_date = unix_date(parse_unix(end)?)?;
            event.setStartDate(Some(&start_date));
            event.setEndDate(Some(&end_date));
        }
        _ => return Err(anyhow!("event draft has mixed date bounds")),
    }
    Ok(())
}

unsafe fn resolve_calendar(
    store: &EKEventStore,
    calendar_id: &str,
    calendar_name: &str,
) -> Result<Retained<objc2_event_kit::EKCalendar>> {
    if !calendar_id.is_empty() {
        if let Some(calendar) = store.calendarWithIdentifier(&NSString::from_str(calendar_id)) {
            if calendar.allowsContentModifications() {
                return Ok(calendar);
            }
            return Err(anyhow!("Selected macOS calendar is read-only"));
        }
    }
    let calendars = store.calendarsForEntityType(EKEntityType::Event);
    if !calendar_name.is_empty() {
        for index in 0..calendars.count() {
            let calendar = calendars.objectAtIndex(index);
            if calendar.title().to_string() == calendar_name
                && calendar.allowsContentModifications()
            {
                return Ok(calendar);
            }
        }
    }
    if let Some(calendar) = store.defaultCalendarForNewEvents() {
        if calendar.allowsContentModifications() {
            return Ok(calendar);
        }
    }
    for index in 0..calendars.count() {
        let calendar = calendars.objectAtIndex(index);
        if calendar.allowsContentModifications() {
            return Ok(calendar);
        }
    }
    Err(anyhow!("No writable macOS calendar is available"))
}

unsafe fn remote_from_event(event: &EKEvent) -> Result<RemoteEvent> {
    let id = event
        .eventIdentifier()
        .map(|value| value.to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("macOS Calendar did not return an event identifier"))?;
    let notes = event.notes().map(|value| value.to_string());
    let (description, task_id) = decode_notes(notes.as_deref());
    let updated_unix = event
        .lastModifiedDate()
        .map(|date| date.timeIntervalSince1970() as i64)
        .filter(|value| *value > 0)
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    Ok(RemoteEvent {
        id,
        summary: event.title().to_string(),
        description,
        start_unix: Some(event.startDate().timeIntervalSince1970() as i64),
        end_unix: Some(event.endDate().timeIntervalSince1970() as i64),
        all_day: event.isAllDay(),
        updated_unix,
        task_id,
    })
}

fn local_midnight(date: &str) -> Result<Retained<NSDate>> {
    let naive = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| anyhow!("invalid all-day date"))?
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("invalid all-day date"))?;
    let local = Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| anyhow!("all-day date is ambiguous in the local timezone"))?;
    Ok(NSDate::dateWithTimeIntervalSince1970(
        local.timestamp() as f64
    ))
}

fn unix_date(unix: i64) -> Result<Retained<NSDate>> {
    Ok(NSDate::dateWithTimeIntervalSince1970(unix as f64))
}

fn parse_unix(value: &str) -> Result<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| date.timestamp())
        .map_err(|_| anyhow!("invalid event timestamp"))
}

fn error_text(error: *mut NSError) -> Option<String> {
    if error.is_null() {
        None
    } else {
        Some(unsafe { &*error }.localizedDescription().to_string())
    }
}
