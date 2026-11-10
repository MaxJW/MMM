//! The two upstreams shared by weather and migraine-risk: one Pirate Weather job and one
//! MeteoAlarm job, started by whichever widget subscribes first. Updates fan out on the
//! UI thread to every subscriber, so each upstream is fetched once per cadence no matter
//! how many widgets use it. Both read their settings from `components.weather`.

use std::cell::RefCell;
use std::sync::Arc;

use serde_json::Value;
use slint::ComponentHandle;

use super::alerts::{self, Alert};
use super::forecast::Forecast;
use crate::config::Config;
use crate::fetch::{self, Job, Update};
use crate::widgets::Ctx;
use crate::{timing, AppWindow};

/// `components.weather`, with the web app's defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    api_key: String,
    latitude: String,
    longitude: String,
    country: String,
    province: String,
}

impl Settings {
    pub fn from(all: &Config) -> Settings {
        let c = all.component(super::ID);
        let text = |key: &str| match c.get(key) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            _ => String::new(),
        };
        let or = |s: String, default: &str| if s.is_empty() { default.to_string() } else { s };
        Settings {
            api_key: text("apiKey"),
            latitude: text("latitude"),
            longitude: text("longitude"),
            country: or(text("alertsCountry"), "united-kingdom"),
            province: text("alertsProvince"),
        }
    }

    /// The forecast URL (it contains the key: never log it), or the web app's error text.
    /// `hourly` is only needed by migraine-risk; minutely data and Pirate Weather's own
    /// alerts are never used, so they are excluded (`exclude`, see
    /// https://docs.pirateweather.net/en/latest/API/) to keep the response small.
    pub fn forecast_url(&self, hourly: bool) -> Result<String, &'static str> {
        if self.api_key.is_empty() {
            return Err("Missing Pirate Weather API key");
        }
        if self.latitude.is_empty() || self.longitude.is_empty() {
            return Err("Missing latitude or longitude");
        }
        let exclude = if hourly {
            "minutely,alerts"
        } else {
            "minutely,hourly,alerts"
        };
        Ok(format!(
            "https://api.pirateweather.net/forecast/{}/{},{}?units=si&exclude={exclude}",
            self.api_key, self.latitude, self.longitude
        ))
    }

    fn alerts_url(&self) -> String {
        format!(
            "https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-{}",
            self.country
        )
    }
}

type Sub<T> = Box<dyn Fn(&AppWindow, &Update<T>)>;

struct Subscribers<T> {
    started: bool,
    list: Vec<Sub<T>>,
}

impl<T> Default for Subscribers<T> {
    fn default() -> Self {
        Subscribers {
            started: false,
            list: Vec::new(),
        }
    }
}

thread_local! {
    static FORECAST: RefCell<Subscribers<Forecast>> = RefCell::default();
    static ALERTS: RefCell<Subscribers<Vec<Alert>>> = RefCell::default();
}

/// Receive every forecast update. Err (the web app's message) if weather is not
/// configured, in which case nothing is fetched.
pub fn on_forecast(
    ctx: &Ctx,
    f: impl Fn(&AppWindow, &Update<Forecast>) + 'static,
) -> Result<(), &'static str> {
    let hourly = ctx.all.placements.iter().any(|p| p.id == "migraine-risk");
    let url = Settings::from(ctx.all).forecast_url(hourly)?;
    let start = FORECAST.with_borrow_mut(|s| {
        s.list.push(Box::new(f));
        !std::mem::replace(&mut s.started, true)
    });
    if start {
        ctx.fetch.spawn(
            Job::new("weather-forecast", timing::STANDARD),
            move |http| {
                let request = http.get(url.clone());
                async move {
                    get(request)
                        .await?
                        .json::<Forecast>()
                        .await
                        .map_err(fetch::clean)
                }
            },
            ctx.ui.as_weak(),
            |ui, update| FORECAST.with_borrow(|s| s.list.iter().for_each(|f| f(ui, update))),
        );
    }
    Ok(())
}

/// Receive every alerts update (all parsed alerts, expired ones included; use
/// `alerts::active`).
pub fn on_alerts(ctx: &Ctx, f: impl Fn(&AppWindow, &Update<Vec<Alert>>) + 'static) {
    let settings = Settings::from(ctx.all);
    let start = ALERTS.with_borrow_mut(|s| {
        s.list.push(Box::new(f));
        !std::mem::replace(&mut s.started, true)
    });
    if start {
        let url = settings.alerts_url();
        let province = Arc::new(settings.province);
        ctx.fetch.spawn(
            Job::new("weather-alerts", timing::STANDARD),
            move |http| {
                let request = http.get(url.clone());
                let province = province.clone();
                async move {
                    let xml = get(request).await?.text().await.map_err(fetch::clean)?;
                    alerts::parse(&xml, &province)
                }
            },
            ctx.ui.as_weak(),
            |ui, update| ALERTS.with_borrow(|s| s.list.iter().for_each(|f| f(ui, update))),
        );
    }
}

/// Send a request; a non-2xx status becomes the web app's "HTTP error! status: N".
/// Errors never contain the URL (it holds the API key).
async fn get(request: reqwest::RequestBuilder) -> Result<reqwest::Response, String> {
    let response = request.send().await.map_err(fetch::clean)?;
    if !response.status().is_success() {
        return Err(format!(
            "HTTP error! status: {}",
            response.status().as_u16()
        ));
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(json: &str) -> Settings {
        let config = Config::parse(&format!(r#"{{"components": {{"weather": {json}}}}}"#)).unwrap();
        Settings::from(&config)
    }

    #[test]
    fn defaults_and_validation() {
        let s = settings("{}");
        assert_eq!(s.forecast_url(false), Err("Missing Pirate Weather API key"));
        assert_eq!(
            s.alerts_url(),
            "https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-united-kingdom"
        );
        let s = settings(r#"{"apiKey": "k", "latitude": "56.0"}"#);
        assert_eq!(s.forecast_url(false), Err("Missing latitude or longitude"));
        let s = settings(
            r#"{"apiKey": "k", "latitude": 56.5, "longitude": "-3.2", "alertsCountry": "germany"}"#,
        );
        assert_eq!(
            s.forecast_url(true).unwrap(),
            "https://api.pirateweather.net/forecast/k/56.5,-3.2?units=si&exclude=minutely,alerts"
        );
        assert!(s
            .forecast_url(false)
            .unwrap()
            .ends_with("exclude=minutely,hourly,alerts"));
        assert!(s.alerts_url().ends_with("-germany"));
    }
}
