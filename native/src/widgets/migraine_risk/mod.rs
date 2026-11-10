//! Migraine risk: port of src/components/migraine-risk/ (api.ts `buildPayload`, the
//! history, and component.svelte). It uses the weather widget's settings and shares its
//! Pirate Weather and MeteoAlarm jobs (src/widgets/weather/source.rs), so neither
//! upstream is fetched twice. A history reading is recorded per new forecast.

mod history;
mod scoring;

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use chrono::Utc;
use serde_json::Value;
use slint::ComponentHandle;

use super::weather::alerts::{self, Alert};
use super::weather::forecast::{js_round, Forecast};
use super::weather::{set_model, source};
use super::{set_if_changed, Ctx};
use crate::fetch::Update;
use crate::{AppWindow, MigraineData, MigraineFactor};
use history::{History, Reading};
use scoring::*;

pub const ID: &str = "migraine-risk";

const NOT_CONFIGURED: &str =
    "Configure the Weather component with Pirate Weather API key, latitude, and longitude.";

/// `resolveMigraineRiskUiOptions` (config.ts): unset options take their defaults.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Options {
    show_gauge: bool,
    show_breakdown: bool,
    only_contributing: bool,
    show_tomorrow: bool,
    show_air_quality_note: bool,
    show_max_score: bool,
    compact: bool,
}

impl Options {
    fn from(c: &Value) -> Options {
        let not_false = |k: &str| c.get(k) != Some(&Value::Bool(false));
        let is_true = |k: &str| c.get(k) == Some(&Value::Bool(true));
        Options {
            show_gauge: not_false("showGauge"),
            show_breakdown: not_false("showBreakdown"),
            only_contributing: is_true("onlyContributingFactors"),
            show_tomorrow: not_false("showTomorrow"),
            show_air_quality_note: not_false("showAirQualityNote"),
            show_max_score: not_false("showMaxScore"),
            compact: is_true("compactLayout"),
        }
    }
}

/// History-derived inputs, computed when a forecast arrives.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Drops {
    pressure_6h: f64,
    pressure_24h: f64,
    temp_change_6h: f64,
}

#[derive(Debug, PartialEq)]
struct Payload {
    score: i32,
    level: Level,
    factors: FactorBreakdown,
    tomorrow: Option<(i32, Level)>,
}

/// TypeScript `num()`: the number, or the fallback when missing.
fn num(v: Option<f64>, fallback: f64) -> f64 {
    v.filter(|v| !v.is_nan()).unwrap_or(fallback)
}

fn alerts_text<'a>(alerts: impl Iterator<Item = &'a Alert>) -> String {
    alerts
        .map(|a| format!("{} {} {}", a.event, a.title, a.description).to_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Thunder in the current conditions or in the hourly forecast for the next 18 h.
fn has_thunder(data: &Forecast, now_secs: f64) -> bool {
    let cur = data.currently.as_ref();
    if cur.and_then(|c| c.icon.as_deref()) == Some("thunderstorm") {
        return true;
    }
    let summary = cur
        .and_then(|c| c.summary.as_deref())
        .unwrap_or_default()
        .to_lowercase();
    if summary.contains("thunder") {
        return true;
    }
    data.hourly.data.iter().any(|h| {
        let t = num(h.time, now_secs);
        (now_secs..=now_secs + 18.0 * 3600.0).contains(&t)
            && (h.icon.as_deref() == Some("thunderstorm")
                || h.summary
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains("thunder"))
    })
}

/// `buildPayload` of api.ts.
fn build(data: &Forecast, alerts_text: String, drops: Drops, now_secs: f64) -> Payload {
    let cur = data.currently.as_ref();
    let field = |f: fn(&super::weather::forecast::Point) -> Option<f64>| num(cur.and_then(f), 0.0);
    // Pirate Weather gives humidity as 0..1 (see forecast.rs); the web app passed it to
    // a percentage threshold unscaled, so it always scored "dry air". Fixed here.
    let humidity_pct = field(|c| c.humidity) * 100.0;
    let uv_index = field(|c| c.uv_index);
    let factors = compute_factor_breakdown(&FactorInputs {
        pressure_drop_6h: drops.pressure_6h,
        pressure_drop_24h: drops.pressure_24h,
        humidity_pct,
        temp_c: field(|c| c.temperature),
        temp_change_6h: drops.temp_change_6h,
        wind_kmh: wind_mps_to_kmh(field(|c| c.wind_speed)),
        uv_index,
        thunderstorm: ThunderstormInputs {
            alerts_text,
            forecast_text: cur
                .and_then(|c| c.summary.as_deref())
                .unwrap_or_default()
                .to_lowercase(),
            has_thunder_condition: has_thunder(data, now_secs),
        },
        air_quality: AirQuality::default(),
    });
    let score = sum_factors(&factors).min(MAX_SCORE);

    let tomorrow = data.daily.data.get(1).and_then(|d1| {
        let hi = d1.temperature_high.or(d1.temperature_max)?;
        let input = TomorrowForecastInput {
            tmax: num(Some(hi), 0.0),
            tmin: num(d1.temperature_low.or(d1.temperature_min), 0.0),
            uv: num(d1.uv_index, uv_index),
            forecast_text_lower: d1.summary.as_deref().unwrap_or_default().to_lowercase(),
            rain_chance_pct: js_round(num(d1.precip_probability, 0.0) * 100.0),
        };
        let s = tomorrow_forecast_score(Some(&input))?;
        Some((s, tomorrow_risk_level(s)))
    });

    Payload {
        score,
        level: today_risk_level(score),
        factors,
        tomorrow,
    }
}

/// The value arc of the gauge as SVG path commands in the 120x72 view box: a semicircle
/// from (12,60) over the top, `score / 22` of the way to (108,60). "" for a score of 0.
fn gauge_arc(score: i32) -> String {
    let fraction = (score as f64 / MAX_SCORE as f64).clamp(0.0, 1.0);
    if fraction == 0.0 {
        return String::new();
    }
    let angle = std::f64::consts::PI * fraction;
    let (x, y) = (60.0 - 48.0 * angle.cos(), 60.0 - 48.0 * angle.sin());
    format!("M 12 60 A 48 48 0 0 1 {x:.3} {y:.3}")
}

fn level_index(level: Level) -> i32 {
    match level {
        Level::Low => 0,
        Level::Moderate => 1,
        Level::High => 2,
        Level::VeryHigh => 3,
    }
}

struct State {
    history: History,
    options: Options,
    last_forecast: Option<Arc<Forecast>>,
    /// The latest forecast with its history-derived inputs and arrival time (s).
    inputs: Option<(Arc<Forecast>, Drops, f64)>,
    alerts: Option<Arc<Vec<Alert>>>,
    error: Option<String>,
}

pub fn start(ctx: &Ctx) {
    let options = Options::from(ctx.config);
    show_options(ctx.ui, options);
    let state = Rc::new(RefCell::new(State {
        history: History::load(&history_dir()),
        options,
        last_forecast: None,
        inputs: None,
        alerts: None,
        error: None,
    }));

    let s = state.clone();
    let subscribed = source::on_forecast(ctx, move |ui, update: &Update<Forecast>| {
        let mut s = s.borrow_mut();
        if let (Some(f), None) = (&update.value, &update.error) {
            if !s
                .last_forecast
                .as_ref()
                .is_some_and(|last| Arc::ptr_eq(last, f))
            {
                s.last_forecast = Some(f.clone());
                record(&mut s, f.clone());
            }
        }
        s.error = update.error.clone();
        render(ui, &s);
    });
    if subscribed.is_err() {
        let d = ctx.ui.global::<MigraineData>();
        d.set_loading(false);
        d.set_error(NOT_CONFIGURED.into());
        return;
    }
    source::on_alerts(ctx, move |ui, update: &Update<Vec<Alert>>| {
        let mut s = state.borrow_mut();
        s.alerts = update.value.clone();
        render(ui, &s);
    });
}

/// The config file's directory (the web app's `data/`), where the history lives.
fn history_dir() -> PathBuf {
    crate::config::path_from_args()
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// A new forecast: derive the history inputs, then append this reading (GET in api.ts).
fn record(s: &mut State, forecast: Arc<Forecast>) {
    let now = Utc::now().timestamp_millis();
    let cur = forecast.currently.as_ref();
    let pressure = num(cur.and_then(|c| c.pressure), 0.0);
    let temp_c = num(cur.and_then(|c| c.temperature), 0.0);
    let readings = &s.history.readings;
    let valid = pressure > 900.0;
    let drops = Drops {
        pressure_6h: if valid {
            history::pressure_drop_from_peak(readings, pressure, now, 6.0)
        } else {
            0.0
        },
        pressure_24h: if valid {
            history::pressure_drop_from_peak(readings, pressure, now, 24.0)
        } else {
            0.0
        },
        temp_change_6h: history::temperature_change_6h(readings, temp_c, now),
    };
    s.history.append(Reading {
        t: now,
        pressure_hpa: if valid { pressure } else { 0.0 },
        temp_c,
    });
    s.inputs = Some((forecast, drops, now as f64 / 1000.0));
}

fn render(ui: &AppWindow, s: &State) {
    let d = ui.global::<MigraineData>();
    let Some((forecast, drops, at)) = &s.inputs else {
        // No data yet: keep "Loading…" until the first forecast attempt has failed, then
        // show its error (errors only show while there is nothing else to show).
        if let Some(error) = &s.error {
            set_if_changed(d.get_loading(), false, |v| d.set_loading(v));
            set_if_changed(d.get_error(), error.into(), |v| d.set_error(v));
        }
        return;
    };
    set_if_changed(d.get_loading(), false, |v| d.set_loading(v));
    set_if_changed(d.get_error(), "".into(), |v| d.set_error(v));
    let alerts: &[Alert] = s.alerts.as_deref().map(Vec::as_slice).unwrap_or_default();
    let text = alerts_text(alerts::active(alerts, Utc::now()));
    let p = build(forecast, text, *drops, *at);

    set_if_changed(d.get_has_data(), true, |v| d.set_has_data(v));
    set_if_changed(d.get_score(), p.score, |v| d.set_score(v));
    set_if_changed(d.get_level(), level_index(p.level), |v| d.set_level(v));
    set_if_changed(d.get_gauge_arc(), gauge_arc(p.score).into(), |v| {
        d.set_gauge_arc(v)
    });
    let rows = p
        .factors
        .rows()
        .into_iter()
        .filter(|(_, pts)| !s.options.only_contributing || *pts > 0)
        .map(|(label, pts)| MigraineFactor {
            label: label.into(),
            pts,
        })
        .collect();
    set_model(d.get_factors(), rows, |m| d.set_factors(m));
    let (has_tomorrow, t_score, t_level) = match p.tomorrow {
        Some((score, level)) => (true, score, level_index(level)),
        None => (false, 0, 0),
    };
    set_if_changed(d.get_has_tomorrow(), has_tomorrow, |v| {
        d.set_has_tomorrow(v)
    });
    set_if_changed(d.get_tomorrow_score(), t_score, |v| d.set_tomorrow_score(v));
    set_if_changed(d.get_tomorrow_level(), t_level, |v| d.set_tomorrow_level(v));
}

fn show_options(ui: &AppWindow, o: Options) {
    let d = ui.global::<MigraineData>();
    d.set_show_gauge(o.show_gauge);
    d.set_show_breakdown(o.show_breakdown);
    d.set_only_contributing(o.only_contributing);
    d.set_show_tomorrow(o.show_tomorrow);
    d.set_show_air_quality_note(o.show_air_quality_note);
    d.set_show_max_score(o.show_max_score);
    d.set_compact(o.compact);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widgets::weather::forecast::tests::fixture;
    use serde_json::json;

    /// 2026-11-10 09:37 UTC, the fixture's `currently.time`.
    const NOW: f64 = 1_794_303_420.0;

    #[test]
    fn options_default_like_config_ts() {
        let o = Options::from(&Value::Null);
        assert!(
            o.show_gauge
                && o.show_breakdown
                && o.show_tomorrow
                && o.show_air_quality_note
                && o.show_max_score
        );
        assert!(!o.only_contributing && !o.compact);
        let o = Options::from(
            &json!({"showGauge": false, "compactLayout": true, "showTomorrow": "no", "onlyContributingFactors": 1}),
        );
        assert!(!o.show_gauge && o.compact && o.show_tomorrow && !o.only_contributing);
    }

    #[test]
    fn payload_from_fixture() {
        let p = build(&fixture(), String::new(), Drops::default(), NOW);
        // 87% humidity (2) + 4.3 °C (2); 4.21 m/s = 15 km/h, UV 0.4: nothing else.
        assert_eq!(
            p.factors,
            FactorBreakdown {
                humidity: 2,
                temperature: 2,
                ..Default::default()
            }
        );
        assert_eq!((p.score, p.level), (4, Level::Moderate));
        // Tomorrow: max 5.9 / low 0.4 -> 2 (cold), swing 5.5 -> 0, UV 1.2 -> 0, 2% rain.
        assert_eq!(p.tomorrow, Some((2, Level::Low)));
    }

    #[test]
    fn history_drops_and_thunder_alerts_feed_the_score() {
        let drops = Drops {
            pressure_6h: 8.0,
            pressure_24h: 14.0,
            temp_change_6h: 5.0,
        };
        let p = build(&fixture(), "yellow thunderstorm warning".into(), drops, NOW);
        assert_eq!(
            (
                p.factors.pressure_6h,
                p.factors.pressure_24h,
                p.factors.temperature_change
            ),
            (3, 3, 1)
        );
        assert_eq!(p.factors.thunderstorm, 2);
        assert_eq!((p.score, p.level), (4 + 3 + 3 + 1 + 2, Level::VeryHigh));
    }

    #[test]
    fn every_factor_at_its_maximum() {
        // Without air quality the factors sum to at most 19, so the cap of 22 in api.ts
        // never applies in standalone mode.
        let mut f = fixture();
        let cur = f.currently.as_mut().unwrap();
        cur.temperature = Some(35.0);
        cur.wind_speed = Some(20.0);
        cur.uv_index = Some(9.0);
        cur.icon = Some("thunderstorm".into());
        let drops = Drops {
            pressure_6h: 12.0,
            pressure_24h: 20.0,
            temp_change_6h: 9.0,
        };
        let p = build(&f, "thunderstorm".into(), drops, NOW);
        assert_eq!(sum_factors(&p.factors), 4 + 3 + 2 + 2 + 2 + 2 + 2 + 2);
        assert_eq!((p.score, p.level), (19, Level::VeryHigh));
    }

    #[test]
    fn thunder_in_hourly_window_only() {
        let mut f = fixture();
        assert!(!has_thunder(&f, NOW));
        f.hourly.data[2].summary = Some("Thunderstorms".into()); // 10:00, within 18 h
        assert!(has_thunder(&f, NOW));
        f.hourly.data[2].time = Some(NOW + 19.0 * 3600.0);
        assert!(!has_thunder(&f, NOW));
        f.hourly.data[2].time = None; // a missing time counts as now
        assert!(has_thunder(&f, NOW));
        f.hourly.data[2].summary = None;
        f.hourly.data[0].icon = Some("thunderstorm".into()); // 09:00: already past
        assert!(!has_thunder(&f, NOW));
    }

    #[test]
    fn tomorrow_needs_a_high_temperature() {
        let mut f = fixture();
        f.daily.data[1].temperature_high = None;
        assert!(
            build(&f, String::new(), Drops::default(), NOW)
                .tomorrow
                .is_some(),
            "falls back to max"
        );
        f.daily.data[1].temperature_max = None;
        assert_eq!(
            build(&f, String::new(), Drops::default(), NOW).tomorrow,
            None
        );
        f.daily.data.truncate(1);
        assert_eq!(
            build(&f, String::new(), Drops::default(), NOW).tomorrow,
            None
        );
    }

    #[test]
    fn empty_forecast_still_scores() {
        let p = build(&Forecast::default(), String::new(), Drops::default(), NOW);
        // num() fallbacks: 0 °C (cold, 2) and 0% humidity (dry, 1).
        assert_eq!(p.score, 3);
        assert_eq!(p.tomorrow, None);
    }

    #[test]
    fn gauge_arc_ends() {
        assert_eq!(gauge_arc(0), "");
        assert_eq!(gauge_arc(11), "M 12 60 A 48 48 0 0 1 60.000 12.000");
        assert_eq!(gauge_arc(22), "M 12 60 A 48 48 0 0 1 108.000 60.000");
        assert_eq!(gauge_arc(30), gauge_arc(22));
    }

    #[test]
    fn alerts_text_is_lowercased_and_joined() {
        let xml = include_str!("../../../tests/fixtures/meteoalarm-united-kingdom.xml");
        let list = alerts::parse(xml, "Northern Ireland").unwrap();
        assert_eq!(
            alerts_text(list.iter()),
            "yellow fog warning yellow fog warning issued for united kingdom - northern ireland "
        );
    }
}
