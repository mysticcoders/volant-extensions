//! Hacker News for Volant: the front page, new, Ask, Show, or a search, read through Algolia's public
//! HN API on hn.algolia.com. Return opens the story; Command-Return opens its discussion on
//! news.ycombinator.com in the browser.
use serde::Deserialize;
use volant_extension_kit::{age, domain, encode_query, is_web_link, volant_extension, Item, Reply, Request, DISCUSSION_TITLE};

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

/// HN's own pages: the query words that pick each one, its Algolia endpoint and tag, and whether it
/// is limited to the last few days so popular posts rank first.
const PAGES: &[(&[&str], &str, bool)] = &[
    (&["", "top", "front"], "search?tags=front_page", false),
    (&["new", "newest"], "search_by_date?tags=story", false),
    (&["ask"], "search?tags=ask_hn", true),
    (&["show"], "search?tags=show_hn", true),
];

#[derive(Deserialize)]
struct Page { hits: Vec<Hit> }

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

/// The Algolia request for a query: one of HN's pages from [`PAGES`], or a story search for
/// anything else.
pub fn url_for(query: &str, now: i64) -> String {
    let query = query.trim().to_lowercase();
    match PAGES.iter().find(|(words, ..)| words.contains(&query.as_str())) {
        Some((_, path, true)) => format!("{API}{path}&{RECENT_FILTER}{}&{FIELDS}", now - RECENT_SECONDS),
        Some((_, path, false)) => format!("{API}{path}&{FIELDS}"),
        None => format!("{API}{SEARCH}{}&{FIELDS}", encode_query(&query)),
    }
}

fn unavailable(status: i64) -> String {
    format!("Hacker News search is unavailable (HTTP {status}). Try again.")
}

fn handle(request: &Request) -> Result<Reply, String> {
    if request.is_plan() {
        return Ok(Reply::Fetch(vec![url_for(&request.query, request.now)]));
    }
    let response = request.responses.first().ok_or(NOTHING)?;
    if response.status != 200 {
        return Err(unavailable(response.status));
    }
    let page: Page = serde_json::from_str(&response.body).map_err(|_| UNREADABLE.to_string())?;
    Ok(Reply::Items(page.hits.into_iter().filter_map(|hit| item(hit, request.now)).collect()))
}

/// One row for a story; hits without a title or with a non-numeric ID are skipped, since the ID
/// goes into the discussion link.
fn item(hit: Hit, now: i64) -> Option<Item> {
    let title = hit.title.filter(|t| !t.trim().is_empty())?;
    if hit.id.is_empty() || !hit.id.bytes().all(|b| b.is_ascii_digit()) { return None; }
    let discussion = format!("{DISCUSSION}{}", hit.id);
    let link = hit.url.filter(|u| is_web_link(u));
    let mut details = vec![link.as_deref().and_then(domain).unwrap_or_else(|| SITE_HOST.to_string())];
    if let Some(points) = hit.points { details.push(format!("{points} points")); }
    if let Some(comments) = hit.num_comments { details.push(format!("{comments} comments")); }
    if let Some(created) = hit.created_at_i { details.push(age(now, created)); }
    if let Some(author) = hit.author { details.push(format!("by {author}")); }
    Some(Item {
        title,
        subtitle: Some(details.join(SEPARATOR)),
        url: Some(link.unwrap_or_else(|| discussion.clone())),
        secondary_url: Some(discussion),
        secondary_title: Some(DISCUSSION_TITLE.to_string()),
    })
}

volant_extension!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::{handle as run, manifest_permits};

    const MANIFEST: &str = include_str!("../manifest.json");

    #[test]
    fn queries_choose_hn_pages_or_search() {
        assert!(url_for("", 0).contains("tags=front_page"));
        assert!(url_for("new", 0).starts_with("https://hn.algolia.com/api/v1/search_by_date?tags=story"));
        assert!(url_for("ask", 1_000_000).contains("tags=ask_hn&numericFilters=created_at_i%3E740800"));
        assert!(url_for("rust wasm", 0).contains("query=rust%20wasm"));
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["", "top", "new", "ask", "show", "rust wasm", "C++ & you?"] {
            let url = url_for(query, 1_000_000);
            assert!(manifest_permits(MANIFEST, &url), "{url}");
        }
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
        let output = run(request.to_string().as_bytes(), handle);
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["subtitle"], "example.com · 42 points · 7 comments · 1h ago · by pat");
        assert_eq!(items[0]["url"], "https://www.example.com/post");
        assert_eq!(items[0]["secondaryURL"], "https://news.ycombinator.com/item?id=1");
        assert_eq!(items[1]["url"], "https://news.ycombinator.com/item?id=2");
    }

    #[test]
    fn failures_become_a_single_explaining_row() {
        let request = r#"{"phase":"render","query":"","now":0,"responses":[{"url":"u","status":0,"body":""}]}"#;
        let output = run(request.as_bytes(), handle);
        assert!(output.contains("unavailable (HTTP 0)"), "{output}");
    }
}
