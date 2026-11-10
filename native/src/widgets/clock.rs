//! Clock: HH:mm, seconds and the long date, updated just after each wall-clock second.

use chrono::{Local, Timelike};
use slint::{ComponentHandle, SharedString};

use super::{set_if_changed, Ctx};
use crate::{timing, AppWindow, ClockData};

pub const ID: &str = "clock";

pub fn start(ctx: &Ctx) {
    update(ctx.ui);
    let weak = ctx.ui.as_weak();
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
    let now = Local::now();
    let text = |s: String| SharedString::from(s);
    set_if_changed(data.get_seconds(), text(format!("{:02}", now.second())), |s| data.set_seconds(s));
    set_if_changed(data.get_time(), text(now.format("%H:%M").to_string()), |s| data.set_time(s));
    set_if_changed(data.get_date(), text(now.format("%A, %-d %B %Y").to_string()), |s| data.set_date(s));
}
