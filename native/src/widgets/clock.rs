//! Clock: HH:mm and the long date. No seconds, so the window is redrawn once a minute
//! rather than every second.

use chrono::{DateTime, TimeZone};
use slint::{ComponentHandle, SharedString};

use super::{set_if_changed, Ctx};
use crate::{timing, AppWindow, ClockData};

pub const ID: &str = "clock";

pub fn start(ctx: &Ctx) {
    update(ctx.ui);
    let weak = ctx.ui.as_weak();
    // Ticks just after each wall-clock second so the minute flips on time (and the
    // watchdog keeps its 1s cadence), but `update` only writes a property whose text
    // changed: the time once a minute, the date once a day.
    timing::every_aligned(timing::REALTIME, move || {
        // systemd watchdog: this tick runs on the UI thread, so a frozen UI stops it.
        crate::sd_notify::watchdog();
        if let Some(ui) = weak.upgrade() {
            update(&ui);
        }
    });
}

fn update(ui: &AppWindow) {
    let data = ui.global::<ClockData>();
    let (time, date) = texts(&chrono::Local::now());
    set_if_changed(data.get_time(), SharedString::from(time), |s| data.set_time(s));
    set_if_changed(data.get_date(), SharedString::from(date), |s| data.set_date(s));
}

/// The displayed time ("HH:MM") and date ("Wednesday, 11 November 2026").
fn texts<Tz: TimeZone>(now: &DateTime<Tz>) -> (String, String)
where
    Tz::Offset: std::fmt::Display,
{
    (now.format("%H:%M").to_string(), now.format("%A, %-d %B %Y").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn at(h: u32, m: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 11, h, m, s).unwrap()
    }

    #[test]
    fn formats_time_without_seconds_and_long_date() {
        let (time, date) = texts(&at(9, 5, 42));
        assert_eq!(time, "09:05");
        assert_eq!(date, "Wednesday, 11 November 2026");
    }

    #[test]
    fn text_only_changes_on_the_minute() {
        // Every second within a minute renders identically, so set_if_changed skips them.
        let first = texts(&at(23, 59, 0));
        for s in 1..60 {
            assert_eq!(texts(&at(23, 59, s)), first);
        }
        let next = texts(&Utc.with_ymd_and_hms(2026, 11, 12, 0, 0, 0).unwrap());
        assert_eq!(next, ("00:00".into(), "Thursday, 12 November 2026".into()));
    }
}
