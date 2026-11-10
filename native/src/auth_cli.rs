//! `mmm-native auth google|spotify`: authorise an account from a terminal (over SSH on
//! the Pi: no browser there, and this app runs no HTTP server).
//!
//! 1. Print the authorisation URL (OAuth 2.0 authorization code grant, RFC 6749 §4.1).
//! 2. The user opens it on any device and approves. The browser is then sent to the
//!    redirect URI, which does not load; that is expected.
//! 3. The user pastes that full URL here; `code` is taken from it and `state` checked.
//! 4. The code is exchanged for tokens, which are saved beside the config file:
//!    Google to `google-tokens.json` (the web app's format, src/google_auth.rs), Spotify
//!    to `spotify-tokens.json` under the account name (src/spotify_tokens.rs).
//!
//! The scopes, `access_type` and redirect path are those of the web app's routes
//! (src/routes/api/{google,spotify}/{auth,callback}/+server.ts), so the OAuth clients
//! already registered with Google and Spotify keep working. The web app built the
//! redirect URI from the request origin; here it is `--redirect-uri`, and it must be
//! one registered for the client.
//!
//! Endpoint references (checked 2026-11-10):
//! - Google: https://developers.google.com/identity/protocols/oauth2/web-server
//!   (#creatingclient: auth endpoint and parameters; Step 4: `code` / `error` / `state`
//!   in the redirect; #exchange-authorization-code: POST to the token endpoint
//!   with code, client_id, client_secret, redirect_uri, grant_type=authorization_code;
//!   a refresh token is only returned with access_type=offline, and again after the
//!   first consent only with prompt=consent).
//! - Spotify: https://developer.spotify.com/documentation/web-api/tutorials/code-flow
//!   (GET /authorize with client_id, response_type=code, redirect_uri, state, scope,
//!   show_dialog; redirect carries code+state or error+state; POST /api/token with
//!   grant_type=authorization_code, code, redirect_uri and Basic client auth) and
//!   https://developer.spotify.com/documentation/web-api/concepts/redirect_uri
//!   ("localhost is not allowed as redirect URI"; HTTP only for loopback IP literals).
//!
//! Token values, the client secret and the pasted URL are never printed or logged.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde_json::Value;

use crate::{config, google_auth, spotify_tokens};

const USAGE: &str = "usage:
  mmm-native auth google  [--config <config.json>] [--redirect-uri <uri>]
  mmm-native auth spotify --name <account name> [--config <config.json>] [--redirect-uri <uri>]";

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
/// google/auth/+server.ts
const GOOGLE_SCOPES: [&str; 2] = [
    "https://www.googleapis.com/auth/calendar.readonly",
    "https://www.googleapis.com/auth/tasks.readonly",
];
/// What google/auth/+server.ts produces for an origin of http://localhost:4000.
/// (Google allows http://localhost redirect URIs for web clients.)
pub const GOOGLE_DEFAULT_REDIRECT: &str = "http://localhost:4000/api/google/callback";

const SPOTIFY_AUTH_URL: &str = "https://accounts.spotify.com/authorize";
const SPOTIFY_TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
/// spotify/auth/+server.ts
const SPOTIFY_SCOPES: [&str; 2] = ["user-read-currently-playing", "user-read-playback-state"];
/// The web app's path on port 4000, but on 127.0.0.1: Spotify rejects `localhost`
/// redirect URIs (redirect_uri docs above), so a localhost one cannot be registered.
pub const SPOTIFY_DEFAULT_REDIRECT: &str = "http://127.0.0.1:4000/api/spotify/callback";

#[derive(Debug, PartialEq)]
pub enum Provider {
    Google,
    Spotify { name: String },
}

#[derive(Debug, PartialEq)]
pub struct Args {
    pub provider: Provider,
    pub config: PathBuf,
    pub redirect_uri: String,
}

/// If the first argument is `auth`, run the command and return its exit code.
pub fn run_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if args.first().map(String::as_str) != Some("auth") {
        return None;
    }
    let default_config =
        std::env::var_os("MMM_CONFIG").map_or_else(|| config::DEFAULT_PATH.into(), PathBuf::from);
    Some(match parse_args(&args[1..], default_config) {
        Ok(args) => match run(&args) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            2
        }
    })
}

pub fn parse_args(args: &[String], default_config: PathBuf) -> Result<Args, String> {
    let mut provider = None;
    let (mut name, mut config, mut redirect) = (None, None, None);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = |flag: &str| {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{flag} needs a value"))
        };
        match a.as_str() {
            "--name" => name = Some(value("--name")?),
            "--config" => config = Some(PathBuf::from(value("--config")?)),
            "--redirect-uri" => redirect = Some(value("--redirect-uri")?),
            "-h" | "--help" => return Err(String::new()),
            p if provider.is_none() && !p.starts_with('-') => provider = Some(p.to_string()),
            other => return Err(format!("unexpected argument: {other}")),
        }
    }
    let (provider, default_redirect) = match provider.as_deref() {
        Some("google") => {
            if name.is_some() {
                return Err("--name is only for spotify".into());
            }
            (Provider::Google, GOOGLE_DEFAULT_REDIRECT)
        }
        Some("spotify") => {
            let name = name
                .filter(|n| !n.trim().is_empty())
                .ok_or("spotify needs --name \"<account name>\"")?;
            (Provider::Spotify { name }, SPOTIFY_DEFAULT_REDIRECT)
        }
        Some(p) => return Err(format!("unknown provider {p:?} (google or spotify)")),
        None => return Err("which provider? (google or spotify)".into()),
    };
    Ok(Args {
        provider,
        config: config.unwrap_or(default_config),
        redirect_uri: redirect.unwrap_or_else(|| default_redirect.into()),
    })
}

/// The authorisation URL the user opens.
pub fn auth_url(provider: &Provider, client_id: &str, redirect_uri: &str, state: &str) -> String {
    let (base, params): (&str, Vec<(&str, String)>) = match provider {
        // As googleapis' generateAuthUrl({access_type: 'offline', scope, prompt}) builds
        // it. prompt=consent always: the web app only sent it when no refresh token was
        // stored, but this command is run to replace one, and without consent Google
        // would return no new refresh token.
        Provider::Google => (
            GOOGLE_AUTH_URL,
            vec![
                ("client_id", client_id.into()),
                ("redirect_uri", redirect_uri.into()),
                ("response_type", "code".into()),
                ("scope", GOOGLE_SCOPES.join(" ")),
                ("access_type", "offline".into()),
                ("prompt", "consent".into()),
                ("state", state.into()),
            ],
        ),
        // As spotify/auth/+server.ts, plus `state` (recommended by Spotify).
        Provider::Spotify { .. } => (
            SPOTIFY_AUTH_URL,
            vec![
                ("client_id", client_id.into()),
                ("response_type", "code".into()),
                ("redirect_uri", redirect_uri.into()),
                ("scope", SPOTIFY_SCOPES.join(" ")),
                ("show_dialog", "true".into()),
                ("state", state.into()),
            ],
        ),
    };
    reqwest::Url::parse_with_params(base, &params)
        .expect("constant base URL")
        .into()
}

/// The authorisation code from the URL the browser was redirected to. Messages never
/// include the URL itself (it carries the code).
pub fn parse_redirect(pasted: &str, state: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(pasted.trim()).map_err(|_| {
        "that is not a URL: paste the whole address from the browser's address bar, starting with http".to_string()
    })?;
    let get = |key: &str| {
        url.query_pairs()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.into_owned())
    };
    if let Some(error) = get("error") {
        let detail = get("error_description")
            .map(|d| format!(" ({d})"))
            .unwrap_or_default();
        return Err(format!("authorisation was not granted: {error}{detail}"));
    }
    if get("state").as_deref() != Some(state) {
        return Err(
            "the URL's state does not match this run: paste the URL from the link printed above"
                .into(),
        );
    }
    get("code")
        .filter(|c| !c.is_empty())
        .ok_or_else(|| "the URL has no authorisation code".into())
}

/// A token endpoint reply to the code exchange.
#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct Exchanged {
    pub access_token: Option<String>,
    /// Seconds.
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
}

/// Success body, or an error message from RFC 6749 §5.2 `error` / `error_description`.
pub fn parse_exchange(status: u16, body: &str) -> Result<Exchanged, String> {
    if (200..300).contains(&status) {
        return serde_json::from_str(body)
            .map_err(|_| "token endpoint: unexpected response".into());
    }
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    Err(match (text("error"), text("error_description")) {
        (Some(e), Some(d)) => format!("token endpoint: HTTP {status} {e} ({d})"),
        (Some(e), None) => format!("token endpoint: HTTP {status} {e}"),
        _ => format!("token endpoint: HTTP {status}"),
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Save Google tokens in google-tokens.json. Like google/callback/+server.ts, the stored
/// refresh token is kept if Google sends no new one.
pub fn save_google(path: &Path, ex: &Exchanged, now_ms: i64) -> Result<(), String> {
    let previous = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<google_auth::Tokens>(&t).ok())
        .map(|t| t.refresh_token)
        .filter(|t| !t.is_empty());
    let refresh_token = ex
        .refresh_token
        .clone()
        .filter(|t| !t.is_empty())
        .or(previous)
        .ok_or(
            "Google sent no refresh token; run the command again and approve all requested access",
        )?;
    let tokens = google_auth::Tokens {
        refresh_token,
        access_token: ex.access_token.clone().filter(|t| !t.is_empty()),
        expiry_date: ex.expires_in.map(|s| now_ms + s.max(0) * 1000),
    };
    google_auth::write_atomic(path, &tokens).map_err(|e| write_error(path, e))
}

/// Save a Spotify refresh token under `name` in spotify-tokens.json.
pub fn save_spotify(path: &Path, name: &str, ex: &Exchanged) -> Result<(), String> {
    let token = ex
        .refresh_token
        .as_deref()
        .filter(|t| !t.is_empty())
        .ok_or("Spotify sent no refresh token; run the command again")?;
    spotify_tokens::save(path, name, token)
}

fn write_error(path: &Path, e: std::io::Error) -> String {
    let hint = if e.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "; the file and its directory must be writable by this user (e.g. sudo chown {} {})",
            std::env::var("USER").unwrap_or_else(|_| "<user>".into()),
            path.display()
        )
    } else {
        String::new()
    };
    format!("cannot write {}: {e}{hint}", path.display())
}

/// Names of `components.spotify.accounts`.
pub fn spotify_account_names(spotify: &Value) -> Vec<String> {
    spotify
        .get("accounts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|a| a.get("name")?.as_str().map(str::to_string))
        .collect()
}

fn random_state() -> Result<String, String> {
    use ring::rand::SecureRandom;
    let mut bytes = [0u8; 24];
    ring::rand::SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "no system randomness".to_string())?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

fn dir_of(config: &Path) -> PathBuf {
    config
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf()
}

fn run(args: &Args) -> Result<(), String> {
    let cfg = config::Config::load(&args.config)?;
    let str_of = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let (label, client_id, client_secret, console) = match &args.provider {
        Provider::Google => {
            let c = google_auth::credentials(cfg.component("calendar"), cfg.component("google-tasks")).ok_or(
                "no Google OAuth client: set clientId and clientSecret in components.calendar (or google-tasks)",
            )?;
            ("Google", c.client_id, c.client_secret, "Google Cloud console (APIs & Services > Credentials > your OAuth client > Authorized redirect URIs)")
        }
        Provider::Spotify { .. } => {
            let s = cfg.component("spotify");
            let (id, secret) = (str_of(s, "clientId"), str_of(s, "clientSecret"));
            if id.is_empty() || secret.is_empty() {
                return Err(
                    "no Spotify app: set clientId and clientSecret in components.spotify".into(),
                );
            }
            (
                "Spotify",
                id,
                secret,
                "Spotify developer dashboard (your app > Settings > Redirect URIs)",
            )
        }
    };
    let state = random_state()?;
    let url = auth_url(&args.provider, &client_id, &args.redirect_uri, &state);

    println!("Redirect URI: {}", args.redirect_uri);
    println!("  This must exactly match a redirect URI registered in the {console}.");
    println!("  Use --redirect-uri <uri> to use a different registered one.");
    println!();
    println!("1. Open this URL in a browser on any device and approve access:");
    println!();
    println!("{url}");
    println!();
    println!("2. The browser is then sent to the redirect URI, and that page will fail to load.");
    println!("   This is expected. Copy the full address from the address bar.");
    print!("3. Paste it here and press Enter: ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut pasted = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut pasted)
        .map_err(|e| format!("cannot read input: {e}"))?;
    if pasted.trim().is_empty() {
        return Err("nothing was pasted".into());
    }
    let code = parse_redirect(&pasted, &state)?;

    println!("Exchanging the code with {label}...");
    let ex = exchange(
        &args.provider,
        &client_id,
        &client_secret,
        &args.redirect_uri,
        &code,
    )?;
    let dir = dir_of(&args.config);
    match &args.provider {
        Provider::Google => {
            let path = dir.join(google_auth::TOKEN_FILE);
            save_google(&path, &ex, now_ms())?;
            println!("Saved Google tokens to {}.", path.display());
            println!("A running dashboard picks them up at its next calendar or tasks refresh (within 5 minutes).");
        }
        Provider::Spotify { name } => {
            let path = spotify_tokens::path_for(&args.config);
            save_spotify(&path, name, &ex)?;
            println!(
                "Saved the refresh token for {name:?} to {}.",
                path.display()
            );
            if !spotify_account_names(cfg.component("spotify"))
                .iter()
                .any(|n| n == name)
            {
                println!();
                println!(
                    "Note: {} has no Spotify account named {name:?}, so it will not be used yet.",
                    args.config.display()
                );
                println!("Add this entry to the components.spotify.accounts list in that file (no refreshToken needed;");
                println!("the token stays in {}):", spotify_tokens::FILE);
                println!();
                println!("    {}", serde_json::json!({ "name": name }));
                println!();
                println!("Saving config.json restarts the dashboard by itself.");
            } else {
                println!("Restart the dashboard to use it: sudo systemctl restart mmm-native");
            }
        }
    }
    Ok(())
}

fn exchange(
    provider: &Provider,
    id: &str,
    secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<Exchanged, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let http = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(crate::fetch::clean)?;
    let request = match provider {
        Provider::Google => http.post(google_auth::TOKEN_URL).form(&[
            ("code", code),
            ("client_id", id),
            ("client_secret", secret),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ]),
        Provider::Spotify { .. } => http
            .post(SPOTIFY_TOKEN_URL)
            .basic_auth(id, Some(secret))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
            ]),
    };
    rt.block_on(async {
        let response = request.send().await.map_err(crate::fetch::clean)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(crate::fetch::clean)?;
        parse_exchange(status, &body)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(list: &[&str]) -> Result<Args, String> {
        parse_args(
            &list.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "./data/config.json".into(),
        )
    }

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> TempDir {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let p =
                std::env::temp_dir().join(format!("mmm-authcli-{tag}-{}-{n}", std::process::id()));
            std::fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn command_line() {
        let g = args(&["google"]).unwrap();
        assert_eq!(g.provider, Provider::Google);
        assert_eq!(g.config, PathBuf::from("./data/config.json"));
        assert_eq!(g.redirect_uri, "http://localhost:4000/api/google/callback");
        let s = args(&[
            "spotify",
            "--name",
            "Example Person",
            "--config",
            "/x/config.json",
        ])
        .unwrap();
        assert_eq!(
            s.provider,
            Provider::Spotify {
                name: "Example Person".into()
            }
        );
        assert_eq!(s.config, PathBuf::from("/x/config.json"));
        assert_eq!(s.redirect_uri, "http://127.0.0.1:4000/api/spotify/callback");
        let r = args(&[
            "--redirect-uri",
            "https://mirror.example.com/api/google/callback",
            "google",
        ])
        .unwrap();
        assert_eq!(
            r.redirect_uri,
            "https://mirror.example.com/api/google/callback"
        );
        assert!(args(&["spotify"]).is_err());
        assert!(args(&["spotify", "--name", " "]).is_err());
        assert!(args(&["google", "--name", "x"]).is_err());
        assert!(args(&["dropbox"]).is_err());
        assert!(args(&[]).is_err());
        assert!(args(&["google", "--config"]).is_err());
        assert!(args(&["google", "extra"]).is_err());
    }

    #[test]
    fn google_url_matches_the_web_app() {
        let url = auth_url(
            &Provider::Google,
            "123.apps.googleusercontent.com",
            GOOGLE_DEFAULT_REDIRECT,
            "st8",
        );
        assert_eq!(
            url,
            "https://accounts.google.com/o/oauth2/v2/auth?client_id=123.apps.googleusercontent.com\
             &redirect_uri=http%3A%2F%2Flocalhost%3A4000%2Fapi%2Fgoogle%2Fcallback&response_type=code\
             &scope=https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fcalendar.readonly+https%3A%2F%2Fwww.googleapis.com%2Fauth%2Ftasks.readonly\
             &access_type=offline&prompt=consent&state=st8"
        );
    }

    #[test]
    fn spotify_url_matches_the_web_app() {
        let p = Provider::Spotify { name: "x".into() };
        let url = auth_url(&p, "abc", SPOTIFY_DEFAULT_REDIRECT, "st8");
        assert_eq!(
            url,
            "https://accounts.spotify.com/authorize?client_id=abc&response_type=code\
             &redirect_uri=http%3A%2F%2F127.0.0.1%3A4000%2Fapi%2Fspotify%2Fcallback\
             &scope=user-read-currently-playing+user-read-playback-state&show_dialog=true&state=st8"
        );
        // Round trip: the scope decodes to the space-separated list.
        let parsed = reqwest::Url::parse(&url).unwrap();
        let scope = parsed.query_pairs().find(|(k, _)| k == "scope").unwrap().1;
        assert_eq!(
            scope,
            "user-read-currently-playing user-read-playback-state"
        );
    }

    #[test]
    fn redirected_urls() {
        let ok = "http://localhost:4000/api/google/callback?state=st8&code=4%2F0Abc&scope=x";
        assert_eq!(parse_redirect(ok, "st8").unwrap(), "4/0Abc");
        assert_eq!(
            parse_redirect(&format!("  {ok}\n"), "st8").unwrap(),
            "4/0Abc"
        );
        let denied = "http://127.0.0.1:4000/api/spotify/callback?error=access_denied&state=st8";
        let e = parse_redirect(denied, "st8").unwrap_err();
        assert!(e.contains("access_denied"), "{e}");
        let mismatch = "http://localhost:4000/api/google/callback?code=SECRETCODE&state=other";
        let e = parse_redirect(mismatch, "st8").unwrap_err();
        assert!(e.contains("state") && !e.contains("SECRETCODE"), "{e}");
        let no_state = "http://localhost:4000/api/google/callback?code=c";
        assert!(parse_redirect(no_state, "st8").is_err());
        let no_code = "http://localhost:4000/api/google/callback?state=st8";
        assert!(parse_redirect(no_code, "st8").is_err());
        let e = parse_redirect("code=SECRETCODE&state=st8", "st8").unwrap_err();
        assert!(e.contains("not a URL") && !e.contains("SECRETCODE"), "{e}");
    }

    #[test]
    fn token_endpoint_replies() {
        let ok = parse_exchange(
            200,
            r#"{"access_token":"a","expires_in":3599,"refresh_token":"r","scope":"s","token_type":"Bearer"}"#,
        );
        assert_eq!(
            ok,
            Ok(Exchanged {
                access_token: Some("a".into()),
                expires_in: Some(3599),
                refresh_token: Some("r".into())
            })
        );
        assert_eq!(
            parse_exchange(
                400,
                r#"{"error":"invalid_grant","error_description":"Malformed auth code."}"#
            ),
            Err("token endpoint: HTTP 400 invalid_grant (Malformed auth code.)".into())
        );
        assert_eq!(
            parse_exchange(500, "oops"),
            Err("token endpoint: HTTP 500".into())
        );
        assert!(parse_exchange(200, "not json").is_err());
    }

    #[test]
    fn saves_google_tokens_in_the_web_format() {
        let dir = TempDir::new("google");
        let path = dir.0.join(google_auth::TOKEN_FILE);
        let ex = Exchanged {
            access_token: Some("a".into()),
            expires_in: Some(3599),
            refresh_token: Some("r1".into()),
        };
        save_google(&path, &ex, 1_000).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\n  \"refreshToken\": \"r1\",\n  \"accessToken\": \"a\",\n  \"expiryDate\": 3600000\n}"
        );
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        // No new refresh token: the stored one is kept (as the web callback does).
        let again = Exchanged {
            access_token: Some("b".into()),
            expires_in: Some(10),
            refresh_token: None,
        };
        save_google(&path, &again, 0).unwrap();
        let t: google_auth::Tokens =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(
            (
                t.refresh_token.as_str(),
                t.access_token.as_deref(),
                t.expiry_date
            ),
            ("r1", Some("b"), Some(10_000))
        );
        // Nothing stored and nothing sent: an error, and no file is created.
        let empty = dir.0.join("other.json");
        assert!(save_google(&empty, &again, 0).is_err());
        assert!(!empty.exists());
    }

    #[test]
    fn saves_spotify_token_under_the_name() {
        let dir = TempDir::new("spotify");
        let config = dir.0.join("config.json");
        let path = spotify_tokens::path_for(&config);
        spotify_tokens::save(&path, "Other", "o").unwrap();
        let ex = Exchanged {
            access_token: Some("a".into()),
            expires_in: Some(3600),
            refresh_token: Some("r".into()),
        };
        save_spotify(&path, "Example Person", &ex).unwrap();
        let saved = spotify_tokens::load(&path).unwrap();
        assert_eq!(saved.get("Example Person").map(String::as_str), Some("r"));
        assert_eq!(saved.get("Other").map(String::as_str), Some("o"));
        assert!(save_spotify(&path, "x", &Exchanged::default()).is_err());
        assert!(!config.exists(), "config.json is never written");
    }

    #[test]
    fn account_names_from_config() {
        let s = json!({ "accounts": [{ "name": "A", "refreshToken": "t" }, { "refreshToken": "u" }, { "name": "B" }] });
        assert_eq!(spotify_account_names(&s), ["A", "B"]);
        assert!(spotify_account_names(&Value::Null).is_empty());
    }
}
