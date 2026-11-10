//! Background fetching for widgets: one thread running a current-thread tokio runtime
//! and one shared `reqwest::Client` (rustls + ring).
//!
//! Each job runs `fetch` every `interval`. Every attempt has a hard total timeout and
//! runs as its own task, so a hang or panic is contained to that attempt. Failures retry
//! with capped exponential backoff; the last good value is kept and re-sent with each
//! update, and `stale` turns on after repeated failures. Updates reach the widget on the
//! UI thread via `slint::invoke_from_event_loop`. The thread starts with the first job.

#![allow(dead_code)] // no Phase 1 widget fetches yet; Phase 2 widgets use all of this

use std::cell::{Cell, OnceCell};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use std::time::Duration;

use serde::de::DeserializeOwned;
use tokio::runtime::Handle;

use crate::AppWindow;

/// Hard limit for a whole attempt (connect, request, body, parsing).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// First retry delay after a failure; doubles per consecutive failure.
const RETRY_BASE: Duration = Duration::from_secs(10);
/// Longest retry delay (and never longer than the job's own interval).
const RETRY_CAP: Duration = Duration::from_secs(15 * 60);
/// Consecutive failures after which an update is marked stale.
const STALE_AFTER: u32 = 3;
/// Job start offsets on boot: 0, 0.75s, 1.5s, ... (wrapping after 12 jobs).
const STAGGER_STEP: Duration = Duration::from_millis(750);
const STAGGER_SLOTS: u32 = 12;

#[derive(Clone, Copy, Debug)]
pub struct Job {
    pub name: &'static str,
    pub interval: Duration,
    pub timeout: Duration,
}

impl Job {
    pub fn new(name: &'static str, interval: Duration) -> Job {
        Job {
            name,
            interval,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn timeout(self, timeout: Duration) -> Job {
        Job { timeout, ..self }
    }
}

/// What a widget receives after every attempt.
#[derive(Debug)]
pub struct Update<T> {
    /// Latest successful result, kept across failures. None until the first success.
    pub value: Option<Arc<T>>,
    /// The latest attempt failed `STALE_AFTER` or more times in a row.
    pub stale: bool,
    /// Error of the latest attempt, None if it succeeded.
    pub error: Option<String>,
}

impl<T> Clone for Update<T> {
    fn clone(&self) -> Self {
        Update {
            value: self.value.clone(),
            stale: self.stale,
            error: self.error.clone(),
        }
    }
}

/// Consecutive-failure bookkeeping (pure, unit-tested).
#[derive(Debug, Default)]
pub struct Health {
    failures: u32,
}

impl Health {
    pub fn succeeded(&mut self) {
        self.failures = 0;
    }

    /// Record a failure; returns how long to wait before retrying.
    pub fn failed(&mut self, interval: Duration) -> Duration {
        self.failures = self.failures.saturating_add(1);
        retry_delay(self.failures, interval)
    }

    pub fn stale(&self) -> bool {
        self.failures >= STALE_AFTER
    }
}

/// 10s, 20s, 40s, ... capped at the job interval and at 15 minutes (but at least 10s).
pub fn retry_delay(failures: u32, interval: Duration) -> Duration {
    let cap = interval.clamp(RETRY_BASE, RETRY_CAP);
    let exp = failures.saturating_sub(1).min(16);
    RETRY_BASE.saturating_mul(1 << exp).min(cap)
}

struct Runtime {
    handle: Handle,
    http: reqwest::Client,
}

/// Owned by the UI thread; widgets reach it through `Ctx::fetch`.
#[derive(Default)]
pub struct Fetcher {
    runtime: OnceCell<Option<Runtime>>,
    next_slot: Cell<u32>,
}

impl Fetcher {
    pub fn new() -> Fetcher {
        Fetcher::default()
    }

    /// Run `fetch` on the job's schedule and hand every outcome to `apply` on the UI thread.
    /// `fetch` gets the shared HTTP client; return `Err` with a short message on failure
    /// (use `get_json`/`clean` so URLs, which may contain keys, never reach the logs).
    pub fn spawn<T, F, Fut, A>(&self, job: Job, fetch: F, ui: slint::Weak<AppWindow>, apply: A)
    where
        T: Send + Sync + 'static,
        F: Fn(reqwest::Client) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, String>> + Send + 'static,
        A: Fn(&AppWindow, &Update<T>) + Send + Sync + 'static,
    {
        self.spawn_paced(job, fetch, move |_: &T| job.interval, ui, apply);
    }

    /// Like `spawn`, but after a success the next fetch is `pace(&value)` later instead of
    /// `job.interval` (e.g. poll faster while something is playing). Failures back off as usual.
    pub fn spawn_paced<T, F, Fut, P, A>(
        &self,
        job: Job,
        fetch: F,
        pace: P,
        ui: slint::Weak<AppWindow>,
        apply: A,
    ) where
        T: Send + Sync + 'static,
        F: Fn(reqwest::Client) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<T, String>> + Send + 'static,
        P: Fn(&T) -> Duration + Send + 'static,
        A: Fn(&AppWindow, &Update<T>) + Send + Sync + 'static,
    {
        let Some(rt) = self.runtime.get_or_init(start_runtime) else {
            eprintln!("fetch {}: no fetch runtime; job not started", job.name);
            return;
        };
        let slot = self.next_slot.get();
        self.next_slot.set(slot + 1);
        let delay = STAGGER_STEP * (slot % STAGGER_SLOTS);

        let http = rt.http.clone();
        let apply = Arc::new(apply);
        let deliver = move |update: Update<T>| {
            let apply = apply.clone();
            let _ =
                ui.upgrade_in_event_loop(move |ui| crate::guard(job.name, || apply(&ui, &update)));
        };
        rt.handle.spawn(run_paced(
            job,
            delay,
            move || fetch(http.clone()),
            pace,
            deliver,
        ));
    }
}

fn start_runtime() -> Option<Runtime> {
    let (tx, rx) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new()
        .name("fetch".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .max_blocking_threads(4)
                .build();
            let http = reqwest::Client::builder()
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(DEFAULT_TIMEOUT)
                .pool_idle_timeout(Duration::from_secs(30))
                .user_agent(concat!("mmm-native/", env!("CARGO_PKG_VERSION")))
                .build();
            match (rt, http) {
                (Ok(rt), Ok(http)) => {
                    let _ = tx.send(Some(Runtime {
                        handle: rt.handle().clone(),
                        http,
                    }));
                    rt.block_on(std::future::pending::<()>());
                }
                (rt, http) => {
                    eprintln!(
                        "fetch: cannot start (runtime ok: {}, http client ok: {})",
                        rt.is_ok(),
                        http.is_ok()
                    );
                    let _ = tx.send(None);
                }
            }
        });
    match spawned {
        Ok(_) => rx.recv().ok().flatten(),
        Err(e) => {
            eprintln!("fetch: cannot spawn thread: {e}");
            None
        }
    }
}

/// `run_paced` at the job's fixed interval.
#[cfg(test)]
async fn run_job<T, F, Fut>(job: Job, start_delay: Duration, fetch: F, deliver: impl Fn(Update<T>))
where
    T: Send + Sync + 'static,
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, String>> + Send + 'static,
{
    run_paced(job, start_delay, fetch, move |_: &T| job.interval, deliver).await
}

/// The job loop. Generic over `deliver` so it can be tested without a UI. After a
/// success it waits `pace(&value)` (at least 1s); after a failure, the backoff delay.
async fn run_paced<T, F, Fut>(
    job: Job,
    start_delay: Duration,
    fetch: F,
    pace: impl Fn(&T) -> Duration,
    deliver: impl Fn(Update<T>),
) where
    T: Send + Sync + 'static,
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, String>> + Send + 'static,
{
    tokio::time::sleep(start_delay).await;
    let mut health = Health::default();
    let mut last: Option<Arc<T>> = None;
    loop {
        let result = match catch_unwind(AssertUnwindSafe(&fetch)) {
            // Its own task: a panic or hang stays inside this attempt.
            Ok(fut) => match tokio::spawn(tokio::time::timeout(job.timeout, fut)).await {
                Ok(Ok(r)) => r,
                Ok(Err(_)) => Err(format!("timed out after {}s", job.timeout.as_secs())),
                Err(e) if e.is_panic() => Err("panicked".into()),
                Err(_) => Err("cancelled".into()),
            },
            Err(_) => Err("panicked".into()),
        };
        let wait = match result {
            Ok(value) => {
                if health.stale() {
                    eprintln!("fetch {}: recovered", job.name);
                }
                health.succeeded();
                let next = catch_unwind(AssertUnwindSafe(|| pace(&value))).unwrap_or(job.interval);
                last = Some(Arc::new(value));
                deliver(Update {
                    value: last.clone(),
                    stale: false,
                    error: None,
                });
                next.max(Duration::from_secs(1))
            }
            Err(error) => {
                let wait = health.failed(job.interval);
                eprintln!("fetch {}: {error} (retry in {}s)", job.name, wait.as_secs());
                deliver(Update {
                    value: last.clone(),
                    stale: health.stale(),
                    error: Some(error),
                });
                wait
            }
        };
        tokio::time::sleep(wait).await;
    }
}

/// Send a request and parse a JSON body; errors are URL-free (see `clean`).
pub async fn get_json<T: DeserializeOwned>(request: reqwest::RequestBuilder) -> Result<T, String> {
    let response = request.send().await.map_err(clean)?;
    let response = response.error_for_status().map_err(clean)?;
    response.json::<T>().await.map_err(clean)
}

/// A reqwest error as text without its URL (URLs can carry API keys), plus its causes.
pub fn clean(e: reqwest::Error) -> String {
    let e = e.without_url();
    let mut text = e.to_string();
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        text = format!("{text}: {cause}");
        source = cause.source();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    const MIN: Duration = Duration::from_secs(60);

    #[test]
    fn backoff_doubles_and_is_capped() {
        let s = Duration::from_secs;
        let hour = 60 * MIN;
        let delays: Vec<_> = (1..=9).map(|n| retry_delay(n, hour).as_secs()).collect();
        assert_eq!(delays, [10, 20, 40, 80, 160, 320, 640, 900, 900]);
        // Never longer than the job's own interval...
        assert_eq!(retry_delay(6, 5 * MIN), 5 * MIN);
        // ...but never shorter than the base, even for fast-polling jobs.
        assert_eq!(retry_delay(1, s(5)), s(10));
        assert_eq!(retry_delay(7, s(5)), s(10));
        // No overflow after very many failures.
        assert_eq!(retry_delay(u32::MAX, hour), s(900));
    }

    #[test]
    fn stale_after_repeated_failures_and_clears_on_success() {
        let mut h = Health::default();
        h.failed(MIN);
        h.failed(MIN);
        assert!(!h.stale());
        h.failed(MIN);
        assert!(h.stale());
        h.succeeded();
        assert!(!h.stale());
        assert_eq!(
            h.failed(MIN),
            RETRY_BASE,
            "backoff restarts after a success"
        );
    }

    type Log = Arc<Mutex<Vec<(Option<u32>, bool, Option<String>)>>>;

    fn recorder() -> (Log, impl Fn(Update<u32>)) {
        let log: Log = Arc::default();
        let l = log.clone();
        (log, move |u: Update<u32>| {
            l.lock()
                .unwrap()
                .push((u.value.map(|v| *v), u.stale, u.error))
        })
    }

    /// Hanging and panicking jobs time out / fail on their own while a healthy job
    /// keeps its schedule. Uses tokio's paused clock, so it runs instantly.
    #[tokio::test(start_paused = true)]
    async fn bad_jobs_do_not_affect_others() {
        let (good_log, good) = recorder();
        let (hang_log, hang) = recorder();
        let (panic_log, panicky) = recorder();
        let n = Arc::new(AtomicU32::new(0));
        tokio::spawn(run_job(
            Job::new("good", MIN),
            Duration::ZERO,
            move || {
                let n = n.fetch_add(1, Ordering::SeqCst);
                async move { Ok(n) }
            },
            good,
        ));
        tokio::spawn(run_job(
            Job::new("hang", MIN),
            Duration::ZERO,
            std::future::pending,
            hang,
        ));
        tokio::spawn(run_job(
            Job::new("panic", MIN),
            Duration::ZERO,
            || async { panic!("boom") },
            panicky,
        ));

        tokio::time::sleep(5 * MIN + Duration::from_secs(1)).await;

        let good = good_log.lock().unwrap().clone();
        assert_eq!(good.len(), 6, "one success per minute: {good:?}");
        assert!(good
            .iter()
            .enumerate()
            .all(|(i, u)| *u == (Some(i as u32), false, None)));

        let hang = hang_log.lock().unwrap().clone();
        assert!(
            hang.len() >= 3
                && hang
                    .iter()
                    .all(|u| u.2.as_deref() == Some("timed out after 20s"))
        );
        assert!(
            !hang[1].1 && hang[2].1,
            "stale from the third failure: {hang:?}"
        );

        let panic = panic_log.lock().unwrap().clone();
        assert!(panic.len() >= 3 && panic.iter().all(|u| u.2.as_deref() == Some("panicked")));
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_last_good_value_through_failures() {
        let (log, deliver) = recorder();
        let calls = Arc::new(AtomicU32::new(0));
        tokio::spawn(run_job(
            Job::new("flaky", MIN),
            Duration::ZERO,
            move || {
                let call = calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    if call == 0 {
                        Ok(7)
                    } else {
                        Err("down".to_string())
                    }
                }
            },
            deliver,
        ));
        tokio::time::sleep(3 * MIN).await;
        let log = log.lock().unwrap().clone();
        assert_eq!(log[0], (Some(7), false, None));
        assert!(log[1..]
            .iter()
            .all(|u| u.0 == Some(7) && u.2.as_deref() == Some("down")));
        assert!(
            log.last().unwrap().1,
            "stale after repeated failures: {log:?}"
        );
    }

    /// `pace` picks the wait after each success from the value just fetched.
    #[tokio::test(start_paused = true)]
    async fn paced_job_waits_what_pace_returns() {
        let (log, deliver) = recorder();
        let n = Arc::new(AtomicU32::new(0));
        tokio::spawn(run_paced(
            Job::new("paced", MIN),
            Duration::ZERO,
            move || {
                let n = n.fetch_add(1, Ordering::SeqCst);
                async move { Ok(n) }
            },
            // Values 0..=2 come 5s apart, then the job slows to one per minute.
            |v: &u32| if *v < 2 { Duration::from_secs(5) } else { MIN },
            deliver,
        ));
        tokio::time::sleep(Duration::from_secs(11)).await;
        assert_eq!(log.lock().unwrap().len(), 3, "0s, 5s, 10s");
        tokio::time::sleep(Duration::from_secs(55)).await;
        assert_eq!(log.lock().unwrap().len(), 3, "next one is due at 70s");
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert_eq!(log.lock().unwrap().len(), 4);
    }
}
