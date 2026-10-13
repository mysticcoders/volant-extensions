//! Lobsters for Volant: hottest, newest, or one tag, read from lobste.rs's public JSON pages.
//! Return opens the story; Command-Return opens its discussion on lobste.rs in the browser. Stories can
//! be pinned or hidden, as can whole domains, and opened stories show as read; all of it is saved on
//! this Mac through Volant's extension storage.
use serde::Deserialize;
use volant_extension_kit::reader::{handle_change, State, Story};
use volant_extension_kit::{age, domain, is_web_link, parse_rfc3339, volant_extension, Item, Reply, Request, DISCUSSION_TITLE};

const SITE: &str = "https://lobste.rs/";
const SITE_HOST: &str = "lobste.rs";
const HOTTEST: &str = "hottest.json";
const NEWEST: &str = "newest.json";
const TAG_PREFIX: &str = "t/";
const TAG_SUFFIX: &str = ".json";
const TAG_WORD: &str = "t";
const HOTTEST_WORDS: &[&str] = &["hot", "hottest"];
const NEWEST_WORDS: &[&str] = &["new", "newest"];
const MAX_TAG_LENGTH: usize = 25;
const USAGE: &str = "Type lobsters, lobsters new, or a tag such as lobsters rust.";
const NOTHING: &str = "Lobsters returned nothing.";
const NO_SUCH_TAG: &str = "Lobsters has no tag by that name.";
const UNREADABLE: &str = "Lobsters sent a page Volant could not read.";
const SEPARATOR: &str = " · ";
const TAG_SEPARATOR: &str = " ";
const STORY_PATH: &str = "https://lobste.rs/s/";

#[derive(Deserialize)]
struct Post {
    title: Option<String>,
    url: Option<String>,
    score: Option<i64>,
    comment_count: Option<i64>,
    created_at: Option<String>,
    submitter_user: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    comments_url: Option<String>,
    short_id_url: Option<String>,
}

/// Whether a word can be a Lobsters tag, so it is safe to put in the page path.
fn is_tag(word: &str) -> bool {
    word.len() <= MAX_TAG_LENGTH && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The page for a query: nothing or a [`HOTTEST_WORDS`] word is hottest, a [`NEWEST_WORDS`] word is
/// newest, and a single tag word such as "rust" (optionally "t rust") is that tag's page.
pub fn url_for(query: &str) -> Result<String, String> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] => Ok(format!("{SITE}{HOTTEST}")),
        [word] if HOTTEST_WORDS.contains(word) => Ok(format!("{SITE}{HOTTEST}")),
        [word] if NEWEST_WORDS.contains(word) => Ok(format!("{SITE}{NEWEST}")),
        [tag] if is_tag(tag) => Ok(format!("{SITE}{TAG_PREFIX}{tag}{TAG_SUFFIX}")),
        [prefix, tag] if *prefix == TAG_WORD && is_tag(tag) => Ok(format!("{SITE}{TAG_PREFIX}{tag}{TAG_SUFFIX}")),
        _ => Err(USAGE.to_string()),
    }
}

fn unavailable(status: i64) -> String {
    format!("Lobsters is unavailable (HTTP {status}). Try again.")
}

/// The story's key: its Lobsters short ID, from a discussion link such as https://lobste.rs/s/abc123/title.
fn key_of(discussion: &str) -> Option<String> {
    let id = discussion.strip_prefix(STORY_PATH)?.split('/').next()?;
    (!id.is_empty() && id.len() <= 12 && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_string())
}

/// Where a story links, except lobste.rs itself, whose text posts cannot be hidden as a domain.
fn story_domain(url: &str) -> Option<String> {
    domain(url).filter(|host| host != SITE_HOST)
}

fn handle(request: &Request) -> Result<Reply, String> {
    if let Some(change) = handle_change(request, key_of, story_domain) {
        return change;
    }
    if request.is_plan() {
        return Ok(Reply::Fetch(vec![url_for(&request.query)?]));
    }
    let response = request.responses.first().ok_or(NOTHING)?;
    match response.status {
        200 => {}
        404 => return Err(NO_SUCH_TAG.to_string()),
        status => return Err(unavailable(status)),
    }
    let stories: Vec<Post> = serde_json::from_str(&response.body).map_err(|_| UNREADABLE.to_string())?;
    let stories = stories.into_iter().filter_map(|story| item(story, request.now)).filter_map(|item| {
        let key = item.secondary_url.as_deref().and_then(key_of)?;
        Some(Story { key, domain: item.url.as_deref().and_then(story_domain), item })
    }).collect();
    Ok(Reply::Items(State::load(request).arrange(stories)))
}

/// One row for a story; stories without a title or a discussion link on lobste.rs are skipped.
fn item(story: Post, now: i64) -> Option<Item> {
    let title = story.title.filter(|t| !t.trim().is_empty())?;
    let discussion = story.comments_url.or(story.short_id_url).filter(|u| u.starts_with(SITE))?;
    let link = story.url.filter(|u| is_web_link(u));
    let mut details = vec![link.as_deref().and_then(domain).unwrap_or_else(|| SITE_HOST.to_string())];
    if let Some(score) = story.score { details.push(format!("{score} points")); }
    if let Some(comments) = story.comment_count { details.push(format!("{comments} comments")); }
    if let Some(created) = story.created_at.as_deref().and_then(parse_rfc3339) { details.push(age(now, created)); }
    if let Some(user) = story.submitter_user { details.push(format!("by {user}")); }
    if !story.tags.is_empty() { details.push(story.tags.join(TAG_SEPARATOR)); }
    Some(Item {
        title,
        subtitle: Some(details.join(SEPARATOR)),
        url: Some(link.unwrap_or_else(|| discussion.clone())),
        secondary_url: Some(discussion),
        secondary_title: Some(DISCUSSION_TITLE.to_string()),
        ..Default::default()
    })
}

volant_extension!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::{handle as run, manifest_permits};

    const MANIFEST: &str = include_str!("../manifest.json");

    #[test]
    fn queries_choose_hottest_newest_or_a_tag() {
        assert_eq!(url_for("").unwrap(), "https://lobste.rs/hottest.json");
        assert_eq!(url_for("New").unwrap(), "https://lobste.rs/newest.json");
        assert_eq!(url_for("rust").unwrap(), "https://lobste.rs/t/rust.json");
        assert_eq!(url_for("t web-dev").unwrap(), "https://lobste.rs/t/web-dev.json");
        assert!(url_for("../etc").is_err());
        assert!(url_for("two words here").is_err());
        assert!(url_for("x rust").is_err());
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["", "hot", "new", "rust", "t web-dev"] {
            let url = url_for(query).unwrap();
            assert!(manifest_permits(MANIFEST, &url), "{url}");
        }
    }

    #[test]
    fn a_bad_query_explains_itself_instead_of_fetching() {
        let output = run(br#"{"phase":"plan","query":"two words here","now":0}"#, handle);
        assert!(output.starts_with(r#"{"items":[{"title":"Type lobsters"#), "{output}");
    }

    #[test]
    fn render_builds_rows_with_story_and_discussion_links() {
        let body = r#"[
            {"title":"Fictional post","url":"https://www.example.org/a","score":12,"comment_count":3,"created_at":"1970-01-01T01:00:00Z","submitter_user":"lee","tags":["rust","wasm"],"comments_url":"https://lobste.rs/s/abc/fictional_post"},
            {"title":"Fictional text post","url":"","score":2,"comment_count":0,"created_at":"1970-01-01T00:00:00Z","submitter_user":"kim","tags":[],"comments_url":"https://lobste.rs/s/def/text"},
            {"title":"Elsewhere","url":"https://example.org/b","comments_url":"https://example.org/comments"}]"#;
        let request = serde_json::json!({"phase":"render","query":"","now":7200,"responses":[{"url":"u","status":200,"body":body}]});
        let value: serde_json::Value = serde_json::from_str(&run(request.to_string().as_bytes(), handle)).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["subtitle"], "example.org · 12 points · 3 comments · 1h ago · by lee · rust wasm");
        assert_eq!(items[0]["secondaryURL"], "https://lobste.rs/s/abc/fictional_post");
        assert_eq!(items[1]["url"], "https://lobste.rs/s/def/text");
    }

    #[test]
    fn saved_state_shapes_the_list_and_actions_save() {
        let body = r#"[
            {"title":"Kept","url":"https://example.org/a","comments_url":"https://lobste.rs/s/abc123/kept"},
            {"title":"Hidden","url":"https://example.org/b","comments_url":"https://lobste.rs/s/def456/hidden"}]"#;
        let request = serde_json::json!({"phase":"render","query":"","now":0,"responses":[{"url":"u","status":200,"body":body}],
                                         "storage":{"hidden":["def456"]}});
        let value: serde_json::Value = serde_json::from_str(&run(request.to_string().as_bytes(), handle)).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["actions"][2]["title"], "Hide Stories from example.org");
        let row = serde_json::json!({"title":"Kept","url":"https://example.org/a","secondaryURL":"https://lobste.rs/s/abc123/kept"});
        let pin = serde_json::json!({"phase":"action","action":"pin","query":"","now":0,"item":row,"storage":{"hidden":["def456"]}});
        let saved: serde_json::Value = serde_json::from_str(&run(pin.to_string().as_bytes(), handle)).unwrap();
        assert_eq!(saved["storage"]["pinned"], serde_json::json!(["abc123"]));
        assert_eq!(saved["storage"]["hidden"], serde_json::json!(["def456"]));
        assert_eq!(key_of("https://lobste.rs/s/../etc"), None);
    }

    #[test]
    fn an_unknown_tag_says_so() {
        let request = r#"{"phase":"render","query":"nosuchtag","now":0,"responses":[{"url":"u","status":404,"body":""}]}"#;
        assert!(run(request.as_bytes(), handle).contains("no tag by that name"));
    }
}
