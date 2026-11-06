//! MDN for Volant: search MDN Web Docs in English through its site search API. Each row shows a
//! page's title and summary, with the part of MDN it belongs to, such as CSS or JavaScript, as a badge.
//! Return opens the page on developer.mozilla.org.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{encode_query, volant_extension_output};

const SEARCH: &str = "https://developer.mozilla.org/api/v1/search?locale=en-US&q=";
const SITE: &str = "https://developer.mozilla.org";
const DOCS: &str = "/docs/";
const WEB_SECTION: &str = "Web";
const PATH_START: char = '/';
const PROTOCOL_RELATIVE: &str = "//";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const QUERY_SLOT: &str = "{query}";
const STATUS_SLOT: &str = "{status}";
const PROMPT: &str = "Type what to look up, such as flexbox or Array.map.";
const NO_MATCH: &str = "MDN has no pages matching {query}.";
const UNREACHABLE: &str = "Volant could not reach MDN. Check your connection and try again.";
const BUSY: &str = "MDN is busy. Try again in a minute.";
const UNAVAILABLE: &str = "MDN is unavailable (HTTP {status}). Try again.";
const UNREADABLE: &str = "MDN sent a reply Volant could not read.";
const NOTHING: &str = "MDN returned nothing.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const MAXIMUM_QUERY: usize = 200;
const MAXIMUM_BADGE: usize = 32;
const OK: i64 = 200;
const UNREACHED: i64 = 0;
const TOO_MANY: i64 = 429;

/// MDN path sections that read better under another name; any other shows with `_` as spaces.
const SECTION_NAMES: [(&str, &str); 2] = [("API", "Web APIs"), ("Learn_web_development", "Learn")];

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

#[derive(Deserialize)]
struct Page { documents: Option<Vec<Document>> }

#[derive(Deserialize)]
struct Document {
    mdn_url: String,
    title: String,
    summary: Option<String>,
}

#[derive(Serialize, Default, Debug, PartialEq)]
struct Row {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

/// A row badge: a symbol Volant draws, if any, and a short text. Volant 0.3.8 and later show them.
#[derive(Serialize, Debug, PartialEq)]
struct Badge {
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<&'static str>,
    text: String,
}

fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The text to search for: trimmed and at most 200 characters. None when nothing is typed.
pub fn search_text(query: &str) -> Option<String> {
    let text: String = query.trim().chars().take(MAXIMUM_QUERY).collect();
    (!text.trim().is_empty()).then(|| text.trim().to_string())
}

/// The MDN search request for a query.
pub fn url_for(text: &str) -> String {
    format!("{SEARCH}{}", encode_query(text))
}

/// The part of MDN a page belongs to, from its path: the area under Web, such as CSS in
/// /en-US/docs/Web/CSS/…, or the top section otherwise, such as Glossary.
fn section(path: &str) -> Option<String> {
    let (_, rest) = path.split_once(DOCS)?;
    let mut parts = rest.split('/').filter(|part| !part.is_empty());
    let first = parts.next()?;
    let name = if first == WEB_SECTION { parts.next().unwrap_or(first) } else { first };
    let text = SECTION_NAMES.iter().find(|(raw, _)| *raw == name).map_or_else(|| name.replace('_', " "), |(_, shown)| shown.to_string());
    (text.chars().count() <= MAXIMUM_BADGE).then_some(text)
}

/// What a failed search says: no connection, a rate limit, or MDN's own status.
fn failure(status: i64) -> String {
    match status {
        UNREACHED => UNREACHABLE.to_string(),
        TOO_MANY => BUSY.to_string(),
        _ => UNAVAILABLE.replace(STATUS_SLOT, &status.to_string()),
    }
}

/// One row for a page, linked on developer.mozilla.org. Pages without a site path or a title are skipped.
fn row(document: Document) -> Option<Row> {
    let path = document.mdn_url;
    let title = document.title.split_whitespace().collect::<Vec<_>>().join(" ");
    if !path.starts_with(PATH_START) || path.starts_with(PROTOCOL_RELATIVE) || path.contains(char::is_whitespace) || title.is_empty() { return None; }
    Some(Row {
        title,
        subtitle: document.summary.map(|text| text.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|text| !text.is_empty()),
        badges: section(&path).map(|text| Badge { symbol: None, text }).into_iter().collect(),
        url: Some(format!("{SITE}{path}")),
    })
}

fn render(request: &Request, text: &str) -> Vec<Row> {
    let Some(response) = request.responses.first() else { return vec![message(NOTHING)] };
    if response.status != OK { return vec![message(&failure(response.status))]; }
    let Ok(page) = serde_json::from_str::<Page>(&response.body) else { return vec![message(UNREADABLE)] };
    let rows: Vec<Row> = page.documents.unwrap_or_default().into_iter().filter_map(row).collect();
    if rows.is_empty() { return vec![message(&NO_MATCH.replace(QUERY_SLOT, text))]; }
    rows
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    let Some(text) = search_text(&request.query) else { return items(vec![message(PROMPT)]) };
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": [url_for(&text)] }).to_string(),
        RENDER => items(render(&request, &text)),
        _ => items(vec![message(ONLY_LISTS)]),
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
    const PAGE_BODY: &str = r#"{"documents":[
        {"mdn_url":"/en-US/docs/Web/CSS/Guides/Flexible_box_layout/Basic_concepts","score":51.2,"title":"Basic concepts of flexbox",
         "locale":"en-us","slug":"web/css/guides/flexible_box_layout/basic_concepts","popularity":0.06,
         "summary":"The flexible box layout module\n lays out items in one dimension.","highlight":{"body":["<mark>flexbox</mark>"],"title":[]}},
        {"mdn_url":"/en-US/docs/Glossary/Flexbox","score":66.2,"title":"Flexbox","summary":"A layout model."},
        {"mdn_url":"/en-US/docs/Web/API/Element/scroll","title":"Element: scroll() method","summary":""},
        {"mdn_url":"/en-US/docs/Learn_web_development/Core/CSS_layout/Flexbox","title":"Flexbox","summary":"Learn flexbox."},
        {"mdn_url":"https://elsewhere.example/x","title":"Off site","summary":"Skipped"}
    ],"metadata":{"took_ms":27,"size":10,"page":1,"total":{"value":5,"relation":"eq"}},"suggestions":[]}"#;

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn rendered(query: &str, status: i64, body: &str) -> Vec<serde_json::Value> {
        let url = url_for(&search_text(query).unwrap());
        let value = reply(serde_json::json!({"phase":"render","query":query,"now":0,"responses":[{"url":url,"status":status,"body":body}]}));
        value["items"].as_array().unwrap().clone()
    }

    #[test]
    fn queries_become_encoded_search_urls_on_the_manifest_host() {
        for query in ["flexbox", "Array.prototype.map", "<div>", "a&b=c", "#id", "100%", "::before", "crème", "🦊", &"x".repeat(500)] {
            let url = url_for(&search_text(query).unwrap());
            assert!(manifest_permits(MANIFEST, &url), "{url}");
            assert!(!url.contains(' ') && !url.contains('#'), "{url}");
        }
        assert_eq!(url_for("<div> element"), "https://developer.mozilla.org/api/v1/search?locale=en-US&q=%3Cdiv%3E%20element");
        let plan = reply(serde_json::json!({"phase":"plan","query":" flexbox ","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://developer.mozilla.org/api/v1/search?locale=en-US&q=flexbox"]));
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"","now":0}))["items"][0]["title"], PROMPT);
    }

    #[test]
    fn pages_show_their_summary_section_and_link() {
        let rows = rendered("flexbox", 200, PAGE_BODY);
        assert_eq!(rows.len(), 4, "The off-site link is skipped");
        assert_eq!(rows[0]["title"], "Basic concepts of flexbox");
        assert_eq!(rows[0]["subtitle"], "The flexible box layout module lays out items in one dimension.");
        assert_eq!(rows[0]["url"], "https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Flexible_box_layout/Basic_concepts");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"CSS"}]));
        assert_eq!(rows[1]["badges"], serde_json::json!([{"text":"Glossary"}]));
        assert_eq!(rows[2]["badges"], serde_json::json!([{"text":"Web APIs"}]));
        assert!(rows[2].get("subtitle").is_none());
        assert_eq!(rows[3]["badges"], serde_json::json!([{"text":"Learn"}]));
    }

    #[test]
    fn sections_come_from_the_path() {
        assert_eq!(section("/en-US/docs/Web/JavaScript/Reference/Global_Objects/Array/map").as_deref(), Some("JavaScript"));
        assert_eq!(section("/en-US/docs/WebAssembly/Reference/Numeric/Division").as_deref(), Some("WebAssembly"));
        assert_eq!(section("/en-US/docs/Web").as_deref(), Some("Web"));
        assert_eq!(section("/en-US/docs/Mozilla/Add-ons").as_deref(), Some("Mozilla"));
        assert_eq!(section("/en-US/blog/post"), None);
    }

    #[test]
    fn failures_and_empty_results_explain_themselves() {
        let empty = r#"{"documents":[],"metadata":{"took_ms":3,"size":10,"page":1,"total":{"value":0,"relation":"eq"}},"suggestions":[]}"#;
        assert_eq!(rendered("zzqx", 200, empty)[0]["title"], "MDN has no pages matching zzqx.");
        assert_eq!(rendered("flexbox", 0, "")[0]["title"], UNREACHABLE);
        assert_eq!(rendered("flexbox", 429, "")[0]["title"], BUSY);
        assert_eq!(rendered("flexbox", 502, "")[0]["title"], "MDN is unavailable (HTTP 502). Try again.");
        assert_eq!(rendered("flexbox", 200, "<html>")[0]["title"], UNREADABLE);
        let none = reply(serde_json::json!({"phase":"render","query":"flexbox","now":0,"responses":[]}));
        assert_eq!(none["items"][0]["title"], NOTHING);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"flexbox","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
