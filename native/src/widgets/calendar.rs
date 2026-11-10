//! Upcoming events from Google Calendar: port of src/components/calendar/api.ts and
//! component.svelte. Every 5 minutes: list the user's calendars, fetch the events of
//! each, group them by local day for today and the next four days, and cap the list
//! at `maxEvents` rows (a day without events counts as one row).
//!
//! REST endpoints (the web app uses the googleapis library for the same calls):
//! - calendarList.list: https://developers.google.com/workspace/calendar/api/v3/reference/calendarList/list
//! - events.list: https://developers.google.com/workspace/calendar/api/v3/reference/events/list
//! - event times: https://developers.google.com/workspace/calendar/api/v3/reference/events
//!   (`start.date` "yyyy-mm-dd" for all-day events, else `start.dateTime` RFC 3339)

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Days, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Value;
use slint::{Color, ComponentHandle, ModelRc, SharedString, VecModel};

use super::{set_if_changed, Ctx};
use crate::fetch::{Job, Update};
use crate::google_auth::{self, api_url, list_all, ApiError, AuthError, GoogleAuth};
use crate::{timing, AppWindow, CalendarData, CalendarDay, CalendarEvent};

pub const ID: &str = "calendar";
const API: &str = "https://www.googleapis.com/calendar/v3";
/// Days shown, starting today (the `for (let i = 0; i < 5; i++)` loop in api.ts).
const DAYS: u64 = 5;
const DEFAULT_MAX_EVENTS: i64 = 12;
/// Event types requested by api.ts (everything except workingLocation).
const EVENT_TYPES: [&str; 5] = ["birthday", "default", "focusTime", "fromGmail", "outOfOffice"];
const MAX_PAGES: usize = 10;

/// `calendarList` entry (fields used by api.ts).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct CalendarEntry {
    id: Option<String>,
    summary: Option<String>,
    deleted: Option<bool>,
    selected: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct ApiEvent {
    summary: Option<String>,
    location: Option<String>,
    start: Option<When>,
    end: Option<When>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct When {
    date: Option<String>,
    date_time: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    title: String,
    /// "9:15 - 9:30", "All day", or for a timed event over several days "From 9:15" /
    /// "All day" / "Until 9:30" (see `time_text`).
    time: String,
    location: Option<String>,
    all_day: bool,
    /// 0xRRGGBB timeline bar colour.
    color: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    date: u32,
    /// "Tue"
    day: String,
    events: Vec<Event>,
}

/// What a successful fetch yields.
#[derive(Debug, PartialEq)]
pub enum View {
    /// No usable Google tokens: the web component shows "Connect Google Calendar".
    NotAuthenticated,
    Days(Vec<Day>),
}

/// `calendarColors` as (calendar name, Tailwind colour class) pairs: the array format
/// `[{calendarName, colorClass}]` or the legacy object `{name: class}`.
pub fn parse_colors(v: &Value) -> Vec<(String, String)> {
    let s = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_string);
    match v {
        Value::Array(list) => list
            .iter()
            .filter_map(|e| Some((s(e.get("calendarName"))?, s(e.get("colorClass")).unwrap_or_default())))
            .collect(),
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| (k.clone(), v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())))
            .collect(),
        _ => Vec::new(),
    }
}

/// getColorClass() in api.ts: exact name, then case-insensitive, then a guess from the
/// name, else gray-400.
pub fn color_class(colors: &[(String, String)], calendar: Option<&str>) -> String {
    let Some(name) = calendar.filter(|n| !n.is_empty()) else {
        return "gray-400".into();
    };
    let lower = name.to_lowercase();
    if let Some((_, c)) = colors
        .iter()
        .find(|(n, _)| n == name)
        .or_else(|| colors.iter().find(|(n, _)| n.to_lowercase() == lower))
    {
        return c.clone();
    }
    let guess = if lower.contains("work") || lower.contains("business") {
        "blue-500"
    } else if lower.contains("personal") || lower.contains("private") {
        "green-500"
    } else if lower.contains("family") {
        "purple-500"
    } else {
        "gray-400"
    };
    guess.into()
}

/// colorClassMap in component.svelte, as Tailwind v4.1 sRGB values (its oklch colours
/// converted the way Chromium draws them). Unknown classes fall back to gray-400.
pub fn bar_color(class: &str) -> u32 {
    match class {
        "gray-500" => 0x6a7282,
        "green-500" => 0x00c950,
        "green-600" => 0x00a63e,
        "blue-500" => 0x2b7fff,
        "blue-600" => 0x155dfc,
        "purple-500" => 0xad46ff,
        "purple-600" => 0x9810fa,
        "red-500" => 0xfb2c36,
        "red-600" => 0xe7000b,
        "yellow-500" => 0xf0b100,
        "yellow-600" => 0xd08700,
        "orange-500" => 0xff6900,
        "orange-600" => 0xf54900,
        "pink-500" => 0xf6339a,
        "pink-600" => 0xe60076,
        "indigo-500" => 0x615fff,
        "indigo-600" => 0x4f39f6,
        "teal-500" => 0x00bba7,
        "teal-600" => 0x009689,
        "cyan-500" => 0x00b8db,
        "cyan-600" => 0x0092b8,
        _ => 0x99a1af, // gray-400
    }
}

/// Text as an HTML page shows it (CSS `white-space: normal`): runs of ASCII whitespace
/// become one space and leading/trailing whitespace goes. Event titles such as
/// " Stand-up" would otherwise be drawn indented.
pub fn collapse_spaces(text: &str) -> String {
    text.split([' ', '\t', '\n', '\r', '\x0c']).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ")
}

/// extractBaseUrl() in api.ts: a URL-looking location becomes its host name
/// ("https://meet.google.com/abc" -> "meet.google.com"); anything else is cut at the
/// first comma. Hand-written equivalent of its regex
/// `^([a-z0-9]([a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}(:\d+)?(/|\?|#|$)` (case-insensitive).
pub fn short_location(location: &str) -> String {
    let trimmed = location.trim();
    let lower = trimmed.to_ascii_lowercase();
    let rest = ["https://", "http://"]
        .iter()
        .find(|p| lower.starts_with(*p))
        .map_or(trimmed, |p| &trimmed[p.len()..]);
    let head = &rest[..rest.find(['/', '?', '#']).unwrap_or(rest.len())];
    let (host, port) = match head.split_once(':') {
        Some((h, p)) => (h, Some(p)),
        None => (head, None),
    };
    let port_ok = port.is_none_or(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    let labels: Vec<&str> = host.split('.').collect();
    let label_ok = |l: &&str| {
        let b = l.as_bytes();
        !b.is_empty()
            && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
            && b[0] != b'-'
            && b[b.len() - 1] != b'-'
    };
    let is_url = port_ok
        && labels.len() >= 2
        && labels[..labels.len() - 1].iter().all(label_ok)
        && labels[labels.len() - 1].len() >= 2
        && labels[labels.len() - 1].bytes().all(|c| c.is_ascii_alphabetic());
    if is_url {
        host.to_string()
    } else {
        trimmed.split(',').next().unwrap_or("").to_string()
    }
}

/// An RFC 3339 `dateTime` in `tz`. A value without an offset (allowed by the API when
/// `timeZone` is given) is read as local time, as dayjs does.
fn instant<Tz: TimeZone>(s: &str, tz: &Tz) -> Option<DateTime<Tz>> {
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Some(t.with_timezone(tz));
    }
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok()?;
    tz.from_local_datetime(&naive).earliest()
}

/// Local midnight starting `date` (the next existing instant if a DST change skips it).
fn midnight<Tz: TimeZone>(date: NaiveDate, tz: &Tz) -> Option<DateTime<Tz>> {
    let t = date.and_hms_opt(0, 0, 0)?;
    (0..=3).find_map(|h| tz.from_local_datetime(&(t + chrono::Duration::hours(h))).earliest())
}

/// `timeMin` and `timeMax` for events.list: from now until the end of the last day
/// shown. (api.ts used now + 3 days, which left day 4 always empty and cut day 3 short.)
/// `timeMin` bounds an event's *end* time, so events that started earlier and are still
/// running are returned too
/// (https://developers.google.com/workspace/calendar/api/v3/reference/events/list).
pub fn window<Tz: TimeZone>(now: &DateTime<Tz>) -> (String, String) {
    let fmt = |t: DateTime<Utc>| t.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let end = now
        .date_naive()
        .checked_add_days(Days::new(DAYS))
        .and_then(|d| midnight(d, &now.timezone()))
        .map(|t| t.with_timezone(&Utc))
        .unwrap_or_else(|| now.with_timezone(&Utc) + chrono::Duration::days(DAYS as i64));
    (fmt(now.with_timezone(&Utc)), fmt(end))
}

/// When an event happens, in the display zone.
#[derive(Clone, Debug)]
enum Span {
    /// All-day: the covered dates come from `start.date` / `end.date` alone.
    AllDay,
    /// Timed, with local start and (if given) end.
    Timed { start: NaiveDateTime, end: Option<NaiveDateTime> },
}

/// One event before it is placed on days: shown on every date in `first..=last`.
struct Placed {
    sort: i64,
    first: NaiveDate,
    last: NaiveDate,
    span: Span,
    event: Event,
}

/// The last local date a timed event covers. It ends *before* `end` (`end` is
/// exclusive, https://developers.google.com/workspace/calendar/api/v3/reference/events),
/// so an event ending exactly at midnight does not reach into the next day.
fn last_timed_day(start: NaiveDateTime, end: NaiveDateTime) -> NaiveDate {
    if end <= start {
        return start.date();
    }
    let d = end.date();
    if end.time() == chrono::NaiveTime::MIN && d > start.date() {
        d.pred_opt().unwrap_or(d)
    } else {
        d
    }
}

/// Time text for `span` on `date`. One-day timed events: "9:15 - 9:30". Timed events
/// over several days: "From 23:30" on the first day, "Until 0:30" on the last and
/// "All day" on the days in between. All-day events (and timed ones with no end, as
/// before): "All day". The flag says whether the row gets the all-day icon.
fn time_text(span: &Span, date: NaiveDate, first: NaiveDate, last: NaiveDate) -> (String, bool) {
    let hm = |t: &NaiveDateTime| t.format("%-H:%M").to_string();
    match span {
        Span::AllDay | Span::Timed { end: None, .. } => ("All day".into(), matches!(span, Span::AllDay)),
        Span::Timed { start, end: Some(end) } if first == last => (format!("{} - {}", hm(start), hm(end)), false),
        Span::Timed { start, .. } if date == first => (format!("From {}", hm(start)), false),
        Span::Timed { end: Some(end), .. } if date == last => (format!("Until {}", hm(end)), false),
        Span::Timed { .. } => ("All day".into(), true),
    }
}

/// The day list shown by the widget, from events of all calendars (each paired with
/// its calendar's name). Mirrors the grouping and `maxEvents` logic of api.ts, except
/// that an event lasting several days is listed on each day it covers (api.ts listed
/// it only under its start date, so one that began before today was missing).
pub fn build_days<Tz: TimeZone>(
    events: &[(ApiEvent, Option<String>)],
    colors: &[(String, String)],
    max_events: i64,
    now: &DateTime<Tz>,
) -> Vec<Day> {
    let tz = now.timezone();
    let mut list: Vec<Placed> = events
        .iter()
        .filter_map(|(e, calendar)| {
            let start = e.start.as_ref()?;
            let timed = start.date_time.as_deref().filter(|s| !s.is_empty());
            let (sort, first, last, span) = match timed {
                Some(s) => {
                    let t = instant(s, &tz)?;
                    let end = e.end.as_ref().and_then(|w| instant(w.date_time.as_deref()?, &tz));
                    let (start, end) = (t.naive_local(), end.map(|e| e.naive_local()));
                    let last = end.map_or(start.date(), |end| last_timed_day(start, end));
                    (t.timestamp_millis(), start.date(), last, Span::Timed { start, end })
                }
                // An all-day date is a calendar date: no time zone conversion. Its
                // `end.date` is exclusive (the `end` property is "The (exclusive) end
                // time of the event", Events resource docs above), so a one-day event
                // on the 10th ends on the 11th. A missing or non-increasing end means
                // one day.
                None => {
                    let date = |w: &When| NaiveDate::parse_from_str(w.date.as_deref()?.get(..10)?, "%Y-%m-%d").ok();
                    let d = date(start)?;
                    let last = e
                        .end
                        .as_ref()
                        .and_then(date)
                        .filter(|end| *end > d)
                        .and_then(|end| end.pred_opt())
                        .unwrap_or(d);
                    (midnight(d, &tz).map_or(i64::MIN, |t| t.timestamp_millis()), d, last, Span::AllDay)
                }
            };
            let location = e.location.as_deref().map(short_location).filter(|l| !l.is_empty());
            let title = e.summary.as_deref().filter(|s| !s.is_empty()).map_or_else(|| "(No title)".into(), collapse_spaces);
            let color = bar_color(&color_class(colors, calendar.as_deref()));
            let all_day = matches!(span, Span::AllDay);
            let event = Event { title, time: String::new(), location, all_day, color };
            Some(Placed { sort, first, last, span, event })
        })
        .collect();
    list.sort_by_key(|p| p.sort); // stable, like Array.sort

    let mut days = Vec::new();
    let mut count: i64 = 0;
    let today = now.date_naive();
    for i in 0..DAYS {
        let Some(date) = today.checked_add_days(Days::new(i)) else { break };
        let remaining = max_events - count;
        if remaining <= 0 {
            break;
        }
        let events: Vec<Event> = list
            .iter()
            .filter(|p| p.first <= date && date <= p.last)
            .take(remaining as usize)
            .map(|p| {
                let (time, all_day) = time_text(&p.span, date, p.first, p.last);
                Event { time, all_day, ..p.event.clone() }
            })
            .collect();
        // A day without events still takes one row ("No events").
        count += (events.len() as i64).max(1);
        days.push(Day {
            date: chrono::Datelike::day(&date),
            day: date.format("%a").to_string(),
            events,
        });
    }
    days
}

pub fn included(c: &CalendarEntry) -> bool {
    // Only calendars ticked in Google Calendar. `selected` is "Whether the calendar
    // content shows up in the calendar UI. Optional. The default is False."
    // (https://developers.google.com/workspace/calendar/api/v3/reference/calendarList),
    // so a missing value counts as unticked. (api.ts used `selected !== false` and so
    // also showed entries without the field.)
    !c.deleted.unwrap_or(false) && c.selected == Some(true) && c.id.as_deref().is_some_and(|id| !id.is_empty())
}

/// Events requested for one calendar.
fn events_query(time_min: &str, time_max: &str) -> Vec<(&'static str, String)> {
    let mut q = vec![
        ("timeMin", time_min.to_string()),
        ("timeMax", time_max.to_string()),
        ("singleEvents", "true".into()),
        ("orderBy", "startTime".into()),
    ];
    // Repeated parameter, one value each (as documented for eventTypes).
    q.extend(EVENT_TYPES.iter().map(|t| ("eventTypes", t.to_string())));
    q
}

async fn load(
    auth: &GoogleAuth,
    http: &reqwest::Client,
    colors: &[(String, String)],
    max_events: i64,
) -> Result<View, String> {
    let fetched = async {
        let calendars: Vec<CalendarEntry> =
            list_all(auth, http, &api_url(API, &["users", "me", "calendarList"]), &[], MAX_PAGES).await?;
        let now = Local::now();
        let (time_min, time_max) = window(&now);
        let query = events_query(&time_min, &time_max);
        let mut all = Vec::new();
        for cal in calendars.iter().filter(|c| included(c)) {
            let id = cal.id.as_deref().unwrap_or_default();
            let url = api_url(API, &["calendars", id, "events"]);
            let events: Vec<ApiEvent> = list_all(auth, http, &url, &query, MAX_PAGES).await?;
            all.extend(events.into_iter().map(|e| (e, cal.summary.clone())));
        }
        Ok::<_, ApiError>(build_days(&all, colors, max_events, &now))
    };
    match fetched.await {
        Ok(days) => Ok(View::Days(days)),
        Err(ApiError::Auth(AuthError::NotAuthenticated)) => Ok(View::NotAuthenticated),
        Err(e) => Err(e.to_string()),
    }
}

pub fn max_events(config: &Value) -> i64 {
    // `typeof maxEvents === 'number' ? maxEvents : 12`
    config.get("maxEvents").and_then(Value::as_f64).map_or(DEFAULT_MAX_EVENTS, |n| n as i64)
}

/// What the widget shows.
#[derive(Clone, Debug, PartialEq)]
enum Shown {
    Loading,
    Error,
    NotAuthenticated,
    Days(Vec<Day>),
}

fn shown(update: &Update<View>) -> Shown {
    match (&update.value, &update.error) {
        (Some(v), _) => match v.as_ref() {
            View::NotAuthenticated => Shown::NotAuthenticated,
            View::Days(d) => Shown::Days(d.clone()),
        },
        (None, Some(_)) => Shown::Error,
        (None, None) => Shown::Loading,
    }
}

pub fn start(ctx: &Ctx) {
    let auth = google_auth::shared(ctx.all);
    let colors = Arc::new(parse_colors(ctx.config.get("calendarColors").unwrap_or(&Value::Null)));
    let max = max_events(ctx.config);
    let last: Mutex<Option<Shown>> = Mutex::new(None);
    ctx.fetch.spawn(
        // Several requests in a row (calendar list + one per calendar).
        Job::new(ID, timing::FREQUENT).timeout(Duration::from_secs(60)),
        move |http| {
            let (auth, colors) = (auth.clone(), colors.clone());
            async move { load(&auth, &http, &colors, max).await }
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
    let data = ui.global::<CalendarData>();
    let mode = match shown {
        Shown::Loading => "loading",
        Shown::Error => "error",
        Shown::NotAuthenticated => "auth",
        Shown::Days(_) => "days",
    };
    if let Shown::Days(days) = shown {
        data.set_days(ModelRc::new(VecModel::from(days.iter().map(to_slint).collect::<Vec<_>>())));
    }
    set_if_changed(data.get_mode(), SharedString::from(mode), |m| data.set_mode(m));
}

fn to_slint(day: &Day) -> CalendarDay {
    let n = day.events.len();
    let events: Vec<CalendarEvent> = day
        .events
        .iter()
        .enumerate()
        .map(|(i, e)| CalendarEvent {
            title: e.title.as_str().into(),
            time: e.time.as_str().into(),
            location: e.location.as_deref().unwrap_or_default().into(),
            all_day: e.all_day,
            color: Color::from_rgb_u8((e.color >> 16) as u8, (e.color >> 8) as u8, e.color as u8),
            first: i == 0,
            last: i + 1 == n,
        })
        .collect();
    CalendarDay {
        date: day.date.to_string().into(),
        day: day.day.as_str().into(),
        events: ModelRc::new(VecModel::from(events)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;
    use serde_json::json;

    fn tz(hours: i32) -> FixedOffset {
        FixedOffset::east_opt(hours * 3600).unwrap()
    }

    fn at(tz: FixedOffset, s: &str) -> DateTime<FixedOffset> {
        tz.from_local_datetime(&NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()).unwrap()
    }

    /// Events as Google returns them (fixture shaped after the Events resource).
    fn fixture() -> Vec<(ApiEvent, Option<String>)> {
        let personal: Vec<ApiEvent> = serde_json::from_value(json!([
            { "kind": "calendar#event", "id": "a", "status": "confirmed", "summary": "Vet appointment",
              "start": { "date": "2026-11-10" }, "end": { "date": "2026-11-11" }, "eventType": "default" },
            { "id": "b", "summary": "Team meeting", "location": "Example Primary School, High Street, Exampletown AB1 2CD, UK",
              "start": { "dateTime": "2026-11-10T11:00:00Z", "timeZone": "Europe/London" },
              "end": { "dateTime": "2026-11-10T12:00:00Z", "timeZone": "Europe/London" } },
            { "id": "c", "summary": "Late one", "start": { "dateTime": "2026-11-11T23:30:00Z" },
              "end": { "dateTime": "2026-11-12T00:30:00Z" } },
        ]))
        .unwrap();
        let work: Vec<ApiEvent> = serde_json::from_value(json!([
            { "id": "d", "summary": "Daily stand-up",
              "start": { "dateTime": "2026-11-10T09:15:00+00:00" }, "end": { "dateTime": "2026-11-10T09:30:00+00:00" } },
            { "id": "e", "summary": "Demo day ✨", "location": "https://meet.example.com/room/42",
              "start": { "dateTime": "2026-11-10T06:00:00-08:00" }, "end": { "dateTime": "2026-11-10T06:15:00-08:00" } },
            { "id": "f", "start": { "dateTime": "2026-11-12T10:00:00Z" }, "end": { "dateTime": "2026-11-12T11:00:00Z" } },
        ]))
        .unwrap();
        personal
            .into_iter()
            .map(|e| (e, Some("Personal".to_string())))
            .chain(work.into_iter().map(|e| (e, Some("Work Stuff".to_string()))))
            .collect()
    }

    fn colors() -> Vec<(String, String)> {
        parse_colors(&json!([
            { "calendarName": "personal", "colorClass": "green-500" },
            { "calendarName": "Work Stuff", "colorClass": "blue-600" },
        ]))
    }

    fn summary(days: &[Day]) -> Vec<(u32, String, Vec<String>)> {
        days.iter()
            .map(|d| (d.date, d.day.clone(), d.events.iter().map(|e| format!("{} | {}", e.title, e.time)).collect()))
            .collect()
    }

    #[test]
    fn groups_by_local_day_and_sorts_across_calendars() {
        let now = at(tz(0), "2026-11-10 08:00");
        let days = build_days(&fixture(), &colors(), 12, &now);
        assert_eq!(
            summary(&days),
            vec![
                (
                    10,
                    "Tue".into(),
                    vec![
                        "Vet appointment | All day".into(),
                        "Daily stand-up | 9:15 - 9:30".into(),
                        "Team meeting | 11:00 - 12:00".into(),
                        "Demo day ✨ | 14:00 - 14:15".into(),
                    ]
                ),
                (11, "Wed".into(), vec!["Late one | From 23:30".into()]),
                (12, "Thu".into(), vec!["Late one | Until 0:30".into(), "(No title) | 10:00 - 11:00".into()]),
                (13, "Fri".into(), vec![]),
                (14, "Sat".into(), vec![]),
            ]
        );
        let first = &days[0].events;
        assert!(first[0].all_day && !first[1].all_day);
        assert_eq!(first[0].color, 0x00c950, "case-insensitive colour match");
        assert_eq!(first[1].color, 0x155dfc);
        assert_eq!(first[2].location.as_deref(), Some("Example Primary School"));
        assert_eq!(first[3].location.as_deref(), Some("meet.example.com"));
    }

    #[test]
    fn timed_events_move_with_the_local_zone_but_all_day_dates_do_not() {
        // In UTC+1 the 23:30Z event is on the 12th at 0:30; the all-day event stays on
        // the 10th, and in UTC-8 too (no shift to the 9th).
        let days = build_days(&fixture(), &colors(), 12, &at(tz(1), "2026-11-10 08:00"));
        let s = summary(&days);
        assert_eq!(s[1].2, Vec::<String>::new());
        assert_eq!(s[2].2[0], "Late one | 0:30 - 1:30");
        assert_eq!(s[0].2[1], "Daily stand-up | 10:15 - 10:30");

        let days = build_days(&fixture(), &colors(), 12, &at(tz(-8), "2026-11-10 00:30"));
        let s = summary(&days);
        assert_eq!(s[0].0, 10);
        assert_eq!(s[0].2[0], "Vet appointment | All day");
        assert_eq!(s[0].2[1], "Daily stand-up | 1:15 - 1:30");
        assert_eq!(s[1].2, vec!["Late one | 15:30 - 16:30".to_string()]);
    }

    #[test]
    fn all_day_events_sort_at_local_midnight() {
        let events: Vec<(ApiEvent, Option<String>)> = serde_json::from_value::<Vec<ApiEvent>>(json!([
            { "summary": "timed", "start": { "dateTime": "2026-11-10T00:30:00+01:00" }, "end": { "dateTime": "2026-11-10T01:00:00+01:00" } },
            { "summary": "all day", "start": { "date": "2026-11-10" }, "end": { "date": "2026-11-11" } },
        ]))
        .unwrap()
        .into_iter()
        .map(|e| (e, None))
        .collect();
        let days = build_days(&events, &[], 12, &at(tz(1), "2026-11-10 00:00"));
        assert_eq!(days[0].events[0].title, "all day");
        assert_eq!(days[0].events[1].color, 0x99a1af, "no calendar name: gray-400");
    }

    #[test]
    fn max_events_counts_empty_days_as_one_row() {
        let now = at(tz(0), "2026-11-10 08:00");
        let rows = |max| summary(&build_days(&fixture(), &colors(), max, &now));
        // 4 events today: 3 fit, nothing after.
        assert_eq!(rows(3).len(), 1);
        assert_eq!(rows(3)[0].2.len(), 3);
        // 4 + 1 + 2 = 7 rows (the midnight-crossing event counts on both days), then
        // the empty Friday takes the 8th.
        assert_eq!(rows(7).len(), 3);
        let r = rows(8);
        assert_eq!(r.len(), 4);
        assert!(r[3].2.is_empty());
        assert!(rows(0).is_empty());
        assert_eq!(rows(100).len(), 5);
    }

    #[test]
    fn no_events_shows_five_empty_days() {
        let days = build_days(&[], &[], 12, &at(tz(0), "2026-12-30 08:00"));
        let dates: Vec<_> = days.iter().map(|d| (d.date, d.day.as_str())).collect();
        assert_eq!(dates, [(30, "Wed"), (31, "Thu"), (1, "Fri"), (2, "Sat"), (3, "Sun")]);
    }

    #[test]
    fn request_window_covers_the_five_days_shown() {
        let (min, max) = window(&at(tz(1), "2026-11-10 09:37"));
        assert_eq!(min, "2026-11-10T08:37:00Z");
        assert_eq!(max, "2026-11-14T23:00:00Z"); // local midnight starting the 15th
        let q = events_query(&min, &max);
        assert_eq!(q.iter().filter(|(k, _)| *k == "eventTypes").count(), 5);
        assert!(q.contains(&("orderBy", "startTime".into())) && q.contains(&("singleEvents", "true".into())));
    }

    #[test]
    fn dst_change_inside_the_window() {
        // Europe/London on 2026-03-29: clocks go forward at 01:00 UTC. Model the two
        // sides with fixed offsets: the event's own offset decides the instant, the
        // display zone decides the shown time.
        let e: Vec<(ApiEvent, Option<String>)> = vec![(
            serde_json::from_value(json!({ "summary": "x",
                "start": { "dateTime": "2026-03-30T09:00:00+01:00" }, "end": { "dateTime": "2026-03-30T10:00:00+01:00" } }))
            .unwrap(),
            None,
        )];
        let days = build_days(&e, &[], 12, &at(tz(1), "2026-03-29 12:00"));
        assert_eq!(summary(&days)[1].2, vec!["x | 9:00 - 10:00".to_string()]);
    }

    #[test]
    fn calendar_list_filter() {
        let list: Vec<CalendarEntry> = serde_json::from_value(json!([
            { "kind": "calendar#calendarListEntry", "id": "me@example.com", "summary": "Personal", "selected": true, "primary": true },
            { "id": "hidden@example.com", "summary": "Off", "selected": false },
            { "id": "gone@example.com", "deleted": true },
            { "id": "en.uk#holiday@group.v.calendar.google.com", "summary": "Holidays" },
            { "id": "shared@example.com", "summary": "Shared", "selected": true },
            { "summary": "no id" },
        ]))
        .unwrap();
        let ids: Vec<_> = list.iter().filter(|c| included(c)).filter_map(|c| c.id.as_deref()).collect();
        // Without `selected` (documented default false) a calendar is unticked: left out.
        assert_eq!(ids, ["me@example.com", "shared@example.com"]);
    }

    #[test]
    fn colour_rules() {
        let c = parse_colors(&json!({ "Family": "pink-600" }));
        assert_eq!(color_class(&c, Some("Family")), "pink-600");
        assert_eq!(color_class(&[], Some("My Work")), "blue-500");
        assert_eq!(color_class(&[], Some("Private stuff")), "green-500");
        assert_eq!(color_class(&[], Some("The family")), "purple-500");
        assert_eq!(color_class(&[], Some("Other")), "gray-400");
        assert_eq!(color_class(&[], None), "gray-400");
        assert_eq!(bar_color("magenta-500"), 0x99a1af);
        assert_eq!(bar_color(""), 0x99a1af);
        // Entries without a name are ignored instead of breaking the whole list.
        assert_eq!(parse_colors(&json!([{ "colorClass": "red-500" }, "x"])).len(), 0);
    }

    #[test]
    fn titles_collapse_whitespace_like_html() {
        assert_eq!(collapse_spaces(" Daily stand-up"), "Daily stand-up");
        assert_eq!(collapse_spaces("a \t\n b  "), "a b");
        assert_eq!(collapse_spaces("no\u{a0}break"), "no\u{a0}break"); // NBSP is not collapsed
        let e: Vec<(ApiEvent, Option<String>)> = vec![(
            serde_json::from_value(json!({ "summary": "  Padded  title ", "start": { "date": "2026-11-10" } })).unwrap(),
            None,
        )];
        let days = build_days(&e, &[], 12, &at(tz(0), "2026-11-10 08:00"));
        assert_eq!(days[0].events[0].title, "Padded title");
    }

    #[test]
    fn locations() {
        assert_eq!(short_location("https://meet.google.com/abc-defg-hij"), "meet.google.com");
        assert_eq!(short_location("HTTP://Example.COM:8080/x"), "Example.COM");
        assert_eq!(short_location("zoom.us"), "zoom.us");
        assert_eq!(short_location("meet.example.com?room=1"), "meet.example.com");
        assert_eq!(short_location(" Example Hospital, Example Road, Exampletown "), "Example Hospital");
        assert_eq!(short_location("St. Andrews, Fife"), "St. Andrews");
        assert_eq!(short_location("10.0.0.1"), "10.0.0.1"); // numeric TLD: not a URL, no comma
        assert_eq!(short_location("example.com:abc"), "example.com:abc");
        assert_eq!(short_location("-bad.com"), "-bad.com");
    }

    fn events(v: Value) -> Vec<(ApiEvent, Option<String>)> {
        serde_json::from_value::<Vec<ApiEvent>>(v).unwrap().into_iter().map(|e| (e, None)).collect()
    }

    #[test]
    fn all_day_events_cover_each_day_up_to_the_exclusive_end_date() {
        let e = events(json!([
            // Started two days before today; end.date 12th is exclusive: shown 10th, 11th.
            { "summary": "Holiday", "start": { "date": "2026-11-08" }, "end": { "date": "2026-11-12" } },
            // One day: the end date is the next day and is not shown.
            { "summary": "Day out", "start": { "date": "2026-11-11" }, "end": { "date": "2026-11-12" } },
            // Missing or non-increasing end: one day.
            { "summary": "No end", "start": { "date": "2026-11-13" } },
            { "summary": "Same end", "start": { "date": "2026-11-14" }, "end": { "date": "2026-11-14" } },
            // Entirely before today: not shown.
            { "summary": "Past", "start": { "date": "2026-11-01" }, "end": { "date": "2026-11-10" } },
        ]));
        let s = summary(&build_days(&e, &[], 12, &at(tz(0), "2026-11-10 08:00")));
        let titles: Vec<Vec<&str>> =
            s.iter().map(|d| d.2.iter().map(|t| t.split(" | ").next().unwrap()).collect()).collect();
        assert_eq!(
            titles,
            vec![vec!["Holiday"], vec!["Holiday", "Day out"], vec![], vec!["No end"], vec!["Same end"]]
        );
        assert!(s.iter().flat_map(|d| &d.2).all(|t| t.ends_with("| All day")));
    }

    #[test]
    fn timed_events_over_several_days() {
        let e = events(json!([
            // Began yesterday evening, ends this afternoon.
            { "summary": "Night shift", "start": { "dateTime": "2026-11-09T20:00:00Z" }, "end": { "dateTime": "2026-11-10T15:00:00Z" } },
            // Four days: first, two middle days, last.
            { "summary": "Conference", "start": { "dateTime": "2026-11-11T09:30:00Z" }, "end": { "dateTime": "2026-11-14T17:00:00Z" } },
            // Ends exactly at midnight: one day only.
            { "summary": "Party", "start": { "dateTime": "2026-11-12T20:00:00Z" }, "end": { "dateTime": "2026-11-13T00:00:00Z" } },
        ]));
        let days = build_days(&e, &[], 12, &at(tz(0), "2026-11-10 08:00"));
        assert_eq!(
            summary(&days).into_iter().map(|d| d.2).collect::<Vec<_>>(),
            vec![
                vec!["Night shift | Until 15:00".to_string()],
                vec!["Conference | From 9:30".into()],
                vec!["Conference | All day".into(), "Party | 20:00 - 0:00".into()],
                vec!["Conference | All day".into()],
                vec!["Conference | Until 17:00".into()],
            ]
        );
        // Middle days get the all-day icon; first and last days keep the clock.
        assert!(!days[1].events[0].all_day && days[2].events[0].all_day && !days[4].events[0].all_day);
        // The local zone decides the days: in UTC+10 the night shift runs from 6:00 on
        // the 10th to 1:00 on the 11th.
        let s = summary(&build_days(&e, &[], 12, &at(tz(10), "2026-11-10 08:00")));
        assert_eq!(s[0].2[0], "Night shift | From 6:00");
        assert_eq!(s[1].2[0], "Night shift | Until 1:00");
    }

    #[test]
    fn config_values() {
        assert_eq!(max_events(&json!({})), 12);
        assert_eq!(max_events(&json!({ "maxEvents": "5" })), 12);
        assert_eq!(max_events(&json!({ "maxEvents": 8 })), 8);
    }
}
