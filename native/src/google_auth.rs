//! Google OAuth access tokens shared by the calendar and google-tasks widgets
//! (port of src/lib/services/googleAuth.ts + tokenStorage.ts, without the googleapis
//! library), plus an authorised JSON GET for the Google REST APIs.
//!
//! - Tokens are read from `google-tokens.json` next to the config file, in the web
//!   app's format (`{"refreshToken", "accessToken"?, "expiryDate"?}`, ms since the
//!   epoch, 2-space indent), so both apps can use the same file.
//! - The access token is refreshed 5 minutes before it expires (google-auth-library's
//!   default `eagerRefreshThresholdMillis`). All callers go through one async mutex,
//!   so the two widgets never run two refreshes at once: the second waits and then
//!   uses the token the first one got.
//! - New tokens are written atomically (temp file + rename). If the file cannot be
//!   written (e.g. owned by another user), the tokens stay in memory and this is
//!   logged once. Token values are never logged.
//! - The file is re-read when it changes on disk, so tokens written by the web app or
//!   a later `mmm auth google` CLI are picked up without a restart.
//! - The interactive consent flow is `mmm-native auth google` (src/auth_cli.rs).

use std::fs;
use std::future::Future;
use std::io::Write;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::config::Config;
use crate::fetch::clean;

pub const TOKEN_FILE: &str = "google-tokens.json";
/// OAuth 2.0 token endpoint. Refresh request and response:
/// https://developers.google.com/identity/protocols/oauth2/web-server#offline
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// Refresh this long before expiry (google-auth-library `eagerRefreshThresholdMillis`).
const EAGER_REFRESH_MS: i64 = 5 * 60 * 1000;

/// OAuth client from the calendar or google-tasks config slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
}

/// getGoogleOAuthConfig(): the calendar slice if it has a clientId, else the
/// google-tasks slice if that has one, else the calendar slice. None unless both
/// clientId and clientSecret are set ("Google OAuth not configured").
pub fn credentials(calendar: &Value, tasks: &Value) -> Option<Credentials> {
    let field = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let source = if !field(calendar, "clientId").is_empty() {
        calendar
    } else if !field(tasks, "clientId").is_empty() {
        tasks
    } else {
        calendar
    };
    let (client_id, client_secret) = (field(source, "clientId"), field(source, "clientSecret"));
    (!client_id.is_empty() && !client_secret.is_empty()).then_some(Credentials {
        client_id,
        client_secret,
    })
}

/// The token file, exactly as TokenStorage writes it.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tokens {
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// Milliseconds since the epoch.
    #[serde(
        default,
        deserialize_with = "millis",
        skip_serializing_if = "Option::is_none"
    )]
    pub expiry_date: Option<i64>,
}

/// Any JSON number (or null) as whole milliseconds.
fn millis<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    Ok(Option::<f64>::deserialize(d)?
        .filter(|n| n.is_finite())
        .map(|n| n as i64))
}

/// Why no access token is available.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthError {
    /// No clientId/clientSecret in config.
    NotConfigured,
    /// No usable tokens: the user has to (re-)authorise. The widgets then show what
    /// the web components show for "Not authenticated".
    NotAuthenticated,
    /// Anything transient (network, 5xx); tokens are kept and retried later.
    Failed(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AuthError::NotConfigured => f.write_str("Google OAuth not configured"),
            AuthError::NotAuthenticated => f.write_str("Not authenticated"),
            AuthError::Failed(e) => write!(f, "token refresh failed: {e}"),
        }
    }
}

/// A successful refresh response.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Refreshed {
    pub access_token: String,
    /// Seconds.
    pub expires_in: i64,
    /// Google does not normally send one on refresh, but if it does it replaces the old
    /// one (googleAuth.ts persists a rotated refresh token too).
    #[serde(default)]
    pub refresh_token: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum RefreshError {
    /// The refresh token was revoked or has expired: re-authorisation is needed.
    InvalidGrant,
    Other(String),
}

/// Interpret the token endpoint's reply. Errors are `{"error": "<code>", ...}` with
/// HTTP 400/401 (RFC 6749 §5.2, https://www.rfc-editor.org/rfc/rfc6749#section-5.2);
/// `invalid_grant` means the refresh token is dead
/// (https://developers.google.com/identity/protocols/oauth2/web-server#offline).
pub fn parse_token_response(status: u16, body: &str) -> Result<Refreshed, RefreshError> {
    if (200..300).contains(&status) {
        return serde_json::from_str::<Refreshed>(body)
            .map_err(|_| RefreshError::Other("token endpoint: unexpected response".into()));
    }
    let code = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(str::to_string));
    match code.as_deref() {
        Some("invalid_grant") => Err(RefreshError::InvalidGrant),
        Some(c) => Err(RefreshError::Other(format!(
            "token endpoint: HTTP {status} {c}"
        ))),
        None => Err(RefreshError::Other(format!(
            "token endpoint: HTTP {status}"
        ))),
    }
}

/// POST grant_type=refresh_token to the token endpoint (form-encoded, as documented in
/// https://developers.google.com/identity/protocols/oauth2/web-server#offline).
async fn refresh(
    http: &reqwest::Client,
    creds: &Credentials,
    refresh_token: &str,
) -> Result<Refreshed, RefreshError> {
    let form = [
        ("client_id", creds.client_id.as_str()),
        ("client_secret", creds.client_secret.as_str()),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];
    let other = |e| RefreshError::Other(clean(e));
    let response = http
        .post(TOKEN_URL)
        .form(&form)
        .send()
        .await
        .map_err(other)?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(other)?;
    parse_token_response(status, &body)
}

/// Identity of the file on disk (mtime, size, inode); None = missing.
type Stamp = Option<(Option<SystemTime>, u64, u64)>;

fn stamp(path: &Path) -> Stamp {
    fs::metadata(path)
        .ok()
        .map(|m| (m.modified().ok(), m.len(), m.ino()))
}

#[derive(Default)]
struct State {
    tokens: Option<Tokens>,
    /// The file as it was when last read or written; None = never read.
    seen: Option<Stamp>,
    /// A refresh token Google rejected with invalid_grant; not retried until the file
    /// brings a different one.
    dead: Option<String>,
    write_warned: bool,
    read_warned: bool,
}

/// The shared token holder. One per process (`shared`); tests make their own.
pub struct GoogleAuth {
    creds: Option<Credentials>,
    path: PathBuf,
    state: tokio::sync::Mutex<State>,
}

/// The process-wide instance (both widgets must share it for the single-flight guard).
pub fn shared(all: &Config) -> Arc<GoogleAuth> {
    static SHARED: OnceLock<Arc<GoogleAuth>> = OnceLock::new();
    SHARED
        .get_or_init(|| {
            let config = crate::config::path_from_args();
            let dir = config
                .parent()
                .filter(|d| !d.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            Arc::new(GoogleAuth::new(
                credentials(all.component("calendar"), all.component("google-tasks")),
                dir.join(TOKEN_FILE),
            ))
        })
        .clone()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl GoogleAuth {
    pub fn new(creds: Option<Credentials>, path: PathBuf) -> GoogleAuth {
        GoogleAuth {
            creds,
            path,
            state: tokio::sync::Mutex::new(State::default()),
        }
    }

    /// A valid access token, refreshing it first if it expires within 5 minutes.
    pub async fn access_token(&self, http: &reqwest::Client) -> Result<String, AuthError> {
        let Some(creds) = &self.creds else {
            return Err(AuthError::NotConfigured);
        };
        self.token_with(now_ms, |rt| async move { refresh(http, creds, &rt).await })
            .await
    }

    /// `access_token` with the clock and the refresh call injected (for tests).
    async fn token_with<F, Fut>(
        &self,
        now: impl Fn() -> i64,
        refresh: F,
    ) -> Result<String, AuthError>
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = Result<Refreshed, RefreshError>>,
    {
        if self.creds.is_none() {
            return Err(AuthError::NotConfigured);
        }
        // Held across the refresh: concurrent callers queue here (single flight).
        let mut s = self.state.lock().await;
        self.reload_if_changed(&mut s);
        let Some(tokens) = s.tokens.clone() else {
            return Err(AuthError::NotAuthenticated);
        };
        if let (Some(token), Some(expiry)) = (&tokens.access_token, tokens.expiry_date) {
            if expiry > now() + EAGER_REFRESH_MS {
                return Ok(token.clone());
            }
        }
        if tokens.refresh_token.is_empty() || s.dead.as_ref() == Some(&tokens.refresh_token) {
            return Err(AuthError::NotAuthenticated);
        }
        match refresh(tokens.refresh_token.clone()).await {
            Ok(r) => {
                let new = Tokens {
                    refresh_token: r
                        .refresh_token
                        .filter(|t| !t.is_empty())
                        .unwrap_or(tokens.refresh_token),
                    access_token: Some(r.access_token.clone()),
                    expiry_date: Some(now() + r.expires_in.max(0) * 1000),
                };
                self.persist(&mut s, &new);
                s.tokens = Some(new);
                Ok(r.access_token)
            }
            Err(RefreshError::InvalidGrant) => {
                // googleAuth.ts deletes the file here; the file is left alone (the web
                // app may share it) and this token is just not retried.
                eprintln!(
                    "google auth: refresh token rejected (invalid_grant); re-authorisation needed"
                );
                s.dead = Some(tokens.refresh_token);
                Err(AuthError::NotAuthenticated)
            }
            Err(RefreshError::Other(e)) => Err(AuthError::Failed(e)),
        }
    }

    /// The API rejected `token` (HTTP 401): make the next call refresh it.
    pub async fn invalidate(&self, token: &str) {
        let mut s = self.state.lock().await;
        if let Some(t) = s
            .tokens
            .as_mut()
            .filter(|t| t.access_token.as_deref() == Some(token))
        {
            t.expiry_date = None;
        }
    }

    fn reload_if_changed(&self, s: &mut State) {
        let now = stamp(&self.path);
        if s.seen.as_ref() == Some(&now) {
            return;
        }
        s.seen = Some(now);
        s.tokens = match now {
            None => {
                if !std::mem::replace(&mut s.read_warned, true) {
                    eprintln!(
                        "google auth: no {}; Google widgets need authorising",
                        self.path.display()
                    );
                }
                None
            }
            Some(_) => match fs::read_to_string(&self.path) {
                // serde errors can quote values; report only that it is invalid.
                Ok(text) => serde_json::from_str::<Tokens>(&text)
                    .inspect_err(|_| {
                        if !std::mem::replace(&mut s.read_warned, true) {
                            eprintln!(
                                "google auth: {} is not valid token JSON",
                                self.path.display()
                            );
                        }
                    })
                    .ok(),
                Err(e) => {
                    if !std::mem::replace(&mut s.read_warned, true) {
                        eprintln!("google auth: cannot read {}: {e}", self.path.display());
                    }
                    None
                }
            },
        };
    }

    /// Write new tokens to disk if possible; otherwise keep them in memory only.
    fn persist(&self, s: &mut State, tokens: &Tokens) {
        if s.tokens.as_ref() == Some(tokens) {
            return;
        }
        match write_atomic(&self.path, tokens) {
            Ok(()) => s.seen = Some(stamp(&self.path)),
            Err(e) => {
                if !std::mem::replace(&mut s.write_warned, true) {
                    eprintln!(
                        "google auth: cannot write {} ({e}); keeping tokens in memory",
                        self.path.display()
                    );
                }
            }
        }
    }

    /// GET a Google API URL with the access token. On HTTP 401 the token is refreshed
    /// and the request retried once (as google-auth-library does).
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        http: &reqwest::Client,
        build: impl Fn(&reqwest::Client) -> reqwest::RequestBuilder,
    ) -> Result<T, ApiError> {
        let mut retried = false;
        loop {
            let token = self.access_token(http).await.map_err(ApiError::Auth)?;
            let response = build(http)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| ApiError::Other(clean(e)))?;
            let status = response.status().as_u16();
            if status == 401 && !retried {
                retried = true;
                self.invalidate(&token).await;
                continue;
            }
            let body = response
                .text()
                .await
                .map_err(|e| ApiError::Other(clean(e)))?;
            return parse_api_response(status, &body);
        }
    }
}

/// Write `tokens` as TokenStorage does (JSON, 2-space indent) via a temp file and a
/// rename, keeping the old file's permission bits. An existing file we may not write
/// to (another user's) is left alone: the rename would otherwise replace it.
pub fn write_atomic(path: &Path, tokens: &Tokens) -> std::io::Result<()> {
    let mode = match fs::metadata(path) {
        Ok(m) => {
            fs::OpenOptions::new().write(true).open(path)?; // permission check, no truncate
            m.permissions().mode() & 0o777
        }
        Err(_) => 0o600,
    };
    let json = serde_json::to_string_pretty(tokens).map_err(std::io::Error::other)?;
    replace_file(path, json.as_bytes(), mode)
}

/// Replace `path` with `contents` atomically: write and fsync a temp file in the same
/// directory with permission bits `mode`, then rename it over `path`. On failure the
/// temp file is removed and `path` is untouched.
pub fn replace_file(path: &Path, contents: &[u8], mode: u32) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let write = || -> std::io::Result<()> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(mode)
            .open(&tmp)?;
        f.set_permissions(fs::Permissions::from_mode(mode))?;
        f.write_all(contents)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    };
    write().inspect_err(|_| {
        let _ = fs::remove_file(&tmp); // our own temp file
    })
}

/// A failed Google API call.
#[derive(Debug, PartialEq)]
pub enum ApiError {
    Auth(AuthError),
    /// Non-2xx with Google's error envelope message (if any).
    Status(u16, String),
    Other(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ApiError::Auth(e) => e.fmt(f),
            ApiError::Status(code, msg) => write!(f, "HTTP {code} {msg}"),
            ApiError::Other(e) => f.write_str(e),
        }
    }
}

/// Parse a 2xx body as `T`; otherwise read Google's error envelope
/// `{"error": {"code", "message", "status"}}` (https://cloud.google.com/apis/design/errors).
pub fn parse_api_response<T: DeserializeOwned>(status: u16, body: &str) -> Result<T, ApiError> {
    if (200..300).contains(&status) {
        // The serde message can quote event titles; keep only the position.
        return serde_json::from_str(body).map_err(|e| {
            ApiError::Other(format!(
                "unexpected response (line {}, column {})",
                e.line(),
                e.column()
            ))
        });
    }
    let message = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message")?.as_str().map(str::to_string))
        .unwrap_or_default();
    Err(ApiError::Status(status, message))
}

/// One page of a Google list response (Calendar v3 / Tasks v1 both use `items` and
/// `nextPageToken`).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<T>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

/// Follow `nextPageToken` (passed back as `pageToken`) for up to `max_pages` pages.
pub async fn list_all<T: DeserializeOwned>(
    auth: &GoogleAuth,
    http: &reqwest::Client,
    url: &reqwest::Url,
    query: &[(&str, String)],
    max_pages: usize,
) -> Result<Vec<T>, ApiError> {
    let mut items = Vec::new();
    let mut token: Option<String> = None;
    for _ in 0..max_pages {
        let page: Page<T> = auth
            .get_json(http, |c| {
                let r = c.get(url.clone()).query(query);
                match &token {
                    Some(t) => r.query(&[("pageToken", t)]),
                    None => r,
                }
            })
            .await?;
        items.extend(page.items);
        token = page.next_page_token.filter(|t| !t.is_empty());
        if token.is_none() {
            break;
        }
    }
    Ok(items)
}

/// `base` with `segments` appended, each percent-encoded (calendar ids contain `#`).
pub fn api_url(base: &str, segments: &[&str]) -> reqwest::Url {
    let mut url = reqwest::Url::parse(base).expect("constant base URL");
    if let Ok(mut path) = url.path_segments_mut() {
        path.pop_if_empty().extend(segments);
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::Cell;
    use std::rc::Rc;

    const HOUR: i64 = 3_600_000;
    const NOW: i64 = 1_762_765_000_000;

    fn creds() -> Option<Credentials> {
        Some(Credentials {
            client_id: "id".into(),
            client_secret: "secret".into(),
        })
    }

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let n = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p =
                std::env::temp_dir().join(format!("mmm-gauth-{tag}-{}-{n}", std::process::id()));
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        fn file(&self) -> PathBuf {
            self.0.join(TOKEN_FILE)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write(path: &Path, v: Value) {
        fs::write(path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }

    async fn never(_: String) -> Result<Refreshed, RefreshError> {
        panic!("must not refresh")
    }

    fn ok(token: &str, refresh: Option<&str>) -> Result<Refreshed, RefreshError> {
        Ok(Refreshed {
            access_token: token.into(),
            expires_in: 3599,
            refresh_token: refresh.map(str::to_string),
        })
    }

    #[test]
    fn credentials_follow_get_google_oauth_config() {
        let full = json!({"clientId": "c", "clientSecret": "s"});
        let other = json!({"clientId": "t", "clientSecret": "ts"});
        assert_eq!(credentials(&full, &other).unwrap().client_id, "c");
        assert_eq!(credentials(&Value::Null, &other).unwrap().client_id, "t");
        assert_eq!(
            credentials(&json!({"clientId": ""}), &other)
                .unwrap()
                .client_id,
            "t"
        );
        // Calendar has an id but no secret: it still wins, so not configured.
        assert_eq!(credentials(&json!({"clientId": "c"}), &other), None);
        assert_eq!(credentials(&Value::Null, &Value::Null), None);
    }

    #[test]
    fn token_file_format_matches_token_storage() {
        let t: Tokens = serde_json::from_str(
            r#"{"refreshToken":"r","accessToken":"a","expiryDate":1762765000123}"#,
        )
        .unwrap();
        assert_eq!(t.expiry_date, Some(1_762_765_000_123));
        assert_eq!(
            serde_json::to_string_pretty(&t).unwrap(),
            "{\n  \"refreshToken\": \"r\",\n  \"accessToken\": \"a\",\n  \"expiryDate\": 1762765000123\n}"
        );
        let bare: Tokens =
            serde_json::from_str(r#"{"refreshToken":"r","expiryDate":null}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&bare).unwrap(),
            r#"{"refreshToken":"r"}"#
        );
        let float: Tokens =
            serde_json::from_str(r#"{"refreshToken":"r","expiryDate":1.5e12}"#).unwrap();
        assert_eq!(float.expiry_date, Some(1_500_000_000_000));
    }

    #[test]
    fn token_endpoint_responses() {
        let body =
            r#"{"access_token":"ya29.x","expires_in":3920,"scope":"s","token_type":"Bearer"}"#;
        assert_eq!(
            parse_token_response(200, body),
            ok_with("ya29.x", 3920, None)
        );
        let rotated = r#"{"access_token":"a","expires_in":10,"refresh_token":"new"}"#;
        assert_eq!(
            parse_token_response(200, rotated),
            ok_with("a", 10, Some("new"))
        );
        let dead =
            r#"{"error":"invalid_grant","error_description":"Token has been expired or revoked."}"#;
        assert_eq!(
            parse_token_response(400, dead),
            Err(RefreshError::InvalidGrant)
        );
        assert_eq!(
            parse_token_response(401, r#"{"error":"invalid_client"}"#),
            Err(RefreshError::Other(
                "token endpoint: HTTP 401 invalid_client".into()
            ))
        );
        assert!(matches!(
            parse_token_response(503, "<html>"),
            Err(RefreshError::Other(_))
        ));
        assert!(matches!(
            parse_token_response(200, "{}"),
            Err(RefreshError::Other(_))
        ));
    }

    fn ok_with(a: &str, e: i64, r: Option<&str>) -> Result<Refreshed, RefreshError> {
        Ok(Refreshed {
            access_token: a.into(),
            expires_in: e,
            refresh_token: r.map(str::to_string),
        })
    }

    #[tokio::test]
    async fn uses_valid_token_and_refreshes_near_expiry() {
        let dir = TempDir::new("expiry");
        let path = dir.file();
        let auth = GoogleAuth::new(creds(), path.clone());

        write(
            &path,
            json!({"refreshToken": "r", "accessToken": "a", "expiryDate": NOW + HOUR}),
        );
        assert_eq!(auth.token_with(|| NOW, never).await.unwrap(), "a");

        // 4 minutes left: inside the 5-minute window, so refresh.
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        let refresh = move |rt: String| {
            c.set(c.get() + 1);
            assert_eq!(rt, "r");
            async { ok("b", None) }
        };
        let later = NOW + HOUR - 4 * 60_000;
        assert_eq!(auth.token_with(|| later, refresh).await.unwrap(), "b");
        assert_eq!(calls.get(), 1);

        // Persisted in the web app's format with the new expiry; refresh token kept.
        let saved: Tokens = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            saved,
            Tokens {
                refresh_token: "r".into(),
                access_token: Some("b".into()),
                expiry_date: Some(later + 3_599_000)
            }
        );
        // And used from memory afterwards.
        assert_eq!(auth.token_with(|| later, never).await.unwrap(), "b");
    }

    #[tokio::test]
    async fn missing_access_token_or_expiry_refreshes() {
        let dir = TempDir::new("noexp");
        let path = dir.file();
        write(&path, json!({"refreshToken": "r", "accessToken": "a"}));
        let auth = GoogleAuth::new(creds(), path);
        assert_eq!(
            auth.token_with(|| NOW, |_| async { ok("b", None) })
                .await
                .unwrap(),
            "b"
        );
    }

    #[tokio::test]
    async fn rotated_refresh_token_is_saved() {
        let dir = TempDir::new("rotate");
        let path = dir.file();
        write(&path, json!({"refreshToken": "old"}));
        let auth = GoogleAuth::new(creds(), path.clone());
        auth.token_with(|| NOW, |_| async { ok("a", Some("new")) })
            .await
            .unwrap();
        let saved: Tokens = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.refresh_token, "new");
        // No temp files left behind.
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn no_file_or_bad_file_is_not_authenticated() {
        let dir = TempDir::new("nofile");
        let path = dir.file();
        let auth = GoogleAuth::new(creds(), path.clone());
        assert_eq!(
            auth.token_with(|| NOW, never).await,
            Err(AuthError::NotAuthenticated)
        );
        fs::write(&path, "{not json").unwrap();
        assert_eq!(
            auth.token_with(|| NOW, never).await,
            Err(AuthError::NotAuthenticated)
        );
        // A token file appearing later is picked up without a restart.
        write(
            &path,
            json!({"refreshToken": "r", "accessToken": "a", "expiryDate": NOW + HOUR}),
        );
        assert_eq!(auth.token_with(|| NOW, never).await.unwrap(), "a");
        assert_eq!(
            GoogleAuth::new(None, path).token_with(|| NOW, never).await,
            Err(AuthError::NotConfigured)
        );
    }

    #[tokio::test]
    async fn invalid_grant_is_not_authenticated_and_not_retried() {
        let dir = TempDir::new("grant");
        let path = dir.file();
        write(&path, json!({"refreshToken": "dead"}));
        let auth = GoogleAuth::new(creds(), path.clone());
        let r = auth
            .token_with(|| NOW, |_| async { Err(RefreshError::InvalidGrant) })
            .await;
        assert_eq!(r, Err(AuthError::NotAuthenticated));
        assert!(path.exists(), "the shared file is not deleted");
        assert_eq!(
            auth.token_with(|| NOW, never).await,
            Err(AuthError::NotAuthenticated)
        );
        // A new refresh token in the file (re-authorised elsewhere) is used.
        write(&path, json!({"refreshToken": "fresh", "accessToken": "x"}));
        assert_eq!(
            auth.token_with(|| NOW, |_| async { ok("a", None) })
                .await
                .unwrap(),
            "a"
        );
    }

    #[tokio::test]
    async fn transient_refresh_failure_keeps_tokens() {
        let dir = TempDir::new("transient");
        let path = dir.file();
        write(&path, json!({"refreshToken": "r"}));
        let auth = GoogleAuth::new(creds(), path);
        let r = auth
            .token_with(
                || NOW,
                |_| async { Err(RefreshError::Other("offline".into())) },
            )
            .await;
        assert_eq!(r, Err(AuthError::Failed("offline".into())));
        assert_eq!(
            auth.token_with(|| NOW, |_| async { ok("a", None) })
                .await
                .unwrap(),
            "a"
        );
    }

    #[tokio::test]
    async fn unwritable_file_keeps_tokens_in_memory() {
        let dir = TempDir::new("ro");
        let path = dir.file();
        write(&path, json!({"refreshToken": "r"}));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o444)).unwrap();
        if fs::OpenOptions::new().write(true).open(&path).is_ok() {
            return; // running as root: permissions are not enforced
        }
        let auth = GoogleAuth::new(creds(), path.clone());
        assert_eq!(
            auth.token_with(|| NOW, |_| async { ok("a", Some("r2")) })
                .await
                .unwrap(),
            "a"
        );
        let on_disk: Tokens = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(on_disk.refresh_token, "r", "file untouched");
        // Memory has the new tokens: no second refresh, and r2 would be used next.
        assert_eq!(auth.token_with(|| NOW, never).await.unwrap(), "a");
        let t = |rt: String| async move {
            assert_eq!(rt, "r2");
            ok("b", None)
        };
        assert_eq!(auth.token_with(|| NOW + 2 * HOUR, t).await.unwrap(), "b");
    }

    #[tokio::test]
    async fn invalidate_forces_a_refresh() {
        let dir = TempDir::new("inval");
        let path = dir.file();
        write(
            &path,
            json!({"refreshToken": "r", "accessToken": "a", "expiryDate": NOW + HOUR}),
        );
        let auth = GoogleAuth::new(creds(), path);
        assert_eq!(auth.token_with(|| NOW, never).await.unwrap(), "a");
        auth.invalidate("someone-else").await;
        assert_eq!(auth.token_with(|| NOW, never).await.unwrap(), "a");
        auth.invalidate("a").await;
        assert_eq!(
            auth.token_with(|| NOW, |_| async { ok("b", None) })
                .await
                .unwrap(),
            "b"
        );
    }

    /// Two widgets asking at once while the token is expired: one refresh, both get it.
    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_share_one_refresh() {
        let dir = TempDir::new("flight");
        let path = dir.file();
        write(
            &path,
            json!({"refreshToken": "r", "accessToken": "old", "expiryDate": NOW - 1}),
        );
        let auth = Arc::new(GoogleAuth::new(creds(), path));
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let caller = |auth: Arc<GoogleAuth>, calls: Arc<std::sync::atomic::AtomicU32>| async move {
            auth.token_with(
                || NOW,
                move |_| async move {
                    calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    // A slow token endpoint, so the second caller arrives mid-refresh.
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    ok("new", None)
                },
            )
            .await
        };
        let a = tokio::spawn(caller(auth.clone(), calls.clone()));
        let b = tokio::spawn(caller(auth.clone(), calls.clone()));
        assert_eq!(a.await.unwrap().unwrap(), "new");
        assert_eq!(b.await.unwrap().unwrap(), "new");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[test]
    fn api_errors_and_pages() {
        let page: Page<Value> = parse_api_response(
            200,
            r#"{"kind":"x","items":[{"id":"1"}],"nextPageToken":"p2"}"#,
        )
        .unwrap();
        assert_eq!(
            (page.items.len(), page.next_page_token.as_deref()),
            (1, Some("p2"))
        );
        let empty: Page<Value> = parse_api_response(200, r#"{"kind":"x"}"#).unwrap();
        assert!(empty.items.is_empty() && empty.next_page_token.is_none());
        let body = r#"{"error":{"code":403,"message":"Request had insufficient authentication scopes.","status":"PERMISSION_DENIED"}}"#;
        assert_eq!(
            parse_api_response::<Value>(403, body),
            Err(ApiError::Status(
                403,
                "Request had insufficient authentication scopes.".into()
            ))
        );
        assert_eq!(
            parse_api_response::<Value>(502, "Bad gateway"),
            Err(ApiError::Status(502, String::new()))
        );
        let bad = parse_api_response::<Page<Value>>(200, r#"{"items": [1, "Secret title"#);
        assert!(
            matches!(&bad, Err(ApiError::Other(m)) if !m.contains("Secret")),
            "{bad:?}"
        );
    }

    #[test]
    fn calendar_ids_are_encoded_in_the_path() {
        let url = api_url(
            "https://www.googleapis.com/calendar/v3/",
            &[
                "calendars",
                "en.uk#holiday@group.v.calendar.google.com",
                "events",
            ],
        );
        assert_eq!(
            url.as_str(),
            "https://www.googleapis.com/calendar/v3/calendars/en.uk%23holiday@group.v.calendar.google.com/events"
        );
        let tasks = api_url(
            "https://tasks.googleapis.com/tasks/v1",
            &["lists", "@default", "tasks"],
        );
        assert_eq!(
            tasks.as_str(),
            "https://tasks.googleapis.com/tasks/v1/lists/@default/tasks"
        );
    }
}
