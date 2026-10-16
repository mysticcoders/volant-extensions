//! Hacker News for Volant: the front page, new, Ask, Show, or a search, read through Algolia's public
//! HN API on hn.algolia.com. Each row shows its points, comments and linked site as badges. Return
//! opens the story; Command-Return, or Open Comments in its Actions, opens its discussion on
//! news.ycombinator.com in the browser. Stories can be pinned or hidden, as can whole domains, and
//! opened stories show as read; all of it is saved on this Mac through Volant's extension storage.
use serde::{Deserialize, Serialize};
use volant_extension_kit::reader::{handle_change, State, Story};
use std::cell::RefCell;
use std::collections::HashMap;

use volant_extension_kit::{age, domain, encode_query, is_web_link, volant_extension_output, Item, Reply, Request, DISCUSSION_TITLE};

const API: &str = "https://hn.algolia.com/api/v1/";
const FIELDS: &str = "attributesToRetrieve=title,url,points,num_comments,author,created_at_i,objectID&attributesToHighlight=none&hitsPerPage=30";
const SEARCH: &str = "search?tags=story&query=";
const RECENT_FILTER: &str = "numericFilters=created_at_i%3E";
const RECENT_SECONDS: i64 = 3 * 86_400;
const SITE_HOST: &str = "news.ycombinator.com";
const DISCUSSION: &str = "https://news.ycombinator.com/item?id=";
const NOTHING: &str = "Hacker News returned nothing.";
const UNREADABLE: &str = "Hacker News sent a page Volant could not read.";
const SEPARATOR: &str = " · ";
const POINTS: &str = "points";
const COMMENTS: &str = "comments";
const SITE: &str = "site";

/// HN's own pages: the query words or dropdown filter ids that pick each one, its Algolia endpoint
/// and tag, and whether it is limited to the last few days so popular posts rank first.
const PAGES: &[(&[&str], &str, bool)] = &[
    (&["", "top", "front"], "search?tags=front_page", false),
    (&["new", "newest"], "search_by_date?tags=story", false),
    (&["ask"], "search?tags=ask_hn", true),
    (&["show"], "search?tags=show_hn", true),
];

#[derive(Deserialize)]
struct Page { hits: Vec<Hit> }

/// A row badge: a symbol Volant draws and a short text, shown at the row's end. Volant versions
/// without badges ignore them.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { symbol: &'static str, text: String }

impl Badge {
    fn new(symbol: &'static str, text: String) -> Badge { Badge { symbol, text } }
}

#[derive(Deserialize)]
struct Hit {
    #[serde(rename = "objectID")]
    id: String,
    title: Option<String>,
    url: Option<String>,
    points: Option<i64>,
    num_comments: Option<i64>,
    author: Option<String>,
    created_at_i: Option<i64>,
}

/// The Algolia request for a query. With a dropdown `filter` (Volant 0.3.9 and later), the filter picks
/// one of HN's pages and any text searches within it. Without one, the query itself may name a page
/// from [`PAGES`], and anything else searches all stories.
pub fn url_for(query: &str, filter: Option<&str>, now: i64) -> String {
    let query = query.trim().to_lowercase();
    if let Some((_, path, recent)) = filter.filter(|id| !id.is_empty()).and_then(|id| PAGES.iter().find(|(words, ..)| words.contains(&id))) {
        return match (query.is_empty(), recent) {
            (true, true) => format!("{API}{path}&{RECENT_FILTER}{}&{FIELDS}", now - RECENT_SECONDS),
            (true, false) => format!("{API}{path}&{FIELDS}"),
            (false, _) => format!("{API}{path}&query={}&{FIELDS}", encode_query(&query)),
        };
    }
    match PAGES.iter().find(|(words, ..)| words.contains(&query.as_str())) {
        Some((_, path, true)) => format!("{API}{path}&{RECENT_FILTER}{}&{FIELDS}", now - RECENT_SECONDS),
        Some((_, path, false)) => format!("{API}{path}&{FIELDS}"),
        None => format!("{API}{SEARCH}{}&{FIELDS}", encode_query(&query)),
    }
}

fn unavailable(status: i64) -> String {
    format!("Hacker News search is unavailable (HTTP {status}). Try again.")
}

/// The story's key: its HN item ID, from the discussion link.
fn key_of(discussion: &str) -> Option<String> {
    discussion.strip_prefix(DISCUSSION).filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).map(str::to_string)
}

/// Where a story links, except HN itself, whose text posts cannot be hidden as a domain.
fn story_domain(url: &str) -> Option<String> {
    domain(url).filter(|host| host != SITE_HOST)
}

fn handle(request: &Request) -> Result<Reply, String> {
    if let Some(change) = handle_change(request, key_of, story_domain) {
        return change;
    }
    if request.is_plan() {
        let filter = FILTER.with(|filter| filter.borrow().clone());
        return Ok(Reply::Fetch(vec![url_for(&request.query, filter.as_deref(), request.now)]));
    }
    let response = request.responses.first().ok_or(NOTHING)?;
    if response.status != 200 {
        return Err(unavailable(response.status));
    }
    let page: Page = serde_json::from_str(&response.body).map_err(|_| UNREADABLE.to_string())?;
    let stories = page.hits.into_iter().filter_map(|hit| item(hit, request.now)).filter_map(|item| {
        let key = item.secondary_url.as_deref().and_then(key_of)?;
        Some(Story { key, domain: item.url.as_deref().and_then(story_domain), item })
    }).collect();
    Ok(Reply::Items(State::load(request).arrange(stories)))
}

/// One row for a story: its age and author as the subtitle, and its points, comments and the site it
/// links to as badges. Hits without a title or with a non-numeric ID are skipped, since the ID goes
/// into the discussion link.
fn item(hit: Hit, now: i64) -> Option<Item> {
    let title = hit.title.filter(|t| !t.trim().is_empty())?;
    if hit.id.is_empty() || !hit.id.bytes().all(|b| b.is_ascii_digit()) { return None; }
    let discussion = format!("{DISCUSSION}{}", hit.id);
    let link = hit.url.filter(|u| is_web_link(u));
    let mut details = Vec::new();
    if let Some(created) = hit.created_at_i { details.push(age(now, created)); }
    if let Some(author) = hit.author { details.push(format!("by {author}")); }
    let mut badges = Vec::new();
    if let Some(points) = hit.points { badges.push(Badge::new(POINTS, points.to_string())); }
    if let Some(comments) = hit.num_comments { badges.push(Badge::new(COMMENTS, comments.to_string())); }
    badges.push(Badge::new(SITE, link.as_deref().and_then(domain).unwrap_or_else(|| SITE_HOST.to_string())));
    BADGES.with(|all| all.borrow_mut().insert(discussion.clone(), badges));
    Some(Item {
        title,
        subtitle: (!details.is_empty()).then(|| details.join(SEPARATOR)),
        url: Some(link.unwrap_or_else(|| discussion.clone())),
        secondary_url: Some(discussion),
        secondary_title: Some(DISCUSSION_TITLE.to_string()),
        ..Default::default()
    })
}

thread_local! {
    /// Each story row's badges, by discussion link, from the request being handled.
    static BADGES: RefCell<HashMap<String, Vec<Badge>>> = RefCell::new(HashMap::new());
    /// The dropdown filter Volant sent with the request being handled, if any.
    static FILTER: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The part of a request the kit's `Request` does not read: the dropdown filter.
#[derive(Deserialize)]
struct Extra { filter: Option<String> }

/// Handles a request and adds each story row's badges to the reply. The kit's rows have no badges
/// field and its requests no filter, so the filter is read here and the badges are attached to the
/// encoded rows by discussion link, which every story row has.
pub fn respond(input: &[u8]) -> String {
    BADGES.with(|all| all.borrow_mut().clear());
    let filter = serde_json::from_slice::<Extra>(input).ok().and_then(|extra| extra.filter);
    FILTER.with(|current| *current.borrow_mut() = filter);
    let output = volant_extension_kit::handle(input, handle);
    let Ok(mut reply) = serde_json::from_str::<serde_json::Value>(&output) else { return output };
    BADGES.with(|all| {
        let all = all.borrow();
        for row in reply.get_mut("items").and_then(|items| items.as_array_mut()).into_iter().flatten() {
            let found = row.get("secondaryURL").and_then(|link| link.as_str()).and_then(|link| all.get(link));
            if let Some(badges) = found.and_then(|badges| serde_json::to_value(badges).ok()) { row["badges"] = badges; }
        }
    });
    reply.to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::{handle as run, manifest_permits};

    const MANIFEST: &str = include_str!("../manifest.json");

    #[test]
    fn queries_choose_hn_pages_or_search() {
        assert!(url_for("", None, 0).contains("tags=front_page"));
        assert!(url_for("new", None, 0).starts_with("https://hn.algolia.com/api/v1/search_by_date?tags=story"));
        assert!(url_for("ask", None, 1_000_000).contains("tags=ask_hn&numericFilters=created_at_i%3E740800"));
        assert!(url_for("rust wasm", None, 0).contains("query=rust%20wasm"));
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["", "top", "new", "ask", "show", "rust wasm", "C++ & you?"] {
            for filter in [None, Some("top"), Some("new"), Some("ask"), Some("show"), Some("unknown")] {
                let url = url_for(query, filter, 1_000_000);
                assert!(manifest_permits(MANIFEST, &url), "{url}");
            }
        }
    }

    #[test]
    fn the_dropdown_filter_picks_the_page_and_text_searches_within_it() {
        assert!(url_for("", Some("new"), 0).starts_with("https://hn.algolia.com/api/v1/search_by_date?tags=story&attributes"));
        assert!(url_for("rust", Some("new"), 0).starts_with("https://hn.algolia.com/api/v1/search_by_date?tags=story&query=rust&"));
        assert!(url_for("rust", Some("ask"), 1_000_000).contains("tags=ask_hn&query=rust&"));
        assert!(!url_for("rust", Some("ask"), 1_000_000).contains("numericFilters"), "A search covers all time");
        assert!(url_for("", Some("ask"), 1_000_000).contains("numericFilters=created_at_i%3E740800"));
        assert!(url_for("new", Some("top"), 0).contains("tags=front_page&query=new"), "With a filter, page words are search text");
        assert_eq!(url_for("new", Some("unknown"), 0), url_for("new", None, 0), "An unknown filter acts like none");
        let output = respond(br#"{"phase":"plan","query":"wasm","now":0,"filter":"show"}"#);
        assert!(output.contains("tags=show_hn&query=wasm"), "{output}");
    }

    #[test]
    fn plan_asks_for_one_url_on_algolia() {
        let output = run(br#"{"phase":"plan","query":"","now":0}"#, handle);
        assert!(output.starts_with(r#"{"fetch":["https://hn.algolia.com/api/v1/search?tags=front_page"#), "{output}");
    }

    #[test]
    fn render_builds_rows_with_story_and_discussion_links() {
        let body = r#"{"hits":[
            {"objectID":"1","title":"Fictional launch","url":"https://www.example.com/post","points":42,"num_comments":7,"author":"pat","created_at_i":900},
            {"objectID":"2","title":"Ask HN: Fictional question","points":5,"num_comments":1,"author":"sam","created_at_i":0},
            {"objectID":"3","title":"","url":"https://example.com"},
            {"objectID":"4&x=y","title":"Odd ID","url":"https://example.com"}]}"#;
        let request = serde_json::json!({"phase":"render","query":"","now":4500,"responses":[{"url":"u","status":200,"body":body}]});
        let output = respond(request.to_string().as_bytes());
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["subtitle"], "1h ago · by pat");
        assert_eq!(items[0]["badges"], serde_json::json!([{"symbol":"points","text":"42"},{"symbol":"comments","text":"7"},{"symbol":"site","text":"example.com"}]));
        assert_eq!(items[0]["secondaryTitle"], "Open Comments");
        assert_eq!(items[1]["badges"][2]["text"], "news.ycombinator.com", "Text posts name HN as their site");
        assert_eq!(items[0]["url"], "https://www.example.com/post");
        assert_eq!(items[0]["secondaryURL"], "https://news.ycombinator.com/item?id=1");
        assert_eq!(items[1]["url"], "https://news.ycombinator.com/item?id=2");
    }

    #[test]
    fn saved_pins_and_hidden_stories_shape_the_list() {
        let body = r#"{"hits":[
            {"objectID":"1","title":"First","url":"https://example.com/a"},
            {"objectID":"2","title":"Second","url":"https://spam.example/b"},
            {"objectID":"3","title":"Third"}]}"#;
        let request = serde_json::json!({"phase":"render","query":"","now":0,"responses":[{"url":"u","status":200,"body":body}],
                                         "storage":{"pinned":["3"],"hiddenDomains":["spam.example"],"read":["1"]}});
        let value: serde_json::Value = serde_json::from_str(&run(request.to_string().as_bytes(), handle)).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.iter().map(|item| item["title"].as_str().unwrap()).collect::<Vec<_>>(), ["Third", "First"]);
        assert!(items[0]["subtitle"].as_str().unwrap().starts_with("Pinned"));
        assert!(items[1]["subtitle"].as_str().unwrap().starts_with("Read"));
        assert_eq!(items[0]["onOpen"], true);
        let ids: Vec<&str> = items[0]["actions"].as_array().unwrap().iter().map(|a| a["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["unpin", "hide", "show-hidden"], "HN's own text posts cannot hide news.ycombinator.com");
    }

    #[test]
    fn actions_and_opened_rows_save_state() {
        let row = serde_json::json!({"title":"First","url":"https://example.com/a","secondaryURL":"https://news.ycombinator.com/item?id=1"});
        let hide = serde_json::json!({"phase":"action","action":"hide-domain","query":"","now":0,"item":row,"storage":{"pinned":["1"]}});
        let saved: serde_json::Value = serde_json::from_str(&run(hide.to_string().as_bytes(), handle)).unwrap();
        assert_eq!(saved["storage"]["hiddenDomains"], serde_json::json!(["example.com"]));
        assert_eq!(saved["storage"]["pinned"], serde_json::json!(["1"]));
        assert!(saved.get("items").is_none());
        let opened = serde_json::json!({"phase":"opened","query":"","now":0,"item":row});
        let saved: serde_json::Value = serde_json::from_str(&run(opened.to_string().as_bytes(), handle)).unwrap();
        assert_eq!(saved["storage"]["read"], serde_json::json!(["1"]));
    }

    #[test]
    fn failures_become_a_single_explaining_row() {
        let request = r#"{"phase":"render","query":"","now":0,"responses":[{"url":"u","status":0,"body":""}]}"#;
        let output = run(request.as_bytes(), handle);
        assert!(output.contains("unavailable (HTTP 0)"), "{output}");
    }
}
