//! Word Count for Volant: counts the words, characters, sentences, paragraphs and lines of the text you
//! type, or of the clipboard's text when the field is empty, with reading and speaking times. It works
//! offline: the manifest lists no hosts, so Volant sends one `render` request and shows the rows. Each
//! row's title is the count, so Return copies it.
//!
//! The counting rules, which are approximations of what people mean:
//! - A word is a run of letters or digits in a whitespace-separated token, so `don't`, `e-mail` and
//!   `3.14` are one word each, and a token of only punctuation, such as `—`, is none. Chinese and
//!   Japanese are written without spaces, so each Han ideograph, hiragana and katakana character counts
//!   as one word, as word processors count them. Korean uses spaces and counts by them. Thai, Lao,
//!   Khmer and Burmese, which are written without spaces, count each unspaced phrase as one word.
//! - A character is what a reader sees as one: combining accents, emoji skin tones, variation selectors,
//!   flags and emoji joined with zero-width joiners stay with the character before them, and CR LF is
//!   one line break. This is a smaller table than Unicode's full grapheme rules, so some scripts' marks
//!   count separately. Spaces and line breaks count; "without spaces" leaves out all whitespace.
//! - A sentence ends at `.`, `!`, `?`, `…` or `‽` followed by whitespace or the end, or at `。`, `！` or
//!   `？`; text after the last end counts as one more. Abbreviations such as `e.g. this` count as an end.
//! - A paragraph is a block of lines between blank lines. Lines are separated by LF or CR LF.
//! - Reading time assumes about 238 words a minute and speaking time about 150, rounded up to a second.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const RENDER: &str = "render";
const PROMPT: &str = "Type or copy some text to count its words.";
const PROMPT_DETAIL: &str = "With the field empty, Word Count reads the clipboard's text when it runs";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const FROM_CLIPBOARD: &str = "From clipboard";
const FROM_TYPED: &str = "From typed text";
const SEPARATOR: &str = " · ";
const SPACE: &str = " ";
const THOUSANDS: &str = ",";
const WORD: (&str, &str) = ("word", "words");
const CHARACTER: (&str, &str) = ("character", "characters");
const CHARACTER_NO_SPACES: (&str, &str) = ("character without spaces", "characters without spaces");
const SENTENCE: (&str, &str) = ("sentence", "sentences");
const PARAGRAPH: (&str, &str) = ("paragraph", "paragraphs");
const LINE: (&str, &str) = ("line", "lines");
const READING: &str = "reading time";
const SPEAKING: &str = "speaking time";
const AT_PACE: &str = "At about ";
const PER_MINUTE: &str = " words per minute";
const HOURS: &str = "hr";
const MINUTES: &str = "min";
const SECONDS: &str = "sec";
const WORDS_HINT: &str = "Letters or digits between spaces; each Chinese or Japanese character counts as a word";
const CHARACTERS_HINT: &str = "Including spaces and line breaks";
const SENTENCES_HINT: &str = "Ending in . ! ? or …, plus any text after the last one";
const PARAGRAPHS_HINT: &str = "Blocks of text between blank lines";
const READING_WPM: usize = 238;
const SPEAKING_WPM: usize = 150;
const SECONDS_PER_MINUTE: usize = 60;
const SECONDS_PER_HOUR: usize = 3600;
const ZERO_WIDTH_JOINER: char = '\u{200D}';
const TERMINATORS: [char; 5] = ['.', '!', '?', '…', '‽'];
const WIDE_TERMINATORS: [char; 3] = ['。', '！', '？'];
const CLOSERS: [char; 10] = ['"', '\'', ')', ']', '”', '’', '»', '」', '』', '）'];

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
}

/// Where the counted text came from, named in every row's subtitle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Source { Typed, Clipboard }

impl Source {
    fn label(self) -> &'static str {
        match self { Source::Typed => FROM_TYPED, Source::Clipboard => FROM_CLIPBOARD }
    }
}

/// Every count the extension shows for one text.
#[derive(Debug, PartialEq)]
pub struct Counts {
    pub words: usize,
    pub characters: usize,
    pub characters_without_spaces: usize,
    pub sentences: usize,
    pub paragraphs: usize,
    pub lines: usize,
}

/// The text to count: what was typed when it has more than whitespace, otherwise the clipboard's text
/// when it does, otherwise nothing.
fn input(request: &Request) -> Option<(&str, Source)> {
    let has_text = |text: &str| !text.trim().is_empty();
    if has_text(&request.query) { return Some((&request.query, Source::Typed)); }
    request.clipboard.as_deref().filter(|text| has_text(text)).map(|text| (text, Source::Clipboard))
}

/// Han ideographs, hiragana and katakana, which are written without spaces and count one word each.
fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x3040..=0x30FF | 0x31F0..=0x31FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0xFF66..=0xFF9F
        | 0x20000..=0x2FA1F | 0x30000..=0x3134F)
}

fn is_regional_indicator(c: char) -> bool {
    matches!(c as u32, 0x1F1E6..=0x1F1FF)
}

/// Marks that belong to the character before them: combining accents and diacritics, Hebrew and Arabic
/// points, Devanagari and Thai vowel signs, kana voicing marks, Hangul vowel and final jamo, variation
/// selectors, emoji skin tones, emoji tags and the zero-width joiner itself.
fn joins_previous(c: char) -> bool {
    c == ZERO_WIDTH_JOINER || matches!(c as u32,
        0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7
        | 0x0610..=0x061A | 0x064B..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4 | 0x06E7..=0x06E8 | 0x06EA..=0x06ED
        | 0x0900..=0x0903 | 0x093A..=0x093C | 0x093E..=0x094F | 0x0951..=0x0957 | 0x0962..=0x0963
        | 0x0E31 | 0x0E34..=0x0E3A | 0x0E47..=0x0E4E | 0x1160..=0x11FF | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF
        | 0x20D0..=0x20FF | 0x3099..=0x309A | 0xFE00..=0xFE0F | 0xFE20..=0xFE2F | 0x1F3FB..=0x1F3FF
        | 0xE0020..=0xE007F | 0xE0100..=0xE01EF)
}

/// The number of user-perceived characters, and of those that are not whitespace, using the joining
/// rules in [`joins_previous`] plus CR LF, emoji joined by a zero-width joiner and flag pairs.
pub fn characters(text: &str) -> (usize, usize) {
    let (mut total, mut visible) = (0, 0);
    let mut previous: Option<char> = None;
    let mut flag_open = false;
    for c in text.chars() {
        let joins = match previous {
            None => false,
            Some(before) => joins_previous(c) || (before == ZERO_WIDTH_JOINER && !c.is_whitespace())
                || (before == '\r' && c == '\n') || (flag_open && is_regional_indicator(c)),
        };
        if joins {
            if is_regional_indicator(c) { flag_open = false; }
        } else {
            total += 1;
            if !c.is_whitespace() { visible += 1; }
            flag_open = is_regional_indicator(c);
        }
        previous = Some(c);
    }
    (total, visible)
}

/// Words as the module documentation describes them: runs of letters or digits within each
/// whitespace-separated token, with each Han ideograph or kana character a word of its own.
pub fn words(text: &str) -> usize {
    let mut count = 0;
    for token in text.split_whitespace() {
        let mut in_word = false;
        for c in token.chars() {
            if is_cjk(c) {
                count += 1;
                in_word = false;
            } else if c.is_alphanumeric() && !in_word {
                count += 1;
                in_word = true;
            }
        }
    }
    count
}

/// Sentences: each run of terminators after some words, followed by whitespace or the end (or a wide
/// terminator anywhere), ends one, after any closing quotes or brackets; words after the last end count
/// as one more.
pub fn sentences(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let is_terminator = |c: char| TERMINATORS.contains(&c) || WIDE_TERMINATORS.contains(&c);
    let (mut count, mut open, mut index) = (0, false, 0);
    while index < chars.len() {
        let c = chars[index];
        if !is_terminator(c) {
            open |= c.is_alphanumeric();
            index += 1;
            continue;
        }
        let mut end = index;
        while end < chars.len() && is_terminator(chars[end]) { end += 1; }
        let wide = chars[index..end].iter().any(|c| WIDE_TERMINATORS.contains(c));
        let mut after = end;
        while after < chars.len() && CLOSERS.contains(&chars[after]) { after += 1; }
        if open && (wide || after == chars.len() || chars[after].is_whitespace()) {
            count += 1;
            open = false;
        }
        index = after;
    }
    count + usize::from(open)
}

/// Blocks of lines that are not blank, separated by blank ones.
pub fn paragraphs(text: &str) -> usize {
    let (mut count, mut inside) = (0, false);
    for line in text.lines() {
        let blank = line.trim().is_empty();
        if !blank && !inside { count += 1; }
        inside = !blank;
    }
    count
}

/// All the counts for one text.
pub fn count(text: &str) -> Counts {
    let (characters, characters_without_spaces) = characters(text);
    Counts {
        words: words(text),
        characters,
        characters_without_spaces,
        sentences: sentences(text),
        paragraphs: paragraphs(text),
        lines: text.lines().count(),
    }
}

/// 1840 as "1,840".
fn thousands(value: usize) -> String {
    let digits = value.to_string();
    digits.as_bytes().rchunks(3).rev().map(|chunk| std::str::from_utf8(chunk).unwrap_or_default()).collect::<Vec<_>>().join(THOUSANDS)
}

/// "1 word" or "1,840 words".
fn counted(value: usize, (singular, plural): (&str, &str)) -> String {
    [thousands(value), (if value == 1 { singular } else { plural }).to_string()].join(SPACE)
}

/// How long `words` take at `per_minute`, rounded up to a second: "45 sec", "1 min 19 sec", "2 min",
/// or "1 hr 27 min" for an hour or more.
pub fn duration(words: usize, per_minute: usize) -> String {
    let seconds = (words * SECONDS_PER_MINUTE).div_ceil(per_minute);
    let part = |value: usize, unit: &str| [value.to_string(), unit.to_string()].join(SPACE);
    let parts = if seconds >= SECONDS_PER_HOUR {
        let minutes = seconds % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
        let mut parts = vec![part(seconds / SECONDS_PER_HOUR, HOURS)];
        if minutes > 0 { parts.push(part(minutes, MINUTES)); }
        parts
    } else if seconds >= SECONDS_PER_MINUTE {
        let rest = seconds % SECONDS_PER_MINUTE;
        let mut parts = vec![part(seconds / SECONDS_PER_MINUTE, MINUTES)];
        if rest > 0 { parts.push(part(rest, SECONDS)); }
        parts
    } else {
        vec![part(seconds, SECONDS)]
    };
    parts.join(SPACE)
}

fn row(title: String, details: &[&str]) -> Row {
    Row { title, subtitle: Some(details.join(SEPARATOR)) }
}

/// One row per count, each titled with the count so Return copies it, and each saying where the text
/// came from.
fn rows(text: &str, source: Source) -> Vec<Row> {
    let counts = count(text);
    let from = source.label();
    let pace = |per_minute: usize| [AT_PACE, &per_minute.to_string(), PER_MINUTE].concat();
    let timed = |per_minute: usize, label: &str| [duration(counts.words, per_minute), label.to_string()].join(SPACE);
    vec![
        row(counted(counts.words, WORD), &[from, WORDS_HINT]),
        row(counted(counts.characters, CHARACTER), &[from, CHARACTERS_HINT]),
        row(counted(counts.characters_without_spaces, CHARACTER_NO_SPACES), &[from]),
        row(counted(counts.sentences, SENTENCE), &[from, SENTENCES_HINT]),
        row(counted(counts.paragraphs, PARAGRAPH), &[from, PARAGRAPHS_HINT]),
        row(counted(counts.lines, LINE), &[from]),
        row(timed(READING_WPM, READING), &[from, &pace(READING_WPM)]),
        row(timed(SPEAKING_WPM, SPEAKING), &[from, &pace(SPEAKING_WPM)]),
    ]
}

fn message(title: &str, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title: title.to_string(), subtitle: subtitle.map(str::to_string) }]
}

/// Handles one request and returns the reply Volant reads. The clipboard's text is only counted, never
/// echoed, logged or stored.
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

    fn titles(request: serde_json::Value) -> Vec<String> {
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().iter().map(|row| row["title"].as_str().unwrap().to_string()).collect()
    }

    fn reply(request: serde_json::Value) -> Vec<serde_json::Value> {
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    #[test]
    fn counts_a_known_paragraph() {
        let text = "The quick brown fox jumps over the lazy dog. It didn't care!\n\nA second paragraph?\nYes";
        assert_eq!(count(text), Counts { words: 16, characters: 85, characters_without_spaces: 69, sentences: 4, paragraphs: 2, lines: 4 });
    }

    #[test]
    fn words_follow_the_documented_rule() {
        assert_eq!(words("don't e-mail 3.14 — & ..."), 3);
        assert_eq!(words("  hello\tworld\nagain  "), 3);
        assert_eq!(words("hello\u{3000}world\u{00A0}again"), 3, "Ideographic and no-break spaces separate words");
        assert_eq!(words("日本語abc"), 4, "Each Han character is a word; a Latin run is one more");
        assert_eq!(words("これはペンです"), 7);
        assert_eq!(words("안녕하세요 세계"), 2, "Korean counts by its spaces");
        assert_eq!(words("café naïve"), 2);
        assert_eq!(words(""), 0);
    }

    #[test]
    fn characters_keep_emoji_and_accents_together() {
        assert_eq!(characters("abc"), (3, 3));
        assert_eq!(characters("a b\r\nc"), (5, 3), "CR LF is one line break");
        assert_eq!(characters("e\u{301}"), (1, 1), "A combining accent joins its letter");
        assert_eq!(characters("👍🏽"), (1, 1), "A skin tone joins its emoji");
        assert_eq!(characters("👨‍👩‍👧"), (1, 1), "A family joined by zero-width joiners is one");
        assert_eq!(characters("🇺🇸🇫🇷"), (2, 2), "Flag pairs");
        assert_eq!(characters("❤️ ok"), (4, 3), "A variation selector joins its symbol");
    }

    #[test]
    fn sentences_and_paragraphs() {
        assert_eq!(sentences("Hello there. How are you? Fine!"), 3);
        assert_eq!(sentences("Pi is 3.14 today"), 1, "A decimal point is not an end");
        assert_eq!(sentences("He said \"stop.\" Then left"), 2);
        assert_eq!(sentences("Wait... what?!"), 2);
        assert_eq!(sentences("今日は晴れ。明日は雨。"), 2);
        assert_eq!(sentences("..."), 0);
        assert_eq!(paragraphs("one\n\n\ntwo\nstill two\n  \nthree\n"), 3);
        assert_eq!(paragraphs("one"), 1);
    }

    #[test]
    fn durations_round_up_to_a_second() {
        assert_eq!(duration(0, READING_WPM), "0 sec");
        assert_eq!(duration(1, READING_WPM), "1 sec");
        assert_eq!(duration(238, READING_WPM), "1 min");
        assert_eq!(duration(312, READING_WPM), "1 min 19 sec");
        assert_eq!(duration(150, SPEAKING_WPM), "1 min");
        assert_eq!(duration(13_050, SPEAKING_WPM), "1 hr 27 min");
        assert_eq!(duration(9_000, SPEAKING_WPM), "1 hr");
    }

    #[test]
    fn rows_are_titled_with_their_counts() {
        let rows = reply(serde_json::json!({"phase":"render","query":"One word. Two words!","now":0}));
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["4 words", "20 characters", "17 characters without spaces", "2 sentences", "1 paragraph", "1 line",
            "2 sec reading time", "2 sec speaking time"]);
        assert!(rows.iter().all(|row| row["subtitle"].as_str().unwrap().starts_with(FROM_TYPED)));
        assert!(rows.iter().all(|row| row.get("url").is_none()), "Without a link, Return copies the title");
        assert_eq!(rows[6]["subtitle"], "From typed text · At about 238 words per minute");
        assert_eq!(counted(1_840, CHARACTER), "1,840 characters");
        assert_eq!(counted(1, WORD), "1 word");
    }

    #[test]
    fn typed_text_wins_over_the_clipboard() {
        let typed = reply(serde_json::json!({"phase":"render","query":"one two","now":0,"clipboard":"a b c d e"}));
        assert_eq!(typed[0]["title"], "2 words");
        assert!(typed[0]["subtitle"].as_str().unwrap().starts_with(FROM_TYPED));
        let clipboard = reply(serde_json::json!({"phase":"render","query":"  ","now":0,"clipboard":"a b c d e"}));
        assert_eq!(clipboard[0]["title"], "5 words");
        assert!(clipboard.iter().all(|row| row["subtitle"].as_str().unwrap().starts_with(FROM_CLIPBOARD)));
        assert!(clipboard.iter().all(|row| !row["title"].as_str().unwrap().contains("a b c")), "The clipboard is never echoed");
    }

    #[test]
    fn empty_input_asks_for_text() {
        assert_eq!(titles(serde_json::json!({"phase":"render","query":"","now":0})), [PROMPT]);
        assert_eq!(titles(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":" \n "})), [PROMPT]);
        assert_eq!(titles(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":null})), [PROMPT]);
        assert_eq!(titles(serde_json::json!({"phase":"plan","query":"x","now":0})), [ONLY_LISTS]);
        assert_eq!(respond(b"not json"), items(message(NOT_UNDERSTOOD, None)));
    }

    #[test]
    fn long_and_emoji_text_stays_within_the_row_limit() {
        let long = "word ".repeat(13_000);
        let rows = reply(serde_json::json!({"phase":"render","query":"","now":0,"clipboard":long}));
        assert_eq!(rows[0]["title"], "13,000 words");
        assert!(rows.len() <= 100 && rows.iter().all(|row| row["title"].as_str().unwrap().chars().count() <= 300));
        let emoji = reply(serde_json::json!({"phase":"render","query":"🎉 party 🎉 time","now":0}));
        assert_eq!(emoji[0]["title"], "2 words");
        assert_eq!(emoji[1]["title"], "14 characters");
    }

    #[test]
    fn manifest_asks_only_for_the_clipboard() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard"]));
        assert!(manifest.get("hosts").is_none() && manifest.get("storage").is_none());
    }
}
