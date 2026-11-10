//! Spotify now-playing for one or more accounts: port of src/components/spotify/.
//!
//! One fetch job polls every account concurrently: refresh-token -> access token (cached
//! until a minute before expiry), then `GET /v1/me/player`. It polls every 5s while
//! anything is playing and every 30s otherwise, as the web component does. Each account
//! fails, backs off and logs on its own, so one revoked token does not hide the others.
//! Album art is downloaded once per track, decoded and scaled down off the UI thread, and
//! only the art of the tracks on screen is kept. The widget takes no space when no
//! account has a current item.
//!
//! Spotify Web API references (checked 2026-11-10):
//! - Get Playback State: <https://developer.spotify.com/documentation/web-api/reference/get-information-about-the-users-current-playback>
//!   (responses 200/204/401/403/429; `item`, `progress_ms` nullable; images "widest first",
//!   image width/height nullable; `currently_playing_type` can be `ad`/`unknown`).
//! - Refreshing tokens: <https://developer.spotify.com/documentation/web-api/tutorials/refreshing-tokens>
//!   (POST form `grant_type=refresh_token`, Basic client auth; response has `expires_in`
//!   and maybe a new `refresh_token`: "When a refresh token is not returned, continue
//!   using the existing token").
//! - Rate limits: <https://developer.spotify.com/documentation/web-api/concepts/rate-limits>
//!   (429 with `Retry-After` in seconds; the limit counts all calls the *app* makes in a
//!   rolling 30s window, so a 429 pauses every account).

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde::Deserialize;
use slint::{ComponentHandle, Image, Model, ModelRc, Rgb8Pixel, SharedPixelBuffer, VecModel};

use super::Ctx;
use crate::fetch::{self, Job, Update};
use crate::{layout, timing, AppWindow, SpotifyData, SpotifyRow};

pub const ID: &str = "spotify";

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const PLAYER_URL: &str = "https://api.spotify.com/v1/me/player?additional_types=track,episode";
/// Refresh an access token this long before Spotify says it expires.
const TOKEN_MARGIN: Duration = Duration::from_secs(60);
/// One account's whole poll (token, player, art); below the job's own timeout.
const ACCOUNT_TIMEOUT: Duration = Duration::from_secs(15);
const JOB_TIMEOUT: Duration = Duration::from_secs(25);
/// A shown track survives this many failed polls of its account (no flicker on a blip).
const KEEP_THROUGH_FAILURES: u32 = 2;
/// Per-account backoff: 10s, 20s, 40s ... up to this (a revoked token needs
/// `mmm-native auth spotify` and a restart anyway).
const ACCOUNT_BACKOFF_CAP: Duration = Duration::from_secs(15 * 60);
/// When a 429 has no usable Retry-After, and the most we wait for one.
const RETRY_AFTER_DEFAULT: Duration = Duration::from_secs(60);
const RETRY_AFTER_MAX: Duration = Duration::from_secs(24 * 60 * 60);
/// Art is decoded to ART_PX square (the largest box is the 140px cassette). The smallest
/// Spotify rendition at least this wide is downloaded (normally the 300px one).
pub const ART_PX: u32 = 140;
const ART_MAX_BYTES: usize = 512 * 1024;
const ART_MAX_SIDE: u32 = 2048;
const ART_TRIES: u8 = 3;

// ---- config ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Settings {
    client_id: Option<String>,
    client_secret: Option<String>,
    accounts: Option<Vec<AccountSettings>>,
    track_style: Option<String>,
    episode_style: Option<String>,
    /// Legacy name for `trackStyle` (still in data/config.json.example).
    album_art_style: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct AccountSettings {
    refresh_token: Option<String>,
    name: Option<String>,
}

/// Art treatment; the discriminant is `SpotifyRow.style` in ui/widgets/spotify.slint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Vinyl = 0,
    Square = 1,
    Cassette = 2,
}

/// The first non-empty value, else `default`. Like the web template, any value other
/// than vinyl/cassette renders the rounded square.
fn style(values: &[Option<&str>], default: Style) -> Style {
    match values.iter().flatten().find(|v| !v.is_empty()) {
        None => default,
        Some(&"vinyl") => Style::Vinyl,
        Some(&"cassette") => Style::Cassette,
        Some(_) => Style::Square,
    }
}

// ---- player response --------------------------------------------------------------------

/// What one account is playing (paused items included, as in the web app).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub art_url: String,
    pub is_playing: bool,
    /// Device name, empty when unknown.
    pub device: String,
    pub is_episode: bool,
}

#[derive(Deserialize)]
struct Player {
    is_playing: Option<bool>,
    device: Option<Named>,
    item: Option<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(rename = "type")]
    kind: Option<String>,
    name: Option<String>,
    artists: Option<Vec<Named>>,
    album: Option<Show>,
    images: Option<Vec<ImageRef>>,
    show: Option<Show>,
}

#[derive(Deserialize)]
struct Named {
    name: Option<String>,
}

/// An album or a show: what we need of either.
#[derive(Deserialize)]
struct Show {
    name: Option<String>,
    images: Option<Vec<ImageRef>>,
}

#[derive(Deserialize)]
struct ImageRef {
    url: Option<String>,
    width: Option<u32>,
}

/// A 200 body of Get Playback State. `None` when there is no current item (e.g. an ad).
pub fn parse_player(body: &[u8]) -> Result<Option<Track>, String> {
    let player: Player =
        serde_json::from_slice(body).map_err(|e| format!("unexpected player response: {e}"))?;
    let Some(item) = player.item else {
        return Ok(None);
    };
    let is_episode = item.kind.as_deref() == Some("episode");
    let (artist, art) = if is_episode {
        // Show name as the artist; the episode's own images, else the show's.
        let show = item.show.as_ref();
        let own = images(&item.images);
        let art = if own.is_empty() {
            show.map(|s| images(&s.images)).unwrap_or_default()
        } else {
            own
        };
        (show.and_then(|s| s.name.clone()).unwrap_or_default(), art)
    } else {
        let artists: Vec<&str> = item
            .artists
            .iter()
            .flatten()
            .filter_map(|a| a.name.as_deref())
            .collect();
        let art = item
            .album
            .as_ref()
            .map(|a| images(&a.images))
            .unwrap_or_default();
        (artists.join(", "), art)
    };
    Ok(Some(Track {
        title: item.name.unwrap_or_default(),
        artist,
        art_url: pick_art(&art),
        is_playing: player.is_playing.unwrap_or(false),
        device: player.device.and_then(|d| d.name).unwrap_or_default(),
        is_episode,
    }))
}

/// The entries of a nullable image list that have a URL.
fn images(list: &Option<Vec<ImageRef>>) -> Vec<&ImageRef> {
    list.iter()
        .flatten()
        .filter(|i| i.url.as_deref().is_some_and(|u| !u.is_empty()))
        .collect()
}

/// The smallest rendition at least ART_PX wide; else the widest known; else the first
/// (the web app always took the first, i.e. the 640px one).
fn pick_art(images: &[&ImageRef]) -> String {
    let big_enough = images
        .iter()
        .filter(|i| i.width.is_some_and(|w| w >= ART_PX))
        .min_by_key(|i| i.width);
    let widest = || images.iter().rev().max_by_key(|i| i.width.unwrap_or(0));
    big_enough
        .or_else(widest)
        .and_then(|i| i.url.clone())
        .unwrap_or_default()
}

/// (icon, text) for the device line; icon 0 speaker, 1 smartphone, 2 car (Polestar).
/// Empty text means no device line.
pub fn device_info(name: &str) -> (i32, String) {
    match name {
        "" | "Unknown Device" => (0, String::new()),
        "PS2" => (2, "Polestar 2".into()),
        n if n.to_lowercase().contains("iphone") => (1, n.into()),
        n => (0, n.into()),
    }
}

// ---- HTTP outcomes (pure) -----------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Reply {
    Body,
    Nothing,
    Unauthorized,
    RateLimited(Duration),
    Failed(String),
}

/// Get Playback State status handling. 204 is the documented "nothing playing"; 404 is
/// treated the same, as the web handler does.
fn classify(status: u16, retry_after: Option<&str>) -> Reply {
    match status {
        200 => Reply::Body,
        204 | 404 => Reply::Nothing,
        401 => Reply::Unauthorized,
        429 => Reply::RateLimited(parse_retry_after(retry_after)),
        s => Reply::Failed(format!("HTTP {s}")),
    }
}

/// `Retry-After` as Spotify sends it: whole seconds. (The HTTP-date form of RFC 9110
/// §10.2.3 is not used by Spotify per its docs; it falls back to the default.)
pub fn parse_retry_after(value: Option<&str>) -> Duration {
    value
        .and_then(|v| v.trim().parse::<u64>().ok())
        .map_or(RETRY_AFTER_DEFAULT, |s| {
            Duration::from_secs(s).min(RETRY_AFTER_MAX)
        })
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: Option<u64>,
    refresh_token: Option<String>,
}

#[derive(Deserialize, Default)]
struct TokenError {
    error: Option<String>,
    error_description: Option<String>,
}

/// Message for a failed token request: OAuth's `error_description` (RFC 6749 §5.2, e.g.
/// "Refresh token revoked"), else `error`, else the web app's fallback text.
fn token_error(status: u16, body: &[u8]) -> String {
    let e: TokenError = serde_json::from_slice(body).unwrap_or_default();
    [e.error_description, e.error]
        .into_iter()
        .flatten()
        .find(|s| !s.is_empty())
        .unwrap_or_else(|| format!("Failed to get access token: HTTP {status}"))
}

/// Next poll: fast while anything plays, slow otherwise, never before a 429 wait is over.
pub fn next_poll(any_playing: bool, blocked_for: Option<Duration>) -> Duration {
    let base = if any_playing {
        timing::VERY_FAST
    } else {
        timing::MEDIUM_FAST
    };
    blocked_for.map_or(base, |b| b.max(base))
}

// ---- per-account state -------------------------------------------------------------------

#[derive(Default)]
struct TokenCache {
    access: Option<(String, Instant)>,
}

impl TokenCache {
    /// The cached access token if it is still good for at least TOKEN_MARGIN.
    fn get(&self, now: Instant) -> Option<String> {
        self.access
            .as_ref()
            .filter(|(_, expires)| now + TOKEN_MARGIN < *expires)
            .map(|(token, _)| token.clone())
    }

    /// Without `expires_in` the token is used once and not cached.
    fn store(&mut self, token: String, expires_in: Option<u64>, now: Instant) {
        let life = Duration::from_secs(expires_in.unwrap_or(0).min(24 * 60 * 60));
        self.access = Some((token, now + life));
    }

    fn clear(&mut self) {
        self.access = None;
    }
}

struct Art {
    url: String,
    pixels: Option<SharedPixelBuffer<Rgb8Pixel>>,
    tries: u8,
}

struct Account {
    /// "account N" (position in the config) for logs; the name is a config value.
    label: String,
    name: String,
    refresh_token: String,
    token: TokenCache,
    failures: u32,
    retry_at: Option<Instant>,
    /// The error last logged; cleared on success, so each change of state logs once.
    logged: Option<String>,
    track: Option<Track>,
    art: Option<Art>,
}

impl Account {
    fn new(index: usize, name: String, refresh_token: String) -> Account {
        Account {
            label: format!("account {}", index + 1),
            name,
            refresh_token,
            token: TokenCache::default(),
            failures: 0,
            retry_at: None,
            logged: None,
            track: None,
            art: None,
        }
    }

    fn due(&self, now: Instant) -> bool {
        self.retry_at.is_none_or(|t| now >= t)
    }

    /// Returns a line to log, if this changes the account's state.
    fn succeeded(&mut self, track: Option<Track>) -> Option<String> {
        let recovered = self
            .logged
            .take()
            .map(|_| format!("{}: recovered", self.label));
        self.failures = 0;
        self.retry_at = None;
        if track.is_none() {
            self.art = None;
        }
        self.track = track;
        recovered
    }

    /// Returns a line to log, if the error differs from the one last logged.
    fn failed(&mut self, error: String, now: Instant) -> Option<String> {
        self.failures = self.failures.saturating_add(1);
        self.retry_at = Some(now + fetch::retry_delay(self.failures, ACCOUNT_BACKOFF_CAP));
        if self.failures > KEEP_THROUGH_FAILURES {
            self.track = None;
            self.art = None;
        }
        if self.logged.as_deref() == Some(error.as_str()) {
            return None;
        }
        let line = format!("{}: {error}", self.label);
        self.logged = Some(error);
        Some(line)
    }
}

struct Shared {
    accounts: Vec<Account>,
    /// Set by a 429: no request to Spotify before this.
    blocked_until: Option<Instant>,
}

impl Shared {
    fn blocked_for(&self, now: Instant) -> Option<Duration> {
        self.blocked_until
            .and_then(|t| t.checked_duration_since(now))
            .filter(|d| !d.is_zero())
    }

    /// Returns a line to log when this starts a rate-limit pause.
    fn rate_limited(&mut self, wait: Duration, now: Instant) -> Option<String> {
        let was_blocked = self.blocked_for(now).is_some();
        let until = now + wait;
        self.blocked_until = Some(self.blocked_until.map_or(until, |t| t.max(until)));
        (!was_blocked).then(|| format!("rate limited (429); no requests for {}s", wait.as_secs()))
    }

    fn snapshot(&self, now: Instant) -> Snapshot {
        let rows: Vec<Row> = self
            .accounts
            .iter()
            .filter_map(|a| {
                let track = a.track.clone()?;
                let art = a
                    .art
                    .as_ref()
                    .filter(|art| art.url == track.art_url)
                    .and_then(|art| art.pixels.clone());
                Some(Row {
                    track,
                    account: a.name.clone(),
                    art,
                })
            })
            .collect();
        let playing = rows.iter().any(|r| r.track.is_playing);
        Snapshot {
            next: next_poll(playing, self.blocked_for(now)),
            rows,
        }
    }
}

fn lock(state: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the UI gets after each poll.
pub struct Snapshot {
    rows: Vec<Row>,
    /// When to poll next (read by the scheduler through `spawn_paced`).
    next: Duration,
}

struct Row {
    track: Track,
    account: String,
    art: Option<SharedPixelBuffer<Rgb8Pixel>>,
}

// ---- network -----------------------------------------------------------------------------

struct Credentials {
    id: String,
    secret: String,
    /// spotify-tokens.json (see src/spotify_tokens.rs).
    tokens_path: std::path::PathBuf,
    write_warned: std::sync::atomic::AtomicBool,
}

impl Credentials {
    /// Save a rotated refresh token under the account's name. If that fails (or the
    /// account has no name) it stays in memory only; logged once per process.
    fn persist(&self, label: &str, name: &str, token: &str) {
        let result = if name.is_empty() {
            Err("the account has no name in config".to_string())
        } else {
            crate::spotify_tokens::save(&self.tokens_path, name, token)
        };
        if let Err(e) = result {
            if !self.write_warned.swap(true, std::sync::atomic::Ordering::Relaxed) {
                eprintln!("spotify: {label}: cannot save the new refresh token ({e}); keeping it in memory");
            }
        }
    }
}

enum Failure {
    RateLimited(Duration),
    Error(String),
}

fn net(e: reqwest::Error) -> Failure {
    Failure::Error(fetch::clean(e))
}

fn retry_after(response: &reqwest::Response) -> Option<String> {
    let value = response.headers().get(reqwest::header::RETRY_AFTER)?;
    value.to_str().ok().map(str::to_owned)
}

/// One poll of every account that is not backing off (all concurrently), unless a 429
/// pause is running. Never fails: per-account errors are recorded per account.
async fn poll(
    http: reqwest::Client,
    creds: Arc<Credentials>,
    state: Arc<Mutex<Shared>>,
) -> Result<Snapshot, String> {
    let now = Instant::now();
    let due: Vec<usize> = {
        let mut s = lock(&state);
        if s.blocked_for(now).is_some() {
            Vec::new()
        } else {
            if s.blocked_until.take().is_some() {
                eprintln!("spotify: rate-limit pause over");
            }
            (0..s.accounts.len())
                .filter(|&i| s.accounts[i].due(now))
                .collect()
        }
    };
    let tasks: Vec<_> = due
        .into_iter()
        .map(|i| {
            let (http, creds, state) = (http.clone(), creds.clone(), state.clone());
            let task = async move {
                tokio::time::timeout(ACCOUNT_TIMEOUT, poll_account(&http, &creds, &state, i)).await
            };
            (i, tokio::spawn(task))
        })
        .collect();
    for (i, task) in tasks {
        let result = match task.await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Failure::Error(format!(
                "timed out after {}s",
                ACCOUNT_TIMEOUT.as_secs()
            ))),
            Err(_) => Err(Failure::Error("panicked".into())),
        };
        let mut s = lock(&state);
        let now = Instant::now();
        let line = match result {
            Ok(track) => s.accounts[i].succeeded(track),
            Err(Failure::Error(e)) => s.accounts[i].failed(e, now),
            Err(Failure::RateLimited(wait)) => s.rate_limited(wait, now),
        };
        if let Some(line) = line {
            eprintln!("spotify: {line}");
        }
    }
    Ok(lock(&state).snapshot(Instant::now()))
}

async fn poll_account(
    http: &reqwest::Client,
    creds: &Credentials,
    state: &Mutex<Shared>,
    i: usize,
) -> Result<Option<Track>, Failure> {
    let cached = {
        let s = lock(state);
        let a = &s.accounts[i];
        a.token
            .get(Instant::now())
            .ok_or_else(|| a.refresh_token.clone())
    };
    let token = match cached {
        Ok(token) => token,
        Err(refresh_token) => {
            let fresh = refresh(http, creds, &refresh_token).await?;
            let mut s = lock(state);
            let a = &mut s.accounts[i];
            a.token
                .store(fresh.access_token.clone(), fresh.expires_in, Instant::now());
            // Rotated: saved to spotify-tokens.json, never to the config file.
            if let Some(rotated) = fresh.refresh_token.filter(|t| !t.is_empty() && *t != a.refresh_token) {
                creds.persist(&a.label, &a.name, &rotated);
                a.refresh_token = rotated;
            }
            fresh.access_token
        }
    };

    let response = http
        .get(PLAYER_URL)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(net)?;
    let track = match classify(
        response.status().as_u16(),
        retry_after(&response).as_deref(),
    ) {
        Reply::Body => {
            parse_player(&response.bytes().await.map_err(net)?).map_err(Failure::Error)?
        }
        Reply::Nothing => None,
        Reply::Unauthorized => {
            lock(state).accounts[i].token.clear();
            return Err(Failure::Error("HTTP 401 (access token rejected)".into()));
        }
        Reply::RateLimited(wait) => return Err(Failure::RateLimited(wait)),
        Reply::Failed(e) => return Err(Failure::Error(e)),
    };
    if let Some(track) = &track {
        load_art(http, state, i, &track.art_url).await;
    }
    Ok(track)
}

async fn refresh(
    http: &reqwest::Client,
    creds: &Credentials,
    refresh_token: &str,
) -> Result<TokenResponse, Failure> {
    let response = http
        .post(TOKEN_URL)
        .basic_auth(&creds.id, Some(&creds.secret))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(net)?;
    let status = response.status().as_u16();
    if status == 429 {
        return Err(Failure::RateLimited(parse_retry_after(
            retry_after(&response).as_deref(),
        )));
    }
    let body = response.bytes().await.map_err(net)?;
    if !(200..300).contains(&status) {
        return Err(Failure::Error(token_error(status, &body)));
    }
    // Generic message: a serde error could quote part of the body (the token).
    serde_json::from_slice(&body).map_err(|_| Failure::Error("unexpected token response".into()))
}

/// Make sure the account's art matches `url`: fetched and decoded once per URL (up to
/// ART_TRIES attempts), and the previous track's art let go before the new one loads.
async fn load_art(http: &reqwest::Client, state: &Mutex<Shared>, i: usize, url: &str) {
    let tries = {
        let mut s = lock(state);
        let a = &mut s.accounts[i];
        match &a.art {
            _ if url.is_empty() => {
                a.art = None;
                return;
            }
            Some(art) if art.url == url && (art.pixels.is_some() || art.tries >= ART_TRIES) => {
                return
            }
            Some(art) if art.url == url => art.tries,
            _ => {
                a.art = None;
                0
            }
        }
    };
    let result = match download(http, url).await {
        Ok(bytes) => decode_art(&bytes),
        Err(e) => Err(e),
    };
    let mut s = lock(state);
    let a = &mut s.accounts[i];
    if let Err(e) = &result {
        if tries == 0 {
            eprintln!("spotify: {}: album art: {e}", a.label);
        }
    }
    a.art = Some(Art {
        url: url.to_owned(),
        pixels: result.ok(),
        tries: tries + 1,
    });
}

/// GET with a hard size limit (Spotify's 300px JPEGs are ~20-60 KB).
async fn download(http: &reqwest::Client, url: &str) -> Result<Vec<u8>, String> {
    let mut response = http
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(fetch::clean)?;
    let too_big = || format!("image larger than {} KB", ART_MAX_BYTES / 1024);
    if response
        .content_length()
        .is_some_and(|n| n > ART_MAX_BYTES as u64)
    {
        return Err(too_big());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(fetch::clean)? {
        if body.len() + chunk.len() > ART_MAX_BYTES {
            return Err(too_big());
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Decode a JPEG/PNG and crop-scale it to ART_PX square RGB (album art is opaque).
pub fn decode_art(bytes: &[u8]) -> Result<SharedPixelBuffer<Rgb8Pixel>, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(ART_MAX_SIDE);
    limits.max_image_height = Some(ART_MAX_SIDE);
    limits.max_alloc = Some(32 << 20);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| e.to_string())?;
    let art = decoded
        .resize_to_fill(ART_PX, ART_PX, image::imageops::FilterType::Triangle)
        .into_rgb8();
    Ok(SharedPixelBuffer::clone_from_slice(
        art.as_raw(),
        art.width(),
        art.height(),
    ))
}

// ---- widget ------------------------------------------------------------------------------

pub fn start(ctx: &Ctx) {
    layout::set_visible(ctx.ui, ID, false); // until something is playing
    let s: Settings = ctx.config_as();
    let id = s.client_id.unwrap_or_default();
    let secret = s.client_secret.unwrap_or_default();
    if id.is_empty() || secret.is_empty() {
        eprintln!("spotify: Missing Spotify credentials; not polling");
        return;
    }
    // Saved (rotated or `mmm-native auth spotify`) tokens win over config ones by name.
    let tokens_path = crate::spotify_tokens::path_for(&crate::config::path_from_args());
    let saved = crate::spotify_tokens::load(&tokens_path).unwrap_or_else(|e| {
        eprintln!("spotify: {e}; using the tokens from config");
        Default::default()
    });
    let configured: Vec<(Option<String>, Option<String>)> = s
        .accounts
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.name, a.refresh_token))
        .collect();
    let accounts: Vec<Account> = crate::spotify_tokens::resolve(&configured, &saved)
        .into_iter()
        .map(|r| {
            if r.from_file {
                eprintln!("spotify: account {}: using the saved refresh token", r.index + 1);
            }
            Account::new(r.index, r.name, r.refresh_token)
        })
        .collect();
    if accounts.is_empty() {
        eprintln!("spotify: no connected accounts; not polling");
        return;
    }
    let styles = (
        style(
            &[s.track_style.as_deref(), s.album_art_style.as_deref()],
            Style::Vinyl,
        ),
        style(&[s.episode_style.as_deref()], Style::Cassette),
    );

    let creds = Arc::new(Credentials {
        id,
        secret,
        tokens_path,
        write_warned: Default::default(),
    });
    let state = Arc::new(Mutex::new(Shared {
        accounts,
        blocked_until: None,
    }));
    ctx.fetch.spawn_paced(
        Job::new(ID, timing::MEDIUM_FAST).timeout(JOB_TIMEOUT),
        move |http| poll(http, creds.clone(), state.clone()),
        |snapshot: &Snapshot| snapshot.next,
        ctx.ui.as_weak(),
        move |ui, update| show(ui, update, styles),
    );
}

fn show(ui: &AppWindow, update: &Update<Snapshot>, styles: (Style, Style)) {
    // Polls never fail as a whole (accounts fail individually), so `stale` means the
    // job itself is stuck; an old "now playing" would then be misleading.
    let rows: &[Row] = match &update.value {
        Some(s) if !update.stale => &s.rows,
        _ => &[],
    };
    let new: Vec<SpotifyRow> = rows.iter().map(|r| ui_row(r, styles)).collect();
    let data = ui.global::<SpotifyData>();
    // Images compare by pixel-buffer identity, so unchanged art does not count as a change.
    if !data.get_rows().iter().eq(new.iter().cloned()) {
        data.set_rows(ModelRc::new(VecModel::from(new)));
    }
    layout::set_visible(ui, ID, !rows.is_empty());
}

fn ui_row(row: &Row, (track_style, episode_style): (Style, Style)) -> SpotifyRow {
    let (device_icon, device) = device_info(&row.track.device);
    let style = if row.track.is_episode {
        episode_style
    } else {
        track_style
    };
    SpotifyRow {
        title: row.track.title.as_str().into(),
        artist: row.track.artist.as_str().into(),
        account: row.account.as_str().into(),
        device: device.into(),
        device_icon,
        has_art: row.art.is_some(),
        art: row.art.clone().map(Image::from_rgb8).unwrap_or_default(),
        style: style as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK: &[u8] = include_bytes!("../../tests/fixtures/spotify/player-track.json");
    const EPISODE: &[u8] = include_bytes!("../../tests/fixtures/spotify/player-episode.json");
    const S: fn(u64) -> Duration = Duration::from_secs;

    #[test]
    fn parses_a_track() {
        let t = parse_player(TRACK).unwrap().unwrap();
        assert_eq!(t.title, "Global Warming (feat. Sensato)");
        assert_eq!(t.artist, "Pitbull, Sensato");
        assert!(t.is_playing && !t.is_episode);
        assert_eq!(t.device, "Kitchen");
        // The 300px rendition, not the 640px first entry or the 64px one.
        assert_eq!(
            t.art_url,
            "https://i.scdn.co/image/ab67616d00001e022c5b24ecfa39523a75c993c4"
        );
    }

    #[test]
    fn parses_an_episode() {
        let t = parse_player(EPISODE).unwrap().unwrap();
        assert!(t.is_episode && !t.is_playing, "paused items still show");
        assert_eq!(t.artist, "Anchor Creators", "show name as artist");
        assert_eq!(t.device, "Max's iPhone");
        assert_eq!(
            t.art_url,
            "https://i.scdn.co/image/ab67656300005f1fepisode300"
        );

        // No episode images: fall back to the show's (here only a 640px one).
        let mut v: serde_json::Value = serde_json::from_slice(EPISODE).unwrap();
        v["item"]["images"] = serde_json::json!([]);
        let t = parse_player(v.to_string().as_bytes()).unwrap().unwrap();
        assert_eq!(t.art_url, "https://i.scdn.co/image/ab6765630000ba8ashow640");
    }

    #[test]
    fn tolerates_nulls_and_missing_fields() {
        // An ad: item null.
        let ad = br#"{"device":null,"progress_ms":null,"is_playing":true,"item":null,"currently_playing_type":"ad"}"#;
        assert_eq!(parse_player(ad).unwrap(), None);
        let bare = br#"{"item":{"type":"track","name":"X","artists":null,"album":{"images":[{"url":"u","width":null,"height":null}]}}}"#;
        let t = parse_player(bare).unwrap().unwrap();
        assert_eq!(
            (t.title.as_str(), t.artist.as_str(), t.art_url.as_str()),
            ("X", "", "u")
        );
        assert_eq!((t.device.as_str(), t.is_playing), ("", false));
        assert!(parse_player(b"<html>").is_err());
        assert!(parse_player(b"").is_err());
    }

    #[test]
    fn art_rendition_choice() {
        let img = |url: &str, width: Option<u32>| ImageRef {
            url: Some(url.into()),
            width,
        };
        let list = [img("a", Some(640)), img("b", Some(300)), img("c", Some(64))];
        assert_eq!(pick_art(&list.iter().collect::<Vec<_>>()), "b");
        let small = [img("a", Some(64)), img("b", Some(100))];
        assert_eq!(
            pick_art(&small.iter().collect::<Vec<_>>()),
            "b",
            "widest when all are small"
        );
        let unknown = [img("a", None), img("b", None)];
        assert_eq!(
            pick_art(&unknown.iter().collect::<Vec<_>>()),
            "a",
            "first when sizes are unknown"
        );
        assert_eq!(pick_art(&[]), "");
    }

    #[test]
    fn status_handling() {
        assert_eq!(classify(200, None), Reply::Body);
        assert_eq!(classify(204, None), Reply::Nothing);
        assert_eq!(classify(404, None), Reply::Nothing);
        assert_eq!(classify(401, None), Reply::Unauthorized);
        assert_eq!(classify(429, Some("7")), Reply::RateLimited(S(7)));
        assert_eq!(classify(503, None), Reply::Failed("HTTP 503".into()));
    }

    #[test]
    fn retry_after_values() {
        assert_eq!(parse_retry_after(Some("120")), S(120));
        assert_eq!(parse_retry_after(Some(" 3 ")), S(3));
        assert_eq!(parse_retry_after(Some("0")), S(0));
        assert_eq!(parse_retry_after(None), RETRY_AFTER_DEFAULT);
        assert_eq!(
            parse_retry_after(Some("Wed, 21 Oct 2015 07:28:00 GMT")),
            RETRY_AFTER_DEFAULT
        );
        assert_eq!(parse_retry_after(Some("99999999999")), RETRY_AFTER_MAX);
    }

    #[test]
    fn token_error_text() {
        let revoked = br#"{"error":"invalid_grant","error_description":"Refresh token revoked"}"#;
        assert_eq!(token_error(400, revoked), "Refresh token revoked");
        assert_eq!(
            token_error(400, br#"{"error":"invalid_client"}"#),
            "invalid_client"
        );
        assert_eq!(
            token_error(502, b"Bad gateway"),
            "Failed to get access token: HTTP 502"
        );
    }

    #[test]
    fn token_cache_until_shortly_before_expiry() {
        let t0 = Instant::now();
        let mut c = TokenCache::default();
        assert_eq!(c.get(t0), None);
        c.store("tok".into(), Some(3600), t0);
        assert_eq!(c.get(t0).as_deref(), Some("tok"));
        assert_eq!(c.get(t0 + S(3539)).as_deref(), Some("tok"));
        assert_eq!(c.get(t0 + S(3540)), None, "refreshed a minute early");
        c.clear();
        assert_eq!(c.get(t0), None);
        c.store("once".into(), None, t0);
        assert_eq!(c.get(t0), None, "no expires_in: not cached");
    }

    #[test]
    fn adaptive_cadence() {
        assert_eq!(next_poll(true, None), S(5));
        assert_eq!(next_poll(false, None), S(30));
        assert_eq!(next_poll(true, Some(S(120))), S(120), "Retry-After wins");
        assert_eq!(next_poll(true, Some(S(2))), S(5));
    }

    fn track(title: &str, playing: bool) -> Track {
        Track {
            title: title.into(),
            art_url: format!("art/{title}"),
            is_playing: playing,
            ..Track::default()
        }
    }

    fn shared(n: usize) -> Shared {
        let accounts = (0..n)
            .map(|i| Account::new(i, format!("name{i}"), "rt".into()))
            .collect();
        Shared {
            accounts,
            blocked_until: None,
        }
    }

    #[test]
    fn account_failures_log_once_per_state_change() {
        let t0 = Instant::now();
        let mut a = Account::new(1, "Max".into(), "rt".into());
        assert_eq!(a.succeeded(Some(track("x", true))), None);
        let revoked = || "Refresh token revoked".to_string();
        assert_eq!(
            a.failed(revoked(), t0).as_deref(),
            Some("account 2: Refresh token revoked")
        );
        assert_eq!(a.failed(revoked(), t0), None, "same error: silent");
        assert!(a.track.is_some(), "kept through two failures");
        assert_eq!(
            a.failed("HTTP 500".into(), t0).as_deref(),
            Some("account 2: HTTP 500")
        );
        assert!(a.track.is_none(), "dropped at the third");
        assert_eq!(a.succeeded(None).as_deref(), Some("account 2: recovered"));
        assert_eq!(a.succeeded(None), None);
    }

    #[test]
    fn failing_account_backs_off() {
        let t0 = Instant::now();
        let mut a = Account::new(0, String::new(), "rt".into());
        assert!(a.due(t0));
        a.failed("x".into(), t0);
        assert!(!a.due(t0 + S(9)) && a.due(t0 + S(10)));
        a.failed("x".into(), t0);
        assert!(!a.due(t0 + S(19)) && a.due(t0 + S(20)));
        for _ in 0..20 {
            a.failed("x".into(), t0);
        }
        assert!(
            !a.due(t0 + S(899)) && a.due(t0 + S(900)),
            "capped at 15 min"
        );
        a.succeeded(None);
        assert!(a.due(t0));
    }

    #[test]
    fn one_failing_account_does_not_hide_the_other() {
        let t0 = Instant::now();
        let mut s = shared(2);
        s.accounts[0].succeeded(Some(track("a", false)));
        for _ in 0..5 {
            s.accounts[1].failed("Refresh token revoked".into(), t0);
        }
        let snap = s.snapshot(t0);
        assert_eq!(snap.rows.len(), 1);
        assert_eq!(snap.rows[0].account, "name0");
        assert_eq!(snap.next, S(30), "paused: slow polling");
        s.accounts[1].succeeded(Some(track("b", true)));
        let snap = s.snapshot(t0);
        assert_eq!(snap.rows.len(), 2);
        assert_eq!(snap.next, S(5), "something playing: fast polling");
        assert!(shared(2).snapshot(t0).rows.is_empty());
    }

    #[test]
    fn rate_limit_pauses_everything_and_logs_once() {
        let t0 = Instant::now();
        let mut s = shared(2);
        s.accounts[0].succeeded(Some(track("a", true)));
        assert!(s.rate_limited(S(90), t0).is_some());
        assert!(s.rate_limited(S(10), t0 + S(1)).is_none(), "already paused");
        let snap = s.snapshot(t0 + S(30));
        assert_eq!(snap.next, S(60), "waits out Retry-After even while playing");
        assert_eq!(snap.rows.len(), 1, "tracks stay up while paused");
        assert_eq!(s.blocked_for(t0 + S(90)), None);
        assert!(
            s.rate_limited(S(5), t0 + S(100)).is_some(),
            "a new pause logs again"
        );
    }

    #[test]
    fn art_shown_only_for_its_own_track() {
        let t0 = Instant::now();
        let mut s = shared(1);
        let px = SharedPixelBuffer::<Rgb8Pixel>::new(2, 2);
        s.accounts[0].succeeded(Some(track("a", true)));
        s.accounts[0].art = Some(Art {
            url: "art/a".into(),
            pixels: Some(px),
            tries: 1,
        });
        assert!(s.snapshot(t0).rows[0].art.is_some());
        s.accounts[0].succeeded(Some(track("b", true)));
        assert!(
            s.snapshot(t0).rows[0].art.is_none(),
            "old art is not shown for a new track"
        );
        s.accounts[0].succeeded(None);
        assert!(
            s.accounts[0].art.is_none(),
            "art dropped when nothing plays"
        );
    }

    #[test]
    fn decodes_and_scales_art() {
        let mut jpeg = Vec::new();
        let src = image::RgbImage::from_pixel(300, 200, image::Rgb([200, 30, 30]));
        image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
            .encode_image(&src)
            .unwrap();
        let px = decode_art(&jpeg).unwrap();
        assert_eq!((px.width(), px.height()), (ART_PX, ART_PX));
        let p = px.as_slice()[px.as_slice().len() / 2];
        assert!(p.r > 150 && p.g < 80, "{p:?}");

        assert!(decode_art(b"not an image").is_err());
        let mut tall = Vec::new();
        image::GrayImage::new(1, ART_MAX_SIDE + 1)
            .write_to(
                &mut std::io::Cursor::new(&mut tall),
                image::ImageFormat::Png,
            )
            .unwrap();
        assert!(decode_art(&tall).is_err(), "over the size limit");
    }

    #[test]
    fn same_art_buffer_is_an_unchanged_image() {
        // `show` relies on this to skip model updates when nothing changed.
        let px = SharedPixelBuffer::<Rgb8Pixel>::new(4, 4);
        assert_eq!(Image::from_rgb8(px.clone()), Image::from_rgb8(px.clone()));
        assert_ne!(
            Image::from_rgb8(px),
            Image::from_rgb8(SharedPixelBuffer::new(4, 4))
        );
    }

    #[test]
    fn styles_and_devices() {
        assert_eq!(style(&[None, None], Style::Vinyl), Style::Vinyl);
        assert_eq!(
            style(&[Some(""), Some("cassette")], Style::Vinyl),
            Style::Cassette,
            "legacy albumArtStyle"
        );
        assert_eq!(style(&[Some("square")], Style::Cassette), Style::Square);
        assert_eq!(
            style(&[Some("bogus")], Style::Vinyl),
            Style::Square,
            "as the web template"
        );
        assert_eq!(device_info("PS2"), (2, "Polestar 2".into()));
        assert_eq!(device_info("Max's IPHONE"), (1, "Max's IPHONE".into()));
        assert_eq!(device_info("Kitchen"), (0, "Kitchen".into()));
        assert_eq!(device_info("Unknown Device").1, "");
        assert_eq!(device_info("").1, "");
    }
}
