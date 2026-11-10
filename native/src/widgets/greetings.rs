//! Greeting: the active event's `customGreeting` if it has one, else the first
//! configured greeting whose hour range contains the current hour. Port of
//! src/components/greetings/component.svelte; re-evaluated every minute.

use chrono::{Local, Timelike};
use serde_json::Value;
use slint::{ComponentHandle, SharedString};

use super::{events, set_if_changed, Ctx};
use crate::{timing, AppWindow, GreetingsData};

pub const ID: &str = "greetings";

#[derive(Debug, PartialEq)]
struct Greeting {
    text: String,
    start: f64,
    end: f64,
}

const DEFAULTS: [(&str, f64, f64); 3] =
    [("Good morning", 0.0, 12.0), ("Good afternoon", 12.0, 18.0), ("Good evening", 18.0, 24.0)];

/// `greetings` from the config slice; the defaults when absent or empty. Hours may
/// be numbers or numeric strings (the web app compared them loosely).
fn parse(config: &Value) -> Vec<Greeting> {
    let hour = |v: Option<&Value>| match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    };
    let list: Vec<Greeting> = config
        .get("greetings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|g| {
            Some(Greeting { text: g.get("text")?.as_str()?.into(), start: hour(g.get("startHour"))?, end: hour(g.get("endHour"))? })
        })
        .collect();
    if list.is_empty() {
        DEFAULTS.iter().map(|&(t, start, end)| Greeting { text: t.into(), start, end }).collect()
    } else {
        list
    }
}

/// First greeting whose [start, end) contains `hour`; a start after the end wraps
/// past midnight (22..6). Falls back to "Good morning" like the web app.
fn pick(greetings: &[Greeting], hour: u32) -> &str {
    let h = f64::from(hour);
    greetings
        .iter()
        .find(|g| if g.start <= g.end { h >= g.start && h < g.end } else { h >= g.start || h < g.end })
        .map_or(DEFAULTS[0].0, |g| g.text.as_str())
}

pub fn start(ctx: &Ctx) {
    let greetings = parse(ctx.config);
    let events = events::parse(ctx.all.component(events::ID));
    let update = move |ui: &AppWindow| {
        let now = Local::now();
        let text = events::active(&events, now.date_naive())
            .map(|e| e.custom_greeting.as_str())
            .filter(|g| !g.is_empty())
            .unwrap_or_else(|| pick(&greetings, now.hour()));
        let data = ui.global::<GreetingsData>();
        set_if_changed(data.get_text(), SharedString::from(text), |s| data.set_text(s));
    };
    update(ctx.ui);
    let weak = ctx.ui.as_weak();
    timing::every_aligned(timing::UI_MINUTE, move || {
        if let Some(ui) = weak.upgrade() {
            update(&ui);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_when_missing_or_empty() {
        for cfg in [json!(null), json!({}), json!({"greetings": []}), json!({"greetings": [{"text": 1}]})] {
            let g = parse(&cfg);
            assert_eq!([pick(&g, 0), pick(&g, 11), pick(&g, 12), pick(&g, 18), pick(&g, 23)],
                ["Good morning", "Good morning", "Good afternoon", "Good evening", "Good evening"]);
        }
    }

    #[test]
    fn configured_ranges_first_match_and_midnight_wrap() {
        let g = parse(&json!({"greetings": [
            {"text": "Night", "startHour": 22, "endHour": "6"},
            {"text": "Day", "startHour": "6", "endHour": 22},
            {"text": "Shadowed", "startHour": 0, "endHour": 24},
        ]}));
        assert_eq!(pick(&g, 23), "Night");
        assert_eq!(pick(&g, 0), "Night");
        assert_eq!(pick(&g, 5), "Night");
        assert_eq!(pick(&g, 6), "Day");
        assert_eq!(pick(&g, 21), "Day");
        assert_eq!(pick(&g, 22), "Night");
    }

    #[test]
    fn no_match_falls_back_to_good_morning() {
        let g = parse(&json!({"greetings": [{"text": "Lunch", "startHour": 12, "endHour": 13}]}));
        assert_eq!(pick(&g, 12), "Lunch");
        assert_eq!(pick(&g, 9), "Good morning");
    }
}
