//! Bin collections and custom reminders: port of src/components/reminders/{api.ts,component.svelte}.
//!
//! Bins: Fife Council's Verint form API, in two steps as api.ts does: GET `/api/citizen`
//! returns an anonymous token in its `Authorization` response header; POST
//! `/api/custom?action=powersuite_bin_calendar_collections…` with that header and
//! `{"name":"bin_calendar","data":{"uprn":…}}` returns `data.tab_collections[]`
//! (`{date, colour}`). Shown as "Today:/Tomorrow: Green bin" when the next collection
//! is today or tomorrow. Custom reminders show by day of week and hour range.

use std::cell::RefCell;

use chrono::{Datelike, Local, NaiveDate, Timelike};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value;
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use super::{set_if_changed, Ctx};
use crate::fetch::{self, Job, Update};
use crate::{layout, timing, AppWindow, ReminderRow, RemindersData};

pub const ID: &str = "reminders";
const DEFAULT_ENDPOINT: &str = "https://fife.form.uk.empro.verintcloudservices.com";
const AUTH_PATH: &str = "/api/citizen?preview=false&locale=en";
const COLLECTIONS_PATH: &str = "/api/custom?action=powersuite_bin_calendar_collections&actionedby=bin_calendar&loadform=true&access=citizen&locale=en";

// ---- Custom reminders (pure) ---------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reminder {
    text: String,
    /// File stem of the icon in ui/icons/.
    icon: &'static str,
    /// 0 = Sunday … 6 = Saturday; empty = every day.
    days: Vec<i64>,
    start_hour: Option<f64>,
    end_hour: Option<f64>,
}

/// Manifest icon names (component.svelte `iconMap`) to Lucide files; unknown or
/// missing names fall back to the bell, as `getIcon` does.
fn icon_file(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        "milk" => "milk",
        "calendar" => "calendar",
        "clock" => "clock",
        "home" => "house",
        "shopping-cart" => "shopping-cart",
        "utensils" => "utensils",
        "package" => "package",
        "heart" => "heart",
        "star" => "star",
        "check-circle" => "circle-check",
        "alert-circle" => "circle-alert",
        "trash" => "trash-2",
        _ => "bell",
    }
}

/// `days` is an array of numbers or a comma-separated string ("1,4"), as `parseDays`.
fn parse_days(v: Option<&Value>) -> Vec<i64> {
    match v {
        Some(Value::Array(list)) => list.iter().filter_map(Value::as_i64).collect(),
        Some(Value::String(s)) => s
            .split(',')
            .filter_map(|d| d.trim().parse().ok())
            .filter(|d| (0..=6).contains(d))
            .collect(),
        _ => Vec::new(),
    }
}

/// An hour from a number field; null or an empty/invalid string means "not set".
fn parse_hour(v: Option<&Value>) -> Option<f64> {
    match v {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
}

pub fn parse_reminders(config: &Value) -> Vec<Reminder> {
    let Some(list) = config.get("reminders").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .map(|r| Reminder {
            text: r.get("text").and_then(Value::as_str).unwrap_or_default().to_string(),
            icon: icon_file(r.get("icon").and_then(Value::as_str).unwrap_or_default()),
            days: parse_days(r.get("days")),
            start_hour: parse_hour(r.get("startHour")),
            end_hour: parse_hour(r.get("endHour")),
        })
        .collect()
}

/// component.svelte `checkReminder`: `weekday` 0 = Sunday, end hour exclusive, a start
/// later than the end wraps over midnight (22..6 shows 22:00–05:59).
pub fn is_active(r: &Reminder, weekday: u32, hour: u32) -> bool {
    if !r.days.is_empty() && !r.days.contains(&i64::from(weekday)) {
        return false;
    }
    let h = f64::from(hour);
    match (r.start_hour, r.end_hour) {
        (Some(start), Some(end)) if start <= end => start <= h && h < end,
        (Some(start), Some(end)) => h >= start || h < end,
        (Some(start), None) => h >= start,
        (None, Some(end)) => h < end,
        (None, None) => true,
    }
}

// ---- Bin collections -----------------------------------------------------------------

#[derive(Deserialize)]
struct BinApiResponse {
    data: Option<BinData>,
}

#[derive(Deserialize)]
struct BinData {
    tab_collections: Option<Vec<Collection>>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Collection {
    pub date: String,
    pub colour: String,
}

async fn fetch_collections(http: reqwest::Client, endpoint: String, uprn: String) -> Result<Vec<Collection>, String> {
    // Like api.ts, the status of the token response is not checked: only the header counts.
    let auth = http
        .get(format!("{endpoint}{AUTH_PATH}"))
        .send()
        .await
        .map_err(|e| format!("Authentication failed: {}", fetch::clean(e)))?;
    let token = auth.headers().get(AUTHORIZATION).cloned().ok_or("Authentication failed")?;
    let body = serde_json::json!({ "name": "bin_calendar", "data": { "uprn": uprn } });
    let response: BinApiResponse = fetch::get_json(
        http.post(format!("{endpoint}{COLLECTIONS_PATH}"))
            .header(AUTHORIZATION, token)
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_string()),
    )
    .await?;
    Ok(response.data.and_then(|d| d.tab_collections).unwrap_or_default())
}

/// A collection date. The format of `tab_collections[].date` could not be confirmed
/// without a real UPRN: the Fife portal renders it as "Wednesday, November 12, 2025"
/// (as parsed by UKBinCollectionData's FifeCouncil scraper), and api.ts hands it to
/// dayjs, so ISO dates are accepted too, plus DD/MM/YYYY.
pub fn collection_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.len() > 10 && matches!(s.as_bytes()[10], b'T' | b' ') {
        if let Some(d) = s.get(..10).and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()) {
            return Some(d);
        }
    }
    ["%A, %B %d, %Y", "%B %d, %Y", "%Y-%m-%d", "%d/%m/%Y", "%A %d %B %Y", "%d %B %Y"]
        .iter()
        .find_map(|f| NaiveDate::parse_from_str(s, f).ok())
}

#[derive(Debug, PartialEq)]
pub struct BinLine {
    pub when: &'static str,
    pub bins: String,
}

/// The next collection if it is today or tomorrow: every colour collected that day,
/// "Green bin" / "Blue, Grey bins".
pub fn next_collection(collections: &[Collection], today: NaiveDate) -> Option<BinLine> {
    let date = collections
        .iter()
        .filter_map(|c| collection_date(&c.date))
        .filter(|d| *d >= today)
        .min()?;
    let when = if date == today {
        "Today"
    } else if Some(date) == today.succ_opt() {
        "Tomorrow"
    } else {
        return None;
    };
    let colours: Vec<&str> = collections
        .iter()
        .filter(|c| collection_date(&c.date) == Some(date))
        .map(|c| c.colour.trim())
        .collect();
    let plural = if colours.len() > 1 { "s" } else { "" };
    Some(BinLine {
        when,
        bins: format!("{} bin{plural}", colours.join(", ")),
    })
}

// ---- UI thread -----------------------------------------------------------------------

/// `bin-mode` in ui/widgets/reminders.slint.
const BIN_NONE: i32 = 0;
const BIN_LOADING: i32 = 1;
const BIN_ERROR: i32 = 2;
const BIN_SHOW: i32 = 3;

#[derive(Default)]
struct State {
    reminders: Vec<Reminder>,
    bins_enabled: bool,
    bins: Option<Update<Vec<Collection>>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

pub fn start(ctx: &Ctx) {
    let field = |key: &str| ctx.config.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string();
    let uprn = field("uprn");
    let endpoint = match field("apiEndpoint").trim_end_matches('/') {
        "" => DEFAULT_ENDPOINT.to_string(),
        e => e.to_string(),
    };
    STATE.with_borrow_mut(|s| {
        s.reminders = parse_reminders(ctx.config);
        s.bins_enabled = !uprn.is_empty();
    });
    refresh(ctx.ui);

    if uprn.is_empty() {
        eprintln!("{ID}: no uprn configured; bin collections off");
    } else {
        ctx.fetch.spawn(
            Job::new(ID, timing::INFREQUENT),
            move |http| fetch_collections(http, endpoint.clone(), uprn.clone()),
            ctx.ui.as_weak(),
            |ui, update| {
                STATE.with_borrow_mut(|s| s.bins = Some(update.clone()));
                refresh(ui);
            },
        );
    }

    // Hour ranges and "Today/Tomorrow" follow the clock; checking each minute is cheap.
    let weak = ctx.ui.as_weak();
    timing::every_aligned(timing::UI_MINUTE, move || {
        if let Some(ui) = weak.upgrade() {
            refresh(&ui);
        }
    });
}

fn refresh(ui: &AppWindow) {
    let now = Local::now();
    let d = ui.global::<RemindersData>();
    let (mode, when, bins, rows) = STATE.with_borrow(|s| {
        let (mode, line) = match &s.bins {
            _ if !s.bins_enabled => (BIN_NONE, None),
            None => (BIN_LOADING, None),
            Some(Update { value: Some(list), .. }) => match next_collection(list, now.date_naive()) {
                Some(line) => (BIN_SHOW, Some(line)),
                None => (BIN_NONE, None),
            },
            Some(_) => (BIN_ERROR, None),
        };
        let (when, bins) = line.map(|l| (l.when, l.bins)).unwrap_or_default();
        let weekday = now.weekday().num_days_from_sunday();
        let rows: Vec<ReminderRow> = s
            .reminders
            .iter()
            .filter(|r| is_active(r, weekday, now.hour()))
            .map(|r| ReminderRow {
                text: r.text.as_str().into(),
                icon: r.icon.into(),
            })
            .collect();
        (mode, when, bins, rows)
    });
    set_if_changed(d.get_bin_mode(), mode, |v| d.set_bin_mode(v));
    set_if_changed(d.get_bin_when(), SharedString::from(when), |v| d.set_bin_when(v));
    set_if_changed(d.get_bin_text(), SharedString::from(bins), |v| d.set_bin_text(v));
    let visible = mode != BIN_NONE || !rows.is_empty();
    if !d.get_rows().iter().eq(rows.iter().cloned()) {
        d.set_rows(ModelRc::new(VecModel::from(rows)));
    }
    layout::set_visible(ui, ID, visible);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reminder(v: Value) -> Reminder {
        parse_reminders(&json!({ "reminders": [v] })).remove(0)
    }

    /// (weekday, hour) pairs for a whole week at which `r` shows.
    fn shown(r: &Reminder) -> Vec<(u32, u32)> {
        (0..7).flat_map(|d| (0..24).map(move |h| (d, h))).filter(|&(d, h)| is_active(r, d, h)).collect()
    }

    #[test]
    fn no_rules_means_always() {
        assert_eq!(shown(&reminder(json!({ "text": "x" }))).len(), 7 * 24);
        let blank = reminder(json!({ "text": "x", "days": "", "startHour": null, "endHour": "" }));
        assert_eq!(shown(&blank).len(), 7 * 24);
    }

    #[test]
    fn days_as_string_or_array() {
        let r = reminder(json!({ "text": "bins", "days": "1, 4,9,x" }));
        assert_eq!(r.days, [1, 4]);
        assert!(is_active(&r, 1, 0) && is_active(&r, 4, 23));
        assert!(!is_active(&r, 0, 12) && !is_active(&r, 2, 12));
        let r = reminder(json!({ "text": "bins", "days": [0, 6] }));
        assert!(is_active(&r, 0, 9) && is_active(&r, 6, 9) && !is_active(&r, 3, 9));
        // Only out-of-range values in a string: treated as every day, as parseDays does.
        assert_eq!(shown(&reminder(json!({ "text": "x", "days": "7,8" }))).len(), 7 * 24);
    }

    #[test]
    fn hour_range_end_exclusive() {
        let r = reminder(json!({ "text": "x", "startHour": 18, "endHour": 23 }));
        assert!(!is_active(&r, 2, 17));
        assert!(is_active(&r, 2, 18) && is_active(&r, 2, 22));
        assert!(!is_active(&r, 2, 23));
    }

    #[test]
    fn hour_range_wraps_midnight() {
        let r = reminder(json!({ "text": "x", "startHour": 22, "endHour": 6 }));
        let hours: Vec<u32> = (0..24).filter(|&h| is_active(&r, 3, h)).collect();
        assert_eq!(hours, [0, 1, 2, 3, 4, 5, 22, 23]);
    }

    #[test]
    fn open_ended_hours() {
        let from = reminder(json!({ "text": "x", "startHour": 20 }));
        assert_eq!((0..24).filter(|&h| is_active(&from, 0, h)).count(), 4);
        let until = reminder(json!({ "text": "x", "endHour": "9" }));
        assert_eq!((0..24).filter(|&h| is_active(&until, 0, h)).count(), 9);
        let equal = reminder(json!({ "text": "x", "startHour": 9, "endHour": 9 }));
        assert!(shown(&equal).is_empty(), "start == end is an empty range, as in the web app");
    }

    #[test]
    fn days_and_hours_combine() {
        // "Bring bottles in": Monday and Thursday evenings.
        let r = reminder(json!({ "text": "Bring bottles in", "icon": "Milk", "days": "1,4", "startHour": 17, "endHour": 22 }));
        assert_eq!(r.icon, "milk");
        assert_eq!(shown(&r), (17..22).map(|h| (1, h)).chain((17..22).map(|h| (4, h))).collect::<Vec<_>>());
    }

    #[test]
    fn icons() {
        assert_eq!(reminder(json!({ "text": "x" })).icon, "bell");
        assert_eq!(reminder(json!({ "text": "x", "icon": "nope" })).icon, "bell");
        let names = ["milk", "bell", "calendar", "clock", "home", "shopping-cart", "utensils", "package", "heart", "star", "check-circle", "alert-circle", "trash"];
        for name in names {
            let file = icon_file(name);
            let path = format!("{}/ui/icons/{file}.svg", env!("CARGO_MANIFEST_DIR"));
            assert!(std::path::Path::new(&path).exists(), "{name} -> {file}.svg");
        }
    }

    fn day(m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, d).unwrap()
    }

    /// Shaped like the real response (`{"action":…,"data":{"results_returned":…,
    /// "tab_collections":[…]}}`, confirmed with a dummy UPRN); entries per types.ts.
    const RESPONSE: &str = r#"{
        "action": "powersuite_bin_calendar_collections",
        "actionedby": "bin_calendar",
        "data": { "results_returned": "true", "tab_collections": [
            { "colour": "Green", "date": "Wednesday, November 11, 2026", "type": "x" },
            { "colour": "Blue", "date": "Wednesday, November 18, 2026" },
            { "colour": "Grey", "date": "Wednesday, November 18, 2026" },
            { "colour": "Brown", "date": "Wednesday, November 25, 2026" }
        ]},
        "loadform": true
    }"#;

    fn collections() -> Vec<Collection> {
        let r: BinApiResponse = serde_json::from_str(RESPONSE).unwrap();
        r.data.and_then(|d| d.tab_collections).unwrap()
    }

    #[test]
    fn parses_api_response() {
        let c = collections();
        assert_eq!(c.len(), 4);
        assert_eq!(c[0], Collection { date: "Wednesday, November 11, 2026".into(), colour: "Green".into() });
        let empty: BinApiResponse = serde_json::from_str(r#"{"data":{"results_returned":"false","tab_collections":[]}}"#).unwrap();
        assert!(empty.data.unwrap().tab_collections.unwrap().is_empty());
        let none: BinApiResponse = serde_json::from_str(r#"{"data":null}"#).unwrap();
        assert!(none.data.is_none());
    }

    #[test]
    fn today_tomorrow_or_nothing() {
        let c = collections();
        let line = |m, d| next_collection(&c, day(m, d));
        assert_eq!(line(11, 10), Some(BinLine { when: "Tomorrow", bins: "Green bin".into() }));
        assert_eq!(line(11, 11), Some(BinLine { when: "Today", bins: "Green bin".into() }));
        assert_eq!(line(11, 12), None, "next is a week away");
        assert_eq!(line(11, 17), Some(BinLine { when: "Tomorrow", bins: "Blue, Grey bins".into() }));
        assert_eq!(line(11, 26), None, "nothing left");
        assert_eq!(next_collection(&[], day(11, 10)), None);
    }

    #[test]
    fn past_and_unparseable_entries_are_skipped() {
        let c = vec![
            Collection { date: "2026-11-09".into(), colour: "Old".into() },
            Collection { date: "garbage".into(), colour: "Bad".into() },
            Collection { date: "2026-11-11T00:00:00".into(), colour: "Blue".into() },
            Collection { date: "11/11/2026".into(), colour: "Grey".into() },
        ];
        assert_eq!(next_collection(&c, day(11, 10)), Some(BinLine { when: "Tomorrow", bins: "Blue, Grey bins".into() }));
    }

    #[test]
    fn date_formats() {
        let want = Some(day(11, 2));
        for s in ["Monday, November 2, 2026", "Monday, November 02, 2026", "2026-11-02", "2026-11-02 07:00:00", "02/11/2026", "Monday 2 November 2026", " 2 November 2026 "] {
            assert_eq!(collection_date(s), want, "{s}");
        }
        assert_eq!(collection_date("Tuesday, November 2, 2026"), None, "wrong weekday");
        assert_eq!(collection_date(""), None);
    }
}
