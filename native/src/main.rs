//! mmm-native: the MMM dashboard as one native Wayland client (see ROADMAP.md).

mod config;
mod fetch;
mod google_auth;
mod layout;
mod qr;
mod timing;
mod widgets;

use std::panic::{catch_unwind, AssertUnwindSafe};

use slint::ComponentHandle;

slint::include_modules!();

/// Colour emoji font on Raspberry Pi OS / Debian (fonts-noto-color-emoji).
const EMOJI_FONT: &str = "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf";

fn main() -> Result<(), slint::PlatformError> {
    prefer_colour_emoji();
    let path = config::path_from_args();
    let (config, error) = match config::Config::load(&path) {
        Ok(c) => (c, None),
        Err(e) => {
            eprintln!("config: {e}; showing the clock only");
            (config::Config::fallback(), Some(e))
        }
    };

    let ui = AppWindow::new()?;
    ui.window().set_fullscreen(true);
    if let Some(e) = &error {
        ui.set_error(format!("Config error: {e}").into());
    }

    let fetcher = fetch::Fetcher::new();
    layout::init(&ui, &config);
    widgets::start_all(&ui, &config, &fetcher);
    start_nudge(&ui);
    let _watch = config::watch(path);

    ui.run()
}

/// Slint's fallback chain is [font-family, sans-serif, system-ui], and fontconfig's
/// sans-serif list on the Pi puts DejaVu Sans / FreeSerif (monochrome ✨ 😀) ahead of
/// Noto Color Emoji. SLINT_FONT_PATH replaces those generic families with the given
/// fonts, so emoji resolve to the colour font. Must run before Slint creates its
/// font collection (i.e. before the first component). An explicit env var wins.
fn prefer_colour_emoji() {
    if std::env::var_os("SLINT_FONT_PATH").is_none() && std::path::Path::new(EMOJI_FONT).exists() {
        std::env::set_var("SLINT_FONT_PATH", EMOJI_FONT);
    }
}

/// Shift everything by 0,1,2,1 px on successive minutes against burn-in (the web
/// app animates the same 2px continuously; a step per minute costs one redraw).
fn start_nudge(ui: &AppWindow) {
    fn update(ui: &AppWindow) {
        let minute = chrono::Timelike::minute(&chrono::Local::now());
        let px = [0.0, 1.0, 2.0, 1.0][minute as usize % 4];
        widgets::set_if_changed(ui.get_nudge(), px, |px| ui.set_nudge(px));
    }
    update(ui);
    let weak = ui.as_weak();
    timing::every_aligned(timing::UI_MINUTE, move || {
        if let Some(ui) = weak.upgrade() {
            update(&ui);
        }
    });
}

/// Run `f`, catching a panic so one widget or job cannot take the display down.
/// The panic message itself is printed by the default panic hook.
pub fn guard(what: &str, f: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(f)).is_err() {
        eprintln!("{what}: panicked; continuing");
    }
}
