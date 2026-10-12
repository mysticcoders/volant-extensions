//! Lobsters for Volant: hottest, newest, or one tag, read from lobste.rs's public JSON pages.
//! Return opens the story; Command-Return opens its discussion on lobste.rs in the browser.
use serde::Deserialize;
use volant_extension_kit::{age, domain, parse_rfc3339, volant_extension, Item, Reply, Request};

#[derive(Deserialize)]
struct Story {
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

/// The page for a query: "" or "hot" is hottest, "new" is newest, and a single tag word such as
/// "rust" (optionally "t rust") is that tag's page.
pub fn url_for(query: &str) -> Result<String, String> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["hot"] | ["hottest"] => Ok("https://lobste.rs/hottest.json".to_string()),
        ["new"] | ["newest"] => Ok("https://lobste.rs/newest.json".to_string()),
        [tag] | ["t", tag] if tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') && tag.len() <= 25 =>
            Ok(format!("https://lobste.rs/t/{tag}.json")),
        _ => Err("Type lobsters, lobsters new, or a tag such as lobsters rust.".to_string()),
    }
}

fn handle(request: &Request) -> Result<Reply, String> {
    if request.phase == "plan" {
        return Ok(Reply::Fetch(vec![url_for(&request.query)?]));
    }
    let response = request.responses.first().ok_or("Lobsters returned nothing.")?;
    match response.status {
        200 => {}
        404 => return Err("Lobsters has no tag by that name.".to_string()),
        status => return Err(format!("Lobsters is unavailable (HTTP {status}). Try again.")),
    }
    let stories: Vec<Story> = serde_json::from_str(&response.body).map_err(|_| "Lobsters sent a page Volant could not read.".to_string())?;
    Ok(Reply::Items(stories.into_iter().filter_map(|story| item(story, request.now)).collect()))
}

fn item(story: Story, now: i64) -> Option<Item> {
    let title = story.title.filter(|t| !t.trim().is_empty())?;
    let discussion = story.comments_url.or(story.short_id_url)?;
    let link = story.url.filter(|u| u.starts_with("http://") || u.starts_with("https://"));
    let mut details = vec![link.as_deref().and_then(domain).unwrap_or_else(|| "lobste.rs".to_string())];
    if let Some(score) = story.score { details.push(format!("{score} points")); }
    if let Some(comments) = story.comment_count { details.push(format!("{comments} comments")); }
    if let Some(created) = story.created_at.as_deref().and_then(parse_rfc3339) { details.push(age(now, created)); }
    if let Some(user) = story.submitter_user { details.push(format!("by {user}")); }
    if !story.tags.is_empty() { details.push(story.tags.join(" ")); }
    Some(Item {
        title,
        subtitle: Some(details.join(" · ")),
        url: Some(link.unwrap_or_else(|| discussion.clone())),
        secondary_url: Some(discussion),
        secondary_title: Some("Open Comments".to_string()),
    })
}

volant_extension!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::handle as run;

    #[test]
    fn queries_choose_hottest_newest_or_a_tag() {
        assert_eq!(url_for("").unwrap(), "https://lobste.rs/hottest.json");
        assert_eq!(url_for("New").unwrap(), "https://lobste.rs/newest.json");
        assert_eq!(url_for("rust").unwrap(), "https://lobste.rs/t/rust.json");
        assert_eq!(url_for("t web-dev").unwrap(), "https://lobste.rs/t/web-dev.json");
        assert!(url_for("../etc").is_err());
        assert!(url_for("two words here").is_err());
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
            {"title":"Fictional text post","url":"","score":2,"comment_count":0,"created_at":"1970-01-01T00:00:00Z","submitter_user":"kim","tags":[],"comments_url":"https://lobste.rs/s/def/text"}]"#;
        let request = serde_json::json!({"phase":"render","query":"","now":7200,"responses":[{"url":"u","status":200,"body":body}]});
        let value: serde_json::Value = serde_json::from_str(&run(request.to_string().as_bytes(), handle)).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["subtitle"], "example.org · 12 points · 3 comments · 1h ago · by lee · rust wasm");
        assert_eq!(items[0]["secondaryURL"], "https://lobste.rs/s/abc/fictional_post");
        assert_eq!(items[1]["url"], "https://lobste.rs/s/def/text");
    }

    #[test]
    fn an_unknown_tag_says_so() {
        let request = r#"{"phase":"render","query":"nosuchtag","now":0,"responses":[{"url":"u","status":404,"body":""}]}"#;
        assert!(run(request.as_bytes(), handle).contains("no tag by that name"));
    }
}
