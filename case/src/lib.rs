//! Case Converter for Volant: shows the text you type, or the clipboard's text when the field is empty,
//! in ten cases, from camelCase to dot.case. It works offline: the manifest lists no hosts, so Volant
//! sends one `render` request and shows the rows. Each row's title is the converted text, so Return
//! copies it, and its badge names the case.
//!
//! The programming cases (camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case and
//! dot.case) split the text into words at anything that is not a letter or digit, at a lowercase letter
//! followed by an uppercase one (`fooBar`), before the last capital of an acronym followed by a
//! lowercase letter (`HTTPServer` is `http server`) and between letters and digits (`utf8` is `utf 8`).
//! Apostrophes inside a word are dropped (`don't` is `dont`).
//!
//! Title Case, Sentence case, lower case and UPPER CASE keep prose as written, punctuation and line
//! breaks included, when the text has spaces; a single identifier such as `fooBar` is split into words
//! first, so its Title Case is `Foo Bar`.
//!
//! Volant shows at most 300 characters of a row's title and copies what it shows, so a result longer
//! than that gets a row saying so, with its length, instead of a cut-off copy. Length is counted in
//! Unicode scalar values, which is never less than what Volant counts.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const RENDER: &str = "render";
const PROMPT: &str = "Type or copy some text to change its case.";
const PROMPT_DETAIL: &str = "With the field empty, Case Converter reads the clipboard's text when it runs";
const NO_WORDS: &str = "There are no letters or digits to convert.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const FROM_CLIPBOARD: &str = "From clipboard";
const FROM_TYPED: &str = "From typed text";
const TOO_LONG_START: &str = "Too long to copy from a row (";
const TOO_LONG_END: &str = " characters)";
const TOO_LONG_HINT: &str = "Volant rows copy at most 300 characters";
const SEPARATOR: &str = " · ";
const THOUSANDS: &str = ",";
const SPACE: &str = " ";
const UNDERSCORE: &str = "_";
const HYPHEN: &str = "-";
const DOT: &str = ".";
const APOSTROPHES: [char; 2] = ['\'', '’'];
const SENTENCE_ENDS: [char; 4] = ['.', '!', '?', '…'];
const MAXIMUM_TITLE: usize = 300;

/// The ten cases, in the order the rows show them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style { Camel, Pascal, Snake, ScreamingSnake, Kebab, Title, Sentence, Lower, Upper, Dot }

impl Style {
    pub const ALL: [Style; 10] = [Style::Camel, Style::Pascal, Style::Snake, Style::ScreamingSnake, Style::Kebab,
        Style::Title, Style::Sentence, Style::Lower, Style::Upper, Style::Dot];

    /// The case's name, shown as the row's badge.
    pub fn name(self) -> &'static str {
        match self {
            Style::Camel => "camelCase",
            Style::Pascal => "PascalCase",
            Style::Snake => "snake_case",
            Style::ScreamingSnake => "SCREAMING_SNAKE_CASE",
            Style::Kebab => "kebab-case",
            Style::Title => "Title Case",
            Style::Sentence => "Sentence case",
            Style::Lower => "lower case",
            Style::Upper => "UPPER CASE",
            Style::Dot => "dot.case",
        }
    }
}

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    clipboard: Option<String>,
}

#[derive(Serialize, Default, Debug, PartialEq)]
struct Row {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

/// A row badge with only text. Volant 0.3.8 and later show it in place of the extension's name.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// Where the converted text came from, named in every row's subtitle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source { Typed, Clipboard }

impl Source {
    fn label(self) -> &'static str {
        match self { Source::Typed => FROM_TYPED, Source::Clipboard => FROM_CLIPBOARD }
    }
}

/// The text to convert: what was typed when it has more than whitespace, otherwise the clipboard's text
/// when it does, otherwise nothing.
fn input(request: &Request) -> Option<(&str, Source)> {
    let has_text = |text: &str| !text.trim().is_empty();
    if has_text(&request.query) { return Some((&request.query, Source::Typed)); }
    request.clipboard.as_deref().filter(|text| has_text(text)).map(|text| (text, Source::Clipboard))
}

/// Whether `previous` and `current`, adjacent in one run of letters and digits, belong to different
/// words, given the character after `current`.
fn is_boundary(previous: char, current: char, next: Option<char>) -> bool {
    (previous.is_lowercase() && current.is_uppercase())
        || (previous.is_uppercase() && current.is_uppercase() && next.is_some_and(char::is_lowercase))
        || (previous.is_numeric() != current.is_numeric())
}

/// The words of `text` by the rules in the module documentation, as written.
pub fn split_words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut current = String::new();
    for (index, &c) in chars.iter().enumerate() {
        let next = chars.get(index + 1).copied();
        if APOSTROPHES.contains(&c) && !current.is_empty() && next.is_some_and(char::is_alphabetic) { continue; }
        if !c.is_alphanumeric() {
            if !current.is_empty() { words.push(std::mem::take(&mut current)); }
            continue;
        }
        if current.chars().last().is_some_and(|previous| is_boundary(previous, c, next)) {
            words.push(std::mem::take(&mut current));
        }
        current.push(c);
    }
    if !current.is_empty() { words.push(current); }
    words
}

/// A word with its first character uppercase and the rest lowercase.
fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars.flat_map(char::to_lowercase)).collect()).unwrap_or_default()
}

/// Prose with the first letter of each whitespace-separated token uppercase and the rest lowercase.
fn title_prose(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at_token_start = true;
    for c in text.chars() {
        if c.is_whitespace() {
            at_token_start = true;
            out.push(c);
        } else if c.is_alphabetic() && at_token_start {
            at_token_start = false;
            out.extend(c.to_uppercase());
        } else {
            if c.is_alphanumeric() { at_token_start = false; }
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// Prose in lowercase except the first letter of the text and of each sentence, which starts after
/// `.`, `!`, `?` or `…` and whitespace.
fn sentence_prose(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let (mut capitalize, mut ended) = (true, false);
    for c in text.chars() {
        if c.is_alphabetic() && capitalize {
            capitalize = false;
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
            if c.is_alphanumeric() { capitalize = false; }
        }
        if SENTENCE_ENDS.contains(&c) {
            ended = true;
        } else if c.is_whitespace() {
            capitalize |= ended;
            ended = false;
        } else if !c.is_alphanumeric() && ended {
            continue;
        } else {
            ended = false;
        }
    }
    out
}

/// `text` in `style`. Prose (text with whitespace inside it) keeps its punctuation in the four prose
/// cases; everything else is built from [`split_words`].
pub fn convert(text: &str, style: Style) -> String {
    let text = text.trim();
    let prose = text.contains(char::is_whitespace);
    let words = split_words(text);
    let lower = || words.iter().map(|word| word.to_lowercase()).collect::<Vec<_>>();
    match style {
        Style::Camel => words.iter().enumerate().map(|(index, word)| if index == 0 { word.to_lowercase() } else { capitalized(word) }).collect(),
        Style::Pascal => words.iter().map(|word| capitalized(word)).collect(),
        Style::Snake => lower().join(UNDERSCORE),
        Style::ScreamingSnake => words.iter().map(|word| word.to_uppercase()).collect::<Vec<_>>().join(UNDERSCORE),
        Style::Kebab => lower().join(HYPHEN),
        Style::Dot => lower().join(DOT),
        Style::Lower if prose => text.to_lowercase(),
        Style::Upper if prose => text.to_uppercase(),
        Style::Title if prose => title_prose(text),
        Style::Sentence if prose => sentence_prose(text),
        Style::Lower => lower().join(SPACE),
        Style::Upper => words.iter().map(|word| word.to_uppercase()).collect::<Vec<_>>().join(SPACE),
        Style::Title => words.iter().map(|word| capitalized(word)).collect::<Vec<_>>().join(SPACE),
        Style::Sentence => sentence_prose(&lower().join(SPACE)),
    }
}

/// 1840 as "1,840".
fn thousands(value: usize) -> String {
    let digits = value.to_string();
    digits.as_bytes().rchunks(3).rev().map(|chunk| std::str::from_utf8(chunk).unwrap_or_default()).collect::<Vec<_>>().join(THOUSANDS)
}

/// A row whose title is `value`, so Return copies it, or one saying it is too long to copy from a row.
fn result_row(value: String, label: &str, source: Source) -> Row {
    let badges = vec![Badge { text: label.to_string() }];
    let length = value.chars().count();
    if length > MAXIMUM_TITLE {
        let title = [TOO_LONG_START, &thousands(length), TOO_LONG_END].concat();
        return Row { title, subtitle: Some([source.label(), TOO_LONG_HINT].join(SEPARATOR)), badges };
    }
    Row { title: value, subtitle: Some(source.label().to_string()), badges }
}

fn rows(text: &str, source: Source) -> Vec<Row> {
    if split_words(text).is_empty() { return message(NO_WORDS, Some(source.label())); }
    Style::ALL.iter().map(|&style| result_row(convert(text, style), style.name(), source)).collect()
}

fn message(title: &str, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title: title.to_string(), subtitle: subtitle.map(str::to_string), ..Default::default() }]
}

/// Handles one request and returns the reply Volant reads. The clipboard's text appears only in the
/// converted rows, never logged or stored.
pub fn respond(input_bytes: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input_bytes) else { return items(message(NOT_UNDERSTOOD, None)) };
    if request.phase != RENDER { return items(message(ONLY_LISTS, None)); }
    match input(&request) {
        Some((text, source)) => items(rows(text, source)),
        None => items(message(PROMPT, Some(PROMPT_DETAIL))),
    }
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../manifest.json");

    fn reply(request: serde_json::Value) -> Vec<serde_json::Value> {
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    fn all(text: &str) -> Vec<String> {
        Style::ALL.iter().map(|&style| convert(text, style)).collect()
    }

    #[test]
    fn splits_at_case_digits_and_punctuation() {
        assert_eq!(split_words("fooBar"), ["foo", "Bar"]);
        assert_eq!(split_words("HTTPServer"), ["HTTP", "Server"]);
        assert_eq!(split_words("parseHTTPResponse2xx"), ["parse", "HTTP", "Response", "2", "xx"]);
        assert_eq!(split_words("utf8_decoder-v2.final"), ["utf", "8", "decoder", "v", "2", "final"]);
        assert_eq!(split_words("  hello,   world!  "), ["hello", "world"]);
        assert_eq!(split_words("don't stop"), ["dont", "stop"]);
        assert_eq!(split_words("ALL_CAPS"), ["ALL", "CAPS"]);
        assert_eq!(split_words("!!!"), Vec::<String>::new());
    }

    #[test]
    fn known_conversions_of_an_identifier() {
        assert_eq!(all("HTTPServer error"), ["httpServerError", "HttpServerError", "http_server_error", "HTTP_SERVER_ERROR",
            "http-server-error", "Httpserver Error", "Httpserver error", "httpserver error", "HTTPSERVER ERROR", "http.server.error"]);
        assert_eq!(all("fooBar"), ["fooBar", "FooBar", "foo_bar", "FOO_BAR", "foo-bar", "Foo Bar", "Foo bar", "foo bar", "FOO BAR", "foo.bar"]);
        assert_eq!(all("XMLHttpRequest"), ["xmlHttpRequest", "XmlHttpRequest", "xml_http_request", "XML_HTTP_REQUEST", "xml-http-request",
            "Xml Http Request", "Xml http request", "xml http request", "XML HTTP REQUEST", "xml.http.request"]);
    }

    #[test]
    fn prose_keeps_its_punctuation() {
        let text = "hello, WORLD! it's a test. another one?";
        assert_eq!(convert(text, Style::Title), "Hello, World! It's A Test. Another One?");
        assert_eq!(convert(text, Style::Sentence), "Hello, world! It's a test. Another one?");
        assert_eq!(convert(text, Style::Lower), "hello, world! it's a test. another one?");
        assert_eq!(convert(text, Style::Upper), "HELLO, WORLD! IT'S A TEST. ANOTHER ONE?");
        assert_eq!(convert(text, Style::Snake), "hello_world_its_a_test_another_one");
        assert_eq!(convert("\"quoted.\" next", Style::Sentence), "\"Quoted.\" Next");
    }

    #[test]
    fn unicode_and_emoji() {
        assert_eq!(convert("straße größe", Style::Upper), "STRASSE GRÖSSE");
        assert_eq!(convert("Ünïcödé wörds", Style::Camel), "ünïcödéWörds");
        assert_eq!(convert("🎉 party time 🎉", Style::Kebab), "party-time");
        assert_eq!(convert("🎉 party time 🎉", Style::Title), "🎉 Party Time 🎉");
        assert_eq!(convert("日本語 テキスト", Style::Snake), "日本語_テキスト");
    }

    #[test]
    fn rows_carry_a_badge_and_their_source() {
        let rows = reply(serde_json::json!({"phase":"render","query":"fooBar","now":0,"clipboard":"ignored text"}));
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[2]["title"], "foo_bar");
        assert_eq!(rows[2]["badges"], serde_json::json!([{"text":"snake_case"}]));
        assert!(rows.iter().all(|row| row["subtitle"] == FROM_TYPED && row.get("url").is_none()));
        let names: Vec<&str> = rows.iter().map(|row| row["badges"][0]["text"].as_str().unwrap()).collect();
        assert_eq!(names, Style::ALL.map(Style::name));
    }

    #[test]
    fn the_clipboard_is_used_only_when_nothing_is_typed() {
        let rows = reply(serde_json::json!({"phase":"render","query":" ","now":0,"clipboard":"user_id"}));
        assert_eq!(rows[0]["title"], "userId");
        assert!(rows.iter().all(|row| row["subtitle"] == FROM_CLIPBOARD));
        let none = reply(serde_json::json!({"phase":"render","query":"","now":0}));
        assert_eq!(none[0]["title"], PROMPT);
        let blank = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":"\n\t"}));
        assert_eq!(blank[0]["title"], PROMPT);
        let symbols = reply(serde_json::json!({"phase":"render","query":"--- !!!","now":0}));
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0]["title"], NO_WORDS);
    }

    #[test]
    fn results_over_300_characters_say_so() {
        let text = "word ".repeat(61);
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":text}));
        assert_eq!(rows[0]["title"], ["word".to_string(), "Word".repeat(60)].concat());
        assert_eq!(rows[2]["title"], "Too long to copy from a row (304 characters)");
        assert_eq!(rows[2]["subtitle"], "From clipboard · Volant rows copy at most 300 characters");
        assert_eq!(rows[2]["badges"][0]["text"], "snake_case");
        let exact = "a".repeat(300);
        assert_eq!(reply(serde_json::json!({"phase":"render","query":exact,"now":0}))[7]["title"], exact);
        assert!(rows.len() <= 100);
    }

    #[test]
    fn other_phases_and_bad_requests() {
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"x","now":0}))[0]["title"], ONLY_LISTS);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_asks_only_for_the_clipboard() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard"]));
        assert!(manifest.get("hosts").is_none() && manifest.get("storage").is_none());
    }
}
