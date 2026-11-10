//! MeteoAlarm "legacy atom" feed, port of `fetchMeteoAlerts` in
//! src/components/weather/api.ts, plus the colour and date rules of component.svelte.
//!
//! Feed: https://feeds.meteoalarm.org/feeds/meteoalarm-legacy-atom-<country>, an Atom
//! feed whose entries carry CAP 1.2 fields (`cap:areaDesc`, `cap:event`, `cap:severity`,
//! `cap:onset`, `cap:expires`, ...). Checked against the live UK and German feeds on
//! 2026-11-10: neither carries `cap:description` or `cap:instruction`, but both are read
//! in case a country feed does.

use chrono::{DateTime, Local, Utc};

#[derive(Clone, Debug, PartialEq)]
pub struct Alert {
    pub title: String,
    pub description: String,
    pub instruction: String,
    pub area: String,
    pub event: String,
    pub severity: String,
    pub start: String,
    pub end: DateTime<Utc>,
}

/// Entries with every required field, a parseable end time and (when `province` is not
/// empty) an area containing `province`. Expiry is checked at display time (`active`).
pub fn parse(xml: &str, province: &str) -> Result<Vec<Alert>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| format!("invalid alerts XML: {e}"))?;
    let feed = doc.root_element();
    if feed.tag_name().name() != "feed" {
        return Ok(Vec::new());
    }
    let alerts = feed
        .children()
        .filter(|n| n.has_tag_name("entry"))
        .filter_map(|entry| {
            // fast-xml-parser keys by prefixed name and trims text; match local names.
            let field = |name: &str| {
                entry
                    .children()
                    .find(|n| n.is_element() && n.tag_name().name() == name)
                    .and_then(|n| n.text())
                    .map(str::trim)
                    .unwrap_or_default()
                    .to_string()
            };
            let alert = Alert {
                title: field("title"),
                description: field("description"),
                instruction: field("instruction"),
                area: field("areaDesc"),
                event: field("event"),
                severity: field("severity"),
                start: field("onset"),
                end: DateTime::parse_from_rfc3339(&field("expires"))
                    .ok()?
                    .to_utc(),
            };
            let complete = [
                &alert.title,
                &alert.area,
                &alert.event,
                &alert.severity,
                &alert.start,
            ]
            .iter()
            .all(|s| !s.is_empty());
            (complete && (province.is_empty() || alert.area.contains(province))).then_some(alert)
        })
        .collect();
    Ok(alerts)
}

/// Alerts that have not yet expired.
pub fn active(alerts: &[Alert], now: DateTime<Utc>) -> impl Iterator<Item = &Alert> {
    alerts.iter().filter(move |a| a.end > now)
}

/// `getAlertColor`: 0 red, 1 amber, 2 yellow, 3 green, 4 grey. The colour word in the
/// event or title wins (UK-style feeds), then the CAP severity.
pub fn level(alert: &Alert) -> i32 {
    let text = format!("{} {}", alert.event, alert.title).to_lowercase();
    if has_word(&text, "red") {
        return 0;
    }
    if has_word(&text, "amber") || has_word(&text, "orange") {
        return 1;
    }
    if has_word(&text, "yellow") {
        return 2;
    }
    if has_word(&text, "green") {
        return 3;
    }
    match alert.severity.to_lowercase().as_str() {
        "extreme" => 0,
        "severe" => 1,
        "moderate" => 2,
        "minor" => 3,
        _ => 4,
    }
}

/// JavaScript `/\bword\b/`: word characters are ASCII letters, digits and `_`.
fn has_word(text: &str, word: &str) -> bool {
    let is_word = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
    text.match_indices(word).any(|(i, _)| {
        !is_word(text[..i].chars().next_back()) && !is_word(text[i + word.len()..].chars().next())
    })
}

/// "<start> – <end>" in local time. The web app used `toLocaleString` with the kiosk's
/// locale; this assumes en-GB ("10 Nov, 14:00"). An unparseable start is shown as is.
pub fn dates(alert: &Alert) -> String {
    let start = DateTime::parse_from_rfc3339(&alert.start)
        .map(|d| short_date(d.with_timezone(&Local)))
        .unwrap_or_else(|_| alert.start.clone());
    format!("{start} – {}", short_date(alert.end.with_timezone(&Local)))
}

fn short_date<Tz: chrono::TimeZone>(d: DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    d.format("%-d %b, %H:%M").to_string()
}

/// The text under the heading. The web template used `description ?? instruction`,
/// which never falls back because a missing description is "" rather than null.
pub fn body(alert: &Alert) -> &str {
    if alert.description.is_empty() {
        &alert.instruction
    } else {
        &alert.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UK: &str = include_str!("../../../tests/fixtures/meteoalarm-united-kingdom.xml");

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().to_utc()
    }

    #[test]
    fn parses_real_feed() {
        let all = parse(UK, "").unwrap();
        assert_eq!(all.len(), 3);
        let a = &all[0];
        assert_eq!(a.event, "Yellow fog warning");
        assert_eq!(a.severity, "Moderate");
        assert!(a
            .area
            .starts_with("East Midlands | East of England | London & South East"));
        assert!(a
            .title
            .starts_with("Yellow Fog Warning issued for United Kingdom"));
        assert_eq!(a.start, "2026-11-09T18:00:00+00:00");
        assert_eq!(a.end, at("2026-11-10T10:00:00Z"));
        assert_eq!(body(a), "");
        assert_eq!(level(a), 2);
    }

    #[test]
    fn province_filter_and_expiry() {
        let ni = parse(UK, "Northern Ireland").unwrap();
        assert_eq!(ni.len(), 1);
        assert!(
            parse(UK, "northern ireland").unwrap().is_empty(),
            "case-sensitive like includes()"
        );
        let all = parse(UK, "").unwrap();
        assert_eq!(active(&all, at("2026-11-09T09:00:00Z")).count(), 3);
        assert_eq!(active(&all, at("2026-11-09T10:30:00Z")).count(), 2);
        assert_eq!(
            active(&all, at("2026-11-10T10:00:00Z")).count(),
            0,
            "expiry is exclusive"
        );
    }

    #[test]
    fn incomplete_entries_are_skipped() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom" xmlns:cap="urn:oasis:names:tc:emergency:cap:1.2">
          <entry><title>Red wind warning</title><cap:areaDesc>A</cap:areaDesc><cap:event>Red wind warning</cap:event>
            <cap:severity>Extreme</cap:severity><cap:onset>2026-11-10T06:00:00+00:00</cap:onset>
            <cap:expires>2026-11-11T06:00:00+00:00</cap:expires><cap:instruction>Stay inside</cap:instruction></entry>
          <entry><title>No area</title><cap:event>e</cap:event><cap:severity>Minor</cap:severity>
            <cap:onset>2026-11-10T06:00:00+00:00</cap:onset><cap:expires>2026-11-11T06:00:00+00:00</cap:expires></entry>
          <entry><title>Bad end</title><cap:areaDesc>A</cap:areaDesc><cap:event>e</cap:event><cap:severity>Minor</cap:severity>
            <cap:onset>2026-11-10T06:00:00+00:00</cap:onset><cap:expires>soon</cap:expires></entry>
        </feed>"#;
        let alerts = parse(xml, "").unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(
            body(&alerts[0]),
            "Stay inside",
            "falls back to the instruction"
        );
        assert_eq!(level(&alerts[0]), 0);
        assert!(parse("<feed/>", "").unwrap().is_empty());
        assert!(parse("<rss/>", "").unwrap().is_empty());
        assert!(parse("Not Found", "").is_err());
    }

    #[test]
    fn colour_word_beats_severity() {
        let mut a = parse(UK, "").unwrap().remove(0);
        a.event = "Amber rain warning".into();
        assert_eq!(level(&a), 1);
        a.event = "Orange-level warning".into();
        assert_eq!(level(&a), 1);
        a.event = "Wind".into();
        a.title = "Reddish skies".into(); // not the word "red"
        a.severity = "Severe".into();
        assert_eq!(level(&a), 1);
        a.severity = "Unknown".into();
        assert_eq!(level(&a), 4);
        a.title = "Green".into();
        assert_eq!(level(&a), 3);
    }

    #[test]
    fn word_boundaries() {
        assert!(has_word("red", "red"));
        assert!(has_word("a red-alert", "red"));
        assert!(!has_word("tired", "red"));
        assert!(!has_word("red_x", "red"));
        assert!(has_word("x tired red", "red"));
    }

    #[test]
    fn date_range_text() {
        let a = parse(UK, "").unwrap().remove(1);
        let text = dates(&a);
        // Local time zone dependent; just check the shape: "8 Nov, 22:00 – 9 Nov, 10:00" in UTC.
        assert!(text.contains(" Nov, ") && text.contains(" – "), "{text}");
    }
}
