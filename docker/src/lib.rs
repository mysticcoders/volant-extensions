//! Docker Hub for Volant: search container images through Docker Hub's repository search. Each row
//! shows an image's name (official images by their bare name, such as postgres), its short description,
//! and its stars, pulls and official status as badges. The image named exactly as typed, or else the
//! top result, comes first with a row whose Return copies its `docker pull` command. Return on an image
//! opens its Docker Hub page.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{encode_query, volant_extension_output};

const SEARCH: &str = "https://hub.docker.com/v2/search/repositories/?page_size=20&query=";
const OFFICIAL_PAGE: &str = "https://hub.docker.com/_/";
const PAGE: &str = "https://hub.docker.com/r/";
const LIBRARY: &str = "library/";
const PULL: &str = "docker pull ";
const COPY_HINT: &str = "Return copies the pull command";
const OFFICIAL: &str = "Official";
const PULLS: &str = " pulls";
const POINTS: &str = "points";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const QUERY_SLOT: &str = "{query}";
const STATUS_SLOT: &str = "{status}";
const PROMPT: &str = "Type an image name or keyword, such as postgres.";
const NO_MATCH: &str = "No Docker Hub images match {query}.";
const UNREACHABLE: &str = "Volant could not reach Docker Hub. Check your connection and try again.";
const BUSY: &str = "Docker Hub is busy. Try again in a minute.";
const UNAVAILABLE: &str = "Docker Hub is unavailable (HTTP {status}). Try again.";
const UNREADABLE: &str = "Docker Hub sent a reply Volant could not read.";
const NOTHING: &str = "Docker Hub returned nothing.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const NAME_PUNCTUATION: &str = "._-/";
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
struct Page { results: Option<Vec<Repository>> }

#[derive(Deserialize)]
struct Repository {
    repo_name: String,
    short_description: Option<String>,
    star_count: Option<u64>,
    pull_count: Option<u64>,
    #[serde(default)]
    is_official: bool,
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

/// An image as a row shows it, with its name as `docker pull` takes it.
struct Image {
    name: String,
    row: Row,
}

fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The text to search for: trimmed and at most 200 characters. None when nothing is typed.
pub fn search_text(query: &str) -> Option<String> {
    let text: String = query.trim().chars().take(MAXIMUM_QUERY).collect();
    (!text.trim().is_empty()).then(|| text.trim().to_string())
}

/// The Docker Hub search request for a query.
pub fn url_for(text: &str) -> String {
    format!("{SEARCH}{}", encode_query(text))
}

/// 12,267,525,604 as "12B", 1,321,301 as "1.3M": at most one decimal, below ten of a unit.
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

/// What a failed search says: no connection, a rate limit, or Docker Hub's own status.
fn failure(status: i64) -> String {
    match status {
        UNREACHED => UNREACHABLE.to_string(),
        TOO_MANY => BUSY.to_string(),
        _ => UNAVAILABLE.replace(STATUS_SLOT, &status.to_string()),
    }
}

/// One image from a search hit, linked to its Docker Hub page. Official images drop the `library/`
/// namespace. Hits whose names could not form a Docker Hub link are skipped.
fn image(hit: Repository) -> Option<Image> {
    let name = hit.repo_name.strip_prefix(LIBRARY).unwrap_or(&hit.repo_name).to_lowercase();
    let valid = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || NAME_PUNCTUATION.contains(c);
    let parts = name.split('/').count();
    if name.is_empty() || !name.chars().all(valid) || parts > 2 || name.split('/').any(str::is_empty) { return None; }
    let official = hit.is_official && parts == 1;
    let mut badges = Vec::new();
    if let Some(stars) = hit.star_count { badges.push(Badge { symbol: Some(POINTS), text: compact(stars) }); }
    if let Some(pulls) = hit.pull_count { badges.push(Badge { symbol: None, text: format!("{}{PULLS}", compact(pulls)) }); }
    if official { badges.push(Badge { symbol: None, text: OFFICIAL.to_string() }); }
    let page = if official { OFFICIAL_PAGE } else { PAGE };
    let row = Row {
        title: name.clone(),
        subtitle: hit.short_description.map(|text| text.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|text| !text.is_empty()),
        url: Some(format!("{page}{name}")),
        badges,
    };
    Some(Image { name, row })
}

/// The rows for a search reply: the image named as typed (or the top result) and its pull command,
/// then the rest in Docker Hub's order.
fn render(request: &Request, text: &str) -> Vec<Row> {
    let Some(response) = request.responses.first() else { return vec![message(NOTHING)] };
    if response.status != OK { return vec![message(&failure(response.status))]; }
    let Ok(page) = serde_json::from_str::<Page>(&response.body) else { return vec![message(UNREADABLE)] };
    let mut images: Vec<Image> = page.results.unwrap_or_default().into_iter().filter_map(image).collect();
    if images.is_empty() { return vec![message(&NO_MATCH.replace(QUERY_SLOT, text))]; }
    let wanted = text.to_lowercase();
    let wanted = wanted.strip_prefix(LIBRARY).unwrap_or(&wanted);
    if let Some(index) = images.iter().position(|image| image.name == wanted) {
        let exact = images.remove(index);
        images.insert(0, exact);
    }
    let pull = Row { title: format!("{PULL}{}", images[0].name), subtitle: Some(COPY_HINT.to_string()), ..Default::default() };
    let mut rows: Vec<Row> = images.into_iter().map(|image| image.row).collect();
    rows.insert(1, pull);
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
    const PAGE_BODY: &str = r#"{"count":4,"next":"","previous":"","results":[
        {"repo_name":"acme/keystore","short_description":"Keystore image\n maintained by Acme","star_count":9,"pull_count":851805627,"repo_owner":"","is_automated":false,"is_official":false},
        {"repo_name":"keystore","short_description":"The Keystore database.","star_count":15048,"pull_count":12267525604,"repo_owner":"","is_automated":false,"is_official":true},
        {"repo_name":"other/keystore-tools","short_description":"","star_count":0,"pull_count":24,"repo_owner":"","is_automated":false,"is_official":false},
        {"repo_name":"bad name/x","short_description":"Unlinkable","star_count":1,"pull_count":1,"is_official":false}
    ]}"#;

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn rendered(query: &str, status: i64, body: &str) -> Vec<serde_json::Value> {
        let url = url_for(&search_text(query).unwrap());
        let value = reply(serde_json::json!({"phase":"render","query":query,"now":0,"responses":[{"url":url,"status":status,"body":body}]}));
        value["items"].as_array().unwrap().clone()
    }

    fn titles(rows: &[serde_json::Value]) -> Vec<&str> {
        rows.iter().map(|row| row["title"].as_str().unwrap()).collect()
    }

    #[test]
    fn queries_become_encoded_search_urls_on_the_manifest_host() {
        for query in ["postgres", "grafana/grafana", "node 22", "a&b=c", "<script>", "#hash", "100%", "crème", "🐳", &"x".repeat(500)] {
            let url = url_for(&search_text(query).unwrap());
            assert!(manifest_permits(MANIFEST, &url), "{url}");
            assert!(!url.contains(' ') && !url.contains('#'), "{url}");
        }
        assert_eq!(url_for("grafana/grafana"), "https://hub.docker.com/v2/search/repositories/?page_size=20&query=grafana%2Fgrafana");
        let plan = reply(serde_json::json!({"phase":"plan","query":" postgres ","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://hub.docker.com/v2/search/repositories/?page_size=20&query=postgres"]));
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"","now":0}))["items"][0]["title"], PROMPT);
    }

    #[test]
    fn the_official_image_leads_with_its_pull_command_and_badges() {
        let rows = rendered("Keystore", 200, PAGE_BODY);
        assert_eq!(titles(&rows), ["keystore", "docker pull keystore", "acme/keystore", "other/keystore-tools"]);
        assert_eq!(rows[0]["url"], "https://hub.docker.com/_/keystore");
        assert_eq!(rows[0]["subtitle"], "The Keystore database.");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"symbol":"points","text":"15K"},{"text":"12B pulls"},{"text":"Official"}]));
        assert!(rows[1].get("url").is_none(), "Without a link, Return copies the title");
        assert_eq!(rows[2]["url"], "https://hub.docker.com/r/acme/keystore");
        assert_eq!(rows[2]["subtitle"], "Keystore image maintained by Acme");
        assert_eq!(rows[2]["badges"], serde_json::json!([{"symbol":"points","text":"9"},{"text":"851M pulls"}]));
        assert!(rows[3].get("subtitle").is_none());
    }

    #[test]
    fn a_namespaced_name_or_the_top_result_gets_the_pull_row() {
        assert_eq!(titles(&rendered("acme/keystore", 200, PAGE_BODY))[..2], ["acme/keystore", "docker pull acme/keystore"]);
        assert_eq!(titles(&rendered("library/keystore", 200, PAGE_BODY))[..2], ["keystore", "docker pull keystore"]);
        assert_eq!(titles(&rendered("keys", 200, PAGE_BODY))[..2], ["acme/keystore", "docker pull acme/keystore"]);
    }

    #[test]
    fn numbers_are_compact() {
        assert_eq!(compact(24), "24");
        assert_eq!(compact(1_321_301), "1.3M");
        assert_eq!(compact(15_048), "15K");
        assert_eq!(compact(12_267_525_604), "12B");
    }

    #[test]
    fn failures_and_empty_results_explain_themselves() {
        assert_eq!(rendered("zzqx", 200, r#"{"count":0,"next":"","previous":"","results":[]}"#)[0]["title"], "No Docker Hub images match zzqx.");
        assert_eq!(rendered("postgres", 0, "")[0]["title"], UNREACHABLE);
        assert_eq!(rendered("postgres", 429, "")[0]["title"], BUSY);
        assert_eq!(rendered("postgres", 500, "")[0]["title"], "Docker Hub is unavailable (HTTP 500). Try again.");
        assert_eq!(rendered("postgres", 200, "<html>")[0]["title"], UNREADABLE);
        let none = reply(serde_json::json!({"phase":"render","query":"postgres","now":0,"responses":[]}));
        assert_eq!(none["items"][0]["title"], NOTHING);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"postgres","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
