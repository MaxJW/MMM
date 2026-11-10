//! The web app's config file, read as-is:
//! `{ dashboard: { components: [{id, enabled, area}] }, components: { <id>: {...} } }`.
//! Unknown ids and keys are ignored. Config *values* are never logged (they hold secrets).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};

pub const DEFAULT_PATH: &str = "./data/config.json";
const POLL: Duration = Duration::from_secs(5);

/// The dashboard areas of Dashboard.svelte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    MiddleRight,
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Notifications,
    Overlay,
}

impl Area {
    /// Index order is the order of `areas` in ui/app.slint.
    pub const ALL: [Area; 11] = [
        Area::TopLeft,
        Area::TopCenter,
        Area::TopRight,
        Area::MiddleLeft,
        Area::MiddleRight,
        Area::Center,
        Area::BottomLeft,
        Area::BottomCenter,
        Area::BottomRight,
        Area::Notifications,
        Area::Overlay,
    ];

    pub fn parse(s: &str) -> Option<Area> {
        let i = [
            "top-left",
            "top-center",
            "top-right",
            "middle-left",
            "middle-right",
            "center",
            "bottom-left",
            "bottom-center",
            "bottom-right",
            "notifications",
            "overlay",
        ]
        .iter()
        .position(|a| *a == s)?;
        Some(Area::ALL[i])
    }
}

/// An enabled component and where it goes. Disabled entries are dropped on load.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub id: String,
    pub area: Area,
}

#[derive(Debug, Default)]
pub struct Config {
    /// Enabled components in config order.
    pub placements: Vec<Placement>,
    components: Map<String, Value>,
}

/// getDefaultConfig() in src/lib/core/config.ts, used when the file has no `dashboard`.
const DEFAULT_PLACEMENTS: &[(&str, Area)] = &[
    ("clock", Area::TopLeft),
    ("weather", Area::TopRight),
    ("calendar", Area::TopLeft),
    ("system-stats", Area::BottomRight),
    ("wifi-qr-code", Area::BottomLeft),
    ("events", Area::Center),
    ("greetings", Area::Center),
    ("reminders", Area::Notifications),
    ("rss-feed", Area::Notifications),
    ("adguard", Area::MiddleRight),
];

static NULL: Value = Value::Null;

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn parse(text: &str) -> Result<Config, String> {
        // serde_json syntax errors carry only a position, never the offending value.
        let root: Value = serde_json::from_str(text).map_err(|e| format!("invalid JSON ({e})"))?;
        let Value::Object(mut root) = root else {
            return Err("top level is not a JSON object".into());
        };
        let components = match root.remove("components") {
            Some(Value::Object(m)) => m,
            None => Map::new(),
            Some(_) => return Err("`components` is not an object".into()),
        };
        let placements = match root.get("dashboard").and_then(|d| d.get("components")) {
            None => DEFAULT_PLACEMENTS
                .iter()
                .map(|&(id, area)| Placement {
                    id: id.into(),
                    area,
                })
                .collect(),
            Some(Value::Array(list)) => list
                .iter()
                .enumerate()
                .filter_map(|(i, c)| placement(i, c))
                .collect(),
            Some(_) => return Err("`dashboard.components` is not a list".into()),
        };
        Ok(Config {
            placements,
            components,
        })
    }

    /// Used when the file is missing or invalid: just the clock.
    pub fn fallback() -> Config {
        Config {
            placements: vec![Placement {
                id: "clock".into(),
                area: Area::TopLeft,
            }],
            ..Default::default()
        }
    }

    /// The `components.<id>` slice (Null when absent).
    pub fn component(&self, id: &str) -> &Value {
        self.components.get(id).unwrap_or(&NULL)
    }
}

fn placement(index: usize, c: &Value) -> Option<Placement> {
    let id = c.get("id")?.as_str()?;
    if c.get("enabled").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    match c.get("area").and_then(Value::as_str).and_then(Area::parse) {
        Some(area) => Some(Placement {
            id: id.into(),
            area,
        }),
        None => {
            eprintln!("config: dashboard.components[{index}] ({id}) has no valid area; skipped");
            None
        }
    }
}

/// `--config <path>`, else `$MMM_CONFIG`, else ./data/config.json.
pub fn path_from_args() -> PathBuf {
    let mut args = std::env::args_os().skip(1);
    while let Some(a) = args.next() {
        if a == "--config" {
            if let Some(p) = args.next() {
                return p.into();
            }
        }
    }
    std::env::var_os("MMM_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| DEFAULT_PATH.into())
}

/// Identity of the file on disk; any change means "reload". None = missing.
type Stamp = Option<(Option<std::time::SystemTime>, u64, u64)>;

fn stamp(path: &Path) -> Stamp {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path)
        .ok()
        .map(|m| (m.modified().ok(), m.len(), m.ino()))
}

/// Poll the file every 5s. Once a change has been stable for one poll (so a half-
/// written file is not picked up), re-exec this process with the same arguments:
/// a fresh start applies any config change, and drops all widget state, timers and
/// jobs. If the new file is invalid, the new process shows the clock and an error.
/// The returned timer must be kept alive.
pub fn watch(path: PathBuf) -> slint::Timer {
    let baseline = stamp(&path);
    let mut pending: Option<Stamp> = None;
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, POLL, move || {
        let now = stamp(&path);
        if now == baseline {
            pending = None;
        } else if pending.as_ref() == Some(&now) {
            eprintln!("config: {} changed; restarting", path.display());
            let err = reexec();
            // Still here: keep running with the old config rather than retrying in a loop.
            eprintln!("config: restart failed ({err}); keeping the current config");
            pending = None;
        } else {
            pending = Some(now);
        }
    });
    timer
}

fn reexec() -> std::io::Error {
    use std::os::unix::process::CommandExt;
    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    // After a deploy replaced the binary, /proc/self/exe points at "<path> (deleted)":
    // fall back to argv[0] so the new binary is started.
    let exe = match std::env::current_exe() {
        Ok(p) if p.exists() => p,
        _ => argv0.into(),
    };
    std::process::Command::new(exe).args(args).exec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_enabled_placements_in_order_and_ignores_extras() {
        let c = Config::parse(
            r#"{
              "dashboard": { "components": [
                {"id": "clock", "enabled": true, "area": "top-left"},
                {"id": "weather", "enabled": false, "area": "top-right"},
                {"id": "energy", "enabled": true, "area": "middle-left", "extra": 1},
                {"id": "greetings", "enabled": true, "area": "center"},
                {"id": "bad", "enabled": true, "area": "nowhere"},
                {"enabled": true, "area": "center"}
              ], "other": true },
              "components": { "greetings": {"greetings": []}, "energy": {"x": 1} },
              "unknown": [1, 2, 3]
            }"#,
        )
        .unwrap();
        let ids: Vec<_> = c
            .placements
            .iter()
            .map(|p| (p.id.as_str(), p.area))
            .collect();
        assert_eq!(
            ids,
            [
                ("clock", Area::TopLeft),
                ("energy", Area::MiddleLeft),
                ("greetings", Area::Center)
            ]
        );
        assert!(c.component("greetings").is_object());
        assert!(c.component("weather").is_null());
    }

    #[test]
    fn missing_dashboard_uses_web_defaults() {
        let c = Config::parse(r#"{"components": {}}"#).unwrap();
        assert_eq!(c.placements.len(), DEFAULT_PLACEMENTS.len());
        assert_eq!(c.placements[0].id, "clock");
    }

    #[test]
    fn invalid_files_are_errors_without_values() {
        let err = Config::parse(r#"{"components": {"x": {"password": "hunter2"}"#).unwrap_err();
        assert!(!err.contains("hunter2"), "{err}");
        assert!(Config::parse("[1]").is_err());
        assert!(Config::parse(r#"{"components": "secret"}"#)
            .unwrap_err()
            .find("secret")
            .is_none());
    }
}
