//! Pirate Weather forecast (Dark Sky compatible JSON), port of `fetchWeather` in
//! src/components/weather/api.ts. One response feeds both weather and migraine-risk.
//!
//! Field meanings are from https://docs.pirateweather.net/en/latest/API/data-blocks/:
//! with `units=si`, temperatures are °C, `pressure` is sea-level hPa, `windSpeed` is m/s;
//! `humidity` and `precipProbability` are 0..1; daily `time` is "midnight of the current
//! day in the current time zone" (the location's), and top-level `offset` is that time
//! zone's offset in hours.

use chrono::{DateTime, Datelike, Local, TimeZone, Utc, Weekday};
use serde::Deserialize;

/// Every field either widget reads; unknown fields are ignored and any field may be
/// missing (the TypeScript `num()` fallbacks are applied by the readers).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Forecast {
    pub offset: Option<f64>,
    pub currently: Option<Point>,
    pub hourly: Block,
    pub daily: Block,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Block {
    pub data: Vec<Point>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Point {
    pub time: Option<f64>,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub temperature: Option<f64>,
    pub pressure: Option<f64>,
    pub humidity: Option<f64>,
    pub wind_speed: Option<f64>,
    pub uv_index: Option<f64>,
    pub temperature_high: Option<f64>,
    pub temperature_low: Option<f64>,
    pub temperature_max: Option<f64>,
    pub temperature_min: Option<f64>,
    pub precip_probability: Option<f64>,
}

/// What the weather widget shows.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub temp: String,
    pub condition: String,
    /// Lucide icon file name (ui/icons/<name>.svg), "" for an unknown icon.
    pub icon: &'static str,
    pub days: Vec<Day>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    pub day: &'static str,
    pub icon: &'static str,
    pub hi: String,
    pub lo: String,
}

/// The weather widget's view of a response. Like the web code (which throws on a
/// missing `currently` block), a response without current conditions is an error.
pub fn view(f: &Forecast) -> Result<View, String> {
    let cur = f
        .currently
        .as_ref()
        .ok_or("Malformed forecast: no current conditions")?;
    let days = f
        .daily
        .data
        .iter()
        .take(3)
        .map(|d| Day {
            day: d.time.map_or("", |t| weekday_name(t, f.offset)),
            icon: icon_file(d.icon.as_deref().unwrap_or_default()),
            hi: degrees(d.temperature_high),
            lo: degrees(d.temperature_low),
        })
        .collect();
    Ok(View {
        temp: format!("{}°C", round_text(cur.temperature)),
        condition: cur.summary.clone().unwrap_or_default(),
        icon: icon_file(cur.icon.as_deref().unwrap_or_default()),
        days,
    })
}

fn degrees(v: Option<f64>) -> String {
    format!("{}°", round_text(v))
}

/// `Math.round(v)` as text; "--" where the web app would have shown NaN.
fn round_text(v: Option<f64>) -> String {
    match v {
        Some(v) if v.is_finite() => format!("{}", js_round(v) as i64),
        _ => "--".into(),
    }
}

/// JavaScript `Math.round`: halves round towards +infinity (-1.5 -> -1, 2.5 -> 3).
pub fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

/// `iconMap` in component.svelte: Pirate Weather icon -> Lucide icon. An unknown icon
/// shows nothing, as `svelte:component this={undefined}` does.
pub fn icon_file(icon: &str) -> &'static str {
    match icon {
        "clear-day" => "sun",
        "clear-night" => "moon",
        "rain" => "cloud-rain",
        "snow" => "snowflake",
        "sleet" | "hail" => "cloud-hail",
        "wind" => "wind",
        "fog" => "cloud-fog",
        "cloudy" => "cloudy",
        "partly-cloudy-day" => "cloud-sun",
        "partly-cloudy-night" => "cloud-moon",
        "thunderstorm" => "cloud-lightning",
        "mixed" => "cloud-sun-rain",
        "none" => "cloud-off",
        _ => "",
    }
}

/// Day name of a daily block. The web app used the server's time zone, which names the
/// wrong day whenever it differs from the location's; the response's own `offset` is
/// used instead (falling back to local time if it is missing).
fn weekday_name(time: f64, offset_hours: Option<f64>) -> &'static str {
    let secs = time as i64;
    let weekday = match offset_hours {
        Some(h) if h.is_finite() => {
            DateTime::<Utc>::from_timestamp(secs + (h * 3600.0) as i64, 0).map(|d| d.weekday())
        }
        _ => Local.timestamp_opt(secs, 0).single().map(|d| d.weekday()),
    };
    match weekday {
        Some(Weekday::Sun) => "Sun",
        Some(Weekday::Mon) => "Mon",
        Some(Weekday::Tue) => "Tue",
        Some(Weekday::Wed) => "Wed",
        Some(Weekday::Thu) => "Thu",
        Some(Weekday::Fri) => "Fri",
        Some(Weekday::Sat) => "Sat",
        None => "",
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const FIXTURE: &str = include_str!("../../../tests/fixtures/pirateweather-si.json");

    pub fn fixture() -> Forecast {
        serde_json::from_str(FIXTURE).unwrap()
    }

    #[test]
    fn parses_fixture_into_view() {
        let v = view(&fixture()).unwrap();
        assert_eq!(v.temp, "4°C");
        assert_eq!(v.condition, "Overcast");
        assert_eq!(v.icon, "cloudy");
        let days: Vec<_> = v
            .days
            .iter()
            .map(|d| (d.day, d.icon, d.hi.as_str(), d.lo.as_str()))
            .collect();
        // 2026-11-10 is a Tuesday; daily times are local midnights (offset 0 in November).
        assert_eq!(
            days,
            [
                ("Tue", "cloudy", "6°", "2°"),
                ("Wed", "sun", "6°", "0°"),
                ("Thu", "sun", "6°", "0°"),
            ]
        );
    }

    #[test]
    fn rounds_like_javascript() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(-0.4), 0.0);
        assert_eq!(round_text(Some(-0.5)), "0", "JS -0 prints as 0");
        assert_eq!(round_text(Some(-2.6)), "-3");
        assert_eq!(round_text(None), "--");
    }

    #[test]
    fn day_name_uses_location_offset() {
        // Midnight BST on Wed 2026-06-10 is 23:00 UTC on Tuesday.
        let t = 1_781_046_000.0;
        assert_eq!(weekday_name(t, Some(1.0)), "Wed");
        assert_eq!(weekday_name(t, Some(0.0)), "Tue");
    }

    #[test]
    fn icon_mapping_and_unknown() {
        assert_eq!(icon_file("partly-cloudy-night"), "cloud-moon");
        assert_eq!(icon_file("none"), "cloud-off");
        assert_eq!(icon_file("mostly-clear-day"), "");
    }

    #[test]
    fn missing_current_block_is_an_error_and_partial_data_is_tolerated() {
        let f: Forecast = serde_json::from_str(r#"{"daily": {"data": [{}]}}"#).unwrap();
        assert!(view(&f).is_err());
        let f: Forecast = serde_json::from_str(
            r#"{"currently": {"temperature": null}, "daily": {"data": [{}]}, "extra": 1}"#,
        )
        .unwrap();
        let v = view(&f).unwrap();
        assert_eq!(v.temp, "--°C");
        assert_eq!(v.days[0].hi, "--°");
        assert!(
            serde_json::from_str::<Forecast>(r#"{"currently": {"temperature": "x"}}"#).is_err()
        );
    }
}
