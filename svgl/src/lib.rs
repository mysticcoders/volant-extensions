//! Svgl for Volant: searches svgl's library of brand logos on api.svgl.app and copies a logo's SVG.
//! Each logo gets a row with its categories; Return opens the brand's website and Command-Return its
//! brand guidelines. A logo with a dark version or a wordmark gets a row for each, whose Return opens
//! that SVG on svgl.app.
//!
//! Every row also carries `copy`, `{"url","type":"svg","title":"Copy SVG"}`, which a Volant release
//! with row copying (and the manifest's `clipboard-write` permission) fetches from svgl.app only when
//! the owner chooses Copy, putting the SVG on the clipboard. Older Volant versions ignore `copy`, so
//! each row's link still leads somewhere useful. A copy URL is offered only when it is https on
//! svgl.app exactly, the host the manifest declares; svgl's list holds a few malformed links.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{encode_query, is_web_link, volant_extension_output};

const API: &str = "https://api.svgl.app?search=";
const LIBRARY: &str = "https://svgl.app/";
const SVG: &str = "svg";
const COPY_SVG: &str = "Copy SVG";
const BRAND_TITLE: &str = "Open Brand Guidelines";
const DARK: &str = "Dark";
const WORDMARK: &str = "Wordmark";
const DARK_WORDMARK: &str = "Dark wordmark";
const DARK_SUFFIX: &str = " (dark)";
const WORDMARK_SUFFIX: &str = " wordmark";
const DARK_WORDMARK_SUFFIX: &str = " wordmark (dark)";
const SVG_HINT: &str = "Return opens the SVG";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const PROMPT: &str = "Type a brand or product, such as Vercel.";
const NOTHING_FOUND_START: &str = "No logo matches “";
const NOTHING_FOUND_END: &str = "”";
const UNREADABLE: &str = "svgl sent a reply Volant could not read.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const SEPARATOR: &str = " · ";
const LIST_SEPARATOR: &str = ", ";
const OK: i64 = 200;
const NOT_FOUND: i64 = 404;
const MAXIMUM_ROWS: usize = 100;
const MAXIMUM_QUERY: usize = 100;

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response { status: i64, body: String }

/// One logo as svgl lists it. `category` and `route` (and `wordmark`) are each either a single value
/// or a list or light and dark pair.
#[derive(Deserialize)]
struct Logo {
    title: String,
    #[serde(default)]
    category: Option<OneOrMany>,
    route: Variants,
    #[serde(default)]
    wordmark: Option<Variants>,
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "brandUrl", default)]
    brand_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany { One(String), Many(Vec<String>) }

#[derive(Deserialize)]
#[serde(untagged)]
enum Variants { One(String), Themed { light: String, dark: String } }

impl Variants {
    fn light(&self) -> &str {
        match self { Variants::One(url) => url, Variants::Themed { light, .. } => light }
    }

    fn dark(&self) -> Option<&str> {
        match self { Variants::One(_) => None, Variants::Themed { dark, .. } => Some(dark) }
    }
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

/// What a Volant release with row copying puts on the clipboard: the file at `url`, fetched when the
/// owner chooses Copy.
#[derive(Serialize, Debug, PartialEq)]
struct Copy { url: String, #[serde(rename = "type")] kind: &'static str, title: &'static str }

/// A row badge with only text. Volant 0.3.8 and later show it in place of the extension's name.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// The search request for a query, or None when it is empty or too long.
pub fn url_for(query: &str) -> Option<String> {
    let query = query.trim();
    (!query.is_empty() && query.chars().count() <= MAXIMUM_QUERY).then(|| [API, &encode_query(query)].concat())
}

/// Whether an SVG link is one Volant may fetch for this manifest: https on svgl.app itself.
pub fn is_library_svg(url: &str) -> bool {
    url.strip_prefix(LIBRARY).is_some_and(|path| !path.is_empty() && !path.contains(['?', '#', '@']))
}

fn copy(url: &str) -> Option<Copy> {
    is_library_svg(url).then(|| Copy { url: url.to_string(), kind: SVG, title: COPY_SVG })
}

/// A row for one of a logo's other SVGs, opened and copied from svgl.app.
fn variant_row(title: &str, suffix: &str, badge: &str, url: &str) -> Option<Row> {
    let copy = copy(url)?;
    Some(Row {
        title: [title, suffix].concat(),
        subtitle: Some(SVG_HINT.to_string()),
        url: Some(url.to_string()),
        copy: Some(copy),
        badges: vec![Badge { text: badge.to_string() }],
        ..Default::default()
    })
}

/// A logo's rows: the logo itself, then its dark version and wordmarks when svgl has them.
fn rows(logo: &Logo) -> Vec<Row> {
    let title = logo.title.trim();
    let categories = match &logo.category {
        Some(OneOrMany::One(category)) => category.clone(),
        Some(OneOrMany::Many(list)) => list.join(LIST_SEPARATOR),
        None => String::new(),
    };
    let website = logo.url.clone().filter(|link| is_web_link(link));
    let brand = logo.brand_url.clone().filter(|link| is_web_link(link));
    let light = logo.route.light();
    let mut details = Vec::new();
    if !categories.is_empty() { details.push(categories); }
    let main = Row {
        title: title.to_string(),
        subtitle: (!details.is_empty()).then(|| details.join(SEPARATOR)),
        url: website.or_else(|| is_library_svg(light).then(|| light.to_string())),
        secondary_title: brand.as_ref().map(|_| BRAND_TITLE.to_string()),
        secondary_url: brand,
        copy: copy(light),
        badges: Vec::new(),
    };
    let mut rows = vec![main];
    rows.extend(logo.route.dark().and_then(|dark| variant_row(title, DARK_SUFFIX, DARK, dark)));
    if let Some(wordmark) = &logo.wordmark {
        rows.extend(variant_row(title, WORDMARK_SUFFIX, WORDMARK, wordmark.light()));
        rows.extend(wordmark.dark().and_then(|dark| variant_row(title, DARK_WORDMARK_SUFFIX, DARK_WORDMARK, dark)));
    }
    rows
}

fn render(request: &Request, query: &str) -> Vec<Row> {
    let Some(response) = request.responses.first() else { return vec![message(unavailable(0), None)] };
    match response.status {
        OK => {}
        NOT_FOUND => return vec![message([NOTHING_FOUND_START, query, NOTHING_FOUND_END].concat(), None)],
        status => return vec![message(unavailable(status), None)],
    }
    let Ok(logos) = serde_json::from_str::<Vec<Logo>>(&response.body) else { return vec![message(UNREADABLE.to_string(), None)] };
    let mut found: Vec<Row> = logos.iter().filter(|logo| !logo.title.trim().is_empty()).flat_map(rows).collect();
    if found.is_empty() { return vec![message([NOTHING_FOUND_START, query, NOTHING_FOUND_END].concat(), None)]; }
    found.truncate(MAXIMUM_ROWS);
    found
}

fn unavailable(status: i64) -> String {
    format!("svgl is unavailable (HTTP {status}). Try again.")
}

fn message(title: String, subtitle: Option<&str>) -> Row {
    Row { title, subtitle: subtitle.map(str::to_string), ..Default::default() }
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD.to_string(), None)]) };
    let Some(url) = url_for(&request.query) else { return items(vec![message(PROMPT.to_string(), None)]) };
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": [url] }).to_string(),
        RENDER => items(render(&request, request.query.trim())),
        _ => items(vec![message(ONLY_LISTS.to_string(), None)]),
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

    /// Fictional logos in the shapes svgl's API returns: a plain route, light and dark routes with
    /// themed wordmarks, a single wordmark, and a malformed link.
    const BODY: &str = r#"[
        {"id":1,"title":"Acme","category":["Hosting","Acme"],"route":{"light":"https://svgl.app/library/acme.svg","dark":"https://svgl.app/library/acme_dark.svg"},
         "wordmark":{"light":"https://svgl.app/library/acme_wordmark.svg","dark":"https://svgl.app/library/acme_wordmark_dark.svg"},
         "brandUrl":"https://acme.example/brand","url":"https://acme.example/"},
        {"id":2,"title":"Plainco","category":"Software","route":"https://svgl.app/library/plainco.svg","wordmark":"https://svgl.app/library/plainco-wordmark.svg","url":"https://plainco.example"},
        {"id":3,"title":"Broken","category":"Software","route":"https://svgl.applibrary/broken.svg","url":"https://broken.example"}]"#;

    fn run(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render(query: &str, status: i64, body: &str) -> Vec<serde_json::Value> {
        let url = url_for(query).unwrap();
        run(serde_json::json!({"phase":"render","query":query,"now":0,"responses":[{"url":url,"status":status,"body":body}]}))["items"].as_array().unwrap().clone()
    }

    #[test]
    fn every_planned_url_is_on_the_manifest_host() {
        for query in ["vercel", "visual studio", "C++ & co", "ünïcode"] {
            let url = url_for(query).unwrap();
            assert!(manifest_permits(MANIFEST, &url), "{url}");
        }
        assert_eq!(url_for(" Visual Studio "), Some("https://api.svgl.app?search=Visual%20Studio".into()));
        assert_eq!(url_for("  "), None);
        assert_eq!(url_for(&"a".repeat(101)), None);
        let plan = run(serde_json::json!({"phase":"plan","query":"vercel","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://api.svgl.app?search=vercel"]));
        assert_eq!(run(serde_json::json!({"phase":"plan","query":"","now":0}))["items"][0]["title"], PROMPT);
    }

    #[test]
    fn copy_urls_are_on_the_declared_svg_host() {
        assert!(is_library_svg("https://svgl.app/library/vercel.svg"));
        assert!(manifest_permits(MANIFEST, "https://svgl.app/library/vercel.svg"));
        assert!(!is_library_svg("https://svgl.applibrary/vercel.svg"));
        assert!(!is_library_svg("http://svgl.app/library/vercel.svg"));
        assert!(!is_library_svg("https://evil.example/svgl.app/x.svg"));
        assert!(!is_library_svg("https://svgl.app/"));
    }

    #[test]
    fn logos_and_their_variants_carry_copy() {
        let rows = render("acme", 200, BODY);
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Acme", "Acme (dark)", "Acme wordmark", "Acme wordmark (dark)", "Plainco", "Plainco wordmark", "Broken"]);
        assert_eq!(rows[0]["subtitle"], "Hosting, Acme");
        assert_eq!(rows[0]["url"], "https://acme.example/", "Older Volant opens the brand's website");
        assert_eq!(rows[0]["secondaryURL"], "https://acme.example/brand");
        assert_eq!(rows[0]["secondaryTitle"], BRAND_TITLE);
        assert_eq!(rows[0]["copy"], serde_json::json!({"url":"https://svgl.app/library/acme.svg","type":"svg","title":"Copy SVG"}));
        assert_eq!(rows[1]["url"], "https://svgl.app/library/acme_dark.svg");
        assert_eq!(rows[1]["copy"]["url"], "https://svgl.app/library/acme_dark.svg");
        assert_eq!(rows[1]["badges"], serde_json::json!([{"text":"Dark"}]));
        assert_eq!(rows[3]["badges"][0]["text"], "Dark wordmark");
        assert_eq!(rows[4]["subtitle"], "Software");
        assert!(rows[4].get("secondaryURL").is_none());
        assert!(rows[6].get("copy").is_none(), "A malformed SVG link is never offered for copying");
        for row in &rows {
            if let Some(url) = row["copy"]["url"].as_str() { assert!(manifest_permits(MANIFEST, url), "{url}"); }
        }
    }

    #[test]
    fn nothing_found_and_failures() {
        let missing = render("zzz", 404, r#"{"error":"SVG not found"}"#);
        assert_eq!(missing[0]["title"], "No logo matches “zzz”");
        assert_eq!(render("zzz", 200, "[]")[0]["title"], "No logo matches “zzz”");
        assert_eq!(render("acme", 0, "")[0]["title"], "svgl is unavailable (HTTP 0). Try again.");
        assert_eq!(render("acme", 200, "<html>")[0]["title"], UNREADABLE);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_asks_to_write_the_clipboard_and_reach_both_hosts() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard-write"]));
        assert_eq!(manifest["hosts"], serde_json::json!(["api.svgl.app", "svgl.app"]));
    }
}
