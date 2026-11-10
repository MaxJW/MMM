//! Tasks due today from Google Tasks: port of src/components/google-tasks/api.ts and
//! component.svelte. Lists open tasks of the default list every 5 minutes, keeps those
//! due today (or undated ones, if enabled), sorted by due date.
//!
//! REST endpoint (the web app uses the googleapis library):
//! - tasks.list: https://developers.google.com/workspace/tasks/reference/rest/v1/tasks/list
//! - Task.due: "Only date information is recorded; the time portion of the timestamp is
//!   discarded" (https://developers.google.com/workspace/tasks/reference/rest/v1/tasks),
//!   so a due value is a date at 00:00 UTC.

use std::sync::{Arc, Mutex};

use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike};
use serde::Deserialize;
use serde_json::Value;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use super::{set_if_changed, Ctx};
use crate::fetch::{Job, Update};
use crate::google_auth::{self, api_url, list_all, ApiError, AuthError, GoogleAuth};
use crate::{timing, AppWindow, GoogleTask, GoogleTasksData};

pub const ID: &str = "google-tasks";
const API: &str = "https://tasks.googleapis.com/tasks/v1";
const DEFAULT_MAX_TASKS: i64 = 20;
const MAX_PAGES: usize = 10;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ApiTask {
    title: Option<String>,
    due: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    title: String,
    /// Due time ("9:30"), empty for date-only tasks (all of them, in practice).
    time: String,
}

#[derive(Debug, PartialEq)]
pub enum View {
    /// No usable tokens, or tokens without the Tasks scope.
    NotAuthenticated,
    Tasks(Vec<Task>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    max_tasks: i64,
    include_undated: bool,
    timezone: String,
}

pub fn settings(config: &Value) -> Settings {
    let undated = config.get("includeTasksWithoutDue");
    Settings {
        // `typeof maxTasks === 'number' ? maxTasks : 20`
        max_tasks: config.get("maxTasks").and_then(Value::as_f64).map_or(DEFAULT_MAX_TASKS, |n| n as i64),
        // undefined, true or "true"
        include_undated: match undated {
            None => true,
            Some(Value::Bool(b)) => *b,
            Some(Value::String(s)) => s == "true",
            Some(_) => false,
        },
        timezone: config.get("timezone").and_then(Value::as_str).unwrap_or("").trim().to_string(),
    }
}

/// "YYYY-MM-DD" part of a due value (isDueToday in api.ts).
fn due_date(due: &str) -> &str {
    match due.split_once('T') {
        Some((d, _)) => d,
        None => due.get(..10).unwrap_or(due),
    }
}

/// A time for a task that has one. Google stores only dates (00:00 UTC), so this is
/// normally empty. (component.svelte checked for local midnight, which showed "1:00"
/// on every task during British Summer Time.)
fn due_time<Tz: TimeZone>(due: &str, tz: &Tz) -> String {
    match DateTime::parse_from_rfc3339(due) {
        Ok(t) if t.to_utc().hour() != 0 || t.to_utc().minute() != 0 => {
            t.with_timezone(tz).naive_local().format("%-H:%M").to_string()
        }
        _ => String::new(),
    }
}

/// The tasks shown: due `today` (or undated, if enabled), dated ones first by due
/// date, at most `max_tasks`.
pub fn select<Tz: TimeZone>(items: &[ApiTask], today: NaiveDate, s: &Settings, tz: &Tz) -> Vec<Task> {
    let today = today.format("%Y-%m-%d").to_string();
    let mut kept: Vec<&ApiTask> = items
        .iter()
        .filter(|t| match t.due.as_deref() {
            Some(due) if !due.is_empty() => due_date(due) == today,
            _ => s.include_undated,
        })
        .collect();
    // Undated last; stable otherwise (Array.sort is stable).
    let key = |t: &ApiTask| match t.due.as_deref().filter(|d| !d.is_empty()) {
        None => (1, 0),
        Some(d) => (0, DateTime::parse_from_rfc3339(d).map_or(i64::MAX, |t| t.timestamp_millis())),
    };
    kept.sort_by_key(|t| key(t));
    kept.into_iter()
        .take(s.max_tasks.max(0) as usize)
        .map(|t| Task {
            title: t.title.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| "(No title)".into()),
            time: t.due.as_deref().map(|d| due_time(d, tz)).unwrap_or_default(),
        })
        .collect()
}

/// Today's date, in the configured IANA zone if there is one (read from the system
/// tzdb, /usr/share/zoneinfo), else the local zone.
fn today(zone: Option<&jiff::tz::TimeZone>) -> NaiveDate {
    let local = Local::now().date_naive();
    let Some(zone) = zone else { return local };
    let d = jiff::Timestamp::now().to_zoned(zone.clone()).date();
    NaiveDate::from_ymd_opt(d.year().into(), d.month() as u32, d.day() as u32).unwrap_or(local)
}

/// tasks.list query: open tasks only, the maximum page size.
const QUERY: [(&str, &str); 2] = [("showCompleted", "false"), ("maxResults", "100")];

async fn load(
    auth: &GoogleAuth,
    http: &reqwest::Client,
    s: &Settings,
    zone: Option<&jiff::tz::TimeZone>,
) -> Result<View, String> {
    let url = api_url(API, &["lists", "@default", "tasks"]);
    let query: Vec<(&str, String)> = QUERY.iter().map(|(k, v)| (*k, v.to_string())).collect();
    match list_all::<ApiTask>(auth, http, &url, &query, MAX_PAGES).await {
        Ok(items) => Ok(View::Tasks(select(&items, today(zone), s, &Local))),
        Err(e) if not_authenticated(&e) => Ok(View::NotAuthenticated),
        Err(e) => Err(e.to_string()),
    }
}

/// api.ts treats a 403 about scopes (Calendar-only tokens) like missing tokens.
fn not_authenticated(e: &ApiError) -> bool {
    match e {
        ApiError::Auth(AuthError::NotAuthenticated) => true,
        ApiError::Status(403, msg) => {
            let m = msg.to_lowercase();
            m.contains("insufficient") || m.contains("scope")
        }
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Shown {
    Loading,
    Error,
    NotAuthenticated,
    Tasks(Vec<Task>),
}

fn shown(update: &Update<View>) -> Shown {
    match (&update.value, &update.error) {
        (Some(v), _) => match v.as_ref() {
            View::NotAuthenticated => Shown::NotAuthenticated,
            View::Tasks(t) => Shown::Tasks(t.clone()),
        },
        (None, Some(_)) => Shown::Error,
        (None, None) => Shown::Loading,
    }
}

pub fn start(ctx: &Ctx) {
    let auth = google_auth::shared(ctx.all);
    let s = Arc::new(settings(ctx.config));
    let zone = match s.timezone.as_str() {
        "" => None,
        name => jiff::tz::TimeZone::get(name)
            .inspect_err(|e| eprintln!("{ID}: unknown timezone ({e}); using the local zone"))
            .ok(),
    };
    let last: Mutex<Option<Shown>> = Mutex::new(None);
    ctx.fetch.spawn(
        Job::new(ID, timing::FREQUENT),
        move |http| {
            let (auth, s, zone) = (auth.clone(), s.clone(), zone.clone());
            async move { load(&auth, &http, &s, zone.as_ref()).await }
        },
        ctx.ui.as_weak(),
        move |ui, update| {
            let now = shown(update);
            let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
            if last.as_ref() != Some(&now) {
                show(ui, &now);
                *last = Some(now);
            }
        },
    );
}

fn show(ui: &AppWindow, shown: &Shown) {
    let data = ui.global::<GoogleTasksData>();
    let mode = match shown {
        Shown::Loading => "loading",
        Shown::Error => "error",
        Shown::NotAuthenticated => "auth",
        Shown::Tasks(t) if t.is_empty() => "empty",
        Shown::Tasks(_) => "tasks",
    };
    if let Shown::Tasks(tasks) = shown {
        let rows: Vec<GoogleTask> = tasks
            .iter()
            .map(|t| GoogleTask {
                title: t.title.as_str().into(),
                time: t.time.as_str().into(),
            })
            .collect();
        data.set_tasks(ModelRc::new(VecModel::from(rows)));
    }
    set_if_changed(data.get_mode(), SharedString::from(mode), |m| data.set_mode(m));
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    use serde_json::json;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// A tasks.list page as documented (Task resource fields).
    fn items() -> Vec<ApiTask> {
        let page: google_auth::Page<ApiTask> = google_auth::parse_api_response(
            200,
            &json!({
                "kind": "tasks#tasks",
                "etag": "\"x\"",
                "items": [
                    { "kind": "tasks#task", "id": "1", "title": "Undated", "status": "needsAction", "position": "0001" },
                    { "id": "2", "title": "Bins out", "due": "2026-11-10T00:00:00.000Z", "status": "needsAction" },
                    { "id": "3", "title": "Tomorrow", "due": "2026-11-11T00:00:00.000Z" },
                    { "id": "4", "title": "", "due": "2026-11-10T00:00:00.000Z" },
                    { "id": "5", "title": "Yesterday", "due": "2026-11-09T00:00:00.000Z" },
                ]
            })
            .to_string(),
        )
        .unwrap();
        page.items
    }

    fn titles(tasks: &[Task]) -> Vec<&str> {
        tasks.iter().map(|t| t.title.as_str()).collect()
    }

    #[test]
    fn due_today_first_then_undated() {
        let s = settings(&json!({}));
        assert_eq!(s, Settings { max_tasks: 20, include_undated: true, timezone: String::new() });
        let tasks = select(&items(), d("2026-11-10"), &s, &Local);
        assert_eq!(titles(&tasks), ["Bins out", "(No title)", "Undated"]);
        assert!(tasks.iter().all(|t| t.time.is_empty()));
    }

    #[test]
    fn options() {
        let s = settings(&json!({ "includeTasksWithoutDue": false, "maxTasks": 1, "timezone": " Europe/London " }));
        assert_eq!(s.timezone, "Europe/London");
        assert_eq!(titles(&select(&items(), d("2026-11-10"), &s, &Local)), ["Bins out"]);
        assert!(settings(&json!({ "includeTasksWithoutDue": "true" })).include_undated);
        assert!(!settings(&json!({ "includeTasksWithoutDue": null })).include_undated);
        assert_eq!(settings(&json!({ "maxTasks": "3" })).max_tasks, 20);
        let none = settings(&json!({ "includeTasksWithoutDue": false }));
        assert!(select(&items(), d("2026-11-12"), &none, &Local).is_empty());
    }

    #[test]
    fn date_only_due_never_shows_a_time() {
        // 00:00Z is 01:00 in BST and 16:00 the day before in PST: still no time.
        for hours in [1, -8, 0] {
            let tz = FixedOffset::east_opt(hours * 3600).unwrap();
            assert_eq!(due_time("2026-06-10T00:00:00.000Z", &tz), "");
        }
        let bst = FixedOffset::east_opt(3600).unwrap();
        assert_eq!(due_time("2026-06-10T08:30:00.000Z", &bst), "9:30");
        assert_eq!(due_time("garbage", &bst), "");
        // The due date is compared as written (a UTC date), not shifted by zone.
        assert_eq!(due_date("2026-11-10T00:00:00.000Z"), "2026-11-10");
        assert_eq!(due_date("2026-11-10"), "2026-11-10");
    }

    #[test]
    fn scope_errors_mean_not_authenticated() {
        let scope = ApiError::Status(403, "Request had insufficient authentication scopes.".into());
        assert!(not_authenticated(&scope));
        assert!(not_authenticated(&ApiError::Auth(AuthError::NotAuthenticated)));
        assert!(!not_authenticated(&ApiError::Status(403, "Tasks API has not been used in project".into())));
        assert!(!not_authenticated(&ApiError::Status(500, String::new())));
        assert!(!not_authenticated(&ApiError::Auth(AuthError::NotConfigured)));
    }

    #[test]
    fn configured_zone_decides_today() {
        // Only checks the lookup works where a tzdb exists (it does on the Pi).
        if let Ok(z) = jiff::tz::TimeZone::get("Pacific/Kiritimati") {
            let there = today(Some(&z));
            let utc = chrono::Utc::now().date_naive();
            assert!(there == utc || there == utc.succ_opt().unwrap(), "UTC+14");
        }
    }
}
