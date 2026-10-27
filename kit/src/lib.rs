//! Helpers for Volant list extensions built for ABI 1 (`wasm32-unknown-unknown`).
//!
//! A list extension receives a JSON request and returns JSON. With network hosts in its manifest it
//! runs twice: `plan` returns `{"fetch": [url, …]}`, then `render` receives the fetched responses and
//! returns `{"items": […]}`. The module never touches the network; Volant fetches the URLs, and only
//! from hosts the owner approved. Declare the entry point with [`volant_extension!`].
//!
//! Extensions whose manifest sets `"storage": true` also receive the JSON object they last saved, and
//! rows can offer actions; see [`reader`] for the pins, hidden stories and read state the readers keep.
//!
//! Extensions that declare `preferences` in their manifest receive the owner's values as `"preferences"`
//! (Volant 0.3.22 and later), one entry per declared id. Read them with [`Preferences::from_input`] and
//! its `bool`, `number` and `string` readers, whose defaults also apply on older Volant versions, which
//! send none.
use serde::{Deserialize, Serialize};

pub mod reader;

/// The label Volant shows for a row's secondary link when it opens a story's discussion.
pub const DISCUSSION_TITLE: &str = "Open Comments";
const HTTPS: &str = "https://";
const HTTP: &str = "http://";
const PLAN: &str = "plan";
const ACTION: &str = "action";
const US_UNITS: &str = "us";
const JUST_NOW: &str = "just now";
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const YEAR: i64 = 365 * DAY;
const AGE_UNITS: [(i64, &str); 4] = [(YEAR, "y"), (DAY, "d"), (HOUR, "h"), (MINUTE, "m")];

#[derive(Deserialize)]
pub struct Request {
    pub phase: String,
    pub query: String,
    /// Seconds since 1970, from the host; modules have no clock of their own.
    pub now: i64,
    #[serde(default)]
    pub responses: Vec<Response>,
    /// The object this extension last saved, when its manifest declares storage.
    #[serde(default)]
    pub storage: Option<serde_json::Value>,
    /// The action chosen in a row's Actions menu, in the `action` phase.
    #[serde(default)]
    pub action: Option<String>,
    /// The row an action or an `opened` notice is about.
    #[serde(default)]
    pub item: Option<RowRef>,
    /// Coarse coordinates, sent only when the manifest declares the `location` permission and the
    /// owner allowed it.
    #[serde(default)]
    pub location: Option<Location>,
    /// The owner's measurement system: "us" or "metric".
    #[serde(default)]
    pub units: Option<String>,
    /// The state a previous round returned with [`Reply::Continue`], in a later `render`.
    #[serde(default)]
    pub state: Option<String>,
}

/// The owner's values for the manifest's declared preferences, every declared id included with
/// Volant's defaults filled in, read from the raw request with [`Preferences::from_input`]. Volant
/// versions before 0.3.22 send none, so every reader falls back to the default the caller passes. Kept
/// apart from [`Request`] so extensions that declare no preferences compile to the same module.
#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
pub struct Preferences {
    #[serde(default)]
    preferences: serde_json::Map<String, serde_json::Value>,
}

impl Preferences {
    /// The preferences in a request's JSON, or none when it has no `preferences` object.
    pub fn from_input(input: &[u8]) -> Preferences {
        serde_json::from_slice(input).unwrap_or_default()
    }

    /// A toggle preference, or `default` when it is missing or not a boolean.
    pub fn bool(&self, id: &str, default: bool) -> bool {
        self.preferences.get(id).and_then(serde_json::Value::as_bool).unwrap_or(default)
    }

    /// A number preference, or `default` when it is missing or not a number. Volant has already checked
    /// it against the manifest's `min` and `max`.
    pub fn number(&self, id: &str, default: f64) -> f64 {
        self.preferences.get(id).and_then(serde_json::Value::as_f64).unwrap_or(default)
    }

    /// A text or dropdown preference, or `default` when it is missing or not a string.
    pub fn string<'a>(&'a self, id: &str, default: &'a str) -> &'a str {
        self.preferences.get(id).and_then(serde_json::Value::as_str).unwrap_or(default)
    }
}

/// Where the owner is, rounded by Volant to about ten kilometers.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
}

/// The links of the row an action is about, as Volant sends it back.
#[derive(Deserialize, Default)]
pub struct RowRef {
    #[serde(default)]
    pub title: String,
    pub url: Option<String>,
    #[serde(rename = "secondaryURL")]
    pub secondary_url: Option<String>,
}

impl Request {
    /// Whether this is the plan phase, which returns URLs to fetch rather than rows.
    pub fn is_plan(&self) -> bool {
        self.phase == PLAN
    }

    /// Whether this is the action phase, after the owner chose a row's action.
    pub fn is_action(&self) -> bool {
        self.phase == ACTION
    }

    /// Whether to show US customary units (Fahrenheit, miles per hour) rather than metric.
    pub fn uses_us_units(&self) -> bool {
        self.units.as_deref() == Some(US_UNITS)
    }
}

#[derive(Deserialize)]
pub struct Response {
    pub url: String,
    pub status: i64,
    pub body: String,
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "secondaryURL")]
    pub secondary_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary_title: Option<String>,
    /// Entries for the row's Actions menu.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Action>,
    /// Asks Volant to send an `opened` notice after Return opens the row's link.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub on_open: bool,
    /// An action `id` that Return runs instead of opening the row's link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_action: Option<String>,
    /// One of Volant's row symbols, such as "sun" or "rain", drawn beside the row. Volant versions
    /// without row symbols, and names it does not know, ignore it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

/// A request that changes something, such as adding or completing a task. Volant sends it only from
/// the `action` phase, which the owner starts by choosing a row or an action, and only to a host the
/// manifest declares. A secret the manifest declares for that host is added by Volant as a header;
/// the module never sees it.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Submission {
    pub url: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<serde_json::Value>,
}

/// One entry in a row's Actions menu: `id` comes back in the `action` phase.
#[derive(Serialize, Clone, PartialEq, Debug)]
pub struct Action {
    pub id: String,
    pub title: String,
}

/// What a handler returns for one phase.
pub enum Reply {
    Fetch(Vec<String>),
    Items(Vec<Item>),
    /// Saves a new storage object; Volant then runs the list again to show the change.
    Save(serde_json::Value),
    /// Fetches URLs and renders again with their responses and this state. From `render`, it is
    /// another round, for a lookup that needs one answer before it can ask the next question.
    Continue(Vec<String>, String),
    /// From `action`: sends these requests, then renders with their responses and this state.
    Submit(Vec<Submission>, String),
    /// Shows a short confirmation, then Volant runs the list again.
    Notice(String),
}

/// Serializes a reply as the JSON Volant expects.
pub fn encode(reply: &Reply) -> String {
    match reply {
        Reply::Fetch(urls) => serde_json::json!({ "fetch": urls }).to_string(),
        Reply::Items(items) => serde_json::json!({ "items": items }).to_string(),
        Reply::Save(storage) => serde_json::json!({ "storage": storage }).to_string(),
        Reply::Continue(urls, state) => serde_json::json!({ "fetch": urls, "state": state }).to_string(),
        Reply::Submit(requests, state) => serde_json::json!({ "fetch": requests, "state": state }).to_string(),
        Reply::Notice(text) => serde_json::json!({ "notice": text }).to_string(),
    }
}

/// Decodes the request, calls the handler, and encodes its reply. A malformed request or a handler
/// error becomes a single row explaining it, so the launcher always has something to show.
pub fn handle(input: &[u8], handler: fn(&Request) -> Result<Reply, String>) -> String {
    let reply = match serde_json::from_slice::<Request>(input) {
        Ok(request) => handler(&request).unwrap_or_else(|message| Reply::Items(vec![Item { title: message, ..Default::default() }])),
        Err(error) => Reply::Items(vec![Item { title: format!("Volant sent a request this extension does not understand: {error}"), ..Default::default() }]),
    };
    encode(&reply)
}

/// Exports `alloc` and `run` for ABI 1 around a handler.
#[macro_export]
macro_rules! volant_extension {
    ($handler:path) => {
        #[no_mangle]
        pub extern "C" fn alloc(len: usize) -> *mut u8 {
            let mut buffer = vec![0u8; len.max(1)].into_boxed_slice();
            let ptr = buffer.as_mut_ptr();
            std::mem::forget(buffer);
            ptr
        }

        #[no_mangle]
        pub extern "C" fn run(ptr: *const u8, len: usize) -> *const u8 {
            let input = unsafe { std::slice::from_raw_parts(ptr, len) };
            let output = $crate::handle(input, $handler);
            let mut record = Vec::with_capacity(4 + output.len());
            record.extend_from_slice(&(output.len() as u32).to_le_bytes());
            record.extend_from_slice(output.as_bytes());
            Box::leak(record.into_boxed_slice()).as_ptr()
        }
    };
}

/// "5m ago", "3h ago", "2d ago", "4y ago" between two Unix times.
pub fn age(now: i64, then: i64) -> String {
    let seconds = (now - then).max(0);
    AGE_UNITS.iter().find(|(size, _)| seconds >= *size)
        .map_or_else(|| JUST_NOW.to_string(), |(size, unit)| format!("{}{unit} ago", seconds / size))
}

/// Whether a link is http or https, the only kinds Volant lets a row open.
pub fn is_web_link(url: &str) -> bool {
    url.starts_with(HTTPS) || url.starts_with(HTTP)
}

/// Whether Volant would fetch a planned URL for this manifest: https on the default port, no
/// credentials or fragment, and a host the manifest's `hosts` declares. Extensions call it from tests
/// so a URL in the source cannot drift from the hosts the owner approves.
pub fn manifest_permits(manifest: &str, url: &str) -> bool {
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(manifest) else { return false };
    let Some(rest) = url.strip_prefix(HTTPS) else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if url.contains('#') || authority.contains(['@', ':']) || authority.is_empty() { return false; }
    manifest["hosts"].as_array().is_some_and(|hosts| hosts.iter().any(|host| host.as_str() == Some(authority)))
}

/// The host of an http or https URL without a leading "www.", for showing where a link goes.
pub fn domain(url: &str) -> Option<String> {
    let rest = url.strip_prefix(HTTPS).or_else(|| url.strip_prefix(HTTP))?;
    let host = rest.split(['/', '?', '#']).next()?.split('@').last()?.split(':').next()?;
    if host.is_empty() { return None; }
    Some(host.trim_start_matches("www.").to_lowercase())
}

/// Percent-encodes a query string value (RFC 3986 unreserved characters pass through).
pub fn encode_query(value: &str) -> String {
    value.bytes().map(|byte| match byte {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (byte as char).to_string(),
        _ => format!("%{byte:02X}"),
    }).collect()
}

/// Parses an RFC 3339 timestamp such as "2026-10-11T20:18:09.541-05:00" to Unix seconds.
pub fn parse_rfc3339(text: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let zone = text.get(19..)?.trim_start_matches(|c: char| c == '.' || c.is_ascii_digit());
    let offset = if zone == "Z" || zone.is_empty() { 0 } else {
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        let hours = zone.get(1..3)?.parse::<i64>().ok()?;
        let minutes = zone.get(4..6)?.parse::<i64>().ok()?;
        sign * (hours * 3_600 + minutes * 60)
    };
    let (y, m) = if month <= 2 { (year - 1, month + 9) } else { (year, month - 3) };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * m + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second - offset)
}

#[cfg(test)]
mod tests {
    #[test]
    fn preferences_read_with_defaults_for_older_volant() {
        let preferences = Preferences::from_input(br#"{"phase":"plan","query":"","now":0,"preferences":{"limit":12,"showScores":false,"sort":"new"}}"#);
        assert_eq!(preferences.number("limit", 30.0), 12.0);
        assert!(!preferences.bool("showScores", true));
        assert_eq!(preferences.string("sort", "top"), "new");
        assert_eq!(preferences.string("limit", "fallback"), "fallback", "A different type falls back");
        let older = Preferences::from_input(br#"{"phase":"plan","query":"","now":0}"#);
        assert_eq!(older.number("limit", 30.0), 30.0);
        assert!(older.bool("showScores", true));
    }

    use super::*;

    #[test]
    fn ages_round_down() {
        assert_eq!(age(1_000, 990), "just now");
        assert_eq!(age(10_000, 10_000 - 300), "5m ago");
        assert_eq!(age(100_000, 100_000 - 7_200), "2h ago");
        assert_eq!(age(1_000_000, 1_000_000 - 3 * 86_400), "3d ago");
        assert_eq!(age(1_000_000_000, 1_000_000_000 - 1_942 * 86_400), "5y ago");
    }

    #[test]
    fn domains_drop_www_ports_and_paths() {
        assert_eq!(domain("https://www.Example.com:8080/a?b").as_deref(), Some("example.com"));
        assert_eq!(domain("http://lobste.rs").as_deref(), Some("lobste.rs"));
        assert_eq!(domain("ftp://example.com"), None);
    }

    #[test]
    fn timestamps_honor_offsets() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("2026-10-11T20:18:09.541-05:00"), Some(1_791_767_889));
        assert_eq!(parse_rfc3339("2000-03-01T00:00:00+01:00"), Some(951_865_200));
        assert_eq!(parse_rfc3339("garbage"), None);
    }

    #[test]
    fn manifests_permit_only_their_https_hosts() {
        let manifest = r#"{"hosts":["example.com"]}"#;
        assert!(manifest_permits(manifest, "https://example.com/a?b=c"));
        assert!(!manifest_permits(manifest, "http://example.com/a"));
        assert!(!manifest_permits(manifest, "https://example.com:8443/a"));
        assert!(!manifest_permits(manifest, "https://user@example.com/a"));
        assert!(!manifest_permits(manifest, "https://example.com/a#b"));
        assert!(!manifest_permits(manifest, "https://other.example.com/a"));
    }

    #[test]
    fn queries_are_percent_encoded() {
        assert_eq!(encode_query("rust & wasm"), "rust%20%26%20wasm");
    }
}

/// Exports `alloc` and `run` for ABI 1 around a function that turns the raw request into the reply
/// text, for an extension that adds to the JSON [`handle`] produces.
#[macro_export]
macro_rules! volant_extension_output {
    ($respond:path) => {
        #[no_mangle]
        pub extern "C" fn alloc(len: usize) -> *mut u8 {
            let mut buffer = vec![0u8; len.max(1)].into_boxed_slice();
            let ptr = buffer.as_mut_ptr();
            std::mem::forget(buffer);
            ptr
        }

        #[no_mangle]
        pub extern "C" fn run(ptr: *const u8, len: usize) -> *const u8 {
            let input = unsafe { std::slice::from_raw_parts(ptr, len) };
            let output: String = $respond(input);
            let mut record = Vec::with_capacity(4 + output.len());
            record.extend_from_slice(&(output.len() as u32).to_le_bytes());
            record.extend_from_slice(output.as_bytes());
            Box::leak(record.into_boxed_slice()).as_ptr()
        }
    };
}
