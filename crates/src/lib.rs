//! crates.io for Volant: search Rust crates through the crates.io API. Each row shows a crate's name,
//! its description and latest version, and its all-time and recent downloads as badges, with the crate
//! whose name matches the search first. Return opens the crate on crates.io; Command-Return opens its
//! repository, or its documentation on docs.rs when it names no repository.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{encode_query, is_web_link, volant_extension_output};

const SEARCH: &str = "https://crates.io/api/v1/crates?per_page=20&q=";
const PAGE: &str = "https://crates.io/crates/";
const DOCS: &str = "https://docs.rs/";
const REPOSITORY_TITLE: &str = "Open Repository";
const DOCS_TITLE: &str = "Open Documentation";
const DOWNLOADS: &str = " downloads";
const RECENT: &str = " recent";
const VERSION_PREFIX: &str = "v";
const SEPARATOR: &str = " · ";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const QUERY_SLOT: &str = "{query}";
const STATUS_SLOT: &str = "{status}";
const PROMPT: &str = "Type a crate name or keyword, such as serde.";
const NO_MATCH: &str = "No crates match {query}.";
const UNREACHABLE: &str = "Volant could not reach crates.io. Check your connection and try again.";
const BUSY: &str = "crates.io is busy. Try again in a minute.";
const UNAVAILABLE: &str = "crates.io is unavailable (HTTP {status}). Try again.";
const UNREADABLE: &str = "crates.io sent a reply Volant could not read.";
const NOTHING: &str = "crates.io returned nothing.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const MAXIMUM_QUERY: usize = 200;
const OK: i64 = 200;
const UNREACHED: i64 = 0;
const TOO_MANY: i64 = 429;
const COMPACT_UNITS: [(u64, &str); 3] = [(1_000_000_000, "B"), (1_000_000, "M"), (1_000, "K")];

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
struct Page { crates: Vec<Crate> }

#[derive(Deserialize)]
struct Crate {
    name: String,
    description: Option<String>,
    max_stable_version: Option<String>,
    default_version: Option<String>,
    max_version: Option<String>,
    downloads: Option<u64>,
    recent_downloads: Option<u64>,
    repository: Option<String>,
    #[serde(default)]
    exact_match: bool,
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

/// The crates.io search request for a query.
pub fn url_for(text: &str) -> String {
    format!("{SEARCH}{}", encode_query(text))
}

/// 412,345,678 as "412M", 1,658,410,158 as "1.6B": at most one decimal, below ten of a unit.
fn compact(value: u64) -> String {
    for (size, unit) in COMPACT_UNITS {
        if value >= size {
            let whole = value / size;
            let tenth = value % size * 10 / size;
            return if whole >= 10 || tenth == 0 { format!("{whole}{unit}") } else { format!("{whole}.{tenth}{unit}") };
        }
    }
    value.to_string()
}

/// A crate name as crates.io compares them: case and `-` against `_` do not matter.
fn same_name(left: &str, right: &str) -> bool {
    let canonical = |name: &str| name.to_lowercase().replace('_', "-");
    canonical(left) == canonical(right)
}

/// What a failed search says: no connection, a rate limit, or crates.io's own status.
fn failure(status: i64) -> String {
    match status {
        UNREACHED => UNREACHABLE.to_string(),
        TOO_MANY => BUSY.to_string(),
        _ => UNAVAILABLE.replace(STATUS_SLOT, &status.to_string()),
    }
}

/// One row for a crate: its description and latest version, its downloads as badges, its crates.io page
/// and its repository or documentation. Crates whose names could not form a crates.io link are skipped.
fn row(found: Crate) -> Option<Row> {
    let valid = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if found.name.is_empty() || !found.name.chars().all(valid) { return None; }
    let version = found.max_stable_version.or(found.default_version).or(found.max_version);
    let details: Vec<String> = [found.description.map(|text| text.split_whitespace().collect::<Vec<_>>().join(" ")),
        version.map(|version| format!("{VERSION_PREFIX}{version}"))]
        .into_iter().flatten().filter(|text| !text.is_empty()).collect();
    let mut badges = Vec::new();
    if let Some(total) = found.downloads { badges.push(Badge { symbol: None, text: format!("{}{DOWNLOADS}", compact(total)) }); }
    if let Some(recent) = found.recent_downloads.filter(|count| *count > 0) {
        badges.push(Badge { symbol: None, text: format!("{}{RECENT}", compact(recent)) });
    }
    let (secondary_url, secondary_title) = match found.repository.filter(|link| is_web_link(link)) {
        Some(repository) => (repository, REPOSITORY_TITLE),
        None => (format!("{DOCS}{}", found.name), DOCS_TITLE),
    };
    Some(Row {
        url: Some(format!("{PAGE}{}", found.name)),
        title: found.name,
        subtitle: Some(details.join(SEPARATOR)).filter(|text| !text.is_empty()),
        secondary_url: Some(secondary_url),
        secondary_title: Some(secondary_title.to_string()),
        badges,
    })
}

/// The rows for a search reply, the crate named exactly as typed first.
fn render(request: &Request, text: &str) -> Vec<Row> {
    let Some(response) = request.responses.first() else { return vec![message(NOTHING)] };
    if response.status != OK { return vec![message(&failure(response.status))]; }
    let Ok(page) = serde_json::from_str::<Page>(&response.body) else { return vec![message(UNREADABLE)] };
    let mut crates = page.crates;
    crates.sort_by_key(|found| !(found.exact_match || same_name(&found.name, text)));
    let rows: Vec<Row> = crates.into_iter().filter_map(row).collect();
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
    const PAGE_BODY: &str = r#"{"crates":[
        {"id":"tidy-yaml","name":"tidy-yaml","description":"A tidy\n  YAML reader","downloads":1250,"recent_downloads":0,
         "default_version":"0.3.0-beta.1","max_stable_version":null,"max_version":"0.3.0-beta.1","repository":null,"exact_match":false},
        {"id":"tidy","name":"tidy","description":"Keeps things tidy","downloads":1658410158,"recent_downloads":411403991,
         "default_version":"1.0.229","max_stable_version":"1.0.229","max_version":"1.0.229",
         "repository":"https://github.com/example/tidy","exact_match":true},
        {"id":"bad name","name":"bad name","description":"Unlinkable","downloads":1,"exact_match":false}
    ],"meta":{"total":3,"next_page":null,"prev_page":null}}"#;

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
        for query in ["serde", "serde json", "tokio::sync", "a&b=c", "<script>", "#hash", "100%", "crème brûlée", "🦀", &"x".repeat(500)] {
            let url = url_for(&search_text(query).unwrap());
            assert!(manifest_permits(MANIFEST, &url), "{url}");
            assert!(!url.contains(' ') && !url.contains('#'), "{url}");
        }
        assert_eq!(url_for("serde json"), "https://crates.io/api/v1/crates?per_page=20&q=serde%20json");
        assert_eq!(search_text(&"x".repeat(500)).unwrap().len(), MAXIMUM_QUERY);
        assert_eq!(search_text("   "), None);
        let plan = reply(serde_json::json!({"phase":"plan","query":"  serde ","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://crates.io/api/v1/crates?per_page=20&q=serde"]));
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"","now":0}))["items"][0]["title"], PROMPT);
    }

    #[test]
    fn the_exact_match_leads_with_its_downloads_version_and_links() {
        let rows = rendered("tidy", 200, PAGE_BODY);
        assert_eq!(rows.len(), 2, "The crate with an unlinkable name is skipped");
        assert_eq!(rows[0]["title"], "tidy");
        assert_eq!(rows[0]["subtitle"], "Keeps things tidy · v1.0.229");
        assert_eq!(rows[0]["url"], "https://crates.io/crates/tidy");
        assert_eq!(rows[0]["secondaryURL"], "https://github.com/example/tidy");
        assert_eq!(rows[0]["secondaryTitle"], REPOSITORY_TITLE);
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"1.6B downloads"},{"text":"411M recent"}]));
        assert_eq!(rows[1]["subtitle"], "A tidy YAML reader · v0.3.0-beta.1");
        assert_eq!(rows[1]["secondaryURL"], "https://docs.rs/tidy-yaml");
        assert_eq!(rows[1]["secondaryTitle"], DOCS_TITLE);
        assert_eq!(rows[1]["badges"], serde_json::json!([{"text":"1.2K downloads"}]));
    }

    #[test]
    fn names_match_across_case_and_separators() {
        let rows = rendered("Tidy_YAML", 200, &PAGE_BODY.replace("\"exact_match\":true", "\"exact_match\":false"));
        assert_eq!(rows[0]["title"], "tidy-yaml");
        assert!(same_name("serde_json", "Serde-JSON"));
        assert!(!same_name("serde", "serde_json"));
    }

    #[test]
    fn numbers_are_compact() {
        assert_eq!(compact(0), "0");
        assert_eq!(compact(999), "999");
        assert_eq!(compact(1_000), "1K");
        assert_eq!(compact(12_345), "12K");
        assert_eq!(compact(412_345_678), "412M");
        assert_eq!(compact(1_658_410_158), "1.6B");
    }

    #[test]
    fn failures_and_empty_results_explain_themselves() {
        assert_eq!(rendered("zzqx", 200, r#"{"crates":[],"meta":{"total":0}}"#)[0]["title"], "No crates match zzqx.");
        assert_eq!(rendered("serde", 0, "")[0]["title"], UNREACHABLE);
        assert_eq!(rendered("serde", 429, "")[0]["title"], BUSY);
        assert_eq!(rendered("serde", 503, "")[0]["title"], "crates.io is unavailable (HTTP 503). Try again.");
        assert_eq!(rendered("serde", 200, "<html>")[0]["title"], UNREADABLE);
        let none = reply(serde_json::json!({"phase":"render","query":"serde","now":0,"responses":[]}));
        assert_eq!(none["items"][0]["title"], NOTHING);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"serde","now":0}))["items"][0]["title"], ONLY_LISTS);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&respond(b"nope")).unwrap()["items"][0]["title"], NOT_UNDERSTOOD);
    }
}
