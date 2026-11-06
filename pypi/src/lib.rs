//! PyPI for Volant: look up a Python package by its exact name. PyPI has no search API, and a
//! package's full JSON can run to megabytes (boto3's is over 3 MB, far past Volant's 512 KB response
//! limit), so the module reads it in two rounds: first the package's small releases feed, to learn its
//! latest release, then that one release's JSON. It shows the package's summary, version, supported
//! Python and license, and rows whose Return copies `uv add` and `pip install` commands. Return on the
//! first row opens its pypi.org page; Command-Return opens its source or homepage.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use volant_extension_kit::{encode_query, is_web_link, volant_extension_output};

const FEED: &str = "https://pypi.org/rss/project/";
const FEED_FILE: &str = "/releases.xml";
const API: &str = "https://pypi.org/pypi/";
const API_FILE: &str = "/json";
const PAGE: &str = "https://pypi.org/project/";
const SLASH: &str = "/";
const ITEM_OPEN: &str = "<item>";
const TITLE_OPEN: &str = "<title>";
const TITLE_CLOSE: &str = "</title>";
const RELEASES_STATE: &str = "releases";
const RELEASE_STATE: &str = "release:";
const UV_ADD: &str = "uv add ";
const PIP_INSTALL: &str = "pip install ";
const COPY_HINT: &str = "Return copies the command";
const SOURCE_TITLE: &str = "Open Source";
const HOMEPAGE_TITLE: &str = "Open Homepage";
const PYTHON_PREFIX: &str = "Python ";
const VERSION_PREFIX: &str = "v";
const YANKED: &str = "Yanked";
const SEPARATOR: &str = " · ";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const NAME_SLOT: &str = "{name}";
const STATUS_SLOT: &str = "{status}";
const PROMPT: &str = "Type a package's exact name, such as requests.";
const BAD_NAME: &str = "Python package names use letters, digits and - _ . only, starting and ending with a letter or digit.";
const NOT_FOUND_TEXT: &str = "No PyPI package is named {name}; PyPI has no search, so type the exact name.";
const NO_RELEASES: &str = "PyPI lists no releases of {name}.";
const UNREACHABLE: &str = "Volant could not reach PyPI. Check your connection and try again.";
const BUSY: &str = "PyPI is busy. Try again in a minute.";
const UNAVAILABLE: &str = "PyPI is unavailable (HTTP {status}). Try again.";
const UNREADABLE: &str = "PyPI sent a reply Volant could not read.";
const NOTHING: &str = "PyPI returned nothing.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const MAXIMUM_NAME: usize = 100;
const MAXIMUM_VERSION: usize = 64;
const MAXIMUM_BADGE: usize = 32;
const NAME_SEPARATORS: [char; 3] = ['-', '_', '.'];
const NORMAL_SEPARATOR: char = '-';
const VERSION_PUNCTUATION: &str = "._!+-";
const OK: i64 = 200;
const UNREACHED: i64 = 0;
const NOT_FOUND: i64 = 404;
const TOO_MANY: i64 = 429;

/// project_urls labels that name a package's source, then its homepage, as maintainers commonly write
/// them; compared without case.
const SOURCE_LABELS: [&str; 5] = ["source", "source code", "repository", "code", "github"];
const HOMEPAGE_LABELS: [&str; 2] = ["homepage", "home"];

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    responses: Vec<Response>,
    #[serde(default)]
    state: Option<String>,
}

#[derive(Deserialize)]
struct Response { status: i64, body: String }

#[derive(Deserialize)]
struct Release { info: Info }

#[derive(Deserialize)]
struct Info {
    name: String,
    summary: Option<String>,
    version: Option<String>,
    requires_python: Option<String>,
    license: Option<String>,
    license_expression: Option<String>,
    home_page: Option<String>,
    project_urls: Option<BTreeMap<String, String>>,
    #[serde(default)]
    yanked: bool,
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

/// What a render asks of Volant: rows to show, or another round of fetches with the state it needs.
enum Next {
    Rows(Vec<Row>),
    Fetch(String, String),
}

fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The name as PEP 503 normalizes it: lowercase, with each run of `-`, `_`, `.` or spaces as one `-`.
/// None when it is empty, too long, uses other characters or starts or ends with a separator.
pub fn name_for(query: &str) -> Option<String> {
    let mut name = String::new();
    for c in query.trim().to_lowercase().chars() {
        if NAME_SEPARATORS.contains(&c) || c.is_whitespace() {
            if !name.ends_with(NORMAL_SEPARATOR) { name.push(NORMAL_SEPARATOR); }
        } else if c.is_ascii_alphanumeric() {
            name.push(c);
        } else {
            return None;
        }
    }
    let edges_ok = !name.starts_with(NORMAL_SEPARATOR) && !name.ends_with(NORMAL_SEPARATOR);
    (!name.is_empty() && name.len() <= MAXIMUM_NAME && edges_ok).then_some(name)
}

/// The releases feed for a normalized name: the first round.
pub fn feed_url(name: &str) -> String {
    format!("{FEED}{name}{FEED_FILE}")
}

/// One release's JSON, which leaves out the files of every other release: the second round.
pub fn release_url(name: &str, version: &str) -> String {
    format!("{API}{name}{SLASH}{}{API_FILE}", encode_query(version))
}

/// The release numbers in a releases feed, newest upload first, read from each item's title.
fn feed_versions(feed: &str) -> Vec<String> {
    feed.split(ITEM_OPEN).skip(1).filter_map(|item| {
        let start = item.find(TITLE_OPEN)? + TITLE_OPEN.len();
        let length = item[start..].find(TITLE_CLOSE)?;
        let version = item[start..start + length].trim();
        let valid = |c: char| c.is_ascii_alphanumeric() || VERSION_PUNCTUATION.contains(c);
        (!version.is_empty() && version.len() <= MAXIMUM_VERSION && version.chars().all(valid)).then(|| version.to_string())
    }).collect()
}

/// A plain release such as 2.32.3 as its numbers; None for pre-releases and anything else.
fn release_numbers(version: &str) -> Option<Vec<u64>> {
    version.split('.').map(|part| part.parse::<u64>().ok()).collect()
}

/// The release to show: the highest plain release in the feed, so a backport uploaded after a newer
/// major does not win, or the newest upload when the feed lists only pre-releases.
fn latest(versions: &[String]) -> Option<&String> {
    versions.iter().filter_map(|version| Some((release_numbers(version)?, version))).max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, version)| version).or_else(|| versions.first())
}

/// What a failed fetch says: no connection, a rate limit, or PyPI's own status.
fn failure(status: i64) -> String {
    match status {
        UNREACHED => UNREACHABLE.to_string(),
        TOO_MANY => BUSY.to_string(),
        _ => UNAVAILABLE.replace(STATUS_SLOT, &status.to_string()),
    }
}

/// A badge for one short line of text; None when it is blank, spans lines or is too long to read.
fn badge(text: String) -> Option<Badge> {
    let text = text.trim().to_string();
    (!text.is_empty() && text.chars().count() <= MAXIMUM_BADGE && !text.contains('\n')).then_some(Badge { symbol: None, text })
}

/// The first project link whose label is one of `labels`, in that order.
fn project_link(links: &BTreeMap<String, String>, labels: &[&str]) -> Option<String> {
    labels.iter().find_map(|label| links.iter().find(|(key, link)| key.eq_ignore_ascii_case(label) && is_web_link(link)))
        .map(|(_, link)| link.clone())
}

/// Where Command-Return goes: the package's source, else its homepage.
fn second_link(info: &Info) -> Option<(String, &'static str)> {
    let links = info.project_urls.clone().unwrap_or_default();
    if let Some(source) = project_link(&links, &SOURCE_LABELS) { return Some((source, SOURCE_TITLE)); }
    project_link(&links, &HOMEPAGE_LABELS).or_else(|| info.home_page.clone().filter(|link| is_web_link(link)))
        .map(|homepage| (homepage, HOMEPAGE_TITLE))
}

/// The package row and its two install commands, which Return copies since they have no link.
fn rows(info: Info, name: &str) -> Vec<Row> {
    let second = second_link(&info);
    let mut details = Vec::new();
    if info.yanked { details.push(YANKED.to_string()); }
    if let Some(summary) = info.summary.map(|text| text.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|text| !text.is_empty()) {
        details.push(summary);
    }
    let license = info.license_expression.filter(|text| !text.trim().is_empty()).or(info.license);
    let badges = [info.version.map(|version| format!("{VERSION_PREFIX}{version}")),
        info.requires_python.map(|python| format!("{PYTHON_PREFIX}{python}")), license]
        .into_iter().flatten().filter_map(badge).collect();
    let command = |prefix: &str| Row { title: format!("{prefix}{}", info.name), subtitle: Some(COPY_HINT.to_string()), ..Default::default() };
    let commands = [command(UV_ADD), command(PIP_INSTALL)];
    let mut rows = vec![Row {
        title: info.name.clone(),
        subtitle: Some(details.join(SEPARATOR)).filter(|text| !text.is_empty()),
        url: Some(format!("{PAGE}{name}{SLASH}")),
        secondary_title: second.as_ref().map(|(_, title)| title.to_string()),
        secondary_url: second.map(|(link, _)| link),
        badges,
    }];
    rows.extend(commands);
    rows
}

/// A render: the releases feed turns into a request for the latest release, and that release into rows.
fn render(request: &Request, name: &str) -> Next {
    let Some(response) = request.responses.first() else { return Next::Rows(vec![message(NOTHING)]) };
    match response.status {
        OK => {}
        NOT_FOUND => return Next::Rows(vec![message(&NOT_FOUND_TEXT.replace(NAME_SLOT, name))]),
        status => return Next::Rows(vec![message(&failure(status))]),
    }
    if request.state.as_deref().is_some_and(|state| state.starts_with(RELEASE_STATE)) {
        return Next::Rows(match serde_json::from_str::<Release>(&response.body) {
            Ok(release) => rows(release.info, name),
            Err(_) => vec![message(UNREADABLE)],
        });
    }
    let versions = feed_versions(&response.body);
    match latest(&versions) {
        Some(version) => Next::Fetch(release_url(name, version), format!("{RELEASE_STATE}{version}")),
        None => Next::Rows(vec![message(&NO_RELEASES.replace(NAME_SLOT, name))]),
    }
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    let Some(name) = name_for(&request.query) else {
        return items(vec![message(if request.query.trim().is_empty() { PROMPT } else { BAD_NAME })]);
    };
    match request.phase.as_str() {
        PLAN => fetch(feed_url(&name), RELEASES_STATE.to_string()),
        RENDER => match render(&request, &name) {
            Next::Rows(rows) => items(rows),
            Next::Fetch(url, state) => fetch(url, state),
        },
        _ => items(vec![message(ONLY_LISTS)]),
    }
}

fn fetch(url: String, state: String) -> String {
    serde_json::json!({ "fetch": [url], "state": state }).to_string()
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
    const FEED_BODY: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>PyPI recent updates for widget.kit</title>
    <link>https://pypi.org/project/widget-kit/</link>
    <description>Recent updates to the Python Package Index for widget.kit</description>
    <language>en</language>    <item>
      <title>1.9.4</title>
      <link>https://pypi.org/project/widget-kit/1.9.4/</link>
      <description>Widgets for Python</description>
      <pubDate>Thu, 05 Nov 2026 20:24:12 GMT</pubDate>
    </item>    <item>
      <title>2.1.0rc1</title>
      <link>https://pypi.org/project/widget-kit/2.1.0rc1/</link>
      <pubDate>Wed, 04 Nov 2026 21:22:33 GMT</pubDate>
    </item>    <item>
      <title>2.0.10</title>
      <link>https://pypi.org/project/widget-kit/2.0.10/</link>
      <pubDate>Fri, 30 Oct 2026 19:18:47 GMT</pubDate>
    </item>    <item>
      <title>2.0.9</title>
      <link>https://pypi.org/project/widget-kit/2.0.9/</link>
      <pubDate>Thu, 29 Oct 2026 19:19:54 GMT</pubDate>
    </item>
  </channel>
</rss>"#;
    const RELEASE_BODY: &str = r#"{"info":{"author":"Example Widgets","name":"widget.kit","summary":"Widgets\n for Python",
        "version":"2.0.10","requires_python":">=3.10","license":"Apache License\nVersion 2.0, January 2004\n...",
        "license_expression":"Apache-2.0","home_page":"https://widgets.example.com",
        "project_urls":{"Documentation":"https://docs.widgets.example.com/","Homepage":"https://widgets.example.com","Source":"https://github.com/example/widget-kit"},
        "yanked":false,"description":"A long README"},"last_serial":1,"urls":[],"vulnerabilities":[]}"#;

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render_with(query: &str, state: &str, status: i64, body: &str) -> serde_json::Value {
        reply(serde_json::json!({"phase":"render","query":query,"now":0,"state":state,"responses":[{"url":"u","status":status,"body":body}]}))
    }

    #[test]
    fn names_are_normalized_as_pep_503_does() {
        assert_eq!(name_for("  Requests "), Some("requests".into()));
        assert_eq!(name_for("zope.interface"), Some("zope-interface".into()));
        assert_eq!(name_for("Typing__Extensions"), Some("typing-extensions".into()));
        assert_eq!(name_for("a-_.b"), Some("a-b".into()));
        assert_eq!(name_for("beautiful soup"), Some("beautiful-soup".into()));
        assert_eq!(name_for("-leading"), None);
        assert_eq!(name_for("trailing."), None);
        assert_eq!(name_for("a/b"), None);
        assert_eq!(name_for("../etc"), None);
        assert_eq!(name_for("crème"), None);
        assert_eq!(name_for(""), None);
        assert_eq!(name_for(&"a".repeat(101)), None);
    }

    #[test]
    fn both_rounds_ask_only_for_manifest_hosts() {
        for query in ["requests", "zope.interface", "Django", "typing_extensions", "x", &"a".repeat(100)] {
            let name = name_for(query).unwrap();
            assert!(manifest_permits(MANIFEST, &feed_url(&name)), "{name}");
            for version in ["1.0", "2.1.0rc1", "1!2.0", "1.0+local", "0.0.1.dev3"] {
                let url = release_url(&name, version);
                assert!(manifest_permits(MANIFEST, &url), "{url}");
            }
        }
        let plan = reply(serde_json::json!({"phase":"plan","query":"Zope.Interface","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://pypi.org/rss/project/zope-interface/releases.xml"]));
        assert_eq!(plan["state"], RELEASES_STATE);
        assert_eq!(release_url("boto3", "1.43.125"), "https://pypi.org/pypi/boto3/1.43.125/json");
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":" ","now":0}))["items"][0]["title"], PROMPT);
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"a/b","now":0}))["items"][0]["title"], BAD_NAME);
    }

    #[test]
    fn the_feed_leads_to_the_highest_plain_release() {
        assert_eq!(feed_versions(FEED_BODY), ["1.9.4", "2.1.0rc1", "2.0.10", "2.0.9"]);
        let next = render_with("widget.kit", RELEASES_STATE, 200, FEED_BODY);
        assert_eq!(next["fetch"], serde_json::json!(["https://pypi.org/pypi/widget-kit/2.0.10/json"]));
        assert_eq!(next["state"], "release:2.0.10");
        assert_eq!(latest(&["3.0a1".to_string(), "3.0a0".to_string()]).unwrap(), "3.0a1");
        let empty = render_with("widget.kit", RELEASES_STATE, 200, "<rss><channel><title>x</title></channel></rss>");
        assert_eq!(empty["items"][0]["title"], "PyPI lists no releases of widget-kit.");
    }

    #[test]
    fn a_release_shows_its_facts_and_install_commands() {
        let rows = render_with("widget.kit", "release:2.0.10", 200, RELEASE_BODY)["items"].as_array().unwrap().clone();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["title"], "widget.kit");
        assert_eq!(rows[0]["subtitle"], "Widgets for Python");
        assert_eq!(rows[0]["url"], "https://pypi.org/project/widget-kit/");
        assert_eq!(rows[0]["secondaryURL"], "https://github.com/example/widget-kit");
        assert_eq!(rows[0]["secondaryTitle"], SOURCE_TITLE);
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"v2.0.10"},{"text":"Python >=3.10"},{"text":"Apache-2.0"}]));
        assert_eq!(rows[1]["title"], "uv add widget.kit");
        assert_eq!(rows[2]["title"], "pip install widget.kit");
        assert!(rows[1].get("url").is_none() && rows[2].get("url").is_none(), "Without a link, Return copies the title");
    }

    #[test]
    fn long_licenses_are_left_out_and_homepages_stand_in_for_source() {
        let body = r#"{"info":{"name":"tiny","summary":"","version":"0.1","requires_python":null,"yanked":true,
            "license":"Permission is hereby granted, free of charge, to any person","license_expression":null,
            "home_page":"https://tiny.example.org","project_urls":null}}"#;
        let rows = render_with("tiny", "release:0.1", 200, body)["items"].as_array().unwrap().clone();
        assert_eq!(rows[0]["subtitle"], YANKED);
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"v0.1"}]));
        assert_eq!(rows[0]["secondaryURL"], "https://tiny.example.org");
        assert_eq!(rows[0]["secondaryTitle"], HOMEPAGE_TITLE);
    }

    #[test]
    fn missing_packages_and_failures_explain_themselves() {
        let missing = render_with("nosuchthing", RELEASES_STATE, 404, "<html>Not Found</html>");
        assert_eq!(missing["items"][0]["title"], "No PyPI package is named nosuchthing; PyPI has no search, so type the exact name.");
        assert_eq!(render_with("requests", RELEASES_STATE, 0, "")["items"][0]["title"], UNREACHABLE);
        assert_eq!(render_with("requests", "release:2.0", 429, "")["items"][0]["title"], BUSY);
        assert_eq!(render_with("requests", "release:2.0", 503, "")["items"][0]["title"], "PyPI is unavailable (HTTP 503). Try again.");
        assert_eq!(render_with("requests", "release:2.0", 200, "<html>")["items"][0]["title"], UNREADABLE);
        let none = reply(serde_json::json!({"phase":"render","query":"requests","now":0,"responses":[]}));
        assert_eq!(none["items"][0]["title"], NOTHING);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"requests","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
