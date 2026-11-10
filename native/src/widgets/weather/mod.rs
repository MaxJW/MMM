//! Weather: port of src/components/weather/ (current conditions, a 3-day forecast and
//! MeteoAlarm warnings). Both upstreams refresh every 30 minutes, as the web app's
//! component and server cache did.

pub mod alerts;
pub mod forecast;
pub mod source;

use chrono::Utc;
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use super::{set_if_changed, Ctx};
use crate::{AppWindow, WeatherAlert, WeatherData, WeatherDay};

pub const ID: &str = "weather";

pub fn start(ctx: &Ctx) {
    let data = ctx.ui.global::<WeatherData>();
    if let Err(message) = source::on_forecast(ctx, show_forecast) {
        data.set_loading(false);
        data.set_error(message.into());
    }
    source::on_alerts(ctx, show_alerts);
}

fn show_forecast(ui: &AppWindow, update: &crate::fetch::Update<forecast::Forecast>) {
    let d = ui.global::<WeatherData>();
    set_if_changed(d.get_loading(), false, |v| d.set_loading(v));
    let view = update.value.as_deref().map(forecast::view);
    let error = match (&view, &update.error) {
        (Some(Ok(_)), _) => String::new(),
        (Some(Err(e)), _) => e.clone(),
        (None, Some(e)) => e.clone(),
        (None, None) => String::new(),
    };
    set_if_changed(d.get_error(), error.into(), |v| d.set_error(v));
    let Some(Ok(v)) = view else { return };
    set_if_changed(d.get_has_data(), true, |b| d.set_has_data(b));
    set_if_changed(d.get_temp(), v.temp.into(), |s| d.set_temp(s));
    set_if_changed(d.get_condition(), v.condition.into(), |s| {
        d.set_condition(s)
    });
    set_if_changed(d.get_icon(), v.icon.into(), |s| d.set_icon(s));
    let days = v
        .days
        .into_iter()
        .map(|day| WeatherDay {
            day: day.day.into(),
            icon: day.icon.into(),
            hi: day.hi.into(),
            lo: day.lo.into(),
        })
        .collect();
    set_model(d.get_days(), days, |m| d.set_days(m));
}

/// Alerts never show an error: the web handler swallowed alert failures (returning
/// none); here the last good list is kept and expired alerts drop out.
fn show_alerts(ui: &AppWindow, update: &crate::fetch::Update<Vec<alerts::Alert>>) {
    let d = ui.global::<WeatherData>();
    let list = update
        .value
        .as_deref()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let rows = alerts::active(list, Utc::now())
        .map(|a| WeatherAlert {
            event: a.event.as_str().into(),
            dates: alerts::dates(a).into(),
            text: alerts::body(a).into(),
            level: alerts::level(a),
        })
        .collect();
    set_model(d.get_alerts(), rows, |m| d.set_alerts(m));
}

/// Replace a model only when its rows differ.
pub fn set_model<T: Clone + PartialEq + 'static>(
    current: ModelRc<T>,
    rows: Vec<T>,
    set: impl FnOnce(ModelRc<T>),
) {
    if !current.iter().eq(rows.iter().cloned()) {
        set(ModelRc::new(VecModel::from(rows)));
    }
}
