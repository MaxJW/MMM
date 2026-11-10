//! Host CPU / memory / disk / temperature: port of src/components/system-stats. The web
//! version shelled out (vmstat, free, df) inside a container; this reads the kernel
//! directly on the UI thread (no subprocesses, no network scheduler) every 5 minutes.
//! A value that cannot be read shows as a dash.
//!
//! Sources:
//! - /proc/stat `cpu` line: proc_stat(5), https://man7.org/linux/man-pages/man5/proc_stat.5.html
//!   (user nice system idle iowait irq softirq steal guest guest_nice, in USER_HZ; guest
//!   and guest_nice are already included in user and nice, so they are not summed).
//! - /proc/meminfo MemTotal / MemAvailable: proc_meminfo(5),
//!   https://man7.org/linux/man-pages/man5/proc_meminfo.5.html
//! - disk: statvfs("/"), rounded like GNU df's Use% (src/df.c: ceil(used*100/(used+avail)),
//!   used = f_blocks - f_bfree, avail = f_bavail).
//! - /sys/class/thermal/thermal_zone0/temp: millidegrees Celsius
//!   (Documentation/ABI/testing/sysfs-class-thermal in the kernel tree).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use slint::{ComponentHandle, SharedString, Timer};

use super::{set_if_changed, Ctx};
use crate::{timing, AppWindow, SystemStatsData};

pub const ID: &str = "system-stats";

const DASH: &str = "–";
/// The first CPU figure is measured over 1s starting 2s after launch, so the app's own
/// start-up work is not shown for the next 5 minutes; later figures cover the 5 minutes
/// since the previous sample.
const FIRST_CPU_START: Duration = Duration::from_secs(2);
const FIRST_CPU_WINDOW: Duration = Duration::from_secs(1);

/// Cumulative CPU time from the aggregate `cpu` line of /proc/stat.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CpuTimes {
    busy: u64,
    total: u64,
}

fn parse_cpu(stat: &str) -> Option<CpuTimes> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    // user nice system idle iowait irq softirq steal (older kernels have fewer fields)
    let fields: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .map(|f| f.parse().ok())
        .collect::<Option<_>>()?;
    if fields.len() < 4 {
        return None;
    }
    let total: u64 = fields.iter().fold(0u64, |a, &b| a.saturating_add(b));
    let idle = fields[3].saturating_add(fields.get(4).copied().unwrap_or(0));
    Some(CpuTimes { busy: total.saturating_sub(idle), total })
}

/// Busy share between two samples, in whole percent.
fn cpu_percent(prev: CpuTimes, now: CpuTimes) -> Option<u32> {
    let total = now.total.checked_sub(prev.total).filter(|t| *t > 0)?;
    // iowait can go backwards on some kernels, so busy is clamped into range.
    let busy = now.busy.saturating_sub(prev.busy).min(total);
    Some(percent(busy, total))
}

fn parse_mem_percent(meminfo: &str) -> Option<u32> {
    let field = |name: &str| {
        meminfo.lines().find_map(|l| {
            let rest = l.strip_prefix(name)?.strip_prefix(':')?;
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
    };
    let total = field("MemTotal").filter(|t| *t > 0)?;
    let available = field("MemAvailable")?.min(total);
    Some(percent(total - available, total))
}

/// GNU df's Use%: used / (used + available to non-root), rounded up.
fn disk_percent(blocks: u64, free: u64, available: u64) -> Option<u32> {
    let used = blocks.checked_sub(free)?;
    let denominator = used.checked_add(available).filter(|d| *d > 0)?;
    let used100 = used.checked_mul(100)?;
    Some((used100.div_ceil(denominator)) as u32)
}

fn parse_temp(millidegrees: &str) -> Option<i64> {
    let m: i64 = millidegrees.trim().parse().ok()?;
    Some((m as f64 / 1000.0).round() as i64)
}

fn percent(part: u64, whole: u64) -> u32 {
    (part as f64 * 100.0 / whole as f64).round() as u32
}

fn read(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn read_cpu() -> Option<CpuTimes> {
    read("/proc/stat").as_deref().and_then(parse_cpu)
}

fn read_disk() -> Option<u32> {
    let s = rustix::fs::statvfs("/").ok()?;
    disk_percent(s.f_blocks, s.f_bfree, s.f_bavail)
}

#[derive(Debug, Default, PartialEq)]
struct Reading {
    cpu: Option<u32>,
    memory: Option<u32>,
    disk: Option<u32>,
    temp: Option<i64>,
}

fn pct_text(v: Option<u32>) -> SharedString {
    v.map_or(DASH.into(), |v| format!("{v}%").into())
}

fn temp_text(v: Option<i64>) -> SharedString {
    v.map_or(DASH.into(), |v| format!("{v}°C").into())
}

/// Sample everything; the CPU figure is the delta since `prev`, which is then advanced.
fn sample(prev: &Cell<Option<CpuTimes>>) -> Reading {
    let now = read_cpu();
    let cpu = prev.get().zip(now).and_then(|(p, n)| cpu_percent(p, n));
    prev.set(now);
    Reading {
        cpu,
        memory: read("/proc/meminfo").as_deref().and_then(parse_mem_percent),
        disk: read_disk(),
        temp: read("/sys/class/thermal/thermal_zone0/temp").as_deref().and_then(parse_temp),
    }
}

fn show(ui: &AppWindow, r: &Reading) {
    let d = ui.global::<SystemStatsData>();
    set_if_changed(d.get_cpu(), pct_text(r.cpu), |v| d.set_cpu(v));
    set_if_changed(d.get_memory(), pct_text(r.memory), |v| d.set_memory(v));
    set_if_changed(d.get_disk(), pct_text(r.disk), |v| d.set_disk(v));
    set_if_changed(d.get_temp(), temp_text(r.temp), |v| d.set_temp(v));
}

pub fn start(ctx: &Ctx) {
    let prev = Rc::new(Cell::new(None));
    show(ctx.ui, &sample(&prev)); // CPU is a dash until the first window has passed
    let baseline = prev.clone();

    let refresh = {
        let weak = ctx.ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                show(&ui, &sample(&prev));
            }
        }
    };
    let first = refresh.clone();
    Timer::single_shot(FIRST_CPU_START, move || {
        baseline.set(read_cpu());
        Timer::single_shot(FIRST_CPU_WINDOW, move || crate::guard(ID, first));
    });
    timing::every_aligned(timing::FREQUENT, refresh);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Shapes as printed by Linux 6.x (Debian 12 on a Pi 4 has 4 CPUs).
    const STAT_A: &str = "cpu  10132153 290696 3084719 46828483 16683 0 25195 0 175628 0\n\
cpu0 1393280 32966 572056 13343292 6130 0 17875 0 23933 0\n\
intr 1462898 0 0 0\nctxt 3145120\nbtime 1700000000\n";
    const STAT_B: &str = "cpu  10132253 290696 3084769 46828828 16688 0 25195 0 175700 0\n\
cpu0 1393300 32966 572060 13343380 6130 0 17875 0 23940 0\n";

    const MEMINFO: &str = "MemTotal:        1891712 kB\n\
MemFree:           81240 kB\n\
MemAvailable:     946132 kB\n\
Buffers:           38244 kB\n\
Cached:           870516 kB\n";

    #[test]
    fn cpu_from_two_samples() {
        let a = parse_cpu(STAT_A).unwrap();
        let b = parse_cpu(STAT_B).unwrap();
        // busy +100 +50 = 150; idle +345 + iowait +5 = 350; total 500 -> 30%
        assert_eq!(b.total - a.total, 500);
        assert_eq!(cpu_percent(a, b), Some(30));
        assert_eq!(cpu_percent(a, a), None, "no time passed");
        assert_eq!(cpu_percent(b, a), None, "counters went backwards");
    }

    #[test]
    fn cpu_line_variants() {
        assert_eq!(parse_cpu("cpu  1 2 3 4\n"), Some(CpuTimes { busy: 6, total: 10 }));
        assert_eq!(parse_cpu("cpu0 1 2 3 4\n"), None, "per-core lines are not the aggregate");
        assert_eq!(parse_cpu("cpu  1 2 x 4\n"), None);
        assert_eq!(parse_cpu("cpu  1 2\n"), None);
        assert_eq!(parse_cpu(""), None);
    }

    #[test]
    fn memory_uses_available() {
        // (1891712 - 946132) / 1891712 = 49.98% -> 50
        assert_eq!(parse_mem_percent(MEMINFO), Some(50));
        assert_eq!(parse_mem_percent("MemTotal: 100 kB\n"), None, "no MemAvailable");
        assert_eq!(parse_mem_percent("MemTotal: 0 kB\nMemAvailable: 0 kB\n"), None);
        assert_eq!(parse_mem_percent("MemTotalX: 100 kB\nMemAvailable: 1 kB\n"), None);
    }

    #[test]
    fn disk_rounds_up_like_df() {
        assert_eq!(disk_percent(1000, 900, 150), Some(40)); // 100*100/250 = 40 exactly
        assert_eq!(disk_percent(1000, 899, 150), Some(41)); // 101*100/251 = 40.2 -> 41
        // This host's `df /` line (1K blocks: size 976236212, used 752993072, avail 218901648) shows 78%.
        assert_eq!(disk_percent(976236212, 976236212 - 752993072, 218901648), Some(78));
        assert_eq!(disk_percent(0, 0, 0), None);
        assert_eq!(disk_percent(10, 20, 5), None);
    }

    #[test]
    fn temperature_in_millidegrees() {
        assert_eq!(parse_temp("69802\n"), Some(70));
        assert_eq!(parse_temp("500"), Some(1));
        assert_eq!(parse_temp("-1500"), Some(-2));
        assert_eq!(parse_temp(""), None);
    }

    #[test]
    fn missing_values_show_a_dash() {
        assert_eq!(pct_text(None), DASH);
        assert_eq!(temp_text(None), DASH);
        assert_eq!(pct_text(Some(17)), "17%");
        assert_eq!(temp_text(Some(70)), "70°C");
    }

    /// Reads this machine's real files (Linux only); prints values for a sanity check.
    #[test]
    fn reads_this_host() {
        let prev = Cell::new(read_cpu());
        std::thread::sleep(Duration::from_millis(200));
        let r = sample(&prev);
        eprintln!("host reading: {r:?}");
        assert!(r.memory.is_some_and(|m| m <= 100));
        assert!(r.disk.is_some_and(|d| d <= 100));
        assert!(r.cpu.is_some_and(|c| c <= 100));
    }
}
