//! __NAME__ for Volant, started from `tools/new-extension.py`. It shows the text you type with its
//! character and word counts, without reading any website.
//!
//! To read a website, list its host in manifest.json's `hosts`. Volant then runs the module twice:
//! `plan`, where you return `Reply::Fetch` with the URLs to read, and `render`, which receives their
//! responses in `request.responses`. See `kit/src/lib.rs` and Volant's network extensions docs.
use volant_extension_kit::{volant_extension, Item, Reply, Request};

const PROMPT: &str = "Type some text to count it.";
const SEPARATOR: &str = " · ";

/// One row for the typed text with its counts, or a prompt when nothing is typed.
fn respond(request: &Request) -> Result<Reply, String> {
    let text = request.query.trim();
    if text.is_empty() {
        return Ok(Reply::Items(vec![Item { title: PROMPT.into(), ..Default::default() }]));
    }
    let counts = [format!("{} characters", text.chars().count()), format!("{} words", text.split_whitespace().count())];
    Ok(Reply::Items(vec![Item { title: text.into(), subtitle: Some(counts.join(SEPARATOR)), ..Default::default() }]))
}

volant_extension!(respond);

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs a request through the extension the way Volant does and returns the reply's rows.
    fn rows(request: &str) -> Vec<serde_json::Value> {
        let reply: serde_json::Value = serde_json::from_str(&volant_extension_kit::handle(request.as_bytes(), respond)).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    #[test]
    fn counts_the_typed_text() {
        let rows = rows(r#"{"phase":"render","query":" hello wide world ","now":0}"#);
        assert_eq!(rows[0]["title"], "hello wide world");
        assert_eq!(rows[0]["subtitle"], "16 characters · 3 words");
    }

    #[test]
    fn asks_for_text_when_nothing_is_typed() {
        assert_eq!(rows(r#"{"phase":"render","query":"","now":0}"#)[0]["title"], PROMPT);
    }
}
