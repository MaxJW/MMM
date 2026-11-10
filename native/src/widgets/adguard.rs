//! AdGuard Home DNS stats: port of src/components/adguard/api.ts + component.svelte.
//! `GET <url>/control/stats` with `Authorization: Basic <token>` (the token is already
//! base64 of `user:password`), every 5 minutes. The URL is usually plain http on the
//! LAN; the shared client allows that (reqwest does not force https by default).
//!
//! API reference: AdGuard Home's openapi/openapi.yaml
//! (https://github.com/AdguardTeam/AdGuardHome/blob/master/openapi/openapi.yaml):
//! server url `/control`, `GET /stats` returns `Stats`, whose `num_dns_queries` and
//! `num_blocked_filtering` are integers; security scheme `basicAuth` (http, basic).

use serde::Deserialize;
use slint::{ComponentHandle, SharedString};

use super::{set_if_changed, Ctx};
use crate::fetch::{self, Job};
use crate::{timing, AdguardData, AppWindow};

pub const ID: &str = "adguard";

#[derive(Deserialize, Default)]
#[serde(default)]
struct Settings {
    url: String,
    token: String,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Stats {
    num_dns_queries: u64,
    num_blocked_filtering: u64,
}

/// `<base>/control/stats`; a trailing slash on the configured URL is tolerated.
fn stats_url(base: &str) -> String {
    format!("{}/control/stats", base.trim().trim_end_matches('/'))
}

/// Blocked share in whole percent, rounded like `Math.round`; 0 when there were no queries.
fn block_rate(queries: u64, blocked: u64) -> u64 {
    if queries == 0 {
        0
    } else {
        (blocked as f64 * 100.0 / queries as f64).round() as u64
    }
}

/// `Intl.NumberFormat().format(n)` for an en locale: 63393 -> "63,393".
fn group(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The text shown for a failed fetch: `HTTP <status>` as the web handler reports it,
/// otherwise a short generic line (transport errors are long; they go to the log only).
fn shown_error(error: &str) -> &str {
    if error.starts_with("HTTP ") {
        error
    } else {
        "Failed to fetch Adguard stats"
    }
}

async fn fetch_stats(request: reqwest::RequestBuilder) -> Result<Stats, String> {
    let response = request.send().await.map_err(fetch::clean)?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }
    response.json::<Stats>().await.map_err(fetch::clean)
}

pub fn start(ctx: &Ctx) {
    let s: Settings = ctx.config_as();
    if s.url.trim().is_empty() || s.token.trim().is_empty() {
        let data = ctx.ui.global::<AdguardData>();
        data.set_error("Missing Adguard URL or token".into());
        data.set_loading(false);
        return;
    }
    let url = stats_url(&s.url);
    let auth = format!("Basic {}", s.token.trim());
    ctx.fetch.spawn(
        Job::new(ID, timing::FREQUENT),
        move |http| {
            let request = http
                .get(&url)
                .header(reqwest::header::AUTHORIZATION, &auth)
                .header(reqwest::header::ACCEPT, "application/json");
            fetch_stats(request)
        },
        ctx.ui.as_weak(),
        |ui, update| show(ui, update.value.as_deref(), update.error.as_deref()),
    );
}

fn show(ui: &AppWindow, stats: Option<&Stats>, error: Option<&str>) {
    let data = ui.global::<AdguardData>();
    match (stats, error) {
        (Some(s), _) => {
            let rate = block_rate(s.num_dns_queries, s.num_blocked_filtering);
            let text = |t: String| SharedString::from(t);
            set_if_changed(data.get_queries(), text(group(s.num_dns_queries)), |v| data.set_queries(v));
            set_if_changed(data.get_blocked(), text(group(s.num_blocked_filtering)), |v| data.set_blocked(v));
            set_if_changed(data.get_rate_text(), text(format!("{rate}%")), |v| data.set_rate_text(v));
            set_if_changed(data.get_rate(), rate as f32 / 100.0, |v| data.set_rate(v));
            set_if_changed(data.get_error(), SharedString::new(), |v| data.set_error(v));
        }
        (None, Some(e)) => {
            set_if_changed(data.get_error(), SharedString::from(shown_error(e)), |v| data.set_error(v));
        }
        (None, None) => {}
    }
    set_if_changed(data.get_loading(), false, |v| data.set_loading(v));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed from the `Stats` schema in AdGuard Home's openapi.yaml (fields the
    /// handler ignores are kept to check they are tolerated).
    const FIXTURE: &str = r#"{
        "time_units": "hours",
        "num_dns_queries": 63393,
        "num_blocked_filtering": 14975,
        "num_replaced_safebrowsing": 0,
        "num_replaced_safesearch": 0,
        "num_replaced_parental": 0,
        "avg_processing_time": 0.0123,
        "top_queried_domains": [{"example.org": 120}],
        "top_clients": [{"192.168.1.20": 3000}],
        "top_blocked_domains": [{"ads.example": 50}],
        "dns_queries": [1, 2, 3],
        "blocked_filtering": [0, 1, 0],
        "replaced_safebrowsing": [0, 0, 0],
        "replaced_parental": [0, 0, 0]
    }"#;

    #[test]
    fn parses_stats_fixture() {
        let s: Stats = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(s, Stats { num_dns_queries: 63393, num_blocked_filtering: 14975 });
        assert_eq!(block_rate(s.num_dns_queries, s.num_blocked_filtering), 24);
    }

    #[test]
    fn missing_or_wrong_fields_are_errors() {
        assert!(serde_json::from_str::<Stats>(r#"{"num_dns_queries": 1}"#).is_err());
        assert!(serde_json::from_str::<Stats>(r#"{"num_dns_queries": "x", "num_blocked_filtering": 0}"#).is_err());
        assert!(serde_json::from_str::<Stats>("<html>login</html>").is_err());
    }

    #[test]
    fn block_rate_rounds_and_handles_zero() {
        assert_eq!(block_rate(0, 0), 0);
        assert_eq!(block_rate(200, 1), 1); // 0.5 rounds up, like Math.round
        assert_eq!(block_rate(1000, 4), 0);
        assert_eq!(block_rate(3, 3), 100);
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(group(0), "0");
        assert_eq!(group(999), "999");
        assert_eq!(group(1000), "1,000");
        assert_eq!(group(63393), "63,393");
        assert_eq!(group(1234567), "1,234,567");
    }

    #[test]
    fn url_and_error_text() {
        assert_eq!(stats_url("http://192.168.1.100:3000"), "http://192.168.1.100:3000/control/stats");
        assert_eq!(stats_url(" http://h:3000/ "), "http://h:3000/control/stats");
        assert_eq!(shown_error("HTTP 401"), "HTTP 401");
        assert_eq!(shown_error("timed out after 20s"), "Failed to fetch Adguard stats");
    }
}
