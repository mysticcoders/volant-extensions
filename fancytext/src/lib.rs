//! Fancy Text for Volant: shows the text you type, or the clipboard's text when the field is empty,
//! in sixteen Unicode styles, from bold and script to circled, upside down and struck through. It
//! works offline: the manifest lists no hosts, so Volant sends one `render` request and shows the rows.
//! Each row's title is the styled text, so Return copies it, and its badge names the style.
//!
//! Most styles come from Unicode's Mathematical Alphanumeric Symbols block (U+1D400–U+1D7FF), which
//! leaves holes where a letter already existed elsewhere: script B is ℬ (U+212C), italic h is ℎ
//! (U+210E), double-struck C is ℂ (U+2102), and so on. Those letters come from the Letterlike Symbols
//! block instead. Characters a style has no form for, such as punctuation in bold, stay as typed.
//!
//! Two styles can give the same result, such as italic and script for digits, and a style may leave
//! the text unchanged; such rows are left out so each row is a different result. Volant shows at most
//! 300 characters of a row's title and copies what it shows, so a longer result gets a row saying so,
//! with its length, instead of a cut-off copy. Length counts every character except combining marks,
//! which is never less than what Volant counts.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const RENDER: &str = "render";
const PROMPT: &str = "Type or copy some text to style it.";
const PROMPT_DETAIL: &str = "With the field empty, Fancy Text reads the clipboard's text when it runs";
const NOTHING_TO_STYLE: &str = "None of the styles change this text.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const FROM_CLIPBOARD: &str = "From clipboard";
const FROM_TYPED: &str = "From typed text";
const TOO_LONG_START: &str = "Too long to copy from a row (";
const TOO_LONG_END: &str = " characters)";
const TOO_LONG_HINT: &str = "Volant rows copy at most 300 characters";
const SEPARATOR: &str = " · ";
const THOUSANDS: &str = ",";
const MAXIMUM_TITLE: usize = 300;
const STRIKE: char = '\u{0336}';
const IDEOGRAPHIC_SPACE: char = '\u{3000}';
const FULLWIDTH_OFFSET: u32 = 0xFEE0;

/// The sixteen styles, in the order the rows show them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Style {
    Bold, Italic, BoldItalic, Script, BoldScript, Fraktur, DoubleStruck, Monospace, SansSerif, SansBold,
    SmallCaps, Circled, Squared, Fullwidth, UpsideDown, Strikethrough,
}

/// Where a style's capitals, small letters and digits start in Unicode, and the letters that live
/// elsewhere because Unicode encoded them before the Mathematical Alphanumeric Symbols block.
struct Alphabet { capitals: u32, small: u32, digits: Option<u32>, holes: &'static [(char, char)] }

const ITALIC_HOLES: &[(char, char)] = &[('h', 'ℎ')];
const SCRIPT_HOLES: &[(char, char)] = &[('B', 'ℬ'), ('E', 'ℰ'), ('F', 'ℱ'), ('H', 'ℋ'), ('I', 'ℐ'), ('L', 'ℒ'), ('M', 'ℳ'), ('R', 'ℛ'),
    ('e', 'ℯ'), ('g', 'ℊ'), ('o', 'ℴ')];
const FRAKTUR_HOLES: &[(char, char)] = &[('C', 'ℭ'), ('H', 'ℌ'), ('I', 'ℑ'), ('R', 'ℜ'), ('Z', 'ℨ')];
const DOUBLE_STRUCK_HOLES: &[(char, char)] = &[('C', 'ℂ'), ('H', 'ℍ'), ('N', 'ℕ'), ('P', 'ℙ'), ('Q', 'ℚ'), ('R', 'ℝ'), ('Z', 'ℤ')];

/// Small capital forms of a to z; x has none, so it stays a small x.
const SMALL_CAPS: [char; 26] = ['ᴀ', 'ʙ', 'ᴄ', 'ᴅ', 'ᴇ', 'ꜰ', 'ɢ', 'ʜ', 'ɪ', 'ᴊ', 'ᴋ', 'ʟ', 'ᴍ', 'ɴ', 'ᴏ', 'ᴘ', 'ǫ', 'ʀ', 'ꜱ', 'ᴛ', 'ᴜ', 'ᴠ', 'ᴡ', 'x', 'ʏ', 'ᴢ'];

/// Each character's upside-down look-alike; the text is also reversed, so it reads turned over.
const UPSIDE_DOWN: &[(char, char)] = &[
    ('a', 'ɐ'), ('b', 'q'), ('c', 'ɔ'), ('d', 'p'), ('e', 'ǝ'), ('f', 'ɟ'), ('g', 'ƃ'), ('h', 'ɥ'), ('i', 'ᴉ'), ('j', 'ɾ'), ('k', 'ʞ'),
    ('l', 'l'), ('m', 'ɯ'), ('n', 'u'), ('o', 'o'), ('p', 'd'), ('q', 'b'), ('r', 'ɹ'), ('s', 's'), ('t', 'ʇ'), ('u', 'n'), ('v', 'ʌ'),
    ('w', 'ʍ'), ('x', 'x'), ('y', 'ʎ'), ('z', 'z'),
    ('A', '∀'), ('B', 'ꓭ'), ('C', 'Ɔ'), ('D', 'ꓷ'), ('E', 'Ǝ'), ('F', 'Ⅎ'), ('G', '⅁'), ('H', 'H'), ('I', 'I'), ('J', 'ſ'), ('K', 'ꓘ'),
    ('L', '˥'), ('M', 'W'), ('N', 'N'), ('O', 'O'), ('P', 'Ԁ'), ('Q', 'Ό'), ('R', 'ꓤ'), ('S', 'S'), ('T', '⊥'), ('U', '∩'), ('V', 'Λ'),
    ('W', 'M'), ('X', 'X'), ('Y', '⅄'), ('Z', 'Z'),
    ('0', '0'), ('1', 'Ɩ'), ('2', 'ᄅ'), ('3', 'Ɛ'), ('4', 'ㄣ'), ('5', 'ϛ'), ('6', '9'), ('7', 'ㄥ'), ('8', '8'), ('9', '6'),
    ('.', '˙'), (',', '\''), ('\'', ','), ('"', '„'), ('?', '¿'), ('!', '¡'), ('(', ')'), (')', '('), ('[', ']'), (']', '['),
    ('{', '}'), ('}', '{'), ('<', '>'), ('>', '<'), ('_', '‾'), ('&', '⅋'), (';', '؛'),
];

impl Style {
    pub const ALL: [Style; 16] = [Style::Bold, Style::Italic, Style::BoldItalic, Style::Script, Style::BoldScript, Style::Fraktur,
        Style::DoubleStruck, Style::Monospace, Style::SansSerif, Style::SansBold, Style::SmallCaps, Style::Circled, Style::Squared,
        Style::Fullwidth, Style::UpsideDown, Style::Strikethrough];

    /// The style's name, shown as the row's badge.
    pub fn name(self) -> &'static str {
        match self {
            Style::Bold => "Bold",
            Style::Italic => "Italic",
            Style::BoldItalic => "Bold Italic",
            Style::Script => "Script",
            Style::BoldScript => "Bold Script",
            Style::Fraktur => "Fraktur",
            Style::DoubleStruck => "Double-Struck",
            Style::Monospace => "Monospace",
            Style::SansSerif => "Sans-Serif",
            Style::SansBold => "Sans-Serif Bold",
            Style::SmallCaps => "Small Caps",
            Style::Circled => "Circled",
            Style::Squared => "Squared",
            Style::Fullwidth => "Fullwidth",
            Style::UpsideDown => "Upside Down",
            Style::Strikethrough => "Strikethrough",
        }
    }

    /// The style's place in the Mathematical Alphanumeric Symbols block, for the styles that have one.
    fn alphabet(self) -> Option<Alphabet> {
        let alphabet = |capitals, small, digits, holes| Some(Alphabet { capitals, small, digits, holes });
        match self {
            Style::Bold => alphabet(0x1D400, 0x1D41A, Some(0x1D7CE), &[]),
            Style::Italic => alphabet(0x1D434, 0x1D44E, None, ITALIC_HOLES),
            Style::BoldItalic => alphabet(0x1D468, 0x1D482, None, &[]),
            Style::Script => alphabet(0x1D49C, 0x1D4B6, None, SCRIPT_HOLES),
            Style::BoldScript => alphabet(0x1D4D0, 0x1D4EA, None, &[]),
            Style::Fraktur => alphabet(0x1D504, 0x1D51E, None, FRAKTUR_HOLES),
            Style::DoubleStruck => alphabet(0x1D538, 0x1D552, Some(0x1D7D8), DOUBLE_STRUCK_HOLES),
            Style::Monospace => alphabet(0x1D670, 0x1D68A, Some(0x1D7F6), &[]),
            Style::SansSerif => alphabet(0x1D5A0, 0x1D5BA, Some(0x1D7E2), &[]),
            Style::SansBold => alphabet(0x1D5D4, 0x1D5EE, Some(0x1D7EC), &[]),
            _ => None,
        }
    }
}

/// `c` moved `offset` places on from `base`.
fn shifted(base: u32, offset: u32) -> Option<char> {
    char::from_u32(base + offset)
}

/// One character in a Mathematical Alphanumeric style, or itself when the style has no form for it.
fn in_alphabet(c: char, alphabet: &Alphabet) -> char {
    if let Some(&(_, hole)) = alphabet.holes.iter().find(|(letter, _)| *letter == c) { return hole; }
    let found = match c {
        'A'..='Z' => shifted(alphabet.capitals, c as u32 - 'A' as u32),
        'a'..='z' => shifted(alphabet.small, c as u32 - 'a' as u32),
        '0'..='9' => alphabet.digits.and_then(|digits| shifted(digits, c as u32 - '0' as u32)),
        _ => None,
    };
    found.unwrap_or(c)
}

fn small_caps(c: char) -> char {
    if c.is_ascii_lowercase() { SMALL_CAPS[(c as u8 - b'a') as usize] } else { c }
}

fn circled(c: char) -> char {
    let found = match c {
        'A'..='Z' => shifted(0x24B6, c as u32 - 'A' as u32),
        'a'..='z' => shifted(0x24D0, c as u32 - 'a' as u32),
        '0' => Some('⓪'),
        '1'..='9' => shifted(0x2460, c as u32 - '1' as u32),
        _ => None,
    };
    found.unwrap_or(c)
}

/// Squared Latin capitals; small letters take the capital's square, since Unicode has no small ones.
fn squared(c: char) -> char {
    if c.is_ascii_alphabetic() { shifted(0x1F130, c.to_ascii_uppercase() as u32 - 'A' as u32).unwrap_or(c) } else { c }
}

/// Fullwidth forms of printable ASCII, with the ideographic space for a space.
fn fullwidth(c: char) -> char {
    match c {
        ' ' => IDEOGRAPHIC_SPACE,
        '!'..='~' => shifted(c as u32, FULLWIDTH_OFFSET).unwrap_or(c),
        _ => c,
    }
}

/// `text` in `style`, trimmed of surrounding whitespace first.
pub fn convert(text: &str, style: Style) -> String {
    let text = text.trim();
    if let Some(alphabet) = style.alphabet() { return text.chars().map(|c| in_alphabet(c, &alphabet)).collect(); }
    match style {
        Style::SmallCaps => text.chars().map(small_caps).collect(),
        Style::Circled => text.chars().map(circled).collect(),
        Style::Squared => text.chars().map(squared).collect(),
        Style::Fullwidth => text.chars().map(fullwidth).collect(),
        Style::UpsideDown => text.chars().rev().map(|c| UPSIDE_DOWN.iter().find(|(from, _)| *from == c).map_or(c, |(_, to)| *to)).collect(),
        Style::Strikethrough => text.chars().flat_map(|c| [c, STRIKE]).collect(),
        _ => text.to_string(),
    }
}

/// Whether `c` is a combining mark, which joins the character before it on screen.
fn is_combining(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

/// The length Volant's 300-character limit is measured against, never less than Volant's own count.
pub fn visible_length(text: &str) -> usize {
    text.chars().filter(|&c| !is_combining(c)).count()
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

/// Where the styled text came from, named in every row's subtitle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source { Typed, Clipboard }

impl Source {
    fn label(self) -> &'static str {
        match self { Source::Typed => FROM_TYPED, Source::Clipboard => FROM_CLIPBOARD }
    }
}

/// The text to style: what was typed when it has more than whitespace, otherwise the clipboard's text
/// when it does, otherwise nothing.
fn input(request: &Request) -> Option<(&str, Source)> {
    let has_text = |text: &str| !text.trim().is_empty();
    if has_text(&request.query) { return Some((&request.query, Source::Typed)); }
    request.clipboard.as_deref().filter(|text| has_text(text)).map(|text| (text, Source::Clipboard))
}

/// 1840 as "1,840".
fn thousands(value: usize) -> String {
    let digits = value.to_string();
    digits.as_bytes().rchunks(3).rev().map(|chunk| std::str::from_utf8(chunk).unwrap_or_default()).collect::<Vec<_>>().join(THOUSANDS)
}

/// A row whose title is `value`, so Return copies it, or one saying it is too long to copy from a row.
fn result_row(value: String, style: Style, source: Source) -> Row {
    let badges = vec![Badge { text: style.name().to_string() }];
    let length = visible_length(&value);
    if length > MAXIMUM_TITLE {
        let title = [TOO_LONG_START, &thousands(length), TOO_LONG_END].concat();
        return Row { title, subtitle: Some([source.label(), TOO_LONG_HINT].join(SEPARATOR)), badges };
    }
    Row { title: value, subtitle: Some(source.label().to_string()), badges }
}

/// One row per style whose result differs from the text and from every row before it.
fn rows(text: &str, source: Source) -> Vec<Row> {
    let original = text.trim();
    let mut seen: Vec<String> = Vec::new();
    let mut rows = Vec::new();
    for style in Style::ALL {
        let styled = convert(text, style);
        if styled == original || seen.contains(&styled) { continue; }
        seen.push(styled.clone());
        rows.push(result_row(styled, style, source));
    }
    if rows.is_empty() { return message(NOTHING_TO_STYLE, Some(source.label())); }
    rows
}

fn message(title: &str, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title: title.to_string(), subtitle: subtitle.map(str::to_string), ..Default::default() }]
}

/// Handles one request and returns the reply Volant reads. The clipboard's text appears only in the
/// styled rows, never logged or stored.
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

    /// The code points Unicode reserves in the Mathematical Alphanumeric Symbols block for letters
    /// encoded earlier in Letterlike Symbols; no style may produce them.
    const RESERVED: [u32; 24] = [0x1D455, 0x1D49D, 0x1D4A0, 0x1D4A1, 0x1D4A3, 0x1D4A4, 0x1D4A7, 0x1D4A8, 0x1D4AD, 0x1D4BA, 0x1D4BC,
        0x1D4C4, 0x1D506, 0x1D50B, 0x1D50C, 0x1D515, 0x1D51D, 0x1D53A, 0x1D53F, 0x1D545, 0x1D547, 0x1D548, 0x1D549, 0x1D551];

    fn reply(request: serde_json::Value) -> Vec<serde_json::Value> {
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    #[test]
    fn known_answers_for_every_style() {
        let expected = [
            (Style::Bold, "𝐇𝐞𝐥𝐥𝐨 𝐖𝐨𝐫𝐥𝐝 𝟏𝟐𝟑"),
            (Style::Italic, "𝐻𝑒𝑙𝑙𝑜 𝑊𝑜𝑟𝑙𝑑 123"),
            (Style::BoldItalic, "𝑯𝒆𝒍𝒍𝒐 𝑾𝒐𝒓𝒍𝒅 123"),
            (Style::Script, "ℋℯ𝓁𝓁ℴ 𝒲ℴ𝓇𝓁𝒹 123"),
            (Style::BoldScript, "𝓗𝓮𝓵𝓵𝓸 𝓦𝓸𝓻𝓵𝓭 123"),
            (Style::Fraktur, "ℌ𝔢𝔩𝔩𝔬 𝔚𝔬𝔯𝔩𝔡 123"),
            (Style::DoubleStruck, "ℍ𝕖𝕝𝕝𝕠 𝕎𝕠𝕣𝕝𝕕 𝟙𝟚𝟛"),
            (Style::Monospace, "𝙷𝚎𝚕𝚕𝚘 𝚆𝚘𝚛𝚕𝚍 𝟷𝟸𝟹"),
            (Style::SansSerif, "𝖧𝖾𝗅𝗅𝗈 𝖶𝗈𝗋𝗅𝖽 𝟣𝟤𝟥"),
            (Style::SansBold, "𝗛𝗲𝗹𝗹𝗼 𝗪𝗼𝗿𝗹𝗱 𝟭𝟮𝟯"),
            (Style::SmallCaps, "Hᴇʟʟᴏ Wᴏʀʟᴅ 123"),
            (Style::Circled, "Ⓗⓔⓛⓛⓞ Ⓦⓞⓡⓛⓓ ①②③"),
            (Style::Squared, "🄷🄴🄻🄻🄾 🅆🄾🅁🄻🄳 123"),
            (Style::Fullwidth, "Ｈｅｌｌｏ　Ｗｏｒｌｄ　１２３"),
            (Style::UpsideDown, "ƐᄅƖ plɹoM ollǝH"),
            (Style::Strikethrough, "H\u{336}e\u{336}l\u{336}l\u{336}o\u{336} \u{336}W\u{336}o\u{336}r\u{336}l\u{336}d\u{336} \u{336}1\u{336}2\u{336}3\u{336}"),
        ];
        for (style, answer) in expected {
            assert_eq!(convert("  Hello World 123 ", style), answer, "{}", style.name());
        }
    }

    #[test]
    fn reserved_holes_come_from_letterlike_symbols() {
        assert_eq!(convert("h", Style::Italic), "ℎ");
        assert_eq!(convert("BEFHILMR", Style::Script), "ℬℰℱℋℐℒℳℛ");
        assert_eq!(convert("ego", Style::Script), "ℯℊℴ");
        assert_eq!(convert("ACDGJ", Style::Script), "𝒜𝒞𝒟𝒢𝒥");
        assert_eq!(convert("CHIRZ", Style::Fraktur), "ℭℌℑℜℨ");
        assert_eq!(convert("CHNPQRZ", Style::DoubleStruck), "ℂℍℕℙℚℝℤ");
        assert_eq!(convert("ABz0", Style::DoubleStruck), "𝔸𝔹𝕫𝟘");
        let alphabet: String = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
        for style in Style::ALL {
            for c in convert(&alphabet, style).chars() {
                assert!(!RESERVED.contains(&(c as u32)), "{} uses the reserved U+{:X}", style.name(), c as u32);
            }
        }
    }

    #[test]
    fn small_caps_circled_squared_and_fullwidth_details() {
        assert_eq!(convert("box", Style::SmallCaps), "ʙᴏx");
        assert_eq!(convert("a0z9", Style::Circled), "ⓐ⓪ⓩ⑨");
        assert_eq!(convert("Ab!", Style::Squared), "🄰🄱!");
        assert_eq!(convert("a b~", Style::Fullwidth), "ａ　ｂ～");
        assert_eq!(convert("Hi? (x)", Style::UpsideDown), "(x) ¿ᴉH");
        assert_eq!(convert("café", Style::Bold), "𝐜𝐚𝐟é", "Letters outside A to Z stay as typed");
    }

    #[test]
    fn rows_carry_a_badge_and_their_source() {
        let rows = reply(serde_json::json!({"phase":"render","query":"Hello","now":0,"clipboard":"ignored"}));
        assert_eq!(rows.len(), 16);
        assert_eq!(rows[0]["title"], "𝐇𝐞𝐥𝐥𝐨");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"Bold"}]));
        assert!(rows.iter().all(|row| row["subtitle"] == FROM_TYPED && row.get("url").is_none()));
        let clipboard = reply(serde_json::json!({"phase":"render","query":" ","now":0,"clipboard":"abc"}));
        assert!(clipboard.iter().all(|row| row["subtitle"] == FROM_CLIPBOARD));
    }

    #[test]
    fn unchanged_and_repeated_results_are_left_out() {
        let rows = reply(serde_json::json!({"phase":"render","query":"42","now":0}));
        let names: Vec<&str> = rows.iter().map(|row| row["badges"][0]["text"].as_str().unwrap()).collect();
        assert_eq!(names, ["Bold", "Double-Struck", "Monospace", "Sans-Serif", "Sans-Serif Bold", "Circled", "Fullwidth", "Upside Down", "Strikethrough"]);
        let punctuation = reply(serde_json::json!({"phase":"render","query":"-","now":0}));
        assert_eq!(punctuation.iter().map(|row| row["badges"][0]["text"].as_str().unwrap()).collect::<Vec<_>>(), ["Fullwidth", "Strikethrough"]);
    }

    #[test]
    fn results_over_300_characters_say_so() {
        let text = "a".repeat(200);
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":text}));
        let strike = rows.iter().find(|row| row["badges"][0]["text"] == "Strikethrough").unwrap();
        assert_eq!(strike["title"], "a\u{336}".repeat(200), "Combining strokes do not count toward the limit");
        let long = reply(serde_json::json!({"phase":"render","query":"b".repeat(301),"now":0}));
        assert_eq!(long[0]["title"], "Too long to copy from a row (301 characters)");
        assert_eq!(long[0]["subtitle"], "From typed text · Volant rows copy at most 300 characters");
        assert_eq!(long[0]["badges"][0]["text"], "Bold");
        let exact = reply(serde_json::json!({"phase":"render","query":"c".repeat(300),"now":0}));
        assert_eq!(exact[0]["title"].as_str().unwrap().chars().count(), 300);
    }

    #[test]
    fn prompts_and_other_requests() {
        assert_eq!(reply(serde_json::json!({"phase":"render","query":"","now":0}))[0]["title"], PROMPT);
        assert_eq!(reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":"\n "}))[0]["title"], PROMPT);
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"x","now":0}))[0]["title"], ONLY_LISTS);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
        for style in Style::ALL { assert!(style.name().len() <= 32); }
    }

    #[test]
    fn manifest_asks_only_for_the_clipboard() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard"]));
        assert!(manifest.get("hosts").is_none() && manifest.get("storage").is_none());
    }
}
