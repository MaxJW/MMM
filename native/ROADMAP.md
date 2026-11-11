# mmm-native roadmap

Native Rust + Slint replacement for the Chromium kiosk + SvelteKit server. It runs as
a fullscreen Wayland client under the Pi's existing labwc session.

## Phase 0: toolkit spike (DONE, 2026-11-10: go)

Result: Slint 1.18.1 works on the Pi 4 under labwc. Recommended renderer is
winit + femtovg (OpenGL ES via Mesa). Colour emoji render correctly up to 80px. That
covers every emoji size the current dashboard uses (greetings are 48px, calendar
titles 18px). Skia is the fallback if emoji larger than 80px are ever needed.

Measured on the Pi (1080x1920 portrait, Chromium still running underneath; PSS from
`smaps_rollup`; CPU from utime+stime over 30s with only the clock ticking):

| Renderer (winit, Wayland) | Colour emoji                                   | PSS        | Idle CPU     | Binary (stripped) |
|---------------------------|------------------------------------------------|------------|--------------|-------------------|
| femtovg (recommended)     | yes, 18–80px; **blank at ≥96px**               | 61–75 MB   | 0.33–0.57 %  | 13.1 MB           |
| software                  | **no**: CBDT emoji glyphs are not drawn at all | 20–29 MB   | 0.13–0.17 %  | 13.5 MB           |
| skia (GL)                 | yes, all sizes tested (18–128px)               | 89–95 MB   | 0.67 %       | 19.9 MB           |

With the 10s headline fade enabled, femtovg averages 2.1 % CPU, because each fade
redraws the whole window at frame rate for about 1s. Chromium uses roughly 470 MB and
40 % of a core.

Findings that Phase 1 must keep:
- Emoji fallback order: Slint's text font stack is `[font-family, sans-serif, system-ui]`.
  On the Pi, fontconfig's sans-serif list puts DejaVu Sans (monochrome 😀) and FreeSerif
  (monochrome ✨) ahead of Noto Color Emoji. `main.rs` therefore sets `SLINT_FONT_PATH`
  to `/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf` before creating the UI. That
  replaces the generic families, so emoji resolve to the colour font. A private
  fontconfig file (`FONTCONFIG_FILE`) also works, but needs an extra file.
- Roboto (Light + Regular, Apache-2.0) is embedded from `ui/fonts/`. The Pi has no Roboto.
  The emoji font comes from the system package (fonts-noto-color-emoji, CBDT bitmaps).
- The femtovg build links only libc/libm. Wayland, EGL/GLES, xkbcommon and fontconfig are
  all dlopened (fontconfig through `fontique`'s `fontconfig-dlopen` feature; see
  Cargo.toml). Every library it needs is already on the Pi.
- The clock re-arms a single-shot timer at each wall-clock second, so nothing else wakes.

### Build and deploy (dev machine, x86_64)

One-time setup: `rustup target add aarch64-unknown-linux-gnu` and `cargo install cargo-zigbuild`
(needs `zig` on PATH).

    cd native
    cargo zigbuild --release --target aarch64-unknown-linux-gnu.2.36      # femtovg (default)
    scp target/aarch64-unknown-linux-gnu/release/mmm-native pi:/home/max/mmm-native-spike/

    # software renderer variant
    cargo zigbuild --release --target aarch64-unknown-linux-gnu.2.36 \
        --no-default-features --features software --target-dir target/software

    # skia variant: links libfreetype, libfontconfig and libstdc++ directly. Copy those
    # .so files from the Pi into a local dir ($L) first. zig would otherwise substitute
    # its own libc++ for -lstdc++.
    RUSTFLAGS="-L $L -C link-arg=$L/libstdc++.so.6" cargo zigbuild --release \
        --target aarch64-unknown-linux-gnu.2.36 --no-default-features --features skia \
        --target-dir target/skia

Run under the existing session (on the Pi):

    XDG_RUNTIME_DIR=/run/user/1000 WAYLAND_DISPLAY=wayland-0 ./mmm-native --config /opt/stacks/mirror/data/config.json

(The spike's demo content and `MMM_NO_FADE` switch were removed in Phase 1.)

Still open:
- Why femtovg drops colour glyphs at 96px and above (probably its 512px glyph atlas or bitmap-strike path).
- GPU load and power use were not measured.
- Behaviour across a labwc restart or output hot-plug.

## Phase 1: core (DONE, 2026-11-10)

Layout of the code:

    src/main.rs        startup, emoji font, burn-in nudge, `guard` (catch a panic, keep going)
    src/config.rs      config file, areas, reload watcher
    src/layout.rs      per-area id lists pushed to Slint; `set_visible`
    src/fetch.rs       fetch scheduler (+ unit tests); `spawn` and `spawn_paced`
    src/google_auth.rs Google OAuth token refresh shared by calendar + google-tasks (Phase 2)
    src/auth_cli.rs    `mmm-native auth google|spotify` (Phase 3)
    src/spotify_tokens.rs  spotify-tokens.json: saved Spotify refresh tokens (Phase 3)
    src/sd_notify.rs   systemd READY=1 / WATCHDOG=1 (Phase 3)
    deploy/            systemd unit, deploy.sh, README.md (install, deploy, rollback) (Phase 3)
    src/timing.rs      cadences from timing.ts; `every_aligned` wall-clock timers
    src/qr.rs          QR code -> Slint image
    src/widgets/       mod.rs (registry, Ctx) + one file per widget
    ui/app.slint       window, areas, widget registry (WidgetSlot)
    ui/theme.slint     colours, Tailwind text sizes, `Label`
    ui/widgets/        one .slint per widget; ui/assets/<widget>/ for images
    ui/icon.slint      shared `Icon` (Lucide SVG, tinted, centred in its row) (Phase 2)
    ui/icons/          Lucide 0.540.0 SVGs + LICENSE-lucide.txt (Phase 2)

- **Config**: `--config <path>`, else `$MMM_CONFIG`, else `./data/config.json`. Same file
  and format as the web app, read only. Unknown ids and keys are ignored. Enabled entries
  with an unknown area are skipped. If `dashboard` is missing, the web defaults are used.
  If the file is missing or invalid, the screen shows the clock plus a red error line.
  Config values are never logged; only ids are logged.
- **Reload**: polls mtime/size/inode every 5s. When a change has been stable for one poll,
  the process re-execs itself with the same arguments. Changes apply 5–10s after saving,
  and the window is recreated, so there is a brief flash. A broken new file shows the
  error line. This replaces the web app's SSE + `location.reload()`.
- **Layout**: `areas` in app.slint is one list of widget ids per area (index order =
  `Area::ALL`). Each id renders through `WidgetSlot`, which has one `if` line per widget.
  Rust fills the lists from config order, leaving out disabled and unimplemented ids.
  `layout::set_visible(ui, ID, false)` takes a widget out of its area while it has nothing
  to show, so the gap closes up as it does in the web app.
- **Fetch** (`src/fetch.rs`): one `fetch` thread runs a current-thread tokio runtime with a
  shared reqwest client (0.12, rustls + ring, bundled webpki roots). The thread starts on
  the first job, so Phase 1 does not run it. Per job:
  - interval;
  - hard total timeout (20s default) plus a 10s connect timeout;
  - each attempt runs as its own task, so a panic or hang stays inside that attempt;
  - retry after 10s, 20s, 40s … up to min(interval, 15 min);
  - the last good value is kept;
  - `stale` is set after 3 failures in a row;
  - start times are staggered 0.75s apart.
  
  Release builds use `panic = "unwind"` so panics can be caught. Verified on the Pi with
  a throwaway probe build (not in the repo): a live HTTPS JSON fetch worked. A job that
  could not connect timed out and went stale after 3 failures. Panicking jobs and a
  panicking UI callback were all contained, and the clock kept ticking.
- **Widgets**: clock, greetings, events (with event image and QR code). Differences from
  the web app, all deliberate:
  - An event range crossing New Year (e.g. 12-30..01-02) also matches in January. The web
    `getEventDates` only matches the December half: a bug.
  - Invalid MM-DD values are skipped. dayjs rolled them over instead (13-01 becomes Jan).
  - Events are re-checked every minute rather than hourly.
  - The burn-in nudge moves 0/1/2/1 px once a minute instead of animating continuously.
- Measured on the Pi with the real config (clock + greetings, 9 configured widgets not yet
  ported): PSS 75 MB (36 MB anon), idle CPU 0.3% (9 ticks in 30s), 5 threads, stripped
  binary 14.5 MB. Adding the fetch stack adds about 2.1 MB of binary. The build still links
  only libc/libm. Geometry matches the Chromium screenshot to within 1–2 px.

### Adding a widget (Phase 2 recipe)

Port from `src/components/<id>/`. Use the manifest id `<id>` and the module name
`<id_with_underscores>`.

1. `ui/widgets/<id>.slint`: an exported data global and an exported component. Import
   `{ Theme, Label } from "../theme.slint"`. Copy Tailwind sizes 1:1 (`Theme.text-xl` and
   so on); the window default is bold #f5f5f5, as in the web app.

       export global WeatherData { in property <string> temp; in property <bool> stale; }
       export component Weather inherits VerticalLayout { Label { text: WeatherData.temp; font-size: Theme.text-5xl; } }

2. `src/widgets/<module>.rs`:

       use slint::{ComponentHandle, SharedString};
       use super::{set_if_changed, Ctx};
       use crate::fetch::{self, Job};
       use crate::{timing, WeatherData};

       pub const ID: &str = "weather";

       #[derive(serde::Deserialize, Default)]
       #[serde(default, rename_all = "camelCase")]
       struct Settings { api_key: String, latitude: String }   // your components.<id> slice

       pub fn start(ctx: &Ctx) {
           let s: Settings = ctx.config_as();               // or ctx.config (serde_json::Value)
           let url = format!("https://…?key={}", s.api_key);
           ctx.fetch.spawn(
               Job::new(ID, timing::FREQUENT),              // .timeout(..) to override 20s
               move |http| {                                 // runs on the fetch thread; must be Send
                   let url = url.clone();
                   async move { fetch::get_json::<Api>(http.get(url)).await.map(|a| a.temp) }
               },
               ctx.ui.as_weak(),
               |ui, update| {                                // runs on the UI thread
                   let d = ui.global::<WeatherData>();
                   if let Some(t) = &update.value {
                       set_if_changed(d.get_temp(), SharedString::from(format!("{t}°C")), |v| d.set_temp(v));
                   }
                   set_if_changed(d.get_stale(), update.stale, |v| d.set_stale(v));
               },
           );
       }

3. Register the widget with a one-line edit in each of two shared files:
   - `src/widgets/mod.rs`: add the module name to `widgets! { … }`.
   - `ui/app.slint`: add `import { Weather, WeatherData } from "widgets/weather.slint"; export { WeatherData }`
     and `if root.widget == "weather": Weather { }` in `WidgetSlot`.

Rules:
- `Update<T>` has three fields:
  - `value` is the last good result and survives failures;
  - `stale` is set after 3 failures in a row;
  - `error` is set when the latest attempt failed.
  
  Show `error` only when `value` is None, as the web components do.
- Make HTTP calls through `fetch::get_json` or `fetch::clean`, so that URLs (which can
  hold keys) never reach the logs. Never log config values.
- Set Slint properties only when they change (`set_if_changed`). Do not use continuous
  animations; a short fade now and then is fine. Keep text that contains emoji at 80px
  or smaller.
- For timers that are not network fetches (rotation, relative times), use
  `timing::every_aligned` on the UI thread. Do not add more threads.
- If the widget has nothing to show, call `layout::set_visible(ui, ID, false)`.
- To read another widget's settings, use `ctx.all.component("<id>")`.
- Images go in `ui/assets/<id>/` and are referenced with `@image-url`. Icons go in
  `ui/icons/` (individual Lucide 0.540.0 SVGs; licence in `LICENSE-lucide.txt`) and are
  drawn with `Icon { source: @image-url("../icons/x.svg"); size: 24px; }` from
  `ui/icon.slint`, which tints them with `colorize`. SVG icons render correctly with
  femtovg on the Pi (checked in Phase 2).
- Put pure logic in testable functions with `#[cfg(test)]` tests. Run `cargo test`,
  `cargo clippy --all-targets` and the cross-build before deploying.

## Phase 2: widgets (PORTED, 2026-11-10; first device test done)

All twelve remaining widgets are ported and merged; 158 unit tests, clippy clean, the
aarch64 build still links only libc/libm. Stripped binary 20.2 MB (Phase 1: 14.5 MB).

Status per widget. "Verified on device" means a run on the Pi with the real config
showed it with live data, laid out where the Chromium screenshot has it.

| Widget         | Status                                                                     |
|----------------|----------------------------------------------------------------------------|
| weather        | verified on device (current conditions, 3-day forecast). No MeteoAlarm warning was shown, so the alert card was not seen on the Pi |
| calendar       | verified on device (5-day list, colours, locations, fade-out)              |
| rss-feed       | verified on device (headline + source line, rotation). The feed that gives the web app a 403 did not fail in either native run |
| reminders      | verified on device (bin line). Custom reminders were outside their hours during the test; covered by unit tests only |
| polestar       | verified on device (login, battery, range, odometer, car image)            |
| adguard        | verified on device                                                         |
| system-stats   | verified on device. Reads the host's /proc, so memory shows a real figure (the web app's container showed 0%) |
| wifi-qr-code   | verified on device                                                         |
| spotify        | unverified: both refresh tokens are revoked, so it is hidden (as designed) and logs "Refresh token revoked" once per account. Art, layout and icons have not been seen on the Pi |
| google-tasks   | unverified: disabled in the real config                                   |
| migraine-risk  | unverified: disabled in the real config; scoring tests ported              |
| google_auth    | partly verified: calendar authenticated from the shared `google-tokens.json`. The refresh path was not exercised on the Pi (the access token was still valid), and the file there is root-owned 0644, so a refresh by this app stays in memory and is logged once |

Measured on the Pi (real config, all enabled widgets, Chromium still running underneath):
PSS 85–87 MB (40 MB anon), 6 threads. CPU 3.4–3.8 % of one core at 60–115 s, all on the
UI thread. femtovg redraws the whole window for every frame, so the clock tick and above
all the rss-feed crossfade (two 400 ms opacity animations every 10 s) now cost more per
frame than in Phase 1 (0.3 %). Estimate, not measured: the crossfade is roughly two
thirds of that.

Shared pieces added in Phase 2:
- `Fetcher::spawn_paced` (src/fetch.rs): like `spawn`, but after a success the next
  fetch is `pace(&value)` later instead of the job's interval (minimum 1 s; a panicking
  `pace` falls back to the interval). Failures back off as usual. Used by spotify to poll
  every 5 s while something plays and every 30 s otherwise.
- `ui/icon.slint` `Icon`: used by adguard, system-stats, wifi-qr-code, calendar,
  google-tasks, rss-feed, reminders and spotify. Two widgets keep their own variant
  because their alignment depends on it: polestar (`Icon inherits Image`, positioned
  with per-use `y` offsets) and weather (`LucideIcon inherits Image`, sized by its
  caller; migraine-risk imports it from weather.slint).
- `Roboto-Medium.ttf` (Roboto v2.138 unhinted, the same release as the other embedded
  faces) is embedded, so `font-weight: 500` (Tailwind `font-medium`) renders Medium
  instead of falling back to Regular.
- Dependencies: jiff (time zones), ring + base64 (Polestar PKCE), rustix (statvfs),
  image (album art), roxmltree (RSS and MeteoAlarm); tokio gains `sync`. All but jiff
  were already in the dependency tree via Slint/reqwest.

Known deviations from the web app (from code comments; the detailed list per widget is
in the Phase 2 session notes):
- No continuous animations: the spotify vinyl/cassette, the polestar charging bolt and
  the polestar loading skeleton are static (the web ones spin or pulse).
- Weather: day names use the forecast location's UTC offset, not the server's zone.
- Migraine risk: humidity (0..1 from Pirate Weather) is scaled to a percentage; the web
  app compared it unscaled, so it always scored "dry air".
- Polestar: fetched hourly (the web app's cache period) instead of a 5-minute poll of
  an hour-long cache; failed logins are rate-limited.
- system-stats: reads /proc and statvfs on the host instead of running vmstat/free/df
  inside a container, so its figures differ from the web app's.
- Calendar: leading/trailing and repeated whitespace in event titles is collapsed, as
  HTML does (found on device: a title with a leading space was drawn indented).
- WiFi QR: "Network:" is Bold, not extrabold (heaviest embedded Roboto face).

Open:
- Idle CPU (3.4–3.8 %) is above the Phase 0/1 figures. Options: drop or shorten the
  rss-feed crossfade, or accept it. Needs a decision.
- Spotify needs re-authorised refresh tokens before it can be checked on the device
  (Phase 3 `mmm-native auth spotify`).
- Token file ownership: the web container writes `google-tokens.json` as root, so this
  app cannot persist refreshed Google tokens while the web app is still installed
  (the cutover steps in deploy/README.md chown it).

## Phase 3: cutover (DONE, 2026-11-10)

Cutover performed 2026-11-10 21:45: `mmm-native.service` enabled and running on the Pi;
`dashboard-kiosk.service`, `dashboard-monitor.service`, `kiosk-page-watchdog.service` and
`dashboard-kiosk-nightly.timer` disabled (not removed); `mirror` container stopped (not
removed); `google-tokens.json` now owned by max (0600). Pre-cutover backup on the Pi:
`/home/max/backups/cutover-2026-11-10/`. Measured after cutover with all widgets:
100 MB PSS, ~3.3% of one core, 0 restarts, watchdog pings arriving.

Follow-up 2026-11-11, after a 10.5 h soak (0 restarts, 97 MB PSS flat, Google token
refresh + save seen working, a feed 403 backed off and recovered):

- Rebooted twice; the app comes up unattended. The first reboot showed it starting before
  AdGuard's DNS, so every first fetch failed and the Polestar login sat in its failure
  cooldown. The unit now has a best-effort 45 s DNS wait in `ExecStartPre`; the second
  reboot had no fetch errors.
- Old kiosk units and scripts removed from the Pi (`dashboard-kiosk`, `dashboard-monitor`,
  `kiosk-page-watchdog`, `dashboard-kiosk-nightly`). Copies are in
  `/home/max/backups/cleanup-2026-11-11/` and `/home/max/backups/cutover-2026-11-10/`.
- Desktop session stripped: `/etc/xdg/labwc/autostart` now starts only `kanshi` (output
  rotation). No panel, desktop file manager or XDG autostart. Original in the same backup.
- `pi-backup.sh` now copies `mmm-native.service` and the units that still exist.
- Seconds removed from the clock to stop per-second full-window redraws (HH:MM only).
- Pi memory work: zram swap, memory cgroup enabled, `MemoryHigh=250M`/`MemoryMax=350M` on
  the unit, caps on the other containers, unneeded user session daemons masked.
- Spotify re-authorised with `mmm-native auth spotify` (both accounts); `deploy.sh` used
  for a real deploy; `mirror` container and image removed.
- CPU after removing seconds: 2.3% of one core (was 3.2%), all on the UI thread. The
  remainder is most likely the 400 ms RSS headline crossfade every 10 s (about 24
  full-window frames each time); not confirmed by experiment.

Still open, not yet exercised on the device: `auth google`, watchdog-triggered restart,
behaviour across an HDMI drop. While
the Polestar login is in cooldown the widget keeps showing the original error text.

### Phase 3 code notes

Code and deploy files are written and unit-tested. Nothing has been installed on the Pi
yet. The install, cutover and rollback steps are in `deploy/README.md`.

- **systemd** (`deploy/mmm-native.service`, systemd 252):
  - `Type=notify`, `NotifyAccess=main`, `WatchdogSec=90`; runs as `max` with the labwc
    session's `XDG_RUNTIME_DIR`/`WAYLAND_DISPLAY`.
  - `ExecStartPre` waits up to 30s for `/run/user/1000/wayland-0`.
  - `Restart=always`, `RestartSec=60`, `StartLimitIntervalSec=0`: at most one start
    (and so one Polestar login) a minute, and the unit never gives up.
  - `WantedBy=`/`After=graphical.target`. Deliberately no ordering on
    `dashboard-kiosk.service`: an ordering cycle of that kind once dropped a unit at boot.
  - Hardening: `NoNewPrivileges=yes`, `ProtectSystem=full`. No `MemoryMax=` (the Pi
    has no memory cgroup controller).
  - `systemd-analyze verify` (systemd 262 on the dev machine) only reports the missing
    binary path.
- **sd_notify** (`src/sd_notify.rs`): no new dependency. It sends a datagram to
  `$NOTIFY_SOCKET` (path or `@` abstract name).
  - `READY=1` goes out from a zero timer once the event loop runs, i.e. after the window
    is shown.
  - `WATCHDOG=1` is sent from the clock widget's 1s UI-thread tick, rate-limited to one
    per `min(WATCHDOG_USEC/2, 10s)`. A frozen UI stops it. A 10s timer stands in if no
    clock is placed.
  - It does nothing without `NOTIFY_SOCKET`. The variables survive the config re-exec,
    which keeps the same PID.
- **`deploy/deploy.sh`**:
  - cross-builds and checks that `readelf` NEEDED shows only libc/libm;
  - `scp`s the binary to a temp name in `/opt/mmm-native/`;
  - hard-links the current binary to `mmm-native.prev`;
  - `mv`s the new one into place;
  - `sudo systemctl restart mmm-native`.
- **Spotify tokens** (`spotify-tokens.json` beside the config, `{name: refreshToken}`,
  written atomically with mode 0600):
  - For an account whose name has an entry in the file, the file's token wins over
    `components.spotify.accounts[].refreshToken`. An account may have only a name in
    config.
  - Rotated refresh tokens are saved there. If saving fails, or the account has no
    name, the token stays in memory and this is logged once.
  - The app never writes `config.json`.
- **`mmm-native auth google|spotify [--name N] [--config P] [--redirect-uri U]`**:
  - Terminal only, no Slint. It prints the authorisation URL, the user pastes back the
    redirected URL, and the command checks `state`, exchanges the code and saves the
    tokens.
  - Scopes, `access_type=offline` and the redirect paths are the web app's.
  - Google always sends `prompt=consent`, so a new refresh token is issued. The web app
    sent it only when none was stored.
  - Spotify adds `state`.
  - Default redirect URIs:
    - Google: `http://localhost:4000/api/google/callback`;
    - Spotify: `http://127.0.0.1:4000/api/spotify/callback`. Spotify rejects
      `localhost`, so this is the web app's path on the loopback IP.
  - Google tokens go to `google-tokens.json` in the existing format; the running app
    picks them up. Spotify tokens go to `spotify-tokens.json`, then restart the app. If
    config has no account with that name, the command says what to add.
- **Calendar** (decisions made in Phase 3):
  - Multi-day events are listed on every day they cover in the 5-day window, including
    ones that began before today. An all-day `end.date` is exclusive.
  - Timed events over several days show "From 9:30" on the first day, "All day" (with
    the all-day icon) on days in between, and "Until 17:00" on the last day. An event
    ending exactly at midnight stays on one day.
  - Only calendars with `selected == true` are shown. The API's default for a missing
    `selected` is false; the web app showed those calendars.

Unverified until the cutover:
- the unit on the Pi (boot ordering, watchdog restarts, ProtectSystem with Mesa);
- the real OAuth flows. Neither redirect URI default is confirmed as registered, and
  the Spotify app must have the 127.0.0.1 URI (or pass `--redirect-uri`);
- `deploy.sh` (written, not run).

## Future

- Editing events from a phone. The old web settings UI could do this and it was valued.
  v1 deliberately has no settings UI.
