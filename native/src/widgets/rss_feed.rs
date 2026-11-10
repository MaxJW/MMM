//! News headlines: port of src/components/rss-feed/{api.ts,component.svelte}.
//!
//! Each configured feed is its own fetch job with its own 15s timeout, retry and
//! last-good value, so a failing feed only drops that feed. Articles from all feeds are
//! merged, limited to the past week, sorted newest first and rotated every
//! `rotationSpeed` seconds (default 10) with a short crossfade.

use std::cell::RefCell;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use roxmltree::{Document, Node, ParsingOptions};
use serde_json::Value;
use slint::{ComponentHandle, SharedString};

use super::{set_if_changed, Ctx};
use crate::fetch::{self, Job, Update};
use crate::{timing, AppWindow, RssFeedData};

pub const ID: &str = "rss-feed";
/// Per feed, as api.ts (`AbortSignal.timeout(15_000)`).
const FEED_TIMEOUT: Duration = Duration::from_secs(15);
/// A feed body larger than this is refused rather than buffered.
const MAX_BODY: usize = 4 << 20;
const MAX_AGE_DAYS: i64 = 7;
const DEFAULT_ROTATION_SECS: f64 = 10.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Article {
    pub title: String,
    pub source: String,
    pub date: DateTime<Utc>,
}

#[derive(Debug, PartialEq)]
struct FeedConfig {
    url: String,
    source_name: String,
}

/// (`rssFeeds`, `rotationSpeed`) from the `components.rss-feed` slice.
fn settings(config: &Value) -> (Vec<FeedConfig>, Duration) {
    let feeds = config
        .get("rssFeeds")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(|f| {
                    let url = f.get("url")?.as_str()?.trim();
                    (!url.is_empty()).then(|| FeedConfig {
                        url: url.into(),
                        source_name: f.get("sourceName").and_then(Value::as_str).unwrap_or("").trim().into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let secs = match config.get("rotationSpeed") {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|s: &f64| s.is_finite())
    .unwrap_or(DEFAULT_ROTATION_SECS)
    .clamp(1.0, 300.0); // manifest min/max
    (feeds, Duration::from_secs_f64(secs))
}

// ---- Parsing (pure) ---------------------------------------------------------------

const DC: &str = "http://purl.org/dc/elements/1.1/";

fn child<'a, 'i>(node: Node<'a, 'i>, name: &str, ns: Option<&str>) -> Option<Node<'a, 'i>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name && n.tag_name().namespace() == ns)
}

fn children<'a, 'i>(node: Node<'a, 'i>, name: &str, ns: Option<&str>) -> Vec<Node<'a, 'i>> {
    node.children()
        .filter(|n| n.is_element() && n.tag_name().name() == name && n.tag_name().namespace() == ns)
        .collect()
}

/// Articles of one RSS 2.0, RSS 1.0 (RDF) or Atom document. Items without a parseable
/// date are dropped (the web app's week filter drops them too); a missing title
/// becomes "Untitled". `source_name` overrides the feed's own title.
pub fn parse_feed(xml: &str, source_name: &str) -> Result<Vec<Article>, String> {
    let options = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(xml.trim_start_matches('\u{feff}'), options)
        .map_err(|e| format!("invalid XML: {e}"))?;
    let root = doc.root_element();
    let (channel, items) = match root.tag_name().name() {
        "rss" => {
            let channel = child(root, "channel", None).ok_or("RSS without <channel>")?;
            (channel, children(channel, "item", None))
        }
        "RDF" => {
            // RSS 1.0: items are siblings of <channel>, both in the RSS 1.0 namespace.
            let channel = root
                .children()
                .find(|n| n.is_element() && n.tag_name().name() == "channel")
                .ok_or("RDF without <channel>")?;
            (channel, children(root, "item", channel.tag_name().namespace()))
        }
        "feed" => (root, children(root, "entry", root.tag_name().namespace())),
        _ => return Err("not an RSS or Atom feed".into()),
    };
    let source = if !source_name.is_empty() {
        source_name.to_string()
    } else {
        child(channel, "title", channel.tag_name().namespace())
            .map(text_of)
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| "RSS Feed".into())
    };
    Ok(items
        .into_iter()
        .filter_map(|item| {
            let ns = item.tag_name().namespace();
            // pubDate (RSS 2.0), published/updated (Atom), dc:date (RSS 1.0 and others).
            let date = ["pubDate", "published", "updated"]
                .iter()
                .filter_map(|name| child(item, name, ns))
                .chain(child(item, "date", Some(DC)))
                .find_map(|n| parse_date(&n.descendants().filter(|t| t.is_text()).filter_map(|t| t.text()).collect::<String>()))?;
            let title = child(item, "title", ns).map(text_of).unwrap_or_default();
            Some(Article {
                title: if title.is_empty() { "Untitled".into() } else { title },
                source: source.clone(),
                date,
            })
        })
        .collect())
}

/// Text of an element (including CDATA and Atom xhtml children), with HTML entities
/// decoded, tags removed from Atom `type="html"` text, and whitespace collapsed.
fn text_of(node: Node) -> String {
    let raw: String = node.descendants().filter(|n| n.is_text()).filter_map(|n| n.text()).collect();
    let raw = if node.attribute("type") == Some("html") { strip_tags(&raw) } else { raw };
    decode_html(&raw).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Decode HTML character references left after XML parsing (CDATA titles, double
/// escaping such as `&amp;#8217;`), in a single pass so nothing is decoded twice.
/// Unknown references are kept as they are.
pub fn decode_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let reference = rest[1..]
            .char_indices()
            .take(12)
            .find(|&(_, c)| c == ';')
            .and_then(|(end, _)| Some((entity(&rest[1..1 + end])?, end + 2)));
        match reference {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    if let Some(num) = name.strip_prefix('#') {
        let code = match num.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => num.parse().ok()?,
        };
        return char::from_u32(code).filter(|&c| c != '\0');
    }
    Some(match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        "copy" => '©',
        "reg" => '®',
        "trade" => '™',
        "pound" => '£',
        "euro" => '€',
        "hellip" => '…',
        "ndash" => '–',
        "mdash" => '—',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201C}',
        "rdquo" => '\u{201D}',
        "bull" => '•',
        "middot" => '·',
        _ => return None,
    })
}

/// Feed dates: RFC 822/2822 (RSS 2.0, RSS Advisory Board spec), RFC 3339 (Atom,
/// RFC 4287 §3.3; dc:date), and the looser ISO forms JavaScript's `Date` accepts.
pub fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();
    let utc = |d: DateTime<chrono::FixedOffset>| d.with_timezone(&Utc);
    if let Ok(d) = DateTime::parse_from_rfc2822(s) {
        return Some(utc(d));
    }
    // JavaScript ignores the weekday; chrono rejects a wrong one, so retry without it.
    if let Some((day, rest)) = s.split_once(',') {
        if day.chars().all(char::is_alphabetic) {
            if let Ok(d) = DateTime::parse_from_rfc2822(rest.trim()) {
                return Some(utc(d));
            }
        }
    }
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(utc(d));
    }
    for f in ["%Y-%m-%dT%H:%M:%S%.f%z", "%Y-%m-%d %H:%M:%S%.f%z", "%Y-%m-%d %H:%M:%S%.f %z"] {
        if let Ok(d) = DateTime::parse_from_str(s, f) {
            return Some(utc(d));
        }
    }
    // Date-time without an offset is local time in JavaScript...
    for f in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%dT%H:%M"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(s, f) {
            return Local.from_local_datetime(&d).earliest().map(|d| d.with_timezone(&Utc));
        }
    }
    // ...but a bare date is UTC midnight.
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()
        .map(|d| d.and_time(chrono::NaiveTime::MIN).and_utc())
}

/// All feeds' articles from the past week, newest first (stable for equal dates).
pub fn select(feeds: &[Option<Arc<Vec<Article>>>], now: DateTime<Utc>) -> Vec<Article> {
    let since = now - chrono::Duration::days(MAX_AGE_DAYS);
    let mut all: Vec<Article> = feeds
        .iter()
        .flatten()
        .flat_map(|f| f.iter())
        .filter(|a| a.date >= since)
        .cloned()
        .collect();
    all.sort_by_key(|a| std::cmp::Reverse(a.date));
    all
}

/// What the widget shows (`mode` in ui/widgets/rss-feed.slint).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Status {
    Loading = 0,
    Error = 1,
    Empty = 2,
    Headline = 3,
}

#[derive(Clone, Debug, Default)]
struct FeedState {
    value: Option<Arc<Vec<Article>>>,
    /// The latest attempt failed and there is no last-good value.
    failed: bool,
}

/// "Feed unavailable" only when every feed failed and none has data, as api.ts
/// ("All RSS feeds failed to load"); "Loading" while some feed has not answered yet.
fn status(feeds: &[FeedState], articles: &[Article]) -> Status {
    if !articles.is_empty() {
        Status::Headline
    } else if feeds.is_empty() || feeds.iter().any(|f| f.value.is_some()) {
        Status::Empty
    } else if feeds.iter().all(|f| f.failed) {
        Status::Error
    } else {
        Status::Loading
    }
}

/// "Source · HH:mm DD/MM" (component.svelte `formatDate`).
fn meta(a: &Article) -> String {
    format!("{} · {}", a.source, a.date.with_timezone(&Local).format("%H:%M %d/%m"))
}

// ---- UI thread ---------------------------------------------------------------------

#[derive(Default)]
struct State {
    feeds: Vec<FeedState>,
    articles: Vec<Article>,
    index: usize,
    /// Which of the two overlaid headline layers is in front.
    front_a: bool,
    shown: Option<(String, String)>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

pub fn start(ctx: &Ctx) {
    let (feeds, rotation) = settings(ctx.config);
    STATE.with_borrow_mut(|s| {
        s.feeds = vec![FeedState::default(); feeds.len()];
        show(ctx.ui, s);
    });
    for (i, FeedConfig { url, source_name }) in feeds.into_iter().enumerate() {
        // Job names are 'static; this runs once per feed per process (config changes re-exec).
        let name: &'static str = Box::leak(format!("{ID}#{}", i + 1).into_boxed_str());
        ctx.fetch.spawn(
            Job::new(name, timing::STANDARD).timeout(FEED_TIMEOUT),
            move |http| {
                let request = http.get(url.as_str());
                let source = source_name.clone();
                async move { parse_feed(&read_body(request).await?, &source) }
            },
            ctx.ui.as_weak(),
            move |ui, update| on_update(ui, i, update),
        );
    }
    let weak = ctx.ui.as_weak();
    timing::every_aligned(rotation, move || {
        if let Some(ui) = weak.upgrade() {
            rotate(&ui);
        }
    });
}

/// The response body as text (UTF-8; other charsets are decoded lossily), size-capped.
async fn read_body(request: reqwest::RequestBuilder) -> Result<String, String> {
    let response = request.send().await.map_err(fetch::clean)?;
    let mut response = response.error_for_status().map_err(fetch::clean)?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(fetch::clean)? {
        if body.len() + chunk.len() > MAX_BODY {
            return Err(format!("feed larger than {} MB", MAX_BODY >> 20));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

fn on_update(ui: &AppWindow, feed: usize, update: &Update<Vec<Article>>) {
    STATE.with_borrow_mut(|s| {
        if let Some(f) = s.feeds.get_mut(feed) {
            f.value = update.value.clone();
            f.failed = update.value.is_none() && update.error.is_some();
        }
        let values: Vec<_> = s.feeds.iter().map(|f| f.value.clone()).collect();
        let current = s.articles.get(s.index).map(|a| a.title.clone());
        s.articles = select(&values, Utc::now());
        // Keep the headline on screen if it is still there, else start from the newest.
        s.index = current
            .and_then(|t| s.articles.iter().position(|a| a.title == t))
            .unwrap_or(0);
        show(ui, s);
    });
}

fn rotate(ui: &AppWindow) {
    STATE.with_borrow_mut(|s| {
        if s.articles.len() > 1 {
            s.index = (s.index + 1) % s.articles.len();
            show(ui, s);
        }
    });
}

fn show(ui: &AppWindow, s: &mut State) {
    let d = ui.global::<RssFeedData>();
    let status = status(&s.feeds, &s.articles);
    set_if_changed(d.get_mode(), status as i32, |v| d.set_mode(v));
    let Some(article) = s.articles.get(s.index) else { return };
    let line = (article.title.clone(), meta(article));
    if s.shown.as_ref() == Some(&line) {
        return;
    }
    // Fill the hidden layer, then swap: the old headline fades out as the new fades in.
    let (title, meta) = (SharedString::from(&line.0), SharedString::from(&line.1));
    if s.front_a {
        d.set_title_b(title);
        d.set_meta_b(meta);
    } else {
        d.set_title_a(title);
        d.set_meta_a(meta);
    }
    s.front_a = !s.front_a;
    d.set_front_a(s.front_a);
    s.shown = Some(line);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(name: &str) -> String {
        let path = format!("{}/tests/fixtures/rss-feed/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(path).unwrap()
    }

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn sky_news() {
        let a = parse_feed(&fixture("skynews.xml"), "").unwrap();
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].title, "Hundreds more prisoners being released early");
        assert_eq!(a[0].source, "The Latest News from the UK and Around the World | Sky News");
        assert_eq!(a[0].date, utc("2026-11-10T06:37:00Z"));
        assert_eq!(a[2].title, "FBI investigating Nvidia CEO over affair with alleged Chinese spy", "trimmed");
    }

    #[test]
    fn guardian_gmt_dates_and_configured_name() {
        let a = parse_feed(&fixture("guardian.xml"), "The Guardian | World").unwrap();
        assert_eq!(a.len(), 4);
        assert!(a.iter().all(|a| a.source == "The Guardian | World"));
        assert_eq!(a[0].title, "Collision of two trains in Egypt kills at least seven");
        assert_eq!(a[0].date, utc("2026-11-08T17:51:17Z"));
    }

    #[test]
    fn ars_technica() {
        let a = parse_feed(&fixture("arstechnica.xml"), "").unwrap();
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].source, "Ars Technica - All content", "channel title, not <image><title>");
        assert_eq!(a[1].title, "Man who lost $308,000 on sports bets sues Kalshi for offering illegal gambling");
        assert_eq!(a[3].date, utc("2026-11-09T19:41:34Z"));
    }

    #[test]
    fn courier_numeric_entities() {
        let a = parse_feed(&fixture("courier.xml"), "").unwrap();
        assert_eq!(a.len(), 4);
        assert_eq!(a[0].source, "The Courier");
        assert_eq!(
            a[0].title,
            "Dundee Brakes workers could strike over 50p \u{2018}pay disparity\u{2019} with colleagues doing same roles"
        );
        assert_eq!(a[3].title, "LEE WILKIE: Dundee\u{2019}s missed opportunity at St Johnstone raises big questions");
    }

    #[test]
    fn atom() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?>
            <feed xmlns="http://www.w3.org/2005/Atom">
              <title type="text">Example Atom</title>
              <entry><title type="html">&lt;b&gt;Bold&lt;/b&gt; &amp;amp; brave</title>
                <published>2026-11-09T08:00:00+01:00</published><updated>2026-11-10T00:00:00Z</updated></entry>
              <entry><title type="xhtml"><div xmlns="http://www.w3.org/1999/xhtml">X <i>html</i></div></title>
                <updated>2026-11-08T12:00:00Z</updated></entry>
              <entry><title>No date</title></entry>
            </feed>"#;
        let a = parse_feed(xml, "").unwrap();
        assert_eq!(a.len(), 2, "undated entry dropped");
        assert_eq!(a[0].title, "Bold & brave");
        assert_eq!(a[0].source, "Example Atom");
        assert_eq!(a[0].date, utc("2026-11-09T07:00:00Z"), "published wins over updated");
        assert_eq!(a[1].title, "X html");
    }

    #[test]
    fn rss1_rdf_with_dc_date() {
        let xml = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
                xmlns="http://purl.org/rss/1.0/" xmlns:dc="http://purl.org/dc/elements/1.1/">
              <channel rdf:about="x"><title>RDF feed</title></channel>
              <item rdf:about="y"><title>One</title><dc:date>2026-11-10T09:00:00Z</dc:date></item>
            </rdf:RDF>"#;
        let a = parse_feed(xml, "").unwrap();
        assert_eq!(a, vec![Article { title: "One".into(), source: "RDF feed".into(), date: utc("2026-11-10T09:00:00Z") }]);
    }

    #[test]
    fn bad_items_drop_alone() {
        let xml = r#"<rss version="2.0"><channel>
              <item><title>Good</title><pubDate>Tue, 10 Nov 2026 06:37:00 +0000</pubDate></item>
              <item><title>Bad date</title><pubDate>sometime soon</pubDate></item>
              <item><title>No date</title></item>
              <item><pubDate>Tue, 10 Nov 2026 05:00:00 GMT</pubDate></item>
              <item><title><![CDATA[Fish &amp; chips &#8211; &quot;cheap&quot; &bogus; & more]]></title>
                    <pubDate>Mon, 10 Nov 2026 05:00:00 GMT</pubDate></item>
            </channel></rss>"#;
        let a = parse_feed(xml, "").unwrap();
        let titles: Vec<_> = a.iter().map(|a| a.title.as_str()).collect();
        assert_eq!(titles, ["Good", "Untitled", "Fish & chips – \"cheap\" &bogus; & more"]);
        assert_eq!(a[0].source, "RSS Feed", "no channel title and no configured name");
        assert_eq!(a[2].date, utc("2026-11-10T05:00:00Z"), "wrong weekday tolerated");
    }

    #[test]
    fn not_a_feed() {
        assert!(parse_feed("<html><body>403</body></html>", "").is_err());
        assert!(parse_feed("<rss><channel>", "").is_err());
        assert!(parse_feed("", "").is_err());
    }

    #[test]
    fn dates() {
        let p = |s| parse_date(s).map(|d| d.to_rfc3339());
        assert_eq!(p("Tue, 10 Nov 2026 10:36:22 GMT").as_deref(), Some("2026-11-10T10:36:22+00:00"));
        assert_eq!(p("10 Nov 2026 10:36:22 +0100").as_deref(), Some("2026-11-10T09:36:22+00:00"));
        assert_eq!(p("Tue, 3 Nov 2026 09:05:00 EST").as_deref(), Some("2026-11-03T14:05:00+00:00"));
        assert_eq!(p(" 2026-11-10T10:36:22Z ").as_deref(), Some("2026-11-10T10:36:22+00:00"));
        assert_eq!(p("2026-11-10T10:36:22.123+0000").as_deref(), Some("2026-11-10T10:36:22.123+00:00"));
        assert_eq!(p("2026-11-10 10:36:22 +0000").as_deref(), Some("2026-11-10T10:36:22+00:00"));
        assert_eq!(p("2026-11-10").as_deref(), Some("2026-11-10T00:00:00+00:00"));
        assert!(parse_date("2026-11-10 10:36:22").is_some(), "local time");
        assert!(parse_date("2026-11-10T10:36").is_some(), "local time");
        for bad in ["", "soon", "32 Nov 2026 10:00:00 GMT", "2026-13-01"] {
            assert_eq!(parse_date(bad), None, "{bad}");
        }
    }

    #[test]
    fn entities() {
        assert_eq!(decode_html("a &#8217; &#x2019; &amp;amp; &nbsp;&pound;5 &hellip;"), "a \u{2019} \u{2019} &amp;  £5 …");
        assert_eq!(decode_html("AT&T; & &;  &#0; &#xZZ; &verylongentityname;"), "AT&T; & &;  &#0; &#xZZ; &verylongentityname;");
        assert_eq!(decode_html("ends with &"), "ends with &");
        assert_eq!(decode_html("é&eacute;"), "é&eacute;");
    }

    fn art(title: &str, date: &str) -> Article {
        Article { title: title.into(), source: "S".into(), date: utc(date) }
    }

    #[test]
    fn week_filter_and_newest_first() {
        let now = utc("2026-11-10T12:00:00Z");
        let a = Arc::new(vec![art("old", "2026-11-03T11:59:59Z"), art("edge", "2026-11-03T12:00:00Z"), art("a1", "2026-11-10T08:00:00Z")]);
        let b = Arc::new(vec![art("b1", "2026-11-09T08:00:00Z"), art("b2", "2026-11-10T09:00:00Z")]);
        let got: Vec<_> = select(&[Some(a), None, Some(b)], now).into_iter().map(|a| a.title).collect();
        assert_eq!(got, ["b2", "a1", "b1", "edge"]);
    }

    #[test]
    fn one_failing_feed_does_not_hide_the_others() {
        let ok = FeedState { value: Some(Arc::new(vec![])), failed: false };
        let failed = FeedState { value: None, failed: true };
        let pending = FeedState::default();
        let some = [art("x", "2026-11-10T08:00:00Z")];
        assert_eq!(status(&[failed.clone(), ok.clone()], &some), Status::Headline);
        assert_eq!(status(&[failed.clone(), ok.clone()], &[]), Status::Empty);
        assert_eq!(status(&[failed.clone(), pending.clone()], &[]), Status::Loading);
        assert_eq!(status(&[failed.clone(), failed.clone()], &[]), Status::Error);
        assert_eq!(status(&[pending], &[]), Status::Loading);
        assert_eq!(status(&[], &[]), Status::Empty, "no feeds configured: 'No articles available'");
    }

    #[test]
    fn config() {
        let (feeds, rotation) = settings(&json!({
            "rssFeeds": [
                { "url": " https://a.example/rss ", "sourceName": "A" },
                { "url": "" },
                { "sourceName": "no url" },
                { "url": "https://b.example/feed" },
            ],
            "rotationSpeed": 0
        }));
        assert_eq!(feeds, vec![
            FeedConfig { url: "https://a.example/rss".into(), source_name: "A".into() },
            FeedConfig { url: "https://b.example/feed".into(), source_name: "".into() },
        ]);
        assert_eq!(rotation, Duration::from_secs(1), "clamped to the manifest minimum");
        assert_eq!(settings(&json!({})).1, Duration::from_secs(10));
        assert_eq!(settings(&json!({ "rotationSpeed": "15" })).1, Duration::from_secs(15));
        assert!(settings(&Value::Null).0.is_empty());
    }
}
