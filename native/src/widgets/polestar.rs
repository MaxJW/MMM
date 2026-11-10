//! Polestar car: battery %, range, odometer and charging state. Port of
//! src/components/polestar/{api.ts,component.svelte}.
//!
//! The login is the web app's scripted OIDC authorization-code + PKCE flow against
//! Polestar ID (PingFederate), which api.ts took from the reverse-engineered pypolestar
//! project (https://github.com/pypolestar/pypolestar). It is ported step for step, with
//! the same client id, redirect URI, scope, user agent, form fields and redirect
//! handling; none of it is documented by Polestar, so do not "fix" it from guesswork.
//!
//! Cadence: api.ts caches a good result for an hour (INFREQUENT) and the component polls
//! every 5 minutes, so upstream is hit about hourly; here the job simply runs hourly.
//! The access token is kept in memory until 60s before it expires. A failed (or
//! timed-out) login blocks further logins for `LOGIN_RETRY_FLOOR`, on top of the
//! scheduler's backoff, so bad credentials cannot hammer the identity provider.
//! Credentials, tokens, cookies and the VIN are never logged; error texts have them
//! redacted.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use base64::Engine;
use reqwest::header::{
    HeaderMap, HeaderValue, AUTHORIZATION, COOKIE, LOCATION, SET_COOKIE, USER_AGENT,
};
use reqwest::redirect::Policy;
use reqwest::{RequestBuilder, Response, Url};
use serde_json::{json, Value};
use slint::{ComponentHandle, SharedString};

use super::{set_if_changed, Ctx};
use crate::config::Area;
use crate::fetch::{self, Job, Update};
use crate::{timing, AppWindow, PolestarData};

pub const ID: &str = "polestar";

// CONSTANTS in api.ts.
const OIDC_PROVIDER_BASE_URL: &str = "https://polestarid.eu.polestar.com";
const OIDC_CLIENT_ID: &str = "l3oopkc_10";
const OIDC_REDIRECT_URI: &str = "https://www.polestar.com/sign-in-callback";
const OIDC_SCOPE: &str = "openid profile email customer:attributes";
const API_MYSTAR_V2_URL: &str = "https://pc-api.polestar.com/eu-north-1/mystar-v2/";
const POLESTAR_USER_AGENT: &str = "Polestar/4.0.0 (iPhone; iOS 17.0; Scale/3.00)";

// NOTE (from api.ts): the battery `chargingStatus` field was renamed to `chargingStatusV2`
// upstream (pypolestar). Requesting the old field name is a GraphQL validation error that
// fails the whole query, so keep this in sync with the schema.
const TELEMATICS_V2: &str = "
    query CarTelematicsV2($vins: [String!]!) {
        carTelematicsV2(vins: $vins) {
            battery {
                vin
                batteryChargeLevelPercentage
                chargingStatusV2
                estimatedChargingTimeToFullMinutes
                estimatedDistanceToEmptyKm
            }
            odometer {
                vin
                odometerMeters
            }
        }
    }
  ";

/// Login (up to ~10 sequential requests) plus the query: more than the default 20s.
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(60);
/// Redirect steps followed after posting the credentials (api.ts: 5).
const LOGIN_STEPS: usize = 5;
/// No new login within this long of one that failed or never finished. 5 minutes is
/// the web component's poll cadence, i.e. the fastest the web app ever retried a login.
const LOGIN_RETRY_FLOOR: Duration = timing::FREQUENT;
/// Tokens are reused until this long before `expires_in` runs out (api.ts: 60s).
const TOKEN_MARGIN_SECS: f64 = 60.0;
const MAX_BODY: usize = 2 << 20;

const MISSING_CONFIG: &str = "Missing Polestar credentials (username, password, vin)";
const TOKEN_REJECTED: &str = "Token expired or invalid.";
const NO_DATA: &str = "No telematics data found for this VIN";
const INCOMPLETE: &str = "Incomplete telematics data for this VIN";

/// `components.polestar`. No Debug: it holds the password.
#[derive(serde::Deserialize, Default)]
#[serde(default)]
struct Settings {
    username: String,
    password: String,
    vin: String,
}

/// What the widget shows (api.ts `PolestarData`, minus the unused charge time).
#[derive(Clone, Debug, PartialEq)]
pub struct Telematics {
    battery_pct: Option<f64>,
    charging_status: String,
    range_km: Option<f64>,
    odometer_m: Option<f64>,
}

pub fn start(ctx: &Ctx) {
    let ui = ctx.ui;
    let right_side = ctx
        .all
        .placements
        .iter()
        .find(|p| p.id == ID)
        .is_some_and(|p| {
            matches!(
                p.area,
                Area::TopRight | Area::MiddleRight | Area::BottomRight
            )
        });
    ui.global::<PolestarData>().set_right_side(right_side);

    let s: Settings = ctx.config_as();
    if s.username.is_empty() || s.password.is_empty() || s.vin.is_empty() {
        // api.ts returns this on every poll; there is nothing to fetch.
        ui.global::<PolestarData>().set_error(MISSING_CONFIG.into());
        return;
    }
    let creds = Arc::new(s);
    let session = Arc::new(Mutex::new(Session::default()));
    ctx.fetch.spawn(
        Job::new(ID, timing::INFREQUENT).timeout(ATTEMPT_TIMEOUT),
        move |http| {
            let (creds, session) = (creds.clone(), session.clone());
            async move {
                telematics(&http, &creds, &session)
                    .await
                    .map_err(|e| redact(&e, &creds))
            }
        },
        ui.as_weak(),
        show,
    );
}

fn show(ui: &AppWindow, update: &Update<Telematics>) {
    let d = ui.global::<PolestarData>();
    if let Some(t) = &update.value {
        let s = SharedString::from;
        set_if_changed(d.get_battery(), s(battery_text(t.battery_pct)), |v| {
            d.set_battery(v)
        });
        set_if_changed(d.get_range(), s(range_text(t.range_km)), |v| d.set_range(v));
        set_if_changed(d.get_odometer(), s(odometer_text(t.odometer_m)), |v| {
            d.set_odometer(v)
        });
        set_if_changed(d.get_charging(), is_charging(&t.charging_status), |v| {
            d.set_charging(v)
        });
        set_if_changed(d.get_low(), t.battery_pct.is_some_and(|p| p <= 20.0), |v| {
            d.set_low(v)
        });
        set_if_changed(d.get_has_data(), true, |v| d.set_has_data(v));
    } else if let Some(e) = &update.error {
        // Only without data, like the web component (which shows the error instead).
        set_if_changed(d.get_error(), SharedString::from(e.as_str()), |v| {
            d.set_error(v)
        });
    }
}

// ---- Fetching ---------------------------------------------------------------------

/// State kept across attempts (the module-level caches of api.ts).
#[derive(Default)]
struct Session {
    endpoints: Option<Endpoints>,
    token: Option<Token>,
    /// Start of the latest login that has not succeeded, and its error if it finished.
    unfinished_login: Option<(Instant, Option<String>)>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
struct Endpoints {
    authorization_endpoint: String,
    token_endpoint: String,
}

struct Token {
    access: String,
    expires: Instant,
}

fn lock(session: &Mutex<Session>) -> MutexGuard<'_, Session> {
    session.lock().unwrap_or_else(|e| e.into_inner())
}

enum GqlError {
    Unauthenticated,
    Other(String),
}

impl From<String> for GqlError {
    fn from(e: String) -> Self {
        GqlError::Other(e)
    }
}

/// `GET` in api.ts: cached token or login, the query, and one re-login if a cached
/// token is rejected.
async fn telematics(
    http: &reqwest::Client,
    creds: &Settings,
    session: &Mutex<Session>,
) -> Result<Telematics, String> {
    let mut browser = Browser::default();
    let cached = lock(session)
        .token
        .as_ref()
        .filter(|t| t.expires > Instant::now())
        .map(|t| t.access.clone());
    let from_cache = cached.is_some();
    let token = match cached {
        Some(t) => t,
        None => login(http, &mut browser, creds, session).await?,
    };
    match query(http, &mut browser, &token, &creds.vin).await {
        Ok(t) => Ok(t),
        Err(GqlError::Other(e)) => Err(e),
        Err(GqlError::Unauthenticated) => {
            lock(session).token = None;
            if !from_cache {
                // A token minted seconds ago was refused; another login will not help.
                return Err(TOKEN_REJECTED.into());
            }
            eprintln!("polestar: token expired, logging in again");
            let token = login(http, &mut browser, creds, session).await?;
            query(http, &mut browser, &token, &creds.vin)
                .await
                .map_err(|e| match e {
                    GqlError::Unauthenticated => TOKEN_REJECTED.into(),
                    GqlError::Other(e) => e,
                })
        }
    }
}

/// Log in unless a recent login failed; caches the token on success.
async fn login(
    http: &reqwest::Client,
    browser: &mut Browser,
    creds: &Settings,
    session: &Mutex<Session>,
) -> Result<String, String> {
    let started = Instant::now();
    {
        let mut s = lock(session);
        if let Some((at, error)) = &s.unfinished_login {
            let ago = at.elapsed();
            if ago < LOGIN_RETRY_FLOOR {
                eprintln!(
                    "polestar: last login failed {}s ago; not retrying yet",
                    ago.as_secs()
                );
                return Err(error
                    .clone()
                    .unwrap_or_else(|| "Polestar login did not finish".into()));
            }
        }
        // Stays set if this attempt is cancelled by the job timeout.
        s.unfinished_login = Some((started, None));
    }
    let result = login_flow(http, browser, creds, session).await;
    let mut s = lock(session);
    match result {
        Ok((access, ttl)) => {
            s.unfinished_login = None;
            s.token = Some(Token {
                access: access.clone(),
                expires: Instant::now() + ttl,
            });
            Ok(access)
        }
        Err(e) => {
            s.unfinished_login = Some((started, Some(e.clone())));
            Err(e)
        }
    }
}

/// `PolestarClient.login` in api.ts. Returns the access token and how long to reuse it.
async fn login_flow(
    http: &reqwest::Client,
    browser: &mut Browser,
    creds: &Settings,
    session: &Mutex<Session>,
) -> Result<(String, Duration), String> {
    let cached = lock(session).endpoints.clone();
    let endpoints = match cached {
        Some(e) => e,
        None => {
            let url = format!("{OIDC_PROVIDER_BASE_URL}/.well-known/openid-configuration");
            let resp = browser.send(http.get(url)).await?;
            if !resp.status().is_success() {
                return Err("Failed to fetch OIDC config".into());
            }
            let e = parse_endpoints(&read_body(resp).await?)?;
            lock(session).endpoints = Some(e.clone());
            e
        }
    };

    let state = random_token()?;
    let code_verifier = random_token()?;
    let code_challenge = code_challenge(&code_verifier);

    // 1 + 2. Authorization request (redirects followed, as fetch does); the login page
    // names the resume path.
    let auth = http.get(&endpoints.authorization_endpoint).query(&[
        ("client_id", OIDC_CLIENT_ID),
        ("redirect_uri", OIDC_REDIRECT_URI),
        ("response_type", "code"),
        ("scope", OIDC_SCOPE),
        ("state", &state),
        ("code_challenge", &code_challenge),
        ("code_challenge_method", "S256"),
        ("response_mode", "query"),
    ]);
    let page = read_body(browser.send(auth).await?).await?;
    let page = String::from_utf8_lossy(&page);
    let resume = resume_path(&page).ok_or("Could not extract resume path from login page.")?;

    // 3. Post the credentials, without following redirects from here on.
    let login_url = resolve(resume)?;
    let manual = reqwest::Client::builder()
        .redirect(Policy::none())
        .connect_timeout(fetch::CONNECT_TIMEOUT)
        .timeout(fetch::DEFAULT_TIMEOUT)
        .build()
        .map_err(fetch::clean)?;
    let form = [
        ("pf.username", creds.username.as_str()),
        ("pf.pass", creds.password.as_str()),
    ];
    let mut resp = browser
        .send(manual.post(login_url.clone()).form(&form))
        .await?;

    // 4. Follow redirects until one carries the code.
    let mut code = None;
    for _ in 0..LOGIN_STEPS {
        let status = resp.status();
        if !status.is_redirection() {
            let text = read_body(resp).await?;
            return Err(login_page_error(
                status.as_u16(),
                &String::from_utf8_lossy(&text),
            ));
        }
        let location = resp
            .headers()
            .get(LOCATION)
            .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned());
        read_body(resp).await?;
        let location = location.ok_or("Redirect with no Location header")?;
        resp = match next_step(&location)? {
            Step::Code(c) => {
                code = Some(c);
                break;
            }
            Step::Submit(uid) => {
                let form = [("pf.submit", "true"), ("subject", uid.as_str())];
                browser
                    .send(manual.post(login_url.clone()).form(&form))
                    .await?
            }
            Step::Follow(url) => browser.send(manual.get(url)).await?,
        };
    }
    let code = code.ok_or("Failed to retrieve authorization code.")?;

    // 5. Exchange the code for a token.
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", OIDC_CLIENT_ID),
        ("code", &code),
        ("redirect_uri", OIDC_REDIRECT_URI),
        ("code_verifier", &code_verifier),
    ];
    let resp = browser
        .send(http.post(&endpoints.token_endpoint).form(&form))
        .await?;
    let body = read_body(resp).await?;
    let body: Value =
        serde_json::from_slice(&body).map_err(|e| format!("Token response is not JSON ({e})"))?;
    parse_token(&body)
}

/// `queryGraphql` + `getTelematics` in api.ts.
async fn query(
    http: &reqwest::Client,
    browser: &mut Browser,
    token: &str,
    vin: &str,
) -> Result<Telematics, GqlError> {
    let mut bearer = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| GqlError::Other("Token Error: unusable access token".into()))?;
    bearer.set_sensitive(true);
    let request = http
        .post(API_MYSTAR_V2_URL)
        .header(AUTHORIZATION, bearer)
        .json(&json!({ "query": TELEMATICS_V2, "variables": { "vins": [vin] } }));
    let body = read_body(browser.send(request).await?).await?;
    // Parsed whatever the status, as api.ts does: GraphQL errors arrive in the body.
    let body: Value = serde_json::from_slice(&body)
        .map_err(|e| GqlError::Other(format!("GraphQL response is not JSON ({e})")))?;
    parse_telematics(&body, vin)
}

/// The `fetch` wrapper of api.ts: Polestar user agent, plus a cookie jar that lives for
/// one attempt and is sent with every request (not scoped by domain, like api.ts).
#[derive(Default)]
struct Browser {
    jar: CookieJar,
}

impl Browser {
    async fn send(&mut self, request: RequestBuilder) -> Result<Response, String> {
        let mut request = request.header(USER_AGENT, POLESTAR_USER_AGENT);
        let cookies = self.jar.header();
        if !cookies.is_empty() {
            let mut value = HeaderValue::from_str(&cookies)
                .map_err(|_| "Invalid cookie from server".to_string())?;
            value.set_sensitive(true);
            request = request.header(COOKIE, value);
        }
        let response = request.send().await.map_err(fetch::clean)?;
        self.jar.update(response.headers());
        Ok(response)
    }
}

/// Whole body, refusing anything over `MAX_BODY`.
async fn read_body(mut response: Response) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(fetch::clean)? {
        if body.len() + chunk.len() > MAX_BODY {
            return Err("Response too large".into());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

// ---- Pure helpers (unit-tested) ---------------------------------------------------

/// `SimpleCookieJar` in api.ts: name -> value in first-seen order, attributes ignored,
/// never expired. No Debug: values are session secrets.
#[derive(Default)]
struct CookieJar {
    cookies: Vec<(String, String)>,
}

impl CookieJar {
    fn update(&mut self, headers: &HeaderMap) {
        for value in headers.get_all(SET_COOKIE) {
            self.set(&String::from_utf8_lossy(value.as_bytes()));
        }
    }

    fn set(&mut self, set_cookie: &str) {
        let name_value = set_cookie.split(';').next().unwrap_or_default();
        if name_value.is_empty() {
            return;
        }
        let (name, value) = name_value.split_once('=').unwrap_or((name_value, ""));
        let (name, value) = (name.trim(), value.trim());
        match self.cookies.iter_mut().find(|(n, _)| n == name) {
            Some(cookie) => cookie.1 = value.to_string(),
            None => self.cookies.push((name.to_string(), value.to_string())),
        }
    }

    fn header(&self) -> String {
        let pairs: Vec<String> = self
            .cookies
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect();
        pairs.join("; ")
    }
}

fn base64_url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// 32 random bytes, base64url: the PKCE code verifier and the OAuth state.
fn random_token() -> Result<String, String> {
    use ring::rand::SecureRandom;
    let mut bytes = [0u8; 32];
    ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "Random number generator failed".to_string())?;
    Ok(base64_url(&bytes))
}

/// PKCE S256 challenge (RFC 7636 section 4.2).
fn code_challenge(verifier: &str) -> String {
    base64_url(ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes()).as_ref())
}

fn parse_endpoints(body: &[u8]) -> Result<Endpoints, String> {
    serde_json::from_slice(body).map_err(|e| format!("Bad OIDC config ({e})"))
}

/// The regex `/(?:url|action):\s*"(.+)"/` of api.ts: the first `url:` or `action:`
/// followed by a quoted value; the value runs to the last `"` on that line (greedy `.+`).
fn resume_path(page: &str) -> Option<&str> {
    let line_end = |c: char| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}');
    page.char_indices().find_map(|(i, _)| {
        let rest = &page[i..];
        let after = rest
            .strip_prefix("url:")
            .or_else(|| rest.strip_prefix("action:"))?;
        let value = after
            .trim_start_matches(char::is_whitespace)
            .strip_prefix('"')?;
        let line = &value[..value.find(line_end).unwrap_or(value.len())];
        line.rfind('"')
            .filter(|&end| end > 0)
            .map(|end| &line[..end])
    })
}

/// `new URL(path, OIDC_PROVIDER_BASE_URL)`.
fn resolve(path: &str) -> Result<Url, String> {
    Url::parse(OIDC_PROVIDER_BASE_URL)
        .and_then(|base| base.join(path))
        .map_err(|_| "Invalid URL".to_string())
}

#[derive(Debug, PartialEq)]
enum Step {
    Code(String),
    Submit(String),
    Follow(Url),
}

/// What to do with a redirect's Location (resolved against the provider, as api.ts does).
fn next_step(location: &str) -> Result<Step, String> {
    let url = resolve(location)?;
    let param = |name: &str| {
        url.query_pairs()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.into_owned())
            .filter(|v| !v.is_empty())
    };
    if let Some(code) = param("code") {
        return Ok(Step::Code(code));
    }
    if let Some(uid) = param("uid") {
        return Ok(Step::Submit(uid));
    }
    Ok(Step::Follow(url))
}

/// Error for a non-redirect response during the login steps.
fn login_page_error(status: u16, page: &str) -> String {
    if page.contains("authMessage: \"ERR001\"") {
        "Authentication failed: Invalid credentials".into()
    } else {
        format!("Unexpected status {status} during login flow.")
    }
}

/// JavaScript truthiness of a JSON value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Token endpoint response -> (access token, how long to reuse it).
fn parse_token(body: &Value) -> Result<(String, Duration), String> {
    if let Some(e) = body.get("error").filter(|e| truthy(e)) {
        let e = e
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| e.to_string());
        return Err(format!("Token Error: {e}"));
    }
    // api.ts would carry on with `undefined` here; fail clearly instead.
    let access = body
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
        .ok_or("Token Error: no access_token in response")?;
    // `expires_in || 3600`
    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_f64().or_else(|| v.as_str()?.trim().parse().ok()))
        .filter(|s| *s != 0.0 && !s.is_nan())
        .unwrap_or(3600.0);
    let reuse = (expires_in - TOKEN_MARGIN_SECS).clamp(0.0, 7.0 * 86400.0);
    Ok((access.to_string(), Duration::from_secs_f64(reuse)))
}

/// `pickForVin`: the entry for `vin`, else the first; a non-array is used as is.
fn pick_for_vin<'a>(entries: Option<&'a Value>, vin: &str) -> Option<&'a Value> {
    match entries {
        Some(Value::Array(list)) => list
            .iter()
            .find(|e| e.get("vin").and_then(Value::as_str) == Some(vin))
            .or(list.first()),
        other => other,
    }
}

/// `normalizeChargingStatus`: chargingStatusV2 values to the strings the UI knows.
fn normalize_charging_status(v2: Option<&str>) -> String {
    match v2 {
        Some("CHARGING_STATUS_V2_CHARGING") => "CHARGING".into(),
        Some("CHARGING_STATUS_V2_IDLE") => "IDLE".into(),
        Some(other) => other.into(),
        None => "UNKNOWN".into(),
    }
}

/// GraphQL response body -> telematics for `vin`.
fn parse_telematics(body: &Value, vin: &str) -> Result<Telematics, GqlError> {
    if let Some(errors) = body.get("errors").filter(|e| truthy(e)) {
        if errors.pointer("/0/extensions/code").and_then(Value::as_str) == Some("UNAUTHENTICATED") {
            return Err(GqlError::Unauthenticated);
        }
        return Err(GqlError::Other(format!("GraphQL Error: {errors}")));
    }
    let raw = body
        .get("data")
        .and_then(|d| d.get("carTelematicsV2"))
        .ok_or_else(|| NO_DATA.to_string())?;
    let car = match raw {
        Value::Array(list) => list.first().unwrap_or(&Value::Null),
        other => other,
    };
    if !truthy(car) {
        return Err(GqlError::Other(NO_DATA.into()));
    }
    let battery = pick_for_vin(car.get("battery"), vin).filter(|v| truthy(v));
    let odometer = pick_for_vin(car.get("odometer"), vin).filter(|v| truthy(v));
    let (Some(battery), Some(odometer)) = (battery, odometer) else {
        return Err(GqlError::Other(INCOMPLETE.into()));
    };
    let num = |v: &Value, key: &str| v.get(key).and_then(Value::as_f64);
    Ok(Telematics {
        battery_pct: num(battery, "batteryChargeLevelPercentage"),
        charging_status: normalize_charging_status(
            battery.get("chargingStatusV2").and_then(Value::as_str),
        ),
        range_km: num(battery, "estimatedDistanceToEmptyKm"),
        odometer_m: num(odometer, "odometerMeters"),
    })
}

/// Error text without the credentials or VIN (GraphQL errors can echo the VIN), cut to
/// a length that fits on screen.
fn redact(error: &str, creds: &Settings) -> String {
    let mut text = error.to_string();
    for (secret, mask) in [
        (&creds.vin, "<vin>"),
        (&creds.username, "<username>"),
        (&creds.password, "<password>"),
    ] {
        if secret.len() >= 3 {
            text = text.replace(secret.as_str(), mask);
        }
    }
    match text.char_indices().nth(300) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

fn is_charging(status: &str) -> bool {
    status == "CHARGING_STATUS_CHARGING" || status == "CHARGING"
}

/// JavaScript `Math.round` (halves go up, also for negatives).
fn js_round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

/// A number as JavaScript prints it in a template (80 -> "80", 80.5 -> "80.5").
fn js_number(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

/// `toLocaleString()` of an integer in en-US/en-GB: thousands separated by commas.
fn grouped(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        out.insert(0, '-');
    }
    out
}

const KM_TO_MILES: f64 = 0.621371;
const MISSING: &str = "--";

fn finite(v: Option<f64>) -> Option<f64> {
    v.filter(|v| v.is_finite())
}

fn battery_text(pct: Option<f64>) -> String {
    format!(
        "{}%",
        finite(pct).map(js_number).as_deref().unwrap_or(MISSING)
    )
}

/// `Math.round(km * 0.621371)`
fn range_text(km: Option<f64>) -> String {
    finite(km).map_or(MISSING.into(), |km| js_number(js_round(km * KM_TO_MILES)))
}

/// `Math.round((m / 1000) * 0.621371).toLocaleString()`
fn odometer_text(m: Option<f64>) -> String {
    finite(m).map_or(MISSING.into(), |m| {
        grouped(js_round(m / 1000.0 * KM_TO_MILES) as i64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIN: &str = "LPSVSEDEEML000001";

    #[test]
    fn pkce_challenge_matches_rfc7636_appendix_b() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        // RFC 7636 appendix B: these 32 octets encode to that verifier.
        let octets = [
            116, 24, 223, 180, 151, 153, 224, 37, 79, 250, 96, 125, 216, 173, 187, 186, 22, 212,
            37, 77, 105, 214, 191, 240, 91, 88, 5, 88, 83, 132, 141, 121,
        ];
        assert_eq!(
            base64_url(&octets),
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
        );
    }

    #[test]
    fn random_tokens_are_43_url_safe_chars_and_differ() {
        let (a, b) = (random_token().unwrap(), random_token().unwrap());
        assert_eq!(a.len(), 43);
        assert!(a
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
        assert_ne!(a, b);
    }

    #[test]
    fn cookie_jar_keeps_first_seen_order_and_overwrites() {
        let mut headers = HeaderMap::new();
        for c in [
            "PF=abc; Path=/; Secure; HttpOnly",
            "lang=en",
            " PF = new=val=ue ; Path=/",
            "flag",
            "; Path=/",
        ] {
            headers.append(SET_COOKIE, HeaderValue::from_str(c).unwrap());
        }
        let mut jar = CookieJar::default();
        assert_eq!(jar.header(), "");
        jar.update(&headers);
        assert_eq!(jar.header(), "PF=new=val=ue; lang=en; flag=");
        let mut more = HeaderMap::new();
        more.append(SET_COOKIE, HeaderValue::from_static("lang=de; Max-Age=0"));
        jar.update(&more);
        assert_eq!(
            jar.header(),
            "PF=new=val=ue; lang=de; flag=",
            "no expiry handling, like api.ts"
        );
    }

    #[test]
    fn discovery_document_parses() {
        let e = parse_endpoints(include_bytes!("../../tests/fixtures/polestar-oidc.json")).unwrap();
        assert_eq!(
            e.authorization_endpoint,
            "https://polestarid.eu.polestar.com/as/authorization.oauth2"
        );
        assert_eq!(
            e.token_endpoint,
            "https://polestarid.eu.polestar.com/as/token.oauth2"
        );
        assert!(parse_endpoints(b"{}").is_err());
    }

    #[test]
    fn resume_path_follows_the_regex() {
        let page = r#"<script>
            var config = {
                "action": "/ignored",
                action: "/as/AbC12/resume/as/authorization.ping",
                url:  "/later"
            };</script>"#;
        assert_eq!(
            resume_path(page),
            Some("/as/AbC12/resume/as/authorization.ping")
        );
        // `url:` or `action:`, whichever comes first; whitespace (even a newline) before the quote.
        assert_eq!(resume_path("x url:\n \"/a/b\" y"), Some("/a/b"));
        // Greedy: up to the last quote on the line, not past it.
        assert_eq!(
            resume_path("action: \"/p?x=\"1\"\", z\nnext\""),
            Some("/p?x=\"1\"")
        );
        // Nothing usable: no quote, empty value, other keys.
        assert_eq!(resume_path("url: /plain\naction: \"\""), None);
        assert_eq!(resume_path("href: \"/x\""), None);
        // An earlier `url:` without a value does not stop the search.
        assert_eq!(resume_path("url: none\naction: \"/ok\""), Some("/ok"));
    }

    #[test]
    fn login_redirect_steps() {
        assert_eq!(
            next_step("https://www.polestar.com/sign-in-callback?code=AbC%2B1&state=s").unwrap(),
            Step::Code("AbC+1".into())
        );
        assert_eq!(
            next_step("/as/x/resume?uid=u-123").unwrap(),
            Step::Submit("u-123".into())
        );
        assert_eq!(
            next_step("/as/next?code=&uid=").unwrap(),
            Step::Follow(
                Url::parse("https://polestarid.eu.polestar.com/as/next?code=&uid=").unwrap()
            ),
            "empty values are ignored, as `searchParams.get` + truthiness does"
        );
        assert_eq!(
            resolve("/as/AbC/resume/as/authorization.ping")
                .unwrap()
                .as_str(),
            "https://polestarid.eu.polestar.com/as/AbC/resume/as/authorization.ping"
        );
    }

    #[test]
    fn login_page_errors() {
        assert_eq!(
            login_page_error(200, "...authMessage: \"ERR001\",..."),
            "Authentication failed: Invalid credentials"
        );
        assert_eq!(
            login_page_error(500, "oops"),
            "Unexpected status 500 during login flow."
        );
    }

    #[test]
    fn token_response() {
        let (t, ttl) = parse_token(&json!({"access_token": "tok", "expires_in": 1800})).unwrap();
        assert_eq!((t.as_str(), ttl), ("tok", Duration::from_secs(1740)));
        let (_, ttl) = parse_token(&json!({"access_token": "tok"})).unwrap();
        assert_eq!(ttl, Duration::from_secs(3540), "expires_in || 3600");
        let (_, ttl) = parse_token(&json!({"access_token": "tok", "expires_in": 0})).unwrap();
        assert_eq!(ttl, Duration::from_secs(3540));
        let (_, ttl) = parse_token(&json!({"access_token": "tok", "expires_in": 30})).unwrap();
        assert_eq!(ttl, Duration::ZERO, "too short to reuse");
        assert_eq!(
            parse_token(&json!({"error": "invalid_grant", "error_description": "x"})).unwrap_err(),
            "Token Error: invalid_grant"
        );
        assert!(parse_token(&json!({"token_type": "Bearer"})).is_err());
    }

    fn fixture() -> Value {
        // Shape of the TELEMATICS_V2 query (types.ts + api.ts): arrays per VIN.
        json!({ "data": { "carTelematicsV2": {
            "battery": [
                { "vin": "OTHER", "batteryChargeLevelPercentage": 99, "chargingStatusV2": "CHARGING_STATUS_V2_IDLE",
                  "estimatedChargingTimeToFullMinutes": 0, "estimatedDistanceToEmptyKm": 400 },
                { "vin": VIN, "batteryChargeLevelPercentage": 30, "chargingStatusV2": "CHARGING_STATUS_V2_CHARGING",
                  "estimatedChargingTimeToFullMinutes": 95, "estimatedDistanceToEmptyKm": 100 }
            ],
            "odometer": [ { "vin": VIN, "odometerMeters": 49_410_000 } ]
        }}})
    }

    fn parsed(body: &Value, vin: &str) -> Result<Telematics, String> {
        parse_telematics(body, vin).map_err(|e| match e {
            GqlError::Unauthenticated => "UNAUTHENTICATED".into(),
            GqlError::Other(e) => e,
        })
    }

    #[test]
    fn telematics_match_by_vin() {
        let t = parsed(&fixture(), VIN).unwrap();
        assert_eq!(
            t,
            Telematics {
                battery_pct: Some(30.0),
                charging_status: "CHARGING".into(),
                range_km: Some(100.0),
                odometer_m: Some(49_410_000.0),
            }
        );
        // Unknown VIN: the first entry.
        let t = parsed(&fixture(), "NOPE").unwrap();
        assert_eq!(
            (t.battery_pct, t.charging_status.as_str()),
            (Some(99.0), "IDLE")
        );
        // carTelematicsV2 as an array of cars, with single objects inside.
        let body = json!({"data": {"carTelematicsV2": [{
            "battery": {"batteryChargeLevelPercentage": 80.5, "chargingStatusV2": null},
            "odometer": {"odometerMeters": null}
        }]}});
        let t = parsed(&body, VIN).unwrap();
        assert_eq!(t.battery_pct, Some(80.5));
        assert_eq!(t.charging_status, "UNKNOWN");
        assert_eq!((t.range_km, t.odometer_m), (None, None));
    }

    #[test]
    fn telematics_errors() {
        let unauth = json!({"errors": [{"message": "no", "extensions": {"code": "UNAUTHENTICATED"}}], "data": null});
        assert_eq!(parsed(&unauth, VIN).unwrap_err(), "UNAUTHENTICATED");
        let other = json!({"errors": [{"message": "Cannot query field \"chargingStatus\""}]});
        assert!(parsed(&other, VIN)
            .unwrap_err()
            .starts_with("GraphQL Error: [{\"message\":"));
        assert_eq!(parsed(&json!({"data": null}), VIN).unwrap_err(), NO_DATA);
        assert_eq!(
            parsed(&json!({"data": {"carTelematicsV2": []}}), VIN).unwrap_err(),
            NO_DATA
        );
        let empty = json!({"data": {"carTelematicsV2": {"battery": [], "odometer": [{"odometerMeters": 1}]}}});
        assert_eq!(parsed(&empty, VIN).unwrap_err(), INCOMPLETE);
        assert_eq!(
            parsed(&json!({"data": {"carTelematicsV2": {"battery": {}}}}), VIN).unwrap_err(),
            INCOMPLETE
        );
    }

    #[test]
    fn charging_status_normalisation() {
        assert_eq!(
            normalize_charging_status(Some("CHARGING_STATUS_V2_CHARGING")),
            "CHARGING"
        );
        assert_eq!(
            normalize_charging_status(Some("CHARGING_STATUS_V2_IDLE")),
            "IDLE"
        );
        assert_eq!(
            normalize_charging_status(Some("CHARGING_STATUS_V2_DONE")),
            "CHARGING_STATUS_V2_DONE"
        );
        assert_eq!(normalize_charging_status(None), "UNKNOWN");
        assert!(is_charging("CHARGING") && is_charging("CHARGING_STATUS_CHARGING"));
        assert!(!is_charging("IDLE") && !is_charging("CHARGING_STATUS_V2_CHARGING"));
    }

    #[test]
    fn unit_conversions() {
        assert_eq!(battery_text(Some(30.0)), "30%");
        assert_eq!(battery_text(Some(80.5)), "80.5%");
        assert_eq!(battery_text(None), "--%");
        // 100 km = 62.1371 mi; the screenshot shows 62 mi.
        assert_eq!(range_text(Some(100.0)), "62");
        assert_eq!(range_text(Some(0.8047)), "1");
        assert_eq!(range_text(None), "--");
        // 49,410 km = 30,701.9 mi
        assert_eq!(odometer_text(Some(49_410_000.0)), "30,702");
        assert_eq!(odometer_text(Some(500.0)), "0");
        assert_eq!(odometer_text(Some(1_609_344_000.0)), "1,000,000");
        assert_eq!(odometer_text(Some(f64::NAN)), "--");
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(grouped(-1234), "-1,234");
        assert_eq!(grouped(999), "999");
    }

    #[test]
    fn errors_are_redacted_and_bounded() {
        let creds = Settings {
            username: "me@example.com".into(),
            password: "hunter22".into(),
            vin: VIN.into(),
        };
        let e = redact(
            &format!("GraphQL Error: vehicle {VIN} of me@example.com (hunter22)"),
            &creds,
        );
        assert_eq!(e, "GraphQL Error: vehicle <vin> of <username> (<password>)");
        assert_eq!(redact(&"x".repeat(400), &creds).chars().count(), 301);
    }
}
