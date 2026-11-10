//! Widget registry, Rust side. A widget is a module with
//!     pub const ID: &str = "<manifest id>";
//!     pub fn start(ctx: &Ctx) { ... }
//! listed once in `widgets!` below. `start` runs once on the UI thread for each enabled
//! widget; it sets up timers and fetch jobs that push data into the widget's Slint global.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::config::Config;
use crate::fetch::Fetcher;
use crate::AppWindow;

macro_rules! widgets {
    ($($module:ident),* $(,)?) => {
        $(mod $module;)*
        const ALL: &[(&str, fn(&Ctx))] = &[$(($module::ID, $module::start)),*];
    };
}

// One module name per line (keeps parallel additions conflict-free).
widgets! {
    clock,
    greetings,
    events,
    weather,
    migraine_risk,
    calendar,
    google_tasks,
    rss_feed,
    reminders,
    spotify,
    polestar,
    adguard,
    system_stats,
    wifi_qr_code,
}

/// What a widget's `start` gets.
pub struct Ctx<'a> {
    pub ui: &'a AppWindow,
    /// This widget's `components.<id>` config slice (Null when absent).
    pub config: &'a Value,
    /// The whole config, for widgets that read another widget's slice.
    pub all: &'a Config,
    pub fetch: &'a Fetcher,
    id: &'static str,
}

impl Ctx<'_> {
    /// This widget's config slice as a serde type; defaults (and a log line without
    /// values) if it does not match. Use `#[serde(default)]` to tolerate missing keys.
    #[allow(dead_code)]
    pub fn config_as<T: DeserializeOwned + Default>(&self) -> T {
        if self.config.is_null() {
            return T::default();
        }
        T::deserialize(self.config).unwrap_or_else(|_| {
            eprintln!(
                "{}: config does not match the expected shape; using defaults",
                self.id
            );
            T::default()
        })
    }
}

pub fn is_known(id: &str) -> bool {
    ALL.iter().any(|(known, _)| *known == id)
}

/// Start every implemented widget that is placed in the layout, once each.
pub fn start_all(ui: &AppWindow, config: &Config, fetch: &Fetcher) {
    for &(id, start) in ALL {
        if config.placements.iter().any(|p| p.id == id) {
            let ctx = Ctx {
                ui,
                config: config.component(id),
                all: config,
                fetch,
                id,
            };
            crate::guard(id, || start(&ctx));
        }
    }
}

/// Set a Slint property only when the value differs, so nothing redraws needlessly.
pub fn set_if_changed<T: PartialEq>(current: T, new: T, set: impl FnOnce(T)) {
    if current != new {
        set(new);
    }
}
