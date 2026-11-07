//! Get Favicon for Volant: type a domain or a URL and copy its site's icon. The extension fetches
//! nothing itself: the plan answers with the rows straight away, since an icon is binary and Volant
//! hands modules text only.
//!
//! Google's favicon service is the main source. Its rows offer a PNG of at most 32, 64, 128 and 256
//! pixels from `https://t1.gstatic.com/faviconV2?…&url=https://<host>&size=<N>`. Google never enlarges
//! an icon: it sends the largest one it has for the site up to that size, so a site with only a
//! 32-pixel icon gets 32 pixels in every row. For a site it knows nothing about it answers 404 with a
//! placeholder globe, which Volant reports as unavailable instead of copying. The last row copies the
//! ICO from DuckDuckGo's icon service, `https://icons.duckduckgo.com/ip3/<host>.ico`; DuckDuckGo answers
//! a site without an ICO with a PNG or an empty body, which Volant refuses to copy as an ICO.
//!
//! Each row carries `copy`, `{"url","type","title"}`, which a Volant release with row copying (and the
//! manifest's `clipboard-write` permission) fetches only when the owner chooses Copy, putting the image
//! on the clipboard. Its link is the same image URL, so Return opens the icon, as older Volant versions,
//! which ignore `copy`, also do; Command-Return opens the site. A site's own SVG or touch icon is out of
//! reach, since Volant lets an extension read only the fixed hosts its manifest lists.
//!
//! Typed text is reduced to a host: the scheme, credentials, port, path, query and fragment are
//! dropped, then each label must be letters, digits and inner hyphens, with at least two labels. Hosts
//! in other scripts must be typed in their `xn--` form.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const GOOGLE_ICONS: &str = "https://t1.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=";
const GOOGLE_SIZE: &str = "&size=";
const DUCKDUCKGO_ICONS: &str = "https://icons.duckduckgo.com/ip3/";
const ICO_SUFFIX: &str = ".ico";
const SITE_SCHEME: &str = "https://";
const SCHEME_SEPARATOR: &str = "://";
const PNG: &str = "png";
const ANY_IMAGE: &str = "image";
const SIZES: [u32; 4] = [32, 64, 128, 256];
const COPY_PNG: [&str; 4] = ["Copy PNG, up to 32 px", "Copy PNG, up to 64 px", "Copy PNG, up to 128 px", "Copy PNG, up to 256 px"];
const PNG_TITLE_END: &str = " px PNG";
const PNG_HINT_START: &str = "From Google · up to ";
const PNG_HINT_END: &str = " px, smaller when the site has no larger icon";
const ICO_TITLE: &str = "ICO";
const COPY_ICO: &str = "Copy ICO (DuckDuckGo)";
const ICO_HINT: &str = "From DuckDuckGo · copies only when the site has an ICO";
const OPEN_SITE: &str = "Open Site";
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
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<Image>,
}

/// What a Volant release with row copying puts on the clipboard: the image at `url`, fetched when the
/// owner chooses Copy.
#[derive(Serialize, Debug, PartialEq)]
struct Copy { url: String, #[serde(rename = "type")] kind: &'static str, title: &'static str }

/// What Volant 0.3.36 and later show at the row's start, through its sandboxed fetch helper, when the
/// owner allows row images: the same icon the row copies.
#[derive(Serialize, Debug, PartialEq)]
struct Image { url: String }

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

/// Google's PNG of at most `size` pixels for a host.
pub fn png_url(host: &str, size: u32) -> String {
    [GOOGLE_ICONS, SITE_SCHEME, host, GOOGLE_SIZE, &size.to_string()].concat()
}

/// DuckDuckGo's ICO URL for a host.
pub fn ico_url(host: &str) -> String {
    [DUCKDUCKGO_ICONS, host, ICO_SUFFIX].concat()
}

/// A row whose link, copy and preview are the image at `url`, with the site as its second link.
fn image_row(host: &str, title: &str, subtitle: String, url: String, kind: &'static str, copy_title: &'static str) -> Row {
    Row {
        title: [host, SEPARATOR, title].concat(),
        subtitle: Some(subtitle),
        url: Some(url.clone()),
        secondary_url: Some([SITE_SCHEME, host].concat()),
        secondary_title: Some(OPEN_SITE.to_string()),
        image: Some(Image { url: url.clone() }),
        copy: Some(Copy { url, kind, title: copy_title }),
    }
}

/// The host's rows: Google's PNG at each size, then DuckDuckGo's ICO.
fn rows(host: &str) -> Vec<Row> {
    let mut rows: Vec<Row> = SIZES.iter().zip(COPY_PNG).map(|(size, copy_title)| {
        let size_text = size.to_string();
        let title = [size_text.as_str(), PNG_TITLE_END].concat();
        let subtitle = [PNG_HINT_START, &size_text, PNG_HINT_END].concat();
        image_row(host, &title, subtitle, png_url(host, *size), PNG, copy_title)
    }).collect();
    rows.push(image_row(host, ICO_TITLE, ICO_HINT.to_string(), ico_url(host), ANY_IMAGE, COPY_ICO));
    rows
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
        Some(host) => items(rows(&host)),
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
        assert_eq!(host_for("example.com&size=1"), None, "A host can never add parameters to Google's URL");
        assert_eq!(host_for(&format!("{}.com", "a".repeat(64))), None);
    }

    #[test]
    fn icon_urls_are_on_the_manifest_hosts() {
        for text in ["apple.com", "https://docs.example.co.uk/x", "xn--bcher-kva.example"] {
            let host = host_for(text).unwrap();
            for size in SIZES {
                let url = png_url(&host, size);
                assert!(manifest_permits(MANIFEST, &url), "{url}");
            }
            assert!(manifest_permits(MANIFEST, &ico_url(&host)), "{host}");
        }
        assert_eq!(png_url("apple.com", 128), "https://t1.gstatic.com/faviconV2?client=SOCIAL&type=FAVICON&fallback_opts=TYPE,SIZE,URL&url=https://apple.com&size=128");
        assert_eq!(ico_url("apple.com"), "https://icons.duckduckgo.com/ip3/apple.com.ico");
    }

    #[test]
    fn the_plan_answers_with_a_png_row_per_size_and_an_ico_row() {
        let reply = run("plan", "https://www.apple.com/mac/");
        assert!(reply.get("fetch").is_none(), "The plan fetches nothing");
        let rows = reply["items"].as_array().unwrap();
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["www.apple.com · 32 px PNG", "www.apple.com · 64 px PNG", "www.apple.com · 128 px PNG", "www.apple.com · 256 px PNG", "www.apple.com · ICO"]);
        let png = png_url("www.apple.com", 128);
        assert_eq!(rows[2]["subtitle"], "From Google · up to 128 px, smaller when the site has no larger icon");
        assert_eq!(rows[2]["url"], png.as_str(), "Return opens the PNG");
        assert_eq!(rows[2]["copy"], serde_json::json!({"url":png,"type":"png","title":"Copy PNG, up to 128 px"}));
        assert_eq!(rows[2]["image"], serde_json::json!({"url":png}), "Each row previews the icon it copies");
        assert_eq!(rows[4]["subtitle"], ICO_HINT);
        assert_eq!(rows[4]["copy"], serde_json::json!({"url":"https://icons.duckduckgo.com/ip3/www.apple.com.ico","type":"image","title":"Copy ICO (DuckDuckGo)"}),
                   "DuckDuckGo answers with ICO or PNG, so Volant identifies the image itself");
        assert_eq!(rows[4]["image"], serde_json::json!({"url":"https://icons.duckduckgo.com/ip3/www.apple.com.ico"}));
        for row in rows {
            assert_eq!(row["secondaryURL"], "https://www.apple.com");
            assert_eq!(row["secondaryTitle"], OPEN_SITE);
            assert_eq!(row["url"], row["copy"]["url"]);
            assert!(row["copy"]["title"].as_str().unwrap().chars().count() <= 40);
            assert!(manifest_permits(MANIFEST, row["copy"]["url"].as_str().unwrap()));
        }
        assert_eq!(run("render", "apple.com")["items"][0]["title"], "apple.com · 32 px PNG");
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
    fn manifest_asks_to_copy_and_show_images_and_reach_both_icon_hosts() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard-write", "row-images"]));
        assert_eq!(manifest["hosts"], serde_json::json!(["t1.gstatic.com", "icons.duckduckgo.com"]));
    }
}
