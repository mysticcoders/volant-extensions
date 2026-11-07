//! Get Favicon for Volant: type a domain or a URL and get its site's icon from DuckDuckGo's icon
//! service, `https://icons.duckduckgo.com/ip3/<host>.ico`. The extension fetches nothing itself: the
//! plan answers with the row straight away, since an icon is binary and Volant hands modules text only.
//!
//! The row carries `copy`, `{"url","type":"ico","title":"Copy Image"}`, which a Volant release with
//! row copying (and the manifest's `clipboard-write` permission) fetches from icons.duckduckgo.com only
//! when the owner chooses Copy, putting the image on the clipboard. Its link is the same icon URL, so
//! older Volant versions, which ignore `copy`, open the icon in the browser instead; Command-Return
//! opens the site. DuckDuckGo answers a site without an icon with 404 and a placeholder image.
//!
//! Typed text is reduced to a host: the scheme, credentials, port, path, query and fragment are
//! dropped, then each label must be letters, digits and inner hyphens, with at least two labels. Hosts
//! in other scripts must be typed in their `xn--` form.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const ICONS: &str = "https://icons.duckduckgo.com/ip3/";
const ICON_SUFFIX: &str = ".ico";
const SITE_SCHEME: &str = "https://";
const SCHEME_SEPARATOR: &str = "://";
const ICO: &str = "ico";
const COPY_IMAGE: &str = "Copy Image";
const OPEN_SITE: &str = "Open Site";
const ICON_HINT: &str = "Favicon from DuckDuckGo";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const PROMPT: &str = "Type a domain or URL, such as apple.com.";
const NOT_A_HOST: &str = "That is not a domain name.";
const NOT_A_HOST_HINT: &str = "Type a domain such as example.com, or a link to a page on it";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const SEPARATOR: &str = " · ";
const MAXIMUM_HOST: usize = 253;
const MAXIMUM_LABEL: usize = 63;

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
}

#[derive(Serialize, Default, Debug, PartialEq)]
struct Row {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "secondaryURL")]
    secondary_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "secondaryTitle")]
    secondary_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    copy: Option<Copy>,
}

/// What a Volant release with row copying puts on the clipboard: the image at `url`, fetched when the
/// owner chooses Copy.
#[derive(Serialize, Debug, PartialEq)]
struct Copy { url: String, #[serde(rename = "type")] kind: &'static str, title: &'static str }

/// Whether one DNS label is letters, digits and hyphens, not starting or ending with a hyphen.
fn is_label(label: &str) -> bool {
    !label.is_empty() && label.len() <= MAXIMUM_LABEL && !label.starts_with('-') && !label.ends_with('-')
        && label.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The host in typed text, lowercased, or None when it is not a domain name.
pub fn host_for(text: &str) -> Option<String> {
    let text = text.trim().to_lowercase();
    let rest = text.split_once(SCHEME_SEPARATOR).map_or(text.as_str(), |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_and_port = authority.rsplit('@').next()?;
    let host = host_and_port.split(':').next()?.trim_end_matches('.');
    let labels: Vec<&str> = host.split('.').collect();
    let valid = host.len() <= MAXIMUM_HOST && labels.len() >= 2 && labels.iter().all(|label| is_label(label))
        && !labels.last().is_some_and(|tld| tld.bytes().all(|b| b.is_ascii_digit()));
    valid.then(|| host.to_string())
}

/// DuckDuckGo's icon URL for a host.
pub fn icon_url(host: &str) -> String {
    [ICONS, host, ICON_SUFFIX].concat()
}

/// The host's row: its icon as the link and the copy, and its site as the second link.
fn row(host: &str) -> Row {
    let icon = icon_url(host);
    Row {
        title: host.to_string(),
        subtitle: Some([ICON_HINT, &icon].join(SEPARATOR)),
        url: Some(icon.clone()),
        secondary_url: Some([SITE_SCHEME, host].concat()),
        secondary_title: Some(OPEN_SITE.to_string()),
        copy: Some(Copy { url: icon, kind: ICO, title: COPY_IMAGE }),
    }
}

fn message(title: &str, subtitle: Option<&str>) -> Row {
    Row { title: title.to_string(), subtitle: subtitle.map(str::to_string), ..Default::default() }
}

/// Handles one request and returns the reply Volant reads. Both phases answer with rows, so Volant
/// never fetches anything for a run.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD, None)]) };
    if request.phase != PLAN && request.phase != RENDER { return items(vec![message(ONLY_LISTS, None)]); }
    if request.query.trim().is_empty() { return items(vec![message(PROMPT, None)]); }
    match host_for(&request.query) {
        Some(host) => items(vec![row(&host)]),
        None => items(vec![message(NOT_A_HOST, Some(NOT_A_HOST_HINT))]),
    }
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::manifest_permits;

    const MANIFEST: &str = include_str!("../manifest.json");

    fn run(phase: &str, query: &str) -> serde_json::Value {
        serde_json::from_str(&respond(serde_json::json!({"phase":phase,"query":query,"now":0}).to_string().as_bytes())).unwrap()
    }

    #[test]
    fn typed_text_becomes_a_host() {
        assert_eq!(host_for("apple.com"), Some("apple.com".into()));
        assert_eq!(host_for("  WWW.Apple.com  "), Some("www.apple.com".into()));
        assert_eq!(host_for("https://user:pass@docs.example.co.uk:8443/path?q=1#top"), Some("docs.example.co.uk".into()));
        assert_eq!(host_for("example.com/some/page"), Some("example.com".into()));
        assert_eq!(host_for("example.com."), Some("example.com".into()));
        assert_eq!(host_for("xn--bcher-kva.example"), Some("xn--bcher-kva.example".into()));
        assert_eq!(host_for("localhost"), None);
        assert_eq!(host_for("192.168.0.1"), None);
        assert_eq!(host_for("-bad.example"), None);
        assert_eq!(host_for("bad_name.example"), None);
        assert_eq!(host_for("bücher.example"), None);
        assert_eq!(host_for("a..b"), None);
        assert_eq!(host_for(&format!("{}.com", "a".repeat(64))), None);
    }

    #[test]
    fn icon_urls_are_on_the_manifest_host() {
        for text in ["apple.com", "https://docs.example.co.uk/x", "xn--bcher-kva.example"] {
            let url = icon_url(&host_for(text).unwrap());
            assert!(manifest_permits(MANIFEST, &url), "{url}");
        }
        assert_eq!(icon_url("apple.com"), "https://icons.duckduckgo.com/ip3/apple.com.ico");
    }

    #[test]
    fn the_plan_answers_with_the_row_and_fetches_nothing() {
        let reply = run("plan", "https://www.apple.com/mac/");
        assert!(reply.get("fetch").is_none());
        let row = &reply["items"][0];
        assert_eq!(row["title"], "www.apple.com");
        assert_eq!(row["subtitle"], "Favicon from DuckDuckGo · https://icons.duckduckgo.com/ip3/www.apple.com.ico");
        assert_eq!(row["url"], "https://icons.duckduckgo.com/ip3/www.apple.com.ico", "Older Volant opens the icon");
        assert_eq!(row["secondaryURL"], "https://www.apple.com");
        assert_eq!(row["secondaryTitle"], OPEN_SITE);
        assert_eq!(row["copy"], serde_json::json!({"url":"https://icons.duckduckgo.com/ip3/www.apple.com.ico","type":"ico","title":"Copy Image"}));
        assert_eq!(run("render", "apple.com")["items"][0]["title"], "apple.com");
    }

    #[test]
    fn prompts_and_bad_input() {
        assert_eq!(run("plan", "")["items"][0]["title"], PROMPT);
        let bad = run("plan", "not a domain");
        assert_eq!(bad["items"][0]["title"], NOT_A_HOST);
        assert_eq!(bad["items"][0]["subtitle"], NOT_A_HOST_HINT);
        assert_eq!(run("action", "apple.com")["items"][0]["title"], ONLY_LISTS);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_asks_to_write_the_clipboard_and_reach_one_host() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard-write"]));
        assert_eq!(manifest["hosts"], serde_json::json!(["icons.duckduckgo.com"]));
    }
}
