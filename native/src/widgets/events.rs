//! Recurring events (Halloween, Christmas, ...): port of src/components/events/api.ts.
//! The first event whose MM-DD range contains today is active; it shows its image,
//! text and QR code here, and its `customGreeting` replaces the greeting.

use chrono::{Datelike, Local, NaiveDate};
use serde_json::Value;
use slint::{ComponentHandle, SharedString};

use super::{set_if_changed, Ctx};
use crate::{layout, qr, timing, AppWindow, EventsData};

pub const ID: &str = "events";
const QR_SIZE: u32 = 240; // size-60

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Event {
    start: (u32, u32), // (month, day)
    end: (u32, u32),
    slug: String,
    text: String,
    qr_link: String,
    pub custom_greeting: String,
}

/// Events from a `components.events` slice. Entries with a bad date are skipped.
pub fn parse(config: &Value) -> Vec<Event> {
    let Some(list) = config.get("events").and_then(Value::as_array) else {
        return Vec::new();
    };
    let field = |e: &Value, key: &str| e.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    list.iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let range = (|| Some((month_day(e.get("start")?.as_str()?)?, month_day(e.get("end")?.as_str()?)?)))();
            let Some((start, end)) = range else {
                eprintln!("events: events[{i}] has no valid start/end (MM-DD); skipped");
                return None;
            };
            Some(Event {
                start,
                end,
                slug: field(e, "eventSlug"),
                text: field(e, "eventText"),
                qr_link: field(e, "qrCodeLink"),
                custom_greeting: field(e, "customGreeting"),
            })
        })
        .collect()
}

/// "MM-DD" (1 or 2 digits each) to (month, day); Feb 29 is allowed.
fn month_day(s: &str) -> Option<(u32, u32)> {
    let (m, d) = s.trim().split_once('-')?;
    let (m, d) = (m.parse().ok()?, d.parse().ok()?);
    NaiveDate::from_ymd_opt(2024, m, d)?; // a leap year, so 02-29 validates
    Some((m, d))
}

/// Inclusive range; a start later than the end wraps over New Year (12-30 .. 01-02).
fn contains(e: &Event, date: NaiveDate) -> bool {
    let today = (date.month(), date.day());
    if e.start <= e.end {
        e.start <= today && today <= e.end
    } else {
        today >= e.start || today <= e.end
    }
}

/// The first event active on `date`.
pub fn active(events: &[Event], date: NaiveDate) -> Option<&Event> {
    events.iter().find(|e| contains(e, date))
}

pub fn start(ctx: &Ctx) {
    let events = parse(ctx.config);
    let mut shown: Option<Option<Event>> = None;
    let mut update = move |ui: &AppWindow| {
        let now = active(&events, Local::now().date_naive()).cloned();
        if shown.as_ref() != Some(&now) {
            show(ui, now.as_ref());
            shown = Some(now);
        }
    };
    update(ctx.ui);
    let weak = ctx.ui.as_weak();
    timing::every_aligned(timing::UI_MINUTE, move || {
        if let Some(ui) = weak.upgrade() {
            update(&ui);
        }
    });
}

fn show(ui: &AppWindow, event: Option<&Event>) {
    let data = ui.global::<EventsData>();
    let e = event.cloned().unwrap_or_default();
    set_if_changed(data.get_slug(), SharedString::from(e.slug), |s| data.set_slug(s));
    set_if_changed(data.get_text(), SharedString::from(e.text), |s| data.set_text(s));
    let qr = if e.qr_link.is_empty() { None } else { qr::image(&e.qr_link, QR_SIZE) };
    data.set_has_qr(qr.is_some());
    data.set_qr(qr.unwrap_or_default());
    // Like the web component, an active event occupies its slot even with nothing to show.
    layout::set_visible(ui, ID, event.is_some());
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn d(m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, day).unwrap()
    }

    fn events() -> Vec<Event> {
        parse(&json!({ "events": [
            { "start": "10-31", "end": "11-01", "eventSlug": "halloween", "customGreeting": "🎃" },
            { "start": "12-30", "end": "01-02", "customGreeting": "🎆" },
            { "start": "12-25", "end": "12-26", "customGreeting": "🎄" },
            { "start": "13-01", "end": "13-02", "customGreeting": "bad month" },
            { "start": "nope", "end": "01-01" },
            { "end": "01-01" },
            { "start": "2-29", "end": "3-1", "customGreeting": "leap" },
        ]}))
    }

    fn greeting(date: NaiveDate) -> Option<String> {
        active(&events(), date).map(|e| e.custom_greeting.clone())
    }

    #[test]
    fn invalid_dates_are_skipped() {
        assert_eq!(events().len(), 4);
        assert!(parse(&json!({})).is_empty());
        assert!(parse(&json!({"events": "x"})).is_empty());
    }

    #[test]
    fn range_is_inclusive() {
        assert_eq!(greeting(d(10, 30)), None);
        assert_eq!(greeting(d(10, 31)).as_deref(), Some("🎃"));
        assert_eq!(greeting(d(11, 1)).as_deref(), Some("🎃"));
        assert_eq!(greeting(d(11, 2)), None);
    }

    #[test]
    fn range_crossing_new_year_matches_both_sides() {
        assert_eq!(greeting(d(12, 29)), None);
        assert_eq!(greeting(d(12, 30)).as_deref(), Some("🎆"));
        assert_eq!(greeting(d(12, 31)).as_deref(), Some("🎆"));
        assert_eq!(greeting(d(1, 1)).as_deref(), Some("🎆"));
        assert_eq!(greeting(d(1, 2)).as_deref(), Some("🎆"));
        assert_eq!(greeting(d(1, 3)), None);
    }

    #[test]
    fn first_matching_event_wins() {
        let list = parse(&json!({ "events": [
            { "start": "12-24", "end": "12-26", "eventText": "first" },
            { "start": "12-25", "end": "12-25", "customGreeting": "second" },
        ]}));
        let e = active(&list, d(12, 25)).unwrap();
        assert_eq!(e.text, "first");
        assert_eq!(e.custom_greeting, "", "no fallthrough to later events");
    }

    #[test]
    fn leap_day_range() {
        assert_eq!(greeting(d(2, 28)), None);
        assert_eq!(greeting(d(3, 1)).as_deref(), Some("leap"));
        assert_eq!(greeting(NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()).as_deref(), Some("leap"));
    }
}
