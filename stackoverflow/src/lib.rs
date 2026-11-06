//! Stack Overflow for Volant: questions matching the typed text, by relevance, from the Stack Exchange
//! API's advanced search on api.stackexchange.com. Each row shows the question's title, its tags and
//! age, and badges for its score, its answer count and whether it has an accepted or upvoted answer.
//! Return opens the question on stackoverflow.com.
//!
//! The API is used without a key, which allows 300 requests a day from one network; when it runs out
//! or asks for a pause, the list says so in one row. Its replies are gzip, which Volant's fetcher
//! decompresses, and its titles carry HTML entities, which are decoded here.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{age, encode_query, volant_extension_output};

const SEARCH: &str = "https://api.stackexchange.com/2.3/search/advanced?order=desc&sort=relevance&site=stackoverflow&pagesize=20&q=";
const SITE: &str = "https://stackoverflow.com/";
const POINTS: &str = "points";
const COMMENTS: &str = "comments";
const ANSWERED: &str = "Answered";
const TAG_SEPARATOR: &str = ", ";
const SEPARATOR: &str = " · ";
const ASKED_PREFIX: &str = "asked ";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const OK: i64 = 200;
const THROTTLED: &str = "throttle_violation";
const BAD_PARAMETER: &str = "bad_parameter";
const TEMPORARILY_UNAVAILABLE: &str = "temporarily_unavailable";
const WAIT_MARKER: &str = "available in ";
const SECONDS_WORD: &str = " seconds";
const HOUR: i64 = 3_600;
const MINUTE: i64 = 60;
const LOW_QUOTA: i64 = 10;
const PROMPT: &str = "Type a question to search Stack Overflow.";
const NOTHING: &str = "Stack Overflow returned nothing.";
const NO_QUESTIONS: &str = "No questions match that search.";
const UNREADABLE: &str = "Stack Overflow sent a reply Volant could not read.";
const LIMIT_REACHED: &str = "Stack Exchange's daily limit for this network was reached.";
const TRY_LATER: &str = " Try again later.";
const TRY_IN_PREFIX: &str = " Try again in about ";
const HOURS_SUFFIX: &str = " hours.";
const MINUTES_SUFFIX: &str = " minutes.";
const NOT_UNDERSTOOD_SEARCH: &str = "Stack Overflow could not understand that search.";
const PAUSED: &str = "Stack Exchange's API is temporarily unavailable. Try again in a minute.";
const UNAVAILABLE_PREFIX: &str = "Stack Overflow is unavailable (HTTP ";
const UNAVAILABLE_SUFFIX: &str = "). Try again.";
const BACKOFF_PREFIX: &str = "Stack Exchange asked for a ";
const BACKOFF_SUFFIX: &str = "-second pause before the next search.";
const QUOTA_SUFFIX: &str = " Stack Exchange searches left today on this network.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";

/// HTML's named entities that appear in Stack Exchange titles, with what they stand for.
const ENTITIES: &[(&str, char)] = &[("amp", '&'), ("lt", '<'), ("gt", '>'), ("quot", '"'), ("apos", '\''), ("nbsp", ' ')];
const LONGEST_ENTITY: usize = 10;

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    now: i64,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response { status: i64, body: String }

/// A search reply: the questions, plus any pause the API asks for and the day's remaining quota.
#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    items: Vec<Question>,
    #[serde(default)]
    backoff: Option<i64>,
    #[serde(default)]
    quota_remaining: Option<i64>,
}

/// The error body Stack Exchange sends with a failed request.
#[derive(Deserialize)]
struct ApiError {
    #[serde(default)]
    error_name: String,
    #[serde(default)]
    error_message: String,
}

#[derive(Deserialize)]
struct Question {
    title: String,
    link: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    answer_count: i64,
    #[serde(default)]
    is_answered: bool,
    #[serde(default)]
    accepted_answer_id: Option<i64>,
    #[serde(default)]
    creation_date: Option<i64>,
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

/// One explaining row, for a prompt, an empty result or a failure.
fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The advanced search for the typed text.
pub fn url_for(query: &str) -> String {
    format!("{SEARCH}{}", encode_query(query.trim()))
}

/// Decodes the HTML entities in a Stack Exchange title: named ones, `&#39;` and `&#x27;`. Anything
/// that is not a known entity is left as it is.
pub fn decode_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let entity = after.find(';').filter(|end| *end <= LONGEST_ENTITY).map(|end| (&after[..end], end));
        match entity.and_then(|(name, end)| entity_char(name).map(|character| (character, end))) {
            Some((character, end)) => { decoded.push(character); rest = &after[end + 1..]; }
            None => { decoded.push('&'); rest = after; }
        }
    }
    decoded.push_str(rest);
    decoded
}

/// The character an entity's name (between `&` and `;`) stands for.
fn entity_char(name: &str) -> Option<char> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse::<u32>().ok()?,
        };
        return char::from_u32(code);
    }
    ENTITIES.iter().find(|(entity, _)| *entity == name).map(|(_, character)| *character)
}

/// How long until the limit resets, from a message such as "more requests available in 82626 seconds".
fn wait_hint(error_message: &str) -> String {
    let seconds = error_message.find(WAIT_MARKER)
        .and_then(|start| error_message[start + WAIT_MARKER.len()..].split(SECONDS_WORD).next())
        .and_then(|number| number.trim().parse::<i64>().ok());
    match seconds {
        Some(seconds) if seconds >= HOUR => format!("{TRY_IN_PREFIX}{}{HOURS_SUFFIX}", (seconds + HOUR - 1) / HOUR),
        Some(seconds) if seconds > 0 => format!("{TRY_IN_PREFIX}{}{MINUTES_SUFFIX}", (seconds + MINUTE - 1) / MINUTE),
        _ => TRY_LATER.to_string(),
    }
}

/// What a failed search says, from Stack Exchange's error body when it sent one.
fn failure(status: i64, body: &str) -> String {
    let error = serde_json::from_str::<ApiError>(body).unwrap_or(ApiError { error_name: String::new(), error_message: String::new() });
    match error.error_name.as_str() {
        THROTTLED => format!("{LIMIT_REACHED}{}", wait_hint(&error.error_message)),
        BAD_PARAMETER => NOT_UNDERSTOOD_SEARCH.to_string(),
        TEMPORARILY_UNAVAILABLE => PAUSED.to_string(),
        _ => format!("{UNAVAILABLE_PREFIX}{status}{UNAVAILABLE_SUFFIX}"),
    }
}

/// One row for a question: its decoded title, its tags and age, and badges for its score, answers
/// and whether it is answered. Questions not on stackoverflow.com are skipped.
fn row(question: Question, now: i64) -> Option<Row> {
    let title = decode_entities(question.title.trim());
    if title.is_empty() || !question.link.starts_with(SITE) { return None; }
    let mut details = Vec::new();
    let tags: Vec<String> = question.tags.iter().map(|tag| decode_entities(tag)).collect();
    if !tags.is_empty() { details.push(tags.join(TAG_SEPARATOR)); }
    if let Some(created) = question.creation_date { details.push(format!("{ASKED_PREFIX}{}", age(now, created))); }
    let mut badges = vec![
        Badge { symbol: Some(POINTS), text: question.score.to_string() },
        Badge { symbol: Some(COMMENTS), text: question.answer_count.to_string() },
    ];
    if question.is_answered || question.accepted_answer_id.is_some() { badges.push(Badge { symbol: None, text: ANSWERED.to_string() }); }
    Some(Row {
        title,
        subtitle: (!details.is_empty()).then(|| details.join(SEPARATOR)),
        url: Some(question.link),
        badges,
    })
}

/// A short notice when Stack Exchange asks for a pause or the day's quota runs low.
fn notice(page: &Page) -> Option<String> {
    if let Some(seconds) = page.backoff.filter(|seconds| *seconds > 0) {
        return Some(format!("{BACKOFF_PREFIX}{seconds}{BACKOFF_SUFFIX}"));
    }
    page.quota_remaining.filter(|left| *left <= LOW_QUOTA).map(|left| format!("{left}{QUOTA_SUFFIX}"))
}

/// The rows for a render and any notice to show with them.
fn render(request: &Request) -> (Vec<Row>, Option<String>) {
    let Some(response) = request.responses.first() else { return (vec![message(NOTHING)], None) };
    if response.status != OK { return (vec![message(&failure(response.status, &response.body))], None); }
    let Ok(page) = serde_json::from_str::<Page>(&response.body) else { return (vec![message(UNREADABLE)], None) };
    let notice = notice(&page);
    let rows: Vec<Row> = page.items.into_iter().filter_map(|question| row(question, request.now)).collect();
    if rows.is_empty() { return (vec![message(NO_QUESTIONS)], notice); }
    (rows, notice)
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)], None) };
    match request.phase.as_str() {
        PLAN if request.query.trim().is_empty() => items(vec![message(PROMPT)], None),
        PLAN => serde_json::json!({ "fetch": [url_for(&request.query)] }).to_string(),
        RENDER => { let (rows, notice) = render(&request); items(rows, notice) }
        _ => items(vec![message(ONLY_LISTS)], None),
    }
}

fn items(rows: Vec<Row>, notice: Option<String>) -> String {
    match notice {
        Some(notice) => serde_json::json!({ "items": rows, "notice": notice }).to_string(),
        None => serde_json::json!({ "items": rows }).to_string(),
    }
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::manifest_permits;

    const MANIFEST: &str = include_str!("../manifest.json");
    const NOW: i64 = 1_793_966_400;

    /// Trimmed, fictional questions in the shape the Stack Exchange API sends.
    const PAGE: &str = r#"{"items":[
        {"tags":["rust","lifetime"],"owner":{"display_name":"pat","link":"https://stackoverflow.com/users/1/pat"},
         "is_answered":true,"view_count":38219,"accepted_answer_id":31612025,"answer_count":12,"score":283,
         "last_activity_date":1767412652,"creation_date":1437736506,"question_id":31609137,"content_license":"CC BY-SA 4.0",
         "link":"https://stackoverflow.com/questions/31609137/fictional-lifetimes","title":"Why does &quot;&amp;&#39;a T&quot; need a lifetime?"},
        {"tags":["c++"],"is_answered":false,"answer_count":0,"score":-2,"creation_date":1793962800,
         "link":"https://stackoverflow.com/questions/2/unanswered","title":"Unanswered &lt;T&gt; question"},
        {"tags":[],"is_answered":false,"answer_count":0,"score":0,"link":"https://example.com/q/3","title":"Elsewhere"}],
        "has_more":true,"quota_max":300,"quota_remaining":298}"#;

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render_body(status: i64, body: &str) -> serde_json::Value {
        reply(serde_json::json!({"phase":"render","query":"rust","now":NOW,"responses":[{"url":"u","status":status,"body":body}]}))
    }

    #[test]
    fn searches_with_the_encoded_text() {
        assert_eq!(url_for(" rust lifetime "),
            "https://api.stackexchange.com/2.3/search/advanced?order=desc&sort=relevance&site=stackoverflow&pagesize=20&q=rust%20lifetime");
        assert!(url_for("C# & .NET?").ends_with("&q=C%23%20%26%20.NET%3F"));
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["rust", "C# & .NET?", "[python] is:question", "日本語", "a/b#c"] {
            let plan = reply(serde_json::json!({"phase":"plan","query":query,"now":0}));
            let fetch = plan["fetch"].as_array().unwrap();
            assert_eq!(fetch.len(), 1);
            assert!(manifest_permits(MANIFEST, fetch[0].as_str().unwrap()), "{query}");
        }
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["hosts"], serde_json::json!(["api.stackexchange.com"]));
        assert_eq!(manifest["mode"], "search");
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":" ","now":0}))["items"][0]["title"], PROMPT);
    }

    #[test]
    fn entities_decode_and_unknown_ones_stay() {
        assert_eq!(decode_entities("&quot;&amp;&#39;a T&quot; &lt;T&gt; &#x27;x&#X27;"), "\"&'a T\" <T> 'x'");
        assert_eq!(decode_entities("AT&T & co &bogus; &#xZZ; &"), "AT&T & co &bogus; &#xZZ; &");
        assert_eq!(decode_entities("caf&#233;"), "café");
    }

    #[test]
    fn render_builds_question_rows_with_badges() {
        let value = render_body(200, PAGE);
        assert!(value.get("notice").is_none());
        let rows = value["items"].as_array().unwrap();
        assert_eq!(rows.len(), 2, "Links off stackoverflow.com are skipped");
        assert_eq!(rows[0]["title"], "Why does \"&'a T\" need a lifetime?");
        assert_eq!(rows[0]["subtitle"], "rust, lifetime · asked 11y ago");
        assert_eq!(rows[0]["url"], "https://stackoverflow.com/questions/31609137/fictional-lifetimes");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"symbol":"points","text":"283"},{"symbol":"comments","text":"12"},{"text":"Answered"}]));
        assert_eq!(rows[1]["title"], "Unanswered <T> question");
        assert_eq!(rows[1]["subtitle"], "c++ · asked 1h ago");
        assert_eq!(rows[1]["badges"], serde_json::json!([{"symbol":"points","text":"-2"},{"symbol":"comments","text":"0"}]));
    }

    #[test]
    fn quota_and_backoff_explain_themselves() {
        let throttled = r#"{"error_id":502,"error_message":"too many requests from this IP, more requests available in 82626 seconds","error_name":"throttle_violation"}"#;
        assert_eq!(render_body(400, throttled)["items"][0]["title"], "Stack Exchange's daily limit for this network was reached. Try again in about 23 hours.");
        let soon = r#"{"error_id":502,"error_message":"more requests available in 90 seconds","error_name":"throttle_violation"}"#;
        assert_eq!(render_body(400, soon)["items"][0]["title"], "Stack Exchange's daily limit for this network was reached. Try again in about 2 minutes.");
        let vague = r#"{"error_id":502,"error_message":"slow down","error_name":"throttle_violation"}"#;
        assert_eq!(render_body(400, vague)["items"][0]["title"], "Stack Exchange's daily limit for this network was reached. Try again later.");
        assert_eq!(render_body(400, r#"{"error_id":400,"error_name":"bad_parameter","error_message":"x"}"#)["items"][0]["title"], NOT_UNDERSTOOD_SEARCH);
        assert_eq!(render_body(503, r#"{"error_id":503,"error_name":"temporarily_unavailable","error_message":"x"}"#)["items"][0]["title"], PAUSED);
        assert_eq!(render_body(0, "")["items"][0]["title"], "Stack Overflow is unavailable (HTTP 0). Try again.");
        let backoff = render_body(200, r#"{"items":[{"title":"A","link":"https://stackoverflow.com/questions/1/a"}],"backoff":10,"quota_remaining":200}"#);
        assert_eq!(backoff["notice"], "Stack Exchange asked for a 10-second pause before the next search.");
        assert_eq!(backoff["items"][0]["title"], "A", "Rows still show with a backoff");
        let low = render_body(200, r#"{"items":[],"quota_remaining":3}"#);
        assert_eq!(low["notice"], "3 Stack Exchange searches left today on this network.");
        assert_eq!(low["items"][0]["title"], NO_QUESTIONS);
        assert_eq!(render_body(200, "<html>")["items"][0]["title"], UNREADABLE);
        assert_eq!(reply(serde_json::json!({"phase":"render","query":"x","now":0,"responses":[]}))["items"][0]["title"], NOTHING);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
