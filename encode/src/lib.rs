//! Encode/Decode for Volant: shows the text you type, or the clipboard's text when the field is empty,
//! Base64, Base64URL, URL (percent) and HTML encoded, and decoded when it already is. It works offline:
//! the manifest lists no hosts, so Volant sends one `render` request and shows the rows. Each row's
//! title is the result, so Return copies it, and its badge names the transform.
//!
//! The rules:
//! - Encoding uses the text exactly as given, as UTF-8. Base64 is RFC 4648's standard alphabet with
//!   padding; Base64URL is its URL-safe alphabet without padding. URL encoding keeps RFC 3986's
//!   unreserved characters (letters, digits, `-`, `.`, `_`, `~`) and percent-encodes every other byte in
//!   uppercase hex. HTML escaping replaces `&`, `<`, `>`, `"` and `'`.
//! - Decode rows appear first, and only when they apply. Base64 decoding ignores ASCII whitespace,
//!   accepts either alphabet (not both at once) with or without padding, and needs unused bits to be
//!   zero; URL decoding needs every `%` to start a two-digit hex escape and leaves `+` as it is; HTML
//!   unescaping knows `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&nbsp;` and numeric entities such as
//!   `&#233;` and `&#xE9;`. A decoded result must be UTF-8 without control characters other than tab
//!   and line breaks, so plain words do not show as Base64.
//! - Volant shows at most 300 characters of a row's title, copies what it shows and trims spaces and
//!   line breaks from its ends. A longer result gets a row saying it is too long to copy, with its
//!   length, and a result with whitespace at its ends says that it copies without it. Length is counted
//!   in Unicode scalar values, which is never less than what Volant counts.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const RENDER: &str = "render";
const PROMPT: &str = "Type or copy some text to encode or decode it.";
const PROMPT_DETAIL: &str = "With the field empty, Encode/Decode reads the clipboard's text when it runs";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const FROM_CLIPBOARD: &str = "From clipboard";
const FROM_TYPED: &str = "From typed text";
const TOO_LONG_START: &str = "Too long to copy from a row (";
const TOO_LONG_END: &str = " characters)";
const TOO_LONG_HINT: &str = "Volant rows copy at most 300 characters";
const TRIMMED_HINT: &str = "Copies without the spaces or line breaks at its ends";
const ONLY_WHITESPACE: &str = "The result is only spaces or line breaks";
const SEPARATOR: &str = " · ";
const THOUSANDS: &str = ",";
const BASE64_ENCODE: &str = "Base64 encode";
const BASE64URL_ENCODE: &str = "Base64URL encode";
const BASE64_DECODE: &str = "Base64 decode";
const URL_ENCODE: &str = "URL encode";
const URL_DECODE: &str = "URL decode";
const HTML_ESCAPE: &str = "HTML escape";
const HTML_UNESCAPE: &str = "HTML unescape";
const STANDARD_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const PAD: u8 = b'=';
const STANDARD_MARKS: &[u8] = b"+/";
const URL_MARKS: &[u8] = b"-_";
const PERCENT: u8 = b'%';
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";
const UNRESERVED_MARKS: &[u8] = b"-._~";
const HTML_ESCAPES: [(char, &str); 5] = [('&', "&amp;"), ('<', "&lt;"), ('>', "&gt;"), ('"', "&quot;"), ('\'', "&#39;")];
const HTML_ENTITIES: [(&str, char); 6] = [("amp", '&'), ("lt", '<'), ("gt", '>'), ("quot", '"'), ("apos", '\''), ("nbsp", '\u{A0}')];
const ENTITY_START: char = '&';
const ENTITY_END: char = ';';
const NUMERIC_MARK: char = '#';
const HEX_MARKS: [char; 2] = ['x', 'X'];
const LONGEST_ENTITY: usize = 10;
const ALLOWED_CONTROLS: [char; 3] = ['\t', '\n', '\r'];
const MAXIMUM_TITLE: usize = 300;

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

/// Where the text came from, named in every row's subtitle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source { Typed, Clipboard }

impl Source {
    fn label(self) -> &'static str {
        match self { Source::Typed => FROM_TYPED, Source::Clipboard => FROM_CLIPBOARD }
    }
}

/// The text to transform: what was typed when it has more than whitespace, otherwise the clipboard's
/// text when it does, otherwise nothing. The text is used exactly as given.
fn input(request: &Request) -> Option<(&str, Source)> {
    let has_text = |text: &str| !text.trim().is_empty();
    if has_text(&request.query) { return Some((&request.query, Source::Typed)); }
    request.clipboard.as_deref().filter(|text| has_text(text)).map(|text| (text, Source::Clipboard))
}

/// `bytes` in Base64 with `alphabet`, padded with `=` when `padded`.
pub fn base64_encode(bytes: &[u8], alphabet: &[u8; 64], padded: bool) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().enumerate().fold(0u32, |value, (index, &byte)| value | (u32::from(byte) << (16 - 8 * index)));
        for index in 0..=chunk.len() {
            out.push(char::from(alphabet[(value >> (18 - 6 * index) & 0x3F) as usize]));
        }
        if padded {
            for _ in chunk.len()..3 { out.push(char::from(PAD)); }
        }
    }
    out
}

/// The bytes Base64 `text` stands for, by the rules in the module documentation, or None.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let compact: Vec<u8> = text.bytes().filter(|byte| !byte.is_ascii_whitespace()).collect();
    let data_length = compact.iter().position(|&byte| byte == PAD).unwrap_or(compact.len());
    let (data, padding) = compact.split_at(data_length);
    if data.is_empty() || padding.iter().any(|&byte| byte != PAD) || data.len() % 4 == 1 { return None; }
    if !padding.is_empty() && (compact.len() % 4 != 0 || padding.len() > 2) { return None; }
    let uses = |marks: &[u8]| data.iter().any(|byte| marks.contains(byte));
    let alphabet = if uses(URL_MARKS) { if uses(STANDARD_MARKS) { return None; } URL_ALPHABET } else { STANDARD_ALPHABET };
    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    for chunk in data.chunks(4) {
        let mut value = 0u32;
        for (index, byte) in chunk.iter().enumerate() {
            let digit = alphabet.iter().position(|letter| letter == byte)? as u32;
            value |= digit << (18 - 6 * index);
        }
        let produced = chunk.len() * 6 / 8;
        let unused = value & (0x00FF_FFFF >> (8 * produced));
        if unused != 0 { return None; }
        out.extend((0..produced).map(|index| (value >> (16 - 8 * index)) as u8));
    }
    Some(out)
}

/// `text` percent-encoded, keeping only RFC 3986 unreserved characters.
pub fn url_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || UNRESERVED_MARKS.contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push(char::from(PERCENT));
            out.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
            out.push(char::from(HEX_DIGITS[usize::from(byte & 0x0F)]));
        }
    }
    out
}

/// `text` with its percent escapes decoded, or None when it has none, a `%` starts no two-digit hex
/// escape, or the result is not UTF-8.
pub fn url_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let (mut index, mut escaped) = (0, false);
    while index < bytes.len() {
        if bytes[index] != PERCENT {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
        if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) { return None; }
        out.push(u8::from_str_radix(hex, 16).ok()?);
        escaped = true;
        index += 3;
    }
    if !escaped { return None; }
    String::from_utf8(out).ok()
}

/// `text` with `&`, `<`, `>`, `"` and `'` replaced by entities.
pub fn html_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match HTML_ESCAPES.iter().find(|(from, _)| *from == c) {
            Some((_, entity)) => out.push_str(entity),
            None => out.push(c),
        }
    }
    out
}

/// The character a named or numeric entity body (between `&` and `;`) stands for.
fn entity(body: &str) -> Option<char> {
    if let Some(number) = body.strip_prefix(NUMERIC_MARK) {
        let value = match number.strip_prefix(HEX_MARKS) {
            Some(hex) if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) => u32::from_str_radix(hex, 16).ok()?,
            Some(_) => return None,
            None if !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()) => number.parse().ok()?,
            None => return None,
        };
        return char::from_u32(value).filter(|&c| c != '\0');
    }
    HTML_ENTITIES.iter().find(|(name, _)| *name == body).map(|&(_, c)| c)
}

/// `text` with the entities in the module documentation decoded, or None when it has none.
pub fn html_unescape(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut changed = false;
    while let Some(start) = rest.find(ENTITY_START) {
        out.push_str(&rest[..start]);
        let after = &rest[start + ENTITY_START.len_utf8()..];
        let decoded = after.char_indices().take(LONGEST_ENTITY + 1).find(|&(_, c)| c == ENTITY_END)
            .and_then(|(end, _)| entity(&after[..end]).map(|c| (c, end)));
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &after[end + ENTITY_END.len_utf8()..];
                changed = true;
            }
            None => {
                out.push(ENTITY_START);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    changed.then_some(out)
}

/// Whether decoded text is fit to show: no control characters besides tab and line breaks.
fn is_readable(text: &str) -> bool {
    text.chars().all(|c| !c.is_control() || ALLOWED_CONTROLS.contains(&c))
}

/// 1840 as "1,840".
fn thousands(value: usize) -> String {
    let digits = value.to_string();
    digits.as_bytes().rchunks(3).rev().map(|chunk| std::str::from_utf8(chunk).unwrap_or_default()).collect::<Vec<_>>().join(THOUSANDS)
}

/// A row whose title is `value`, so Return copies it, or one saying it is too long to copy from a row.
/// Volant trims a title's ends, so a value with whitespace there says it copies without it.
fn result_row(value: &str, label: &str, source: Source) -> Row {
    let badges = vec![Badge { text: label.to_string() }];
    let trimmed = value.trim();
    let length = trimmed.chars().count();
    let (title, hint) = if trimmed.is_empty() {
        (ONLY_WHITESPACE.to_string(), None)
    } else if length > MAXIMUM_TITLE {
        ([TOO_LONG_START, &thousands(length), TOO_LONG_END].concat(), Some(TOO_LONG_HINT))
    } else {
        (trimmed.to_string(), (trimmed.len() != value.len()).then_some(TRIMMED_HINT))
    };
    let subtitle = std::iter::once(source.label()).chain(hint).collect::<Vec<_>>().join(SEPARATOR);
    Row { title, subtitle: Some(subtitle), badges }
}

/// Decode rows that apply, then every encoding.
fn rows(text: &str, source: Source) -> Vec<Row> {
    let mut results: Vec<(&str, String)> = Vec::new();
    let decoded = base64_decode(text).and_then(|bytes| String::from_utf8(bytes).ok()).filter(|decoded| is_readable(decoded));
    if let Some(decoded) = decoded { results.push((BASE64_DECODE, decoded)); }
    if let Some(decoded) = url_decode(text).filter(|decoded| is_readable(decoded)) { results.push((URL_DECODE, decoded)); }
    if let Some(decoded) = html_unescape(text) { results.push((HTML_UNESCAPE, decoded)); }
    results.push((BASE64_ENCODE, base64_encode(text.as_bytes(), STANDARD_ALPHABET, true)));
    results.push((BASE64URL_ENCODE, base64_encode(text.as_bytes(), URL_ALPHABET, false)));
    results.push((URL_ENCODE, url_encode(text)));
    results.push((HTML_ESCAPE, html_escape(text)));
    results.iter().map(|(label, value)| result_row(value, label, source)).collect()
}

fn message(title: &str, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title: title.to_string(), subtitle: subtitle.map(str::to_string), ..Default::default() }]
}

/// Handles one request and returns the reply Volant reads. The clipboard's text appears only in the
/// transformed rows, never logged or stored.
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

    fn labelled(rows: &[serde_json::Value]) -> Vec<(String, String)> {
        rows.iter().map(|row| (row["badges"][0]["text"].as_str().unwrap().to_string(), row["title"].as_str().unwrap().to_string())).collect()
    }

    #[test]
    fn base64_matches_rfc_4648_vectors() {
        let vectors = [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("foob", "Zm9vYg=="), ("fooba", "Zm9vYmE="), ("foobar", "Zm9vYmFy")];
        for (plain, encoded) in vectors {
            assert_eq!(base64_encode(plain.as_bytes(), STANDARD_ALPHABET, true), encoded);
            if !plain.is_empty() { assert_eq!(base64_decode(encoded).unwrap(), plain.as_bytes()); }
        }
        assert_eq!(base64_encode(b"<<???>>", STANDARD_ALPHABET, true), "PDw/Pz8+Pg==");
        assert_eq!(base64_encode(b"<<???>>", URL_ALPHABET, false), "PDw_Pz8-Pg");
        assert_eq!(base64_decode("PDw_Pz8-Pg").unwrap(), b"<<???>>");
        assert_eq!(base64_decode("Zm9v\nYmFy\n").unwrap(), b"foobar", "Line-wrapped Base64");
        assert_eq!(base64_decode("Zm8"), Some(b"fo".to_vec()), "Unpadded");
    }

    #[test]
    fn base64_rejects_what_is_not() {
        for text in ["hello", "Zg=", "Zm9=v", "Zh==", "PDw/Pz8-Pg", "Zg===", "@@@@", "===="] {
            assert_eq!(base64_decode(text), None, "{text}");
        }
    }

    #[test]
    fn url_and_html_known_answers() {
        assert_eq!(url_encode("a b&c=d/é~"), "a%20b%26c%3Dd%2F%C3%A9~");
        assert_eq!(url_encode("😀"), "%F0%9F%98%80");
        assert_eq!(url_decode("a%20b%26c%3Dd%2F%c3%a9~+").unwrap(), "a b&c=d/é~+");
        assert_eq!(url_decode("no escapes"), None);
        assert_eq!(url_decode("100% sure"), None);
        assert_eq!(url_decode("%FF"), None, "Not UTF-8");
        assert_eq!(html_escape("<a href=\"x\">Tom & Jerry's</a>"), "&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&#39;s&lt;/a&gt;");
        assert_eq!(html_unescape("&lt;b&gt; &amp;amp; &#233;&#xE9;&#X1F600; &nbsp;&apos;&quot;").unwrap(), "<b> &amp; éé😀 \u{A0}'\"");
        assert_eq!(html_unescape("AT&T & &unknown; &#xZZ; &#0; &#;"), None);
    }

    #[test]
    fn encoded_input_shows_its_decoding_first() {
        let rows = reply(serde_json::json!({"phase":"render","query":"aGVsbG8gd29ybGQ=","now":0}));
        assert_eq!(labelled(&rows)[0], (BASE64_DECODE.to_string(), "hello world".to_string()));
        let rows = reply(serde_json::json!({"phase":"render","query":"caf%C3%A9","now":0}));
        assert_eq!(labelled(&rows)[0], (URL_DECODE.to_string(), "café".to_string()));
        let rows = reply(serde_json::json!({"phase":"render","query":"&lt;p&gt;","now":0}));
        assert_eq!(labelled(&rows)[0], (HTML_UNESCAPE.to_string(), "<p>".to_string()));
    }

    #[test]
    fn plain_text_shows_only_encodings() {
        let rows = reply(serde_json::json!({"phase":"render","query":"Tom & Jerry","now":0}));
        assert_eq!(labelled(&rows), [
            (BASE64_ENCODE.to_string(), "VG9tICYgSmVycnk=".to_string()),
            (BASE64URL_ENCODE.to_string(), "VG9tICYgSmVycnk".to_string()),
            (URL_ENCODE.to_string(), "Tom%20%26%20Jerry".to_string()),
            (HTML_ESCAPE.to_string(), "Tom &amp; Jerry".to_string()),
        ]);
        assert!(rows.iter().all(|row| row["subtitle"] == FROM_TYPED && row.get("url").is_none()));
        let word = reply(serde_json::json!({"phase":"render","query":"test","now":0}));
        assert_eq!(word.len(), 4, "A word that is valid Base64 but not text shows no decoding");
    }

    #[test]
    fn emoji_round_trip() {
        let rows = reply(serde_json::json!({"phase":"render","query":"8J+YgCBoaQ==","now":0}));
        assert_eq!(rows[0]["title"], "😀 hi");
        let rows = reply(serde_json::json!({"phase":"render","query":"😀 hi","now":0}));
        assert_eq!(rows[0]["title"], "8J+YgCBoaQ==");
    }

    #[test]
    fn the_clipboard_is_used_only_when_nothing_is_typed() {
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":"Zm9v"}));
        assert_eq!(rows[0]["title"], "foo");
        assert!(rows.iter().all(|row| row["subtitle"] == FROM_CLIPBOARD));
        let typed = reply(serde_json::json!({"phase":"render","query":"a","now":0,"clipboard":"Zm9v"}));
        assert_eq!(typed[0]["title"], "YQ==");
        assert_eq!(typed[0]["subtitle"], FROM_TYPED);
        assert_eq!(reply(serde_json::json!({"phase":"render","query":" ","now":0,"clipboard":"  "}))[0]["title"], PROMPT);
        assert_eq!(reply(serde_json::json!({"phase":"render","query":"","now":0}))[0]["title"], PROMPT);
    }

    #[test]
    fn whitespace_at_the_ends_is_named() {
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":"aGVsbG8K"}));
        assert_eq!(rows[0]["title"], "hello");
        assert_eq!(rows[0]["subtitle"], "From clipboard · Copies without the spaces or line breaks at its ends");
        let rows = reply(serde_json::json!({"phase":"render","query":"%20%20","now":0}));
        assert_eq!(rows[0]["title"], ONLY_WHITESPACE);
    }

    #[test]
    fn results_over_300_characters_say_so() {
        let text = "a".repeat(225);
        let rows = reply(serde_json::json!({"phase":"render","query":text,"now":0}));
        assert_eq!(rows[0]["title"], base64_encode(text.as_bytes(), STANDARD_ALPHABET, true), "300 characters fit");
        let text = "a".repeat(226);
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":text}));
        assert_eq!(rows[0]["title"], "Too long to copy from a row (304 characters)");
        assert_eq!(rows[0]["subtitle"], "From clipboard · Volant rows copy at most 300 characters");
        assert_eq!(rows[2]["title"], text, "URL encoding leaves letters alone");
        assert!(rows.len() <= 100);
    }

    #[test]
    fn other_phases_and_bad_requests() {
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"x","now":0}))[0]["title"], ONLY_LISTS);
        assert!(respond(b"[]").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_asks_only_for_the_clipboard() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard"]));
        assert!(manifest.get("hosts").is_none() && manifest.get("storage").is_none());
    }
}
