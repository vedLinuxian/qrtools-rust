//! Structured payload builders for well-known QR code data formats.
//!
//! Each builder returns a plain string that can be passed directly to the QR
//! engine.  All fields are properly escaped where the format requires it.

use serde::{Deserialize, Serialize};
use crate::error::AppError;

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PayloadRequest {
    Wifi(WifiPayload),
    Vcard(VcardPayload),
    Sms(SmsPayload),
    Email(EmailPayload),
    Geo(GeoPayload),
    Phone(PhonePayload),
    Url(UrlPayload),
    Text(TextPayload),
    CalEvent(CalEventPayload),
    Bitcoin(BitcoinPayload),
}

/// Build the canonical QR string for the given payload type.
pub fn build(req: &PayloadRequest) -> Result<String, AppError> {
    let s = match req {
        PayloadRequest::Wifi(p)     => build_wifi(p),
        PayloadRequest::Vcard(p)    => build_vcard(p),
        PayloadRequest::Sms(p)      => build_sms(p),
        PayloadRequest::Email(p)    => build_email(p),
        PayloadRequest::Geo(p)      => build_geo(p),
        PayloadRequest::Phone(p)    => build_phone(p),
        PayloadRequest::Url(p)      => build_url(p),
        PayloadRequest::Text(p)     => build_text(p),
        PayloadRequest::CalEvent(p) => build_cal_event(p),
        PayloadRequest::Bitcoin(p)  => build_bitcoin(p),
    };
    Ok(s)
}

// ── WiFi ─────────────────────────────────────────────────────────────────────

/// WPA/WPA2/WEP/open network credentials.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiPayload {
    pub ssid: String,
    #[serde(default)]
    pub password: String,
    /// `WPA`, `WEP`, or `nopass`.
    #[serde(default = "default_wifi_auth")]
    pub auth: String,
    #[serde(default)]
    pub hidden: bool,
}
fn default_wifi_auth() -> String { "WPA".into() }

fn build_wifi(p: &WifiPayload) -> String {
    let auth = match p.auth.to_uppercase().as_str() {
        "WEP" => "WEP",
        "NOPASS" | "" => "nopass",
        _ => "WPA",
    };
    format!(
        "WIFI:T:{auth};S:{ssid};P:{pw};H:{hidden};;",
        auth = auth,
        ssid = wifi_escape(&p.ssid),
        pw   = wifi_escape(&p.password),
        hidden = if p.hidden { "true" } else { "false" },
    )
}

fn wifi_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('"', "\\\"")
}

// ── vCard 3.0 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcardPayload {
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub org: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub note: String,
}

fn build_vcard(p: &VcardPayload) -> String {
    let mut out = String::from("BEGIN:VCARD\nVERSION:3.0\n");
    out.push_str(&format!("N:{};{};;;\n", vc(&p.last_name), vc(&p.first_name)));
    out.push_str(&format!("FN:{} {}\n", vc(&p.first_name), vc(&p.last_name)));
    if !p.org.is_empty()     { out.push_str(&format!("ORG:{}\n", vc(&p.org))); }
    if !p.title.is_empty()   { out.push_str(&format!("TITLE:{}\n", vc(&p.title))); }
    if !p.phone.is_empty()   { out.push_str(&format!("TEL;TYPE=CELL:{}\n", vc(&p.phone))); }
    if !p.email.is_empty()   { out.push_str(&format!("EMAIL:{}\n", vc(&p.email))); }
    if !p.url.is_empty()     { out.push_str(&format!("URL:{}\n", vc(&p.url))); }
    if !p.address.is_empty() { out.push_str(&format!("ADR:;;{};;;;\n", vc(&p.address))); }
    if !p.note.is_empty()    { out.push_str(&format!("NOTE:{}\n", vc(&p.note))); }
    out.push_str("END:VCARD");
    out
}

/// Escape vCard special characters.
fn vc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

// ── SMS ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsPayload {
    pub phone: String,
    #[serde(default)]
    pub message: String,
}

fn build_sms(p: &SmsPayload) -> String {
    if p.message.is_empty() {
        format!("sms:{}", p.phone)
    } else {
        format!("sms:{}?body={}", p.phone, url_encode(&p.message))
    }
}

// ── Email ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailPayload {
    pub to: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub cc: String,
}

fn build_email(p: &EmailPayload) -> String {
    let mut params: Vec<String> = Vec::new();
    if !p.subject.is_empty() { params.push(format!("subject={}", url_encode(&p.subject))); }
    if !p.body.is_empty()    { params.push(format!("body={}", url_encode(&p.body))); }
    if !p.cc.is_empty()      { params.push(format!("cc={}", url_encode(&p.cc))); }
    if params.is_empty() {
        format!("mailto:{}", p.to)
    } else {
        format!("mailto:{}?{}", p.to, params.join("&"))
    }
}

// ── Geo location ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPayload {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub altitude: Option<f64>,
    #[serde(default)]
    pub query: String,
}

fn build_geo(p: &GeoPayload) -> String {
    if !p.query.is_empty() {
        format!("geo:{},{}", p.latitude, p.longitude)
    } else if let Some(alt) = p.altitude {
        format!("geo:{},{},{}", p.latitude, p.longitude, alt)
    } else {
        format!("geo:{},{}", p.latitude, p.longitude)
    }
}

// ── Phone ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhonePayload {
    pub phone: String,
}

fn build_phone(p: &PhonePayload) -> String {
    format!("tel:{}", p.phone)
}

// ── URL ───────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlPayload {
    pub url: String,
}

fn build_url(p: &UrlPayload) -> String {
    p.url.clone()
}

// ── Plain text ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPayload {
    pub text: String,
}

fn build_text(p: &TextPayload) -> String {
    p.text.clone()
}

// ── iCalendar / calendar event (RFC 5545) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalEventPayload {
    pub summary: String,
    /// ISO 8601 date-time string, e.g. `"2026-06-01T10:00:00"`
    pub dtstart: String,
    pub dtend: String,
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub description: String,
}

fn build_cal_event(p: &CalEventPayload) -> String {
    let start = ical_dt(&p.dtstart);
    let end = ical_dt(&p.dtend);
    let mut out = String::from("BEGIN:VCALENDAR\nVERSION:2.0\nBEGIN:VEVENT\n");
    out.push_str(&format!("DTSTART:{}\n", start));
    out.push_str(&format!("DTEND:{}\n", end));
    out.push_str(&format!("SUMMARY:{}\n", vc(&p.summary)));
    if !p.location.is_empty()    { out.push_str(&format!("LOCATION:{}\n", vc(&p.location))); }
    if !p.description.is_empty() { out.push_str(&format!("DESCRIPTION:{}\n", vc(&p.description))); }
    out.push_str("END:VEVENT\nEND:VCALENDAR");
    out
}

fn ical_dt(s: &str) -> String {
    // Normalise "2026-06-01T10:00:00" → "20260601T100000"
    s.replace('-', "").replace(':', "").replace("T", "T")
}

// ── Bitcoin / crypto payment URI ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitcoinPayload {
    pub address: String,
    pub amount: Option<f64>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub message: String,
}

fn build_bitcoin(p: &BitcoinPayload) -> String {
    let mut params: Vec<String> = Vec::new();
    if let Some(a) = p.amount { params.push(format!("amount={a}")); }
    if !p.label.is_empty()   { params.push(format!("label={}", url_encode(&p.label))); }
    if !p.message.is_empty() { params.push(format!("message={}", url_encode(&p.message))); }
    if params.is_empty() {
        format!("bitcoin:{}", p.address)
    } else {
        format!("bitcoin:{}?{}", p.address, params.join("&"))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn url_encode(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => encoded.push(byte as char),
            b' ' => encoded.push('+'),
            b => encoded.push_str(&format!("%{:02X}", b)),
        }
    }
    encoded
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wifi_basic() {
        let r = build(&PayloadRequest::Wifi(WifiPayload {
            ssid: "MyNet".into(),
            password: "s3cr3t".into(),
            auth: "WPA".into(),
            hidden: false,
        })).unwrap();
        assert_eq!(r, "WIFI:T:WPA;S:MyNet;P:s3cr3t;H:false;;");
    }

    #[test]
    fn vcard_full() {
        let r = build(&PayloadRequest::Vcard(VcardPayload {
            first_name: "John".into(),
            last_name: "Doe".into(),
            org: "ACME".into(),
            title: "Engineer".into(),
            phone: "+1234567890".into(),
            email: "john@example.com".into(),
            url: "https://example.com".into(),
            address: "123 Main St".into(),
            note: "Test note".into(),
        })).unwrap();
        assert!(r.contains("BEGIN:VCARD"));
        assert!(r.contains("N:Doe;John;;;"));
        assert!(r.contains("END:VCARD"));
    }

    #[test]
    fn geo_coords() {
        let r = build(&PayloadRequest::Geo(GeoPayload {
            latitude: 48.8566,
            longitude: 2.3522,
            altitude: None,
            query: String::new(),
        })).unwrap();
        assert_eq!(r, "geo:48.8566,2.3522");
    }

    #[test]
    fn email_with_subject() {
        let r = build(&PayloadRequest::Email(EmailPayload {
            to: "a@b.com".into(),
            subject: "Hello World".into(),
            body: String::new(),
            cc: String::new(),
        })).unwrap();
        assert!(r.starts_with("mailto:a@b.com?subject="));
    }
}
