//! GitHub for Volant: pull requests waiting for your review, your own pull requests, and issues
//! assigned to or involving you, read through GitHub's search API on api.github.com. The launcher's
//! dropdown picks the list and any typed text narrows it with GitHub's search syntax, such as
//! `repo:owner/name` or `label:bug`. Each row shows its repository, number, age and author, with
//! comments and draft state as badges. Return opens it on github.com; Command-Return opens its
//! repository.
//!
//! Volant adds the owner's personal access token to requests to api.github.com itself, so this
//! module never sees it. Older Volant versions without the dropdown pick the list from the first word
//! instead: `github mine`, `github assigned`, `github involved`.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{age, encode_query, parse_rfc3339, volant_extension_output};

const SEARCH: &str = "https://api.github.com/search/issues?per_page=25&sort=updated&q=";
const REPOSITORY_API: &str = "https://api.github.com/repos/";
const SITE: &str = "https://github.com/";
const REPOSITORY_TITLE: &str = "Open Repository";
const COMMENTS: &str = "comments";
const DRAFT: &str = "Draft";
const SEPARATOR: &str = " · ";
const NUMBER_PREFIX: &str = "#";
const AUTHOR_PREFIX: &str = "by ";
const SPACE: &str = " ";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const UNREADABLE: &str = "GitHub sent a reply Volant could not read.";
const REFUSED: &str = "GitHub refused the token. Update it in Settings, Extensions.";
const NOTHING: &str = "GitHub returned nothing.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const OK: i64 = 200;
const UNAUTHORIZED: i64 = 401;
const FORBIDDEN: i64 = 403;
const INVALID: i64 = 422;
const TOO_MANY: i64 = 429;

/// The lists by dropdown id (which older Volant versions read from the query's first word), the search
/// qualifiers each runs, and what an empty one says.
const LISTS: &[(&str, &str, &str)] = &[
    ("review", "is:open is:pr review-requested:@me archived:false", "No pull requests are waiting for your review."),
    ("mine", "is:open is:pr author:@me archived:false", "You have no open pull requests."),
    ("assigned", "is:open assignee:@me archived:false", "Nothing open is assigned to you."),
    ("involved", "is:open involves:@me archived:false", "Nothing open involves you."),
];

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    now: i64,
    #[serde(default)]
    responses: Vec<Response>,
    #[serde(default)]
    filter: Option<String>,
}

#[derive(Deserialize)]
struct Response { status: i64, body: String }

#[derive(Deserialize)]
struct Page { items: Vec<Hit> }

#[derive(Deserialize)]
struct Hit {
    title: String,
    html_url: String,
    number: i64,
    repository_url: String,
    #[serde(default)]
    comments: i64,
    updated_at: Option<String>,
    #[serde(default)]
    draft: bool,
    user: Option<User>,
}

#[derive(Deserialize)]
struct User { login: String }

#[derive(Serialize, Default, Debug, PartialEq)]
pub struct Row {
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

/// One explaining row, for an empty list or a failure.
fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The list to run and the search text: the dropdown's filter when Volant sent one, otherwise a list
/// named by the query's first word, otherwise the first list with the whole query as text.
fn list_for<'a>(query: &'a str, filter: Option<&str>) -> (&'static (&'static str, &'static str, &'static str), &'a str) {
    let query = query.trim();
    if let Some(list) = filter.and_then(|id| LISTS.iter().find(|(list_id, ..)| *list_id == id)) {
        return (list, query);
    }
    let (word, rest) = query.split_once(SPACE).unwrap_or((query, ""));
    match LISTS.iter().find(|(id, ..)| id.eq_ignore_ascii_case(word)) {
        Some(list) => (list, rest.trim()),
        None => (&LISTS[0], query),
    }
}

/// The search request for a query and filter: the list's qualifiers plus any typed text.
pub fn url_for(query: &str, filter: Option<&str>) -> String {
    let ((_, qualifiers, _), text) = list_for(query, filter);
    let search = if text.is_empty() { qualifiers.to_string() } else { format!("{qualifiers}{SPACE}{text}") };
    format!("{SEARCH}{}", encode_query(&search))
}

/// What a failed search says: a refused token, a rate limit, or GitHub's own status.
fn failure(status: i64) -> String {
    match status {
        UNAUTHORIZED => REFUSED.to_string(),
        FORBIDDEN | TOO_MANY => "GitHub's search limit was reached. Try again in a minute.".to_string(),
        INVALID => "GitHub could not understand that search. Check its qualifiers.".to_string(),
        _ => format!("GitHub is unavailable (HTTP {status}). Try again."),
    }
}

/// `owner/name` from an API repository URL.
fn repository(api_url: &str) -> Option<&str> {
    api_url.strip_prefix(REPOSITORY_API).filter(|name| name.split('/').count() == 2 && !name.contains(['?', '#']))
}

/// One row for an issue or pull request: its repository, number, age and author as the subtitle, and
/// comments and draft state as badges. Hits without a github.com link or a readable repository are skipped.
fn row(hit: Hit, now: i64) -> Option<Row> {
    let name = repository(&hit.repository_url)?;
    if !hit.html_url.starts_with(SITE) || hit.title.trim().is_empty() { return None; }
    let mut details = vec![format!("{name}{SPACE}{NUMBER_PREFIX}{}", hit.number)];
    if let Some(updated) = hit.updated_at.as_deref().and_then(parse_rfc3339) { details.push(age(now, updated)); }
    if let Some(user) = hit.user { details.push(format!("{AUTHOR_PREFIX}{}", user.login)); }
    let mut badges = Vec::new();
    if hit.draft { badges.push(Badge { symbol: None, text: DRAFT.to_string() }); }
    if hit.comments > 0 { badges.push(Badge { symbol: Some(COMMENTS), text: hit.comments.to_string() }); }
    Some(Row {
        title: hit.title,
        subtitle: Some(details.join(SEPARATOR)),
        url: Some(hit.html_url),
        secondary_url: Some(format!("{SITE}{name}")),
        secondary_title: Some(REPOSITORY_TITLE.to_string()),
        badges,
    })
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": [url_for(&request.query, request.filter.as_deref())] }).to_string(),
        RENDER => items(render(&request)),
        _ => items(vec![message(ONLY_LISTS)]),
    }
}

fn render(request: &Request) -> Vec<Row> {
    let Some(response) = request.responses.first() else { return vec![message(NOTHING)] };
    if response.status != OK { return vec![message(&failure(response.status))]; }
    let Ok(page) = serde_json::from_str::<Page>(&response.body) else { return vec![message(UNREADABLE)] };
    let rows: Vec<Row> = page.items.into_iter().filter_map(|hit| row(hit, request.now)).collect();
    if rows.is_empty() {
        let ((_, _, empty), text) = list_for(&request.query, request.filter.as_deref());
        return vec![message(if text.is_empty() { empty } else { "Nothing matches that search." })];
    }
    rows
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

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    #[test]
    fn the_filter_picks_the_list_and_text_narrows_it() {
        assert!(url_for("", Some("review")).ends_with("q=is%3Aopen%20is%3Apr%20review-requested%3A%40me%20archived%3Afalse"));
        assert!(url_for("repo:a/b", Some("mine")).contains("author%3A%40me%20archived%3Afalse%20repo%3Aa%2Fb"));
        assert!(url_for("mine", Some("assigned")).contains("assignee%3A%40me%20archived%3Afalse%20mine"), "With a filter, list words are search text");
        assert!(url_for("assigned label:bug", None).contains("assignee%3A%40me%20archived%3Afalse%20label%3Abug"), "Without one, the first word picks the list");
        assert_eq!(url_for("flaky test", None), url_for("flaky test", Some("review")));
        assert_eq!(url_for("", Some("unknown")), url_for("", None), "An unknown filter acts like none");
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["", "mine", "repo:a/b is:closed", "C++ & you?"] {
            for filter in [None, Some("review"), Some("mine"), Some("assigned"), Some("involved")] {
                let url = url_for(query, filter);
                assert!(manifest_permits(MANIFEST, &url), "{url}");
            }
        }
        let plan = reply(serde_json::json!({"phase":"plan","query":"","now":0,"filter":"involved"}));
        assert!(plan["fetch"][0].as_str().unwrap().contains("involves%3A%40me"));
    }

    #[test]
    fn render_builds_rows_with_repository_links_and_badges() {
        let body = r#"{"items":[
            {"title":"Fictional fix","html_url":"https://github.com/fictional/app/pull/7","number":7,
             "repository_url":"https://api.github.com/repos/fictional/app","comments":3,"updated_at":"1970-01-01T00:15:00Z",
             "draft":true,"user":{"login":"pat"},"body":"long text"},
            {"title":"Quiet issue","html_url":"https://github.com/fictional/app/issues/8","number":8,
             "repository_url":"https://api.github.com/repos/fictional/app","comments":0},
            {"title":"Elsewhere","html_url":"https://example.com/9","number":9,"repository_url":"https://api.github.com/repos/fictional/app"}]}"#;
        let value = reply(serde_json::json!({"phase":"render","query":"","now":4500,"responses":[{"url":"u","status":200,"body":body}]}));
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 2, "Rows must link to github.com");
        assert_eq!(items[0]["subtitle"], "fictional/app #7 · 1h ago · by pat");
        assert_eq!(items[0]["url"], "https://github.com/fictional/app/pull/7");
        assert_eq!(items[0]["secondaryURL"], "https://github.com/fictional/app");
        assert_eq!(items[0]["secondaryTitle"], "Open Repository");
        assert_eq!(items[0]["badges"], serde_json::json!([{"text":"Draft"},{"symbol":"comments","text":"3"}]));
        assert!(items[1].get("badges").is_none());
        assert_eq!(items[1]["subtitle"], "fictional/app #8");
    }

    #[test]
    fn empty_lists_and_failures_explain_themselves() {
        let empty = |query: &str, filter: &str| reply(serde_json::json!({"phase":"render","query":query,"now":0,"filter":filter,
            "responses":[{"url":"u","status":200,"body":r#"{"items":[]}"#}]}))["items"][0]["title"].clone();
        assert_eq!(empty("", "mine"), "You have no open pull requests.");
        assert_eq!(empty("flaky", "mine"), "Nothing matches that search.");
        for (status, text) in [(401, "refused the token"), (403, "search limit"), (422, "could not understand"), (0, "HTTP 0")] {
            let value = reply(serde_json::json!({"phase":"render","query":"","now":0,"responses":[{"url":"u","status":status,"body":""}]}));
            assert!(value["items"][0]["title"].as_str().unwrap().contains(text), "{status}");
        }
        let unreadable = reply(serde_json::json!({"phase":"render","query":"","now":0,"responses":[{"url":"u","status":200,"body":"<html>"}]}));
        assert_eq!(unreadable["items"][0]["title"], UNREADABLE);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
