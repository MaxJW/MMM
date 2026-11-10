//! Rolling 48 h pressure/temperature history, port of
//! src/components/migraine-risk/history.ts. Same file and format as the web app:
//! `migraine-risk-history.json` = `{"readings": [{"t": <ms>, "pressure_hPa": .., "tempC": ..}]}`,
//! pretty-printed with 2 spaces, in the directory of the config file (the web app's `data/`).
//!
//! The file is read once at start and then kept in memory; each new reading is written
//! back atomically (temp file + rename). If writing fails (for example the directory
//! belongs to another user), the history carries on in memory and the failure is
//! logged once.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

pub const FILE_NAME: &str = "migraine-risk-history.json";
const HISTORY_MS: i64 = 48 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    /// Unix time in ms.
    pub t: i64,
    /// 0 when the API gave no valid pressure (as the web app stores it).
    pub pressure_hpa: f64,
    pub temp_c: f64,
}

pub struct History {
    path: PathBuf,
    pub readings: Vec<Reading>,
    write_failing: bool,
}

impl History {
    /// Load `dir/migraine-risk-history.json`. Missing or unreadable: empty, like the web
    /// app. Malformed entries are skipped.
    pub fn load(dir: &Path) -> History {
        let path = dir.join(FILE_NAME);
        let readings = match std::fs::read_to_string(&path) {
            Ok(text) => parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(e) => {
                eprintln!(
                    "migraine-risk: cannot read {}: {e}; starting empty",
                    path.display()
                );
                Vec::new()
            }
        };
        History {
            path,
            readings,
            write_failing: false,
        }
    }

    /// `appendReading`: add, drop readings older than 48 h before `reading.t`, save.
    pub fn append(&mut self, reading: Reading) {
        self.readings.push(reading);
        prune(&mut self.readings, reading.t);
        match write_atomically(&self.path, &to_json(&self.readings)) {
            Ok(()) if self.write_failing => {
                eprintln!("migraine-risk: {} is writable again", self.path.display());
                self.write_failing = false;
            }
            Ok(()) => {}
            Err(e) if !self.write_failing => {
                eprintln!(
                    "migraine-risk: cannot write {} ({e}); keeping history in memory",
                    self.path.display()
                );
                self.write_failing = true;
            }
            Err(_) => {}
        }
    }
}

fn parse(text: &str) -> Vec<Reading> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        eprintln!("migraine-risk: history file is not valid JSON; starting empty");
        return Vec::new();
    };
    let Some(list) = value.get("readings").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|r| {
            Some(Reading {
                t: r.get("t")?.as_f64()? as i64,
                pressure_hpa: r.get("pressure_hPa")?.as_f64()?,
                temp_c: r.get("tempC")?.as_f64()?,
            })
        })
        .collect()
}

fn prune(readings: &mut Vec<Reading>, now: i64) {
    let cutoff = now - HISTORY_MS;
    readings.retain(|r| r.t >= cutoff);
}

/// `JSON.stringify({ readings }, null, 2)`: same key order, whole numbers without ".0".
fn to_json(readings: &[Reading]) -> String {
    #[derive(Serialize)]
    struct File {
        readings: Vec<Out>,
    }
    #[derive(Serialize)]
    struct Out {
        t: i64,
        #[serde(rename = "pressure_hPa")]
        pressure_hpa: Num,
        #[serde(rename = "tempC")]
        temp_c: Num,
    }
    struct Num(f64);
    impl Serialize for Num {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            if self.0.fract() == 0.0 && self.0.abs() < 9e15 {
                s.serialize_i64(self.0 as i64)
            } else {
                s.serialize_f64(self.0)
            }
        }
    }
    let file = File {
        readings: readings
            .iter()
            .map(|r| Out {
                t: r.t,
                pressure_hpa: Num(r.pressure_hpa),
                temp_c: Num(r.temp_c),
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_default()
}

fn write_atomically(path: &Path, text: &str) -> std::io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let result = (|| {
        let mut file = std::fs::File::create(&tmp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

/// Drop from the peak pressure in [now - window, now] (readings above 900 hPa only,
/// and the current pressure) to the current pressure, rounded to 0.1 hPa.
pub fn pressure_drop_from_peak(
    readings: &[Reading],
    current: f64,
    now_ms: i64,
    window_hours: f64,
) -> f64 {
    let start = now_ms as f64 - window_hours * 3_600_000.0;
    let mut peak = current;
    for r in readings
        .iter()
        .filter(|r| r.t as f64 >= start && r.t <= now_ms)
    {
        if r.pressure_hpa > 900.0 && r.pressure_hpa > peak {
            peak = r.pressure_hpa;
        }
    }
    if current > 900.0 && current > peak {
        peak = current;
    }
    round1(peak - current).max(0.0)
}

/// Absolute °C change against the reading nearest to (now - 6 h); the first of equally
/// near readings wins. 0 without history.
pub fn temperature_change_6h(readings: &[Reading], current: f64, now_ms: i64) -> f64 {
    let target = now_ms - 6 * 3_600_000;
    let Some(mut best) = readings.first() else {
        return 0.0;
    };
    for r in readings {
        if (r.t - target).abs() < (best.t - target).abs() {
            best = r;
        }
    }
    round1((current - best.temp_c).abs())
}

/// `Math.round(x * 10) / 10`.
fn round1(x: f64) -> f64 {
    crate::widgets::weather::forecast::js_round(x * 10.0) / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: i64 = 3_600_000;

    fn r(hours_ago: f64, p: f64, t: f64) -> Reading {
        Reading {
            t: 100 * H - (hours_ago * H as f64) as i64,
            pressure_hpa: p,
            temp_c: t,
        }
    }

    #[test]
    fn pressure_drop_uses_peak_in_window_only() {
        let now = 100 * H;
        let history = [
            r(30.0, 1030.0, 5.0),
            r(20.0, 1020.0, 5.0),
            r(5.0, 1012.34, 5.0),
            r(1.0, 0.0, 5.0),
        ];
        assert_eq!(pressure_drop_from_peak(&history, 1005.0, now, 6.0), 7.3);
        assert_eq!(pressure_drop_from_peak(&history, 1005.0, now, 24.0), 15.0);
        // Rising pressure is no drop; no history is no drop.
        assert_eq!(pressure_drop_from_peak(&history, 1040.0, now, 24.0), 0.0);
        assert_eq!(pressure_drop_from_peak(&[], 1000.0, now, 6.0), 0.0);
        // Readings at or below 900 hPa (invalid, stored as 0) are ignored.
        assert_eq!(
            pressure_drop_from_peak(&[r(1.0, 899.0, 0.0)], 850.0, now, 6.0),
            0.0
        );
    }

    #[test]
    fn temperature_change_against_reading_nearest_6h_ago() {
        let now = 100 * H;
        let history = [
            r(12.0, 0.0, 1.0),
            r(6.5, 0.0, 3.0),
            r(5.5, 0.0, 9.0),
            r(1.0, 0.0, 20.0),
        ];
        // 6.5 h and 5.5 h ago are equally near: the first wins.
        assert_eq!(temperature_change_6h(&history, 10.25, now), 7.3);
        assert_eq!(
            temperature_change_6h(&[r(40.0, 0.0, 15.0)], 10.0, now),
            5.0,
            "any age"
        );
        assert_eq!(temperature_change_6h(&[], 10.0, now), 0.0);
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mmm-migraine-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips_the_web_format_and_prunes_after_48h() {
        let dir = scratch_dir("roundtrip");
        // As written by the web app (JSON.stringify(.., null, 2)), plus junk entries.
        std::fs::write(
            dir.join(FILE_NAME),
            r#"{
  "readings": [
    { "t": 1000, "pressure_hPa": 1013.2, "tempC": 4 },
    { "t": "bad" },
    { "t": 180001000, "pressure_hPa": 0, "tempC": -1.5 }
  ]
}"#,
        )
        .unwrap();
        let mut h = History::load(&dir);
        assert_eq!(h.readings.len(), 2);
        assert_eq!(
            h.readings[0],
            Reading {
                t: 1000,
                pressure_hpa: 1013.2,
                temp_c: 4.0
            }
        );

        h.append(Reading {
            t: 1000 + 48 * H + 1,
            pressure_hpa: 1009.0,
            temp_c: 7.25,
        });
        let text = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
        assert_eq!(
            text,
            "{\n  \"readings\": [\n    {\n      \"t\": 180001000,\n      \"pressure_hPa\": 0,\n      \"tempC\": -1.5\n    },\n    {\n      \"t\": 172801001,\n      \"pressure_hPa\": 1009,\n      \"tempC\": 7.25\n    }\n  ]\n}"
        );
        assert!(!dir.join(format!("{FILE_NAME}.tmp")).exists());
        assert_eq!(History::load(&dir).readings, h.readings);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unwritable_location_keeps_history_in_memory() {
        let dir = scratch_dir("unwritable").join("missing-subdir");
        let mut h = History::load(&dir);
        assert!(h.readings.is_empty());
        h.append(Reading {
            t: 1,
            pressure_hpa: 1000.0,
            temp_c: 1.0,
        });
        h.append(Reading {
            t: 2,
            pressure_hpa: 1001.0,
            temp_c: 2.0,
        });
        assert_eq!(h.readings.len(), 2);
        assert!(h.write_failing);
        assert!(!dir.exists());
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    #[test]
    fn invalid_file_starts_empty() {
        let dir = scratch_dir("invalid");
        std::fs::write(dir.join(FILE_NAME), "{not json").unwrap();
        assert!(History::load(&dir).readings.is_empty());
        std::fs::write(dir.join(FILE_NAME), r#"{"readings": 3}"#).unwrap();
        assert!(History::load(&dir).readings.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
