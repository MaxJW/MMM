//! Refresh cadences (mirroring `TIMING_STRATEGIES` in src/lib/core/timing.ts) and
//! wall-clock-aligned UI timers.

#![allow(dead_code)] // the cadences are for Phase 2 widgets

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use slint::Timer;

const fn secs(s: u64) -> Duration {
    Duration::from_secs(s)
}

pub const REALTIME: Duration = secs(1); // clock tick (minute flip + watchdog)
pub const VERY_FAST: Duration = secs(5); // spotify playing
pub const FAST: Duration = secs(10);
pub const MEDIUM_FAST: Duration = secs(30);
pub const MEDIUM: Duration = secs(60);
pub const FREQUENT: Duration = secs(5 * 60); // system stats, weather
pub const STANDARD: Duration = secs(30 * 60); // RSS, forecasts
pub const INFREQUENT: Duration = secs(60 * 60); // bin collections, static data
pub const UI_MINUTE: Duration = secs(60);
pub const UI_FADE: Duration = secs(10); // RSS headline rotation

/// Call `f` on the UI thread just after every wall-clock multiple of `period`
/// (each second, each minute, ...), forever. A single-shot timer is re-armed each
/// time, so nothing wakes in between, the schedule never drifts, and a wall-clock
/// jump (NTP at boot) is corrected on the next tick. A panic in `f` is caught and
/// the timer keeps running.
pub fn every_aligned(period: Duration, f: impl FnMut() + 'static) {
    arm(period, Rc::new(RefCell::new(f)));
}

fn arm(period: Duration, f: Rc<RefCell<dyn FnMut()>>) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timer::single_shot(until_next(period, now), move || {
        crate::guard("timer", || (f.borrow_mut())());
        arm(period, f);
    });
}

/// Time from `now` (since the epoch) until just after the next multiple of `period`.
fn until_next(period: Duration, now: Duration) -> Duration {
    let p = period.as_millis().max(1);
    let into = now.as_millis() % p;
    Duration::from_millis((p - into) as u64 + 5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_to_next_boundary() {
        let s = Duration::from_secs;
        let ms = Duration::from_millis;
        assert_eq!(until_next(s(1), ms(10_250)), ms(755));
        assert_eq!(until_next(s(1), ms(10_000)), ms(1_005));
        assert_eq!(until_next(s(60), s(125)), ms(55_005));
    }
}
