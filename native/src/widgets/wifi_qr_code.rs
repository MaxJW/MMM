//! WiFi join QR code: port of src/components/wifi-qr-code. Static: built once from
//! config at start (a config change re-execs the app). The password is never logged.
//!
//! Payload format and escaping follow the ZXing wiki, "Barcode Contents", section
//! "Wi-Fi Network config (Android, iOS 11+)"
//! (https://github.com/zxing/zxing/wiki/Barcode-Contents#wi-fi-network-config-android-ios-11):
//! `WIFI:T:<WEP|WPA|nopass>;S:<ssid>;P:<password>;;`, P is ignored/omitted for
//! `nopass`, and the special characters `\`, `;`, `,`, `"` and `:` are escaped with a
//! backslash (e.g. SSID `"foo;bar\baz"` -> `S:\"foo\;bar\\baz\";`).
//! The wiki also suggests double-quoting an SSID/password that could be read as hex;
//! like the web app, this does not (scanners differ on stripping the quotes).

use slint::{ComponentHandle, SharedString};

use super::Ctx;
use crate::{qr, WifiQrCodeData};

pub const ID: &str = "wifi-qr-code";
const QR_SIZE: u32 = 160; // size-40

#[derive(serde::Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Settings {
    network_name: String,
    password: String,
    security_type: String,
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | ';' | ',' | '"' | ':') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The `WIFI:` payload. An empty password or security type `nopass` means an open
/// network (`T:nopass`, no `P:`); an empty security type defaults to WPA.
fn payload(ssid: &str, password: &str, security: &str) -> String {
    let security = security.trim();
    let open = password.is_empty() || security.eq_ignore_ascii_case("nopass");
    let kind = match security {
        _ if open => "nopass",
        "" => "WPA",
        other => other,
    };
    let mut s = format!("WIFI:T:{};S:{};", escape(kind), escape(ssid));
    if !open {
        s.push_str(&format!("P:{};", escape(password)));
    }
    s.push(';');
    s
}

pub fn start(ctx: &Ctx) {
    let s: Settings = ctx.config_as();
    let data = ctx.ui.global::<WifiQrCodeData>();
    if s.network_name.is_empty() {
        data.set_error("WiFi network name not configured".into());
        return;
    }
    match qr::image(&payload(&s.network_name, &s.password, &s.security_type), QR_SIZE) {
        Some(image) => {
            data.set_qr(image);
            data.set_network(SharedString::from(s.network_name.as_str()));
        }
        None => {
            eprintln!("{ID}: payload too long for a QR code");
            data.set_error("Failed to generate QR code".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpa_network() {
        assert_eq!(payload("ExampleNet", "secret123", "WPA"), "WIFI:T:WPA;S:ExampleNet;P:secret123;;");
        assert_eq!(payload("Home", "pw", "WEP"), "WIFI:T:WEP;S:Home;P:pw;;");
        assert_eq!(payload("Home", "pw", ""), "WIFI:T:WPA;S:Home;P:pw;;", "web default is WPA");
    }

    #[test]
    fn open_network() {
        assert_eq!(payload("Cafe", "", "WPA"), "WIFI:T:nopass;S:Cafe;;");
        assert_eq!(payload("Cafe", "", "nopass"), "WIFI:T:nopass;S:Cafe;;");
        assert_eq!(payload("Cafe", "left over", "nopass"), "WIFI:T:nopass;S:Cafe;;", "P omitted for nopass");
    }

    #[test]
    fn special_characters_are_escaped() {
        // The ZXing wiki example: SSID "foo;bar\baz" with the quotes part of the name.
        assert_eq!(payload(r#""foo;bar\baz""#, "", "nopass"), r#"WIFI:T:nopass;S:\"foo\;bar\\baz\";;"#);
        assert_eq!(payload("a,b:c", r"p;w\d,:", "WPA"), r"WIFI:T:WPA;S:a\,b\:c;P:p\;w\\d\,\:;;");
        assert_eq!(escape("plain ünïcode 🙂"), "plain ünïcode 🙂");
    }
}
