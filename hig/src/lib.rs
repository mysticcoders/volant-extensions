//! Apple HIG for Volant: finds topics in Apple's Human Interface Guidelines by title or summary and
//! opens them on developer.apple.com. It reads the same DocC JSON the HIG site itself loads, from
//! developer.apple.com/tutorials/data/design/human-interface-guidelines/, fresh on every run.
//!
//! The HIG's top page lists only its six sections; each section's page lists its topics, and
//! Components lists eight groups whose pages list the components. Volant fetches at most four URLs a
//! round and three rounds a run, which covers twelve pages: Foundations, Patterns, Inputs and
//! Technologies, then the eight Components groups. So the dropdown's All searches those five sections
//! (Getting Started, with its platform overviews, has its own entry), and each section can be browsed
//! alone. All with nothing typed lists the sections without fetching anything.
//!
//! A render sees only the responses of its own round, so the matches found so far travel to the next
//! round in the plan's `state`, which Volant caps at 4 KB: best matches first, dropping a summary or,
//! for very broad searches, the weakest matches when they would not fit. Each row shows a topic's
//! summary (with its group, for components) and a badge naming its section.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use volant_extension_kit::volant_extension_output;

const DATA: &str = "https://developer.apple.com/tutorials/data/design/human-interface-guidelines/";
const DATA_SUFFIX: &str = ".json";
const SITE: &str = "https://developer.apple.com";
const PAGE_PREFIX: &str = "/design/human-interface-guidelines/";
const TOPIC: &str = "topic";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const GETTING_STARTED: &str = "Getting Started";
const FOUNDATIONS: &str = "Foundations";
const PATTERNS: &str = "Patterns";
const COMPONENTS: &str = "Components";
const INPUTS: &str = "Inputs";
const TECHNOLOGIES: &str = "Technologies";
const SECTION_HINT: &str = "Choose it from the menu at the end of the field to browse its topics";
const NOTHING_FOUND_START: &str = "No HIG topic matches “";
const NOTHING_FOUND_END: &str = "”";
const NOTHING_FOUND_HINT: &str = "Try another section from the menu at the end of the field";
const EMPTY_SECTION: &str = "This section listed no topics.";
const PARTLY_LOADED_START: &str = "Some HIG pages did not load (";
const PARTLY_LOADED_END: &str = "), so this list may be incomplete.";
const UNREADABLE: &str = "Apple’s design site sent a page Volant could not read.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const SEPARATOR: &str = " · ";
const ELLIPSIS: &str = "…";
const OK: i64 = 200;
const MAXIMUM_ROWS: usize = 100;
/// Volant's limit on `state` is 4,096 bytes; this leaves room for the wrapper object.
const STATE_BUDGET: usize = 3_900;
const SHORT_ABSTRACT: usize = 80;

/// A page the extension reads: its slug, the section badge its topics carry, and for a Components
/// group, the group's name shown before each component's summary.
struct Page { slug: &'static str, section: &'static str, group: Option<&'static str> }

const PAGES: [Page; 13] = [
    Page { slug: "foundations", section: FOUNDATIONS, group: None },
    Page { slug: "patterns", section: PATTERNS, group: None },
    Page { slug: "inputs", section: INPUTS, group: None },
    Page { slug: "technologies", section: TECHNOLOGIES, group: None },
    Page { slug: "content", section: COMPONENTS, group: Some("Content") },
    Page { slug: "status", section: COMPONENTS, group: Some("Status") },
    Page { slug: "navigation-and-search", section: COMPONENTS, group: Some("Navigation and search") },
    Page { slug: "presentation", section: COMPONENTS, group: Some("Presentation") },
    Page { slug: "layout-and-organization", section: COMPONENTS, group: Some("Layout and organization") },
    Page { slug: "menus-and-actions", section: COMPONENTS, group: Some("Menus and actions") },
    Page { slug: "selection-and-input", section: COMPONENTS, group: Some("Selection and input") },
    Page { slug: "system-experiences", section: COMPONENTS, group: Some("System experiences") },
    Page { slug: "getting-started", section: GETTING_STARTED, group: None },
];

/// The Components groups in the order they are fetched: the four smallest first, so the matches
/// carried to the second round fit in the state.
const COMPONENT_ROUNDS: [&[usize]; 2] = [&[4, 5, 6, 7], &[8, 9, 10, 11]];
const ALL_ROUNDS: [&[usize]; 3] = [&[0, 1, 2, 3], &[4, 5, 6, 7], &[8, 9, 10, 11]];

/// The dropdown's sections: filter id, the pages read for it round by round, and the section page.
const SECTIONS: [(&str, &str, &[&[usize]]); 6] = [
    ("getting-started", GETTING_STARTED, &[&[12]]),
    ("foundations", FOUNDATIONS, &[&[0]]),
    ("patterns", PATTERNS, &[&[1]]),
    ("components", COMPONENTS, &COMPONENT_ROUNDS),
    ("inputs", INPUTS, &[&[2]]),
    ("technologies", TECHNOLOGIES, &[&[3]]),
];

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response { url: String, status: i64, body: String }

/// The parts of a DocC page this extension reads.
#[derive(Deserialize)]
struct DocPage {
    #[serde(rename = "topicSections", default)]
    topic_sections: Vec<TopicSection>,
    #[serde(default)]
    references: HashMap<String, Reference>,
}

#[derive(Deserialize)]
struct TopicSection { #[serde(default)] identifiers: Vec<String> }

#[derive(Deserialize)]
struct Reference {
    #[serde(rename = "type", default)]
    kind: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(rename = "abstract", default)]
    summary: Vec<Inline>,
}

/// One piece of DocC inline text: plain text, or code voice, the only kinds a summary uses.
#[derive(Deserialize)]
struct Inline {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

/// A topic found so far, as carried between rounds: how well it matched, its place in the HIG, the
/// page it came from, its title, its slug and its summary.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Found(pub u8, pub u32, pub u8, pub String, pub String, pub String);

/// What one render hands the next: the round just fetched, pages that failed, and matches so far.
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct State { round: usize, failed: Vec<i64>, found: Vec<Found> }

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

/// A row badge with only text. Volant 0.3.8 and later show it in place of the extension's name.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// The DocC JSON URL of a page.
pub fn page_url(slug: &str) -> String {
    [DATA, slug, DATA_SUFFIX].concat()
}

/// The pages to read for a dropdown filter, round by round. No filter (Volant before 0.3.9) or an
/// unknown one means All.
fn rounds(filter: Option<&str>) -> &'static [&'static [usize]] {
    SECTIONS.iter().find(|(id, ..)| Some(*id) == filter).map_or(&ALL_ROUNDS, |(_, _, rounds)| rounds)
}

/// The URLs of one round.
pub fn round_urls(filter: Option<&str>, round: usize) -> Vec<String> {
    rounds(filter).get(round).map(|pages| pages.iter().map(|&page| page_url(PAGES[page].slug)).collect()).unwrap_or_default()
}

/// How well a topic matches the lowercase `query`, lower being better: the title itself, then a title
/// starting with it, a title word starting with it, a title containing it, and last a title and summary
/// that together hold every word. An empty query matches everything equally.
pub fn rank(title: &str, summary: &str, query: &str) -> Option<u8> {
    if query.is_empty() { return Some(0); }
    let title = title.to_lowercase();
    if title == query { return Some(0); }
    if title.starts_with(query) { return Some(1); }
    if title.split_whitespace().any(|word| word.starts_with(query)) { return Some(2); }
    if title.contains(query) { return Some(3); }
    let both = [title.as_str(), &summary.to_lowercase()].join(" ");
    query.split_whitespace().all(|word| both.contains(word)).then_some(4)
}

/// The topics a page lists that match `query`, in the page's order.
fn page_matches(body: &str, page: usize, query: &str) -> Option<Vec<Found>> {
    let doc: DocPage = serde_json::from_str(body).ok()?;
    let identifiers = doc.topic_sections.iter().flat_map(|section| section.identifiers.iter());
    let found = identifiers.enumerate().filter_map(|(position, identifier)| {
        let reference = doc.references.get(identifier).filter(|reference| reference.kind == TOPIC)?;
        let slug = reference.url.strip_prefix(PAGE_PREFIX).filter(|slug| !slug.is_empty() && !slug.contains(['?', '#']))?;
        let title = reference.title.trim();
        if title.is_empty() { return None; }
        let summary: String = reference.summary.iter().filter_map(|inline| inline.text.as_deref().or(inline.code.as_deref())).collect();
        let quality = rank(title, &summary, query)?;
        Some(Found(quality, (page * 100 + position) as u32, page as u8, title.to_string(), slug.to_string(), summary.trim().to_string()))
    });
    Some(found.collect())
}

/// `found` sorted best first, then in the HIG's own order.
fn sorted(mut found: Vec<Found>) -> Vec<Found> {
    found.sort_by_key(|Found(quality, order, ..)| (*quality, *order));
    found
}

/// The state for the next round, kept within Volant's limit: matches are added best first, a summary
/// is shortened and then dropped when the match would not fit with it, and the rest are left out once
/// even a bare title would not.
fn fitted_state(round: usize, failed: Vec<i64>, found: Vec<Found>) -> String {
    let mut state = State { round, failed, found: Vec::new() };
    let size = |state: &State| serde_json::to_string(state).map_or(usize::MAX, |text| text.len());
    for item in sorted(found) {
        let short: String = item.5.chars().take(SHORT_ABSTRACT).collect();
        let shortened = if short.len() < item.5.len() { [short.as_str(), ELLIPSIS].concat() } else { short };
        let candidates = [item.clone(), Found(item.0, item.1, item.2, item.3.clone(), item.4.clone(), shortened), Found(item.0, item.1, item.2, item.3, item.4, String::new())];
        let mut added = false;
        for candidate in candidates {
            state.found.push(candidate);
            if size(&state) <= STATE_BUDGET { added = true; break; }
            state.found.pop();
        }
        if !added { break; }
    }
    serde_json::to_string(&state).unwrap_or_default()
}

/// A topic's row: its title, its summary (after its group, for a component), its page, and a badge
/// naming its section.
fn row(found: &Found) -> Row {
    let page = PAGES.get(found.2 as usize);
    let details: Vec<&str> = [page.and_then(|page| page.group), Some(found.5.as_str())].into_iter().flatten().filter(|text| !text.is_empty()).collect();
    Row {
        title: found.3.clone(),
        subtitle: (!details.is_empty()).then(|| details.join(SEPARATOR)),
        url: Some([SITE, PAGE_PREFIX, &found.4].concat()),
        badges: page.map(|page| vec![Badge { text: page.section.to_string() }]).unwrap_or_default(),
    }
}

/// The sections, for All with nothing typed: each opens its page, and the menu browses it.
fn overview() -> Vec<Row> {
    SECTIONS.iter().map(|(id, title, _)| Row {
        title: title.to_string(),
        subtitle: Some(SECTION_HINT.to_string()),
        url: Some([SITE, PAGE_PREFIX, id].concat()),
        ..Default::default()
    }).collect()
}

fn message(title: String, subtitle: Option<&str>) -> Row {
    Row { title, subtitle: subtitle.map(str::to_string), ..Default::default() }
}

fn unavailable(status: i64) -> String {
    format!("Apple’s design site is unavailable (HTTP {status}). Try again.")
}

/// The rows once every round is in, or the next round's fetch and state.
fn render(request: &Request, filter: Option<&str>, query: &str) -> String {
    let previous: State = request.state.as_deref().and_then(|text| serde_json::from_str(text).ok()).unwrap_or_default();
    let (mut found, mut failed) = (previous.found, previous.failed);
    let mut unreadable = false;
    let pages = rounds(filter).get(previous.round).copied().unwrap_or_default();
    for response in &request.responses {
        let Some(&page) = pages.iter().find(|&&page| page_url(PAGES[page].slug) == response.url) else { continue };
        if response.status != OK { failed.push(response.status); continue; }
        match page_matches(&response.body, page, query) {
            Some(matches) => found.extend(matches),
            None => unreadable = true,
        }
    }
    let next = previous.round + 1;
    if next < rounds(filter).len() {
        return serde_json::json!({ "fetch": round_urls(filter, next), "state": fitted_state(next, failed, found) }).to_string();
    }
    let mut rows: Vec<Row> = sorted(found).iter().take(MAXIMUM_ROWS).map(row).collect();
    if rows.is_empty() {
        if unreadable { return items(vec![message(UNREADABLE.to_string(), None)]); }
        if let Some(&status) = failed.first() { return items(vec![message(unavailable(status), None)]); }
        if query.is_empty() { return items(vec![message(EMPTY_SECTION.to_string(), None)]); }
        return items(vec![message([NOTHING_FOUND_START, query, NOTHING_FOUND_END].concat(), Some(NOTHING_FOUND_HINT))]);
    }
    if !failed.is_empty() && rows.len() < MAXIMUM_ROWS {
        let statuses: Vec<String> = failed.iter().map(|status| format!("HTTP {status}")).collect();
        rows.push(message([PARTLY_LOADED_START, &statuses.join(", "), PARTLY_LOADED_END].concat(), None));
    }
    items(rows)
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD.to_string(), None)]) };
    let filter = request.filter.as_deref().filter(|id| SECTIONS.iter().any(|(section, ..)| section == id));
    let query = request.query.trim().to_lowercase();
    match request.phase.as_str() {
        PLAN if filter.is_none() && query.is_empty() => items(overview()),
        PLAN => serde_json::json!({ "fetch": round_urls(filter, 0) }).to_string(),
        RENDER => render(&request, filter, &query),
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
    const ALL: &str = "all";

    /// A fictional DocC section page in the shape developer.apple.com serves.
    fn page(topics: &[(&str, &str, &str)]) -> String {
        let identifiers: Vec<String> = topics.iter().map(|(_, slug, _)| format!("doc://com.apple.HIG/design/Human-Interface-Guidelines/{slug}")).collect();
        let mut references = serde_json::Map::new();
        for ((title, slug, summary), identifier) in topics.iter().zip(&identifiers) {
            references.insert(identifier.clone(), serde_json::json!({"abstract":[{"text":summary,"type":"text"}],"identifier":identifier,
                "kind":"article","role":"article","title":title,"type":"topic","url":format!("/design/human-interface-guidelines/{slug}")}));
        }
        references.insert("icon.svg".into(), serde_json::json!({"type":"image","identifier":"icon.svg"}));
        serde_json::json!({"topicSections":[{"anchor":"Topics","identifiers":identifiers}],"references":references}).to_string()
    }

    fn run(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    /// Runs every round of a filter and query against fixture pages, as Volant would, and returns the rows.
    fn all_rounds(filter: Option<&str>, query: &str, body_for: impl Fn(&str) -> (i64, String)) -> Vec<serde_json::Value> {
        let mut reply = run(serde_json::json!({"phase":"plan","query":query,"now":0,"filter":filter}));
        let mut state: Option<String> = None;
        for _ in 0..3 {
            let Some(urls) = reply["fetch"].as_array().cloned() else { break };
            assert!(urls.len() <= 4);
            let responses: Vec<serde_json::Value> = urls.iter().map(|url| {
                let url = url.as_str().unwrap();
                assert!(manifest_permits(MANIFEST, url), "{url}");
                let (status, body) = body_for(url);
                serde_json::json!({"url":url,"status":status,"body":body})
            }).collect();
            reply = run(serde_json::json!({"phase":"render","query":query,"now":0,"filter":filter,"state":state,"responses":responses}));
            state = reply["state"].as_str().map(str::to_string);
            if let Some(text) = &state { assert!(text.len() <= 4096, "state is {} bytes", text.len()); }
        }
        reply["items"].as_array().cloned().expect("every run ends in rows within three rounds")
    }

    fn fixture(url: &str) -> (i64, String) {
        let slug = url.trim_start_matches(DATA).trim_end_matches(DATA_SUFFIX);
        let topics: Vec<(String, String, String)> = match slug {
            "foundations" => vec![("Color".into(), "color".into(), "Judicious use of color helps.".into()),
                                  ("Typography".into(), "typography".into(), "Legible text matters.".into())],
            "menus-and-actions" => vec![("Buttons".into(), "buttons".into(), "A button starts an action.".into()),
                                        ("Pop-up buttons".into(), "pop-up-buttons".into(), "Choose from a menu.".into())],
            other => vec![(format!("Topic in {other}"), format!("{other}-topic"), "Fictional summary.".into())],
        };
        let borrowed: Vec<(&str, &str, &str)> = topics.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str())).collect();
        (200, page(&borrowed))
    }

    #[test]
    fn every_planned_url_is_on_the_manifest_host() {
        for filter in [None, Some("all"), Some("getting-started"), Some("foundations"), Some("patterns"), Some("components"), Some("inputs"), Some("technologies"), Some("bogus")] {
            for round in 0..3 {
                let urls = round_urls(filter, round);
                assert!(urls.len() <= 4);
                for url in urls { assert!(manifest_permits(MANIFEST, &url), "{url}"); }
            }
        }
        assert_eq!(page_url("foundations"), "https://developer.apple.com/tutorials/data/design/human-interface-guidelines/foundations.json");
        assert_eq!(round_urls(Some("components"), 0).len(), 4);
        assert_eq!(round_urls(None, 2).len(), 4);
        assert!(round_urls(Some("patterns"), 1).is_empty());
    }

    #[test]
    fn all_with_nothing_typed_lists_the_sections_without_fetching() {
        let reply = run(serde_json::json!({"phase":"plan","query":"","now":0}));
        let rows = reply["items"].as_array().unwrap();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[3]["title"], "Components");
        assert_eq!(rows[3]["url"], "https://developer.apple.com/design/human-interface-guidelines/components");
        assert!(reply.get("fetch").is_none());
    }

    #[test]
    fn all_searches_twelve_pages_in_three_rounds() {
        let rows = all_rounds(None, "button", fixture);
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Buttons", "Pop-up buttons"]);
        assert_eq!(rows[0]["subtitle"], "Menus and actions · A button starts an action.");
        assert_eq!(rows[0]["url"], "https://developer.apple.com/design/human-interface-guidelines/buttons");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"Components"}]));
        let color = all_rounds(Some("all"), "legible", fixture);
        assert_eq!(color[0]["title"], "Typography", "Summaries match too");
        assert_eq!(color[0]["subtitle"], "Legible text matters.");
        assert_eq!(color[0]["badges"][0]["text"], "Foundations");
    }

    #[test]
    fn a_section_lists_its_topics_in_order() {
        let rows = all_rounds(Some("foundations"), "", fixture);
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Color", "Typography"]);
        let components = all_rounds(Some("components"), "", fixture);
        assert_eq!(components.len(), 9, "Seven fictional groups with one topic and Menus and actions with two");
        assert_eq!(components[0]["title"], "Topic in content");
        let starts = all_rounds(Some("getting-started"), "", fixture);
        assert_eq!(starts[0]["badges"][0]["text"], "Getting Started");
    }

    #[test]
    fn ranking_prefers_titles() {
        assert_eq!(rank("Buttons", "", "buttons"), Some(0));
        assert_eq!(rank("Buttons", "", "but"), Some(1));
        assert_eq!(rank("Pop-up buttons", "", "but"), Some(2));
        assert_eq!(rank("Toolbars", "", "bar"), Some(3));
        assert_eq!(rank("Sheets", "A sheet helps people focus on a task", "focus task"), Some(4));
        assert_eq!(rank("Sheets", "", "zzz"), None);
    }

    #[test]
    fn broad_searches_keep_the_state_under_four_kilobytes() {
        let long = "word ".repeat(40);
        let many: Vec<(String, String, String)> = (0..40).map(|i| (format!("Topic number {i}"), format!("topic-{i}"), long.clone())).collect();
        let body = { let b: Vec<(&str, &str, &str)> = many.iter().map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str())).collect(); page(&b) };
        let rows = all_rounds(None, "topic", |_| (200, body.clone()));
        assert!(!rows.is_empty() && rows.len() <= MAXIMUM_ROWS);
        let state = fitted_state(1, vec![], (0..200).map(|i| Found(4, i, 0, "Title".into(), "slug".into(), "x".repeat(150))).collect());
        assert!(state.len() <= 4096);
        let parsed: State = serde_json::from_str(&state).unwrap();
        assert!(parsed.found.len() > 20, "Summaries shorten before matches are dropped");
    }

    #[test]
    fn failures_and_nothing_found() {
        let down = all_rounds(Some("patterns"), "", |_| (0, String::new()));
        assert_eq!(down[0]["title"], "Apple’s design site is unavailable (HTTP 0). Try again.");
        let unreadable = all_rounds(Some("patterns"), "", |_| (200, "<html>".into()));
        assert_eq!(unreadable[0]["title"], UNREADABLE);
        let partly = all_rounds(None, "button", |url| if url.contains("status") { (503, String::new()) } else { fixture(url) });
        assert_eq!(partly.last().unwrap()["title"], "Some HIG pages did not load (HTTP 503), so this list may be incomplete.");
        let none = all_rounds(None, "qqqq", fixture);
        assert_eq!(none[0]["title"], "No HIG topic matches “qqqq”");
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
        assert!(respond(br#"{"phase":"action","query":"","now":0}"#).contains(ONLY_LISTS));
    }

    #[test]
    fn manifest_declares_the_sections_as_filters() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let ids: Vec<&str> = manifest["filters"].as_array().unwrap().iter().map(|f| f["id"].as_str().unwrap()).collect();
        let mut expected = vec![ALL];
        expected.extend(SECTIONS.iter().map(|(id, ..)| *id));
        assert_eq!(ids, expected);
        assert_eq!(manifest["hosts"], serde_json::json!(["developer.apple.com"]));
    }
}
