//! systemd readiness and watchdog notifications, without libsystemd.
//!
//! Protocol (sd_notify(3), https://www.freedesktop.org/software/systemd/man/latest/sd_notify.html):
//! send a datagram such as "READY=1" to the AF_UNIX socket named by `$NOTIFY_SOCKET`.
//! A name starting with "/" is a file system path; one starting with "@" is a Linux
//! abstract-namespace socket (the "@" stands for the leading NUL byte). Anything else
//! (e.g. `vsock:`) is not supported here and turns notifications off.
//!
//! Watchdog (sd_watchdog_enabled(3)): only when `$WATCHDOG_USEC` is set and
//! `$WATCHDOG_PID` is unset or our PID; systemd recommends a keep-alive every half of
//! that time. `watchdog()` is called from the UI thread's clock tick, so a frozen UI
//! stops the keep-alives and systemd restarts the service. It sends at most once per
//! `min(WATCHDOG_USEC / 2, 10s)`.
//!
//! Everything is a no-op when `$NOTIFY_SOCKET` is unset (running by hand). The
//! variables are deliberately left in the environment: the config watcher re-execs
//! this process with the same PID, and the new image must notify too.

use std::cell::RefCell;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Upper bound on the keep-alive interval (WatchdogSec=90 needs one every 45s).
const MAX_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, PartialEq)]
pub enum Addr {
    Path(PathBuf),
    /// Abstract name without the leading "@".
    Abstract(Vec<u8>),
}

/// `$NOTIFY_SOCKET` as an address; None for empty or unsupported values.
pub fn parse_addr(value: &OsStr) -> Option<Addr> {
    let b = value.as_bytes();
    match b.first()? {
        b'/' => Some(Addr::Path(PathBuf::from(value))),
        b'@' if b.len() > 1 => Some(Addr::Abstract(b[1..].to_vec())),
        _ => None,
    }
}

/// Keep-alive interval from `$WATCHDOG_USEC` / `$WATCHDOG_PID`; None = no watchdog.
pub fn watchdog_interval(usec: Option<&str>, pid: Option<&str>, my_pid: u32) -> Option<Duration> {
    if let Some(pid) = pid {
        if pid.trim().parse::<u32>().ok() != Some(my_pid) {
            return None;
        }
    }
    let usec = usec?.trim().parse::<u64>().ok().filter(|u| *u > 0)?;
    Some((Duration::from_micros(usec) / 2).min(MAX_INTERVAL))
}

fn send(addr: &Addr, message: &str) -> std::io::Result<()> {
    let target = match addr {
        Addr::Path(p) => SocketAddr::from_pathname(p)?,
        Addr::Abstract(name) => {
            use std::os::linux::net::SocketAddrExt;
            SocketAddr::from_abstract_name(name)?
        }
    };
    let socket = UnixDatagram::unbound()?;
    socket.send_to_addr(message.as_bytes(), &target)?;
    Ok(())
}

struct State {
    addr: Option<Addr>,
    interval: Option<Duration>,
    last: Option<Instant>,
    warned: bool,
}

thread_local! {
    // Only the UI thread notifies, so no locking.
    static STATE: RefCell<State> = RefCell::new(State {
        addr: std::env::var_os("NOTIFY_SOCKET").and_then(|v| parse_addr(&v)),
        interval: watchdog_interval(
            std::env::var("WATCHDOG_USEC").ok().as_deref(),
            std::env::var("WATCHDOG_PID").ok().as_deref(),
            std::process::id(),
        ),
        last: None,
        warned: false,
    });
}

fn notify(s: &mut State, message: &str) {
    let Some(addr) = &s.addr else { return };
    if let Err(e) = send(addr, message) {
        if !std::mem::replace(&mut s.warned, true) {
            eprintln!("sd_notify: cannot notify systemd ({e})");
        }
    }
}

/// Tell systemd the window is up (Type=notify).
pub fn ready() {
    STATE.with_borrow_mut(|s| {
        if s.addr.is_none() && std::env::var_os("NOTIFY_SOCKET").is_some() {
            eprintln!("sd_notify: unsupported NOTIFY_SOCKET; not notifying");
        }
        notify(s, "READY=1");
    });
}

/// Keep-alive for WatchdogSec=; call often (every second is fine), it rate-limits.
pub fn watchdog() {
    STATE.with_borrow_mut(|s| {
        let Some(interval) = s.interval else { return };
        let now = Instant::now();
        // A little slack so a 1s tick does not slip to every 11s.
        if s.last
            .is_some_and(|t| now.duration_since(t) + Duration::from_millis(500) < interval)
        {
            return;
        }
        s.last = Some(now);
        notify(s, "WATCHDOG=1");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_socket_addresses() {
        let p = |s: &str| parse_addr(OsStr::new(s));
        assert_eq!(
            p("/run/systemd/notify"),
            Some(Addr::Path("/run/systemd/notify".into()))
        );
        assert_eq!(
            p("@/org/freedesktop/systemd1/notify/123"),
            Some(Addr::Abstract(
                b"/org/freedesktop/systemd1/notify/123".to_vec()
            ))
        );
        assert_eq!(p("@x"), Some(Addr::Abstract(b"x".to_vec())));
        assert_eq!(p("@"), None);
        assert_eq!(p(""), None);
        assert_eq!(p("relative/path"), None);
        assert_eq!(p("vsock:2:1234"), None);
    }

    #[test]
    fn watchdog_intervals() {
        let s = Duration::from_secs;
        assert_eq!(watchdog_interval(Some("90000000"), None, 7), Some(s(10)));
        assert_eq!(
            watchdog_interval(Some("90000000"), Some("7"), 7),
            Some(s(10))
        );
        assert_eq!(watchdog_interval(Some("6000000"), None, 7), Some(s(3)));
        assert_eq!(
            watchdog_interval(Some("90000000"), Some("8"), 7),
            None,
            "meant for another process"
        );
        assert_eq!(watchdog_interval(None, None, 7), None);
        assert_eq!(watchdog_interval(Some("0"), None, 7), None);
        assert_eq!(watchdog_interval(Some("soon"), None, 7), None);
    }

    #[test]
    fn sends_a_datagram_to_a_path_and_an_abstract_socket() {
        let dir = std::env::temp_dir().join(format!("mmm-sdnotify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("notify");
        let _ = std::fs::remove_file(&path);
        let server = UnixDatagram::bind(&path).unwrap();
        send(&Addr::Path(path.clone()), "READY=1").unwrap();
        let mut buf = [0u8; 64];
        let n = server.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"READY=1");
        let _ = std::fs::remove_dir_all(&dir);

        use std::os::linux::net::SocketAddrExt;
        let name = format!("mmm-sdnotify-test-{}", std::process::id());
        let server =
            UnixDatagram::bind_addr(&SocketAddr::from_abstract_name(name.as_bytes()).unwrap())
                .unwrap();
        send(&Addr::Abstract(name.into_bytes()), "WATCHDOG=1").unwrap();
        let n = server.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"WATCHDOG=1");
    }
}
