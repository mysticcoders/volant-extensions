//! Characters for Volant: typographic and special characters by name, such as the em dash, curly
//! quotes, arrows, math and currency signs, legal marks, fractions and the Mac's keyboard symbols.
//! It works offline: the manifest lists no hosts, so Volant sends one `render` request and shows the
//! rows. Typed text matches a character's name, its aliases, its group, its HTML entity, its code point
//! (`U+2014` or `2014`) or the character itself; with the field empty the common ones show at once.
//!
//! Each row's title is the character, so Return copies it, and its subtitle reads name, code point and
//! HTML entity. Volant trims whitespace from a row's title before showing or copying it, which would
//! leave a space row empty and drop it, and zero-width characters would show as a blank row. Those
//! invisible characters instead show and copy their HTML entity, such as `&nbsp;`, and say so.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const RENDER: &str = "render";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const NO_MATCH_START: &str = "No character matches “";
const NO_MATCH_END: &str = "”";
const NO_MATCH_HINT: &str = "Try a name such as em dash, arrow, degree or command";
const INVISIBLE_HINT: &str = "Invisible, so Return copies its HTML entity";
const SEPARATOR: &str = " · ";
const CODE_POINT_PREFIX: &str = "u+";
const ENTITY_START: &str = "&#x";
const ENTITY_END: &str = ";";
const MAXIMUM_ROWS: usize = 100;

/// The groups characters are listed in, named on each row's badge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Group { Dashes, Quotes, Punctuation, Bullets, Arrows, Math, Currency, Legal, Units, Fractions, Keyboard, Symbols, Spaces }

impl Group {
    /// The group's name, shown as the row's badge and matched by typed text.
    pub fn label(self) -> &'static str {
        match self {
            Group::Dashes => "Dashes",
            Group::Quotes => "Quotes",
            Group::Punctuation => "Punctuation",
            Group::Bullets => "Bullets",
            Group::Arrows => "Arrows",
            Group::Math => "Math",
            Group::Currency => "Currency",
            Group::Legal => "Legal",
            Group::Units => "Units",
            Group::Fractions => "Fractions",
            Group::Keyboard => "Keyboard",
            Group::Symbols => "Symbols",
            Group::Spaces => "Spaces",
        }
    }
}

/// One character: its text, its Unicode-style name, other names people search for (separated by
/// commas), its named HTML entity when HTML has one, its group, whether it shows with the field empty,
/// and whether it is invisible in a row, so the row shows and copies its entity instead.
pub struct Character {
    pub text: &'static str,
    pub name: &'static str,
    pub aliases: &'static str,
    pub entity: Option<&'static str>,
    pub group: Group,
    pub common: bool,
    pub invisible: bool,
}

/// Shorthand for a visible character in the list below.
const fn visible(text: &'static str, name: &'static str, aliases: &'static str, entity: Option<&'static str>, group: Group, common: bool) -> Character {
    Character { text, name, aliases, entity, group, common, invisible: false }
}

/// Shorthand for an invisible character, which rows show and copy as its HTML entity.
const fn invisible(text: &'static str, name: &'static str, aliases: &'static str, entity: Option<&'static str>, common: bool) -> Character {
    Character { text, name, aliases, entity, group: Group::Spaces, common, invisible: true }
}

use Group::*;

/// Every character, in the order rows show them.
pub const CHARACTERS: &[Character] = &[
    visible("—", "Em dash", "long dash, mdash", Some("&mdash;"), Dashes, true),
    visible("–", "En dash", "short dash, ndash, range", Some("&ndash;"), Dashes, true),
    visible("‒", "Figure dash", "numeric dash, phone number dash", None, Dashes, false),
    visible("−", "Minus sign", "subtract, negative", Some("&minus;"), Dashes, true),
    visible("‐", "Hyphen", "unicode hyphen", Some("&dash;"), Dashes, false),
    visible("‑", "Non-breaking hyphen", "no-break hyphen, nbhy", None, Dashes, false),
    visible("―", "Horizontal bar", "quotation dash, quote dash", Some("&horbar;"), Dashes, false),
    visible("‘", "Left single quotation mark", "curly single quote, opening single quote, smart quote", Some("&lsquo;"), Quotes, true),
    visible("’", "Right single quotation mark", "curly apostrophe, closing single quote, smart apostrophe", Some("&rsquo;"), Quotes, true),
    visible("“", "Left double quotation mark", "curly double quote, opening double quote, smart quote", Some("&ldquo;"), Quotes, true),
    visible("”", "Right double quotation mark", "curly double quote, closing double quote, smart quote", Some("&rdquo;"), Quotes, true),
    visible("‚", "Single low-9 quotation mark", "low single quote, german quote", Some("&sbquo;"), Quotes, false),
    visible("„", "Double low-9 quotation mark", "low double quote, german quote", Some("&bdquo;"), Quotes, false),
    visible("′", "Prime", "feet, minutes, arcminute", Some("&prime;"), Quotes, true),
    visible("″", "Double prime", "inches, seconds, arcsecond", Some("&Prime;"), Quotes, true),
    visible("‴", "Triple prime", "", Some("&tprime;"), Quotes, false),
    visible("«", "Left-pointing double angle quotation mark", "guillemet, french quote, opening guillemet", Some("&laquo;"), Quotes, true),
    visible("»", "Right-pointing double angle quotation mark", "guillemet, french quote, closing guillemet", Some("&raquo;"), Quotes, true),
    visible("‹", "Single left-pointing angle quotation mark", "single guillemet", Some("&lsaquo;"), Quotes, false),
    visible("›", "Single right-pointing angle quotation mark", "single guillemet", Some("&rsaquo;"), Quotes, false),
    visible("…", "Horizontal ellipsis", "ellipsis, dots, three dots", Some("&hellip;"), Punctuation, true),
    visible("¿", "Inverted question mark", "spanish question", Some("&iquest;"), Punctuation, false),
    visible("¡", "Inverted exclamation mark", "spanish exclamation", Some("&iexcl;"), Punctuation, false),
    visible("‽", "Interrobang", "question exclamation", None, Punctuation, false),
    visible("†", "Dagger", "obelisk, footnote", Some("&dagger;"), Punctuation, false),
    visible("‡", "Double dagger", "footnote", Some("&Dagger;"), Punctuation, false),
    visible("№", "Numero sign", "number sign, no.", Some("&numero;"), Punctuation, false),
    visible("•", "Bullet", "dot, list", Some("&bull;"), Bullets, true),
    visible("·", "Middle dot", "interpunct, centered dot, separator", Some("&middot;"), Bullets, true),
    visible("‣", "Triangular bullet", "", None, Bullets, false),
    visible("◦", "White bullet", "hollow bullet, circle bullet", None, Bullets, false),
    visible("⁃", "Hyphen bullet", "", Some("&hybull;"), Bullets, false),
    visible("●", "Black circle", "filled circle, large dot", None, Bullets, false),
    visible("→", "Rightwards arrow", "right arrow", Some("&rarr;"), Arrows, true),
    visible("←", "Leftwards arrow", "left arrow", Some("&larr;"), Arrows, true),
    visible("↑", "Upwards arrow", "up arrow", Some("&uarr;"), Arrows, true),
    visible("↓", "Downwards arrow", "down arrow", Some("&darr;"), Arrows, true),
    visible("↔", "Left right arrow", "both ways arrow, horizontal arrow", Some("&harr;"), Arrows, false),
    visible("↕", "Up down arrow", "vertical arrow", Some("&varr;"), Arrows, false),
    visible("⇒", "Rightwards double arrow", "implies, double right arrow", Some("&rArr;"), Arrows, false),
    visible("⇐", "Leftwards double arrow", "double left arrow", Some("&lArr;"), Arrows, false),
    visible("⇔", "Left right double arrow", "if and only if, iff", Some("&hArr;"), Arrows, false),
    visible("↗", "North east arrow", "up right arrow, diagonal", Some("&nearr;"), Arrows, false),
    visible("↘", "South east arrow", "down right arrow, diagonal", Some("&searr;"), Arrows, false),
    visible("↖", "North west arrow", "up left arrow, diagonal", Some("&nwarr;"), Arrows, false),
    visible("↙", "South west arrow", "down left arrow, diagonal", Some("&swarr;"), Arrows, false),
    visible("×", "Multiplication sign", "times, multiply, by, dimensions", Some("&times;"), Math, true),
    visible("÷", "Division sign", "divide, obelus", Some("&divide;"), Math, true),
    visible("±", "Plus-minus sign", "plus or minus, tolerance", Some("&plusmn;"), Math, true),
    visible("≠", "Not equal to", "unequal, not equals", Some("&ne;"), Math, true),
    visible("≈", "Almost equal to", "approximately, about", Some("&asymp;"), Math, true),
    visible("≤", "Less-than or equal to", "less than or equal", Some("&le;"), Math, true),
    visible("≥", "Greater-than or equal to", "greater than or equal", Some("&ge;"), Math, true),
    visible("∞", "Infinity", "infinite, lemniscate", Some("&infin;"), Math, true),
    visible("√", "Square root", "radical, root", Some("&radic;"), Math, true),
    visible("≡", "Identical to", "equivalent, congruent, triple bar", Some("&equiv;"), Math, false),
    visible("∑", "N-ary summation", "sum, sigma", Some("&sum;"), Math, false),
    visible("∏", "N-ary product", "product, pi", Some("&prod;"), Math, false),
    visible("∆", "Increment", "delta, change, difference", None, Math, false),
    visible("∂", "Partial differential", "partial derivative, del", Some("&part;"), Math, false),
    visible("∫", "Integral", "integration", Some("&int;"), Math, false),
    visible("π", "Greek small letter pi", "pi", Some("&pi;"), Math, false),
    visible("µ", "Micro sign", "micro, mu", Some("&micro;"), Math, false),
    visible("‰", "Per mille sign", "per thousand, permille", Some("&permil;"), Math, false),
    visible("¬", "Not sign", "negation, logical not", Some("&not;"), Math, false),
    visible("∅", "Empty set", "null set", Some("&empty;"), Math, false),
    visible("∈", "Element of", "in, member of", Some("&isin;"), Math, false),
    visible("⁄", "Fraction slash", "solidus", Some("&frasl;"), Math, false),
    visible("€", "Euro sign", "eur, euro", Some("&euro;"), Currency, true),
    visible("£", "Pound sign", "gbp, sterling, pound", Some("&pound;"), Currency, true),
    visible("¥", "Yen sign", "jpy, yuan, cny, yen", Some("&yen;"), Currency, true),
    visible("¢", "Cent sign", "cents, penny", Some("&cent;"), Currency, true),
    visible("₹", "Indian rupee sign", "inr, rupee", None, Currency, false),
    visible("₩", "Won sign", "krw, korean won", None, Currency, false),
    visible("₽", "Ruble sign", "rub, rouble", None, Currency, false),
    visible("₺", "Turkish lira sign", "try, lira", None, Currency, false),
    visible("₪", "New sheqel sign", "ils, shekel", None, Currency, false),
    visible("₿", "Bitcoin sign", "btc, bitcoin", None, Currency, false),
    visible("¤", "Currency sign", "generic currency", Some("&curren;"), Currency, false),
    visible("©", "Copyright sign", "copyright, c in circle", Some("&copy;"), Legal, true),
    visible("®", "Registered sign", "registered trademark, r in circle", Some("&reg;"), Legal, true),
    visible("™", "Trade mark sign", "trademark, tm", Some("&trade;"), Legal, true),
    visible("℠", "Service mark", "sm", None, Legal, false),
    visible("℗", "Sound recording copyright", "phonogram, p in circle", Some("&copysr;"), Legal, false),
    visible("§", "Section sign", "section, silcrow", Some("&sect;"), Legal, true),
    visible("¶", "Pilcrow sign", "paragraph, paragraph mark", Some("&para;"), Legal, true),
    visible("°", "Degree sign", "degrees, temperature, angle", Some("&deg;"), Units, true),
    visible("℃", "Degree Celsius", "celsius, centigrade", None, Units, false),
    visible("℉", "Degree Fahrenheit", "fahrenheit", None, Units, false),
    visible("¹", "Superscript one", "first power", Some("&sup1;"), Units, false),
    visible("²", "Superscript two", "squared, second power", Some("&sup2;"), Units, false),
    visible("³", "Superscript three", "cubed, third power", Some("&sup3;"), Units, false),
    visible("½", "Vulgar fraction one half", "half, 1/2", Some("&frac12;"), Fractions, true),
    visible("⅓", "Vulgar fraction one third", "third, 1/3", Some("&frac13;"), Fractions, false),
    visible("⅔", "Vulgar fraction two thirds", "two thirds, 2/3", Some("&frac23;"), Fractions, false),
    visible("¼", "Vulgar fraction one quarter", "quarter, 1/4", Some("&frac14;"), Fractions, true),
    visible("¾", "Vulgar fraction three quarters", "three quarters, 3/4", Some("&frac34;"), Fractions, true),
    visible("⅕", "Vulgar fraction one fifth", "fifth, 1/5", Some("&frac15;"), Fractions, false),
    visible("⅛", "Vulgar fraction one eighth", "eighth, 1/8", Some("&frac18;"), Fractions, false),
    visible("⅜", "Vulgar fraction three eighths", "three eighths, 3/8", Some("&frac38;"), Fractions, false),
    visible("⅝", "Vulgar fraction five eighths", "five eighths, 5/8", Some("&frac58;"), Fractions, false),
    visible("⅞", "Vulgar fraction seven eighths", "seven eighths, 7/8", Some("&frac78;"), Fractions, false),
    visible("⌘", "Place of interest sign", "command, cmd, command key", None, Keyboard, true),
    visible("⌥", "Option key", "option, alt, alt key", None, Keyboard, true),
    visible("⇧", "Upwards white arrow", "shift, shift key", None, Keyboard, true),
    visible("⌃", "Up arrowhead", "control, ctrl, control key", None, Keyboard, true),
    visible("⏎", "Return symbol", "return, enter, return key", None, Keyboard, true),
    visible("↩", "Leftwards arrow with hook", "return, enter", Some("&larrhk;"), Keyboard, false),
    visible("⌫", "Erase to the left", "delete, backspace, delete key", None, Keyboard, true),
    visible("⌦", "Erase to the right", "forward delete, del", None, Keyboard, false),
    visible("⇥", "Rightwards arrow to bar", "tab, tab key", Some("&rarrb;"), Keyboard, true),
    visible("⇤", "Leftwards arrow to bar", "back tab, shift tab", Some("&larrb;"), Keyboard, false),
    visible("⎋", "Broken circle with northwest arrow", "escape, esc, escape key", None, Keyboard, true),
    visible("⇪", "Upwards white arrow from bar", "caps lock", None, Keyboard, false),
    visible("⏏", "Eject symbol", "eject, media eject", None, Keyboard, false),
    visible("␣", "Open box", "visible space, space bar, spacebar", Some("&blank;"), Keyboard, false),
    visible("✓", "Check mark", "tick, checkmark, done, yes", Some("&check;"), Symbols, true),
    visible("✗", "Ballot X", "cross, no, wrong", Some("&cross;"), Symbols, false),
    visible("★", "Black star", "filled star, rating", Some("&starf;"), Symbols, true),
    visible("☆", "White star", "hollow star, empty star", Some("&star;"), Symbols, false),
    visible("♥", "Black heart suit", "heart, love", Some("&hearts;"), Symbols, false),
    visible("♠", "Black spade suit", "spade, cards", Some("&spades;"), Symbols, false),
    visible("♣", "Black club suit", "club, cards", Some("&clubs;"), Symbols, false),
    visible("♦", "Black diamond suit", "diamond, cards", Some("&diams;"), Symbols, false),
    visible("¦", "Broken bar", "pipe, vertical bar", Some("&brvbar;"), Symbols, false),
    invisible("\u{00A0}", "No-break space", "nbsp, non-breaking space", Some("&nbsp;"), true),
    invisible("\u{202F}", "Narrow no-break space", "narrow nbsp, nnbsp", None, false),
    invisible("\u{2009}", "Thin space", "thinsp", Some("&thinsp;"), true),
    invisible("\u{200A}", "Hair space", "hairsp, very thin space", Some("&hairsp;"), true),
    invisible("\u{2002}", "En space", "ensp, nut", Some("&ensp;"), false),
    invisible("\u{2003}", "Em space", "emsp, mutton, quad", Some("&emsp;"), false),
    invisible("\u{2007}", "Figure space", "numsp, digit width space", Some("&numsp;"), false),
    invisible("\u{2008}", "Punctuation space", "puncsp", Some("&puncsp;"), false),
    invisible("\u{200B}", "Zero width space", "zwsp, invisible space", Some("&ZeroWidthSpace;"), false),
    invisible("\u{200D}", "Zero width joiner", "zwj", Some("&zwj;"), false),
    invisible("\u{200C}", "Zero width non-joiner", "zwnj", Some("&zwnj;"), false),
    invisible("\u{2060}", "Word joiner", "wj, no break", Some("&NoBreak;"), false),
    invisible("\u{00AD}", "Soft hyphen", "shy, discretionary hyphen, optional hyphen", Some("&shy;"), false),
];

impl Character {
    /// The code point, such as `U+2014`.
    pub fn code_point(&self) -> String {
        self.text.chars().next().map(|c| format!("U+{:04X}", c as u32)).unwrap_or_default()
    }

    /// The named HTML entity, or the numeric one when HTML names none.
    pub fn html_entity(&self) -> String {
        match self.entity {
            Some(entity) => entity.to_string(),
            None => self.text.chars().next().map(|c| format!("{ENTITY_START}{:X}{ENTITY_END}", c as u32)).unwrap_or_default(),
        }
    }

    /// How well the lowercase `query` matches, lower being better, or None when it does not. Every word
    /// of the query must appear in the name, aliases, group or entity.
    fn rank(&self, query: &str) -> Option<u8> {
        let name = self.name.to_lowercase();
        let aliases: Vec<String> = self.aliases.split(',').map(|alias| alias.trim().to_lowercase()).filter(|alias| !alias.is_empty()).collect();
        if query == self.text || matches_code_point(query, self) { return Some(0); }
        if name == query || aliases.iter().any(|alias| alias == query) { return Some(1); }
        if name.starts_with(query) || aliases.iter().any(|alias| alias.starts_with(query)) { return Some(2); }
        let haystack = [name.as_str(), &aliases.join(" "), &self.group.label().to_lowercase(), &self.html_entity().to_lowercase()].join(" ");
        query.split_whitespace().all(|word| haystack.contains(word)).then_some(3)
    }
}

/// Whether `query` is the character's code point, written `U+2014` or `2014` in any case.
fn matches_code_point(query: &str, character: &Character) -> bool {
    let digits = query.strip_prefix(CODE_POINT_PREFIX).unwrap_or(query);
    if !(4..=6).contains(&digits.len()) || !digits.chars().all(|c| c.is_ascii_hexdigit()) { return false; }
    let wanted = u32::from_str_radix(digits, 16).ok();
    character.text.chars().next().map(|c| c as u32) == wanted
}

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
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

/// A character's row: the character as its title, so Return copies it, or its entity when invisible.
fn row(character: &Character) -> Row {
    let entity = character.html_entity();
    let mut details = vec![character.name.to_string(), character.code_point(), entity.clone()];
    if character.invisible { details.push(INVISIBLE_HINT.to_string()); }
    Row {
        title: if character.invisible { entity } else { character.text.to_string() },
        subtitle: Some(details.join(SEPARATOR)),
        badges: vec![Badge { text: character.group.label().to_string() }],
    }
}

/// The characters for a query, best first: the common ones when it is empty.
pub fn matches(query: &str) -> Vec<&'static Character> {
    let query = query.trim();
    if query.is_empty() { return CHARACTERS.iter().filter(|character| character.common).take(MAXIMUM_ROWS).collect(); }
    let lower = query.to_lowercase();
    let exact = CHARACTERS.iter().filter_map(|character| {
        let rank = if character.text == query { Some(0) } else { character.rank(&lower) };
        rank.map(|rank| (rank, character))
    });
    let mut found: Vec<(u8, &'static Character)> = exact.collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, character)| character).take(MAXIMUM_ROWS).collect()
}

fn message(title: String, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title, subtitle: subtitle.map(str::to_string), ..Default::default() }]
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(message(NOT_UNDERSTOOD.to_string(), None)) };
    if request.phase != RENDER { return items(message(ONLY_LISTS.to_string(), None)); }
    let found = matches(&request.query);
    if found.is_empty() {
        let title = [NO_MATCH_START, request.query.trim(), NO_MATCH_END].concat();
        return items(message(title, Some(NO_MATCH_HINT)));
    }
    items(found.into_iter().map(row).collect())
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../manifest.json");

    /// The characters Swift's `trimmingCharacters(in: .whitespacesAndNewlines)`, which Volant applies to
    /// every row title, removes entirely, checked against Volant's own code on macOS.
    const TRIMMED_BY_VOLANT: [char; 10] = ['\u{00A0}', '\u{202F}', '\u{2002}', '\u{2003}', '\u{2009}', '\u{200A}', '\u{2007}', '\u{2008}', '\u{200B}', '\u{3000}'];

    fn reply(query: &str) -> Vec<serde_json::Value> {
        let request = serde_json::json!({"phase":"render","query":query,"now":0});
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    #[test]
    fn the_empty_field_shows_the_common_characters() {
        let rows = reply("");
        assert_eq!(rows.len(), CHARACTERS.iter().filter(|c| c.common).count());
        assert!(rows.len() <= MAXIMUM_ROWS && rows.len() > 40);
        assert_eq!(rows[0]["title"], "—");
        assert_eq!(rows[0]["subtitle"], "Em dash · U+2014 · &mdash;");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"Dashes"}]));
        assert!(rows.iter().all(|row| row.get("url").is_none()), "Without a link, Return copies the title");
    }

    #[test]
    fn names_aliases_and_code_points_find_characters() {
        assert_eq!(reply("em dash")[0]["title"], "—");
        assert_eq!(reply("EN DASH")[0]["title"], "–");
        assert_eq!(reply("command")[0]["title"], "⌘");
        assert_eq!(reply("shift")[0]["title"], "⇧");
        assert_eq!(reply("U+2212")[0]["title"], "−");
        assert_eq!(reply("00b0")[0]["title"], "°");
        assert_eq!(reply("×")[0]["subtitle"], "Multiplication sign · U+00D7 · &times;");
        assert_eq!(reply("trademark")[0]["title"], "™");
        assert_eq!(reply("guillemet").len(), 4);
        assert_eq!(reply("rupee")[0]["subtitle"], "Indian rupee sign · U+20B9 · &#x20B9;");
        assert_eq!(reply("frac12")[0]["title"], "½");
        let quotes: Vec<String> = reply("curly double").iter().map(|row| row["title"].as_str().unwrap().to_string()).collect();
        assert_eq!(quotes, ["“", "”"]);
    }

    #[test]
    fn invisible_characters_show_and_copy_their_entity() {
        let rows = reply("nbsp");
        assert_eq!(rows[0]["title"], "&nbsp;");
        assert_eq!(rows[0]["subtitle"], "No-break space · U+00A0 · &nbsp; · Invisible, so Return copies its HTML entity");
        assert_eq!(reply("narrow no-break")[0]["title"], "&#x202F;");
        assert_eq!(reply("zwj")[0]["title"], "&zwj;");
        for character in CHARACTERS {
            let c = character.text.chars().next().unwrap();
            assert_eq!(character.text.chars().count(), 1, "{}", character.name);
            if TRIMMED_BY_VOLANT.contains(&c) || c.is_whitespace() { assert!(character.invisible, "{} would be trimmed away", character.name); }
            if character.invisible { assert!(row(character).title.starts_with('&')); }
        }
    }

    #[test]
    fn every_character_is_listed_once_with_a_well_formed_entity() {
        let mut seen = std::collections::HashSet::new();
        for character in CHARACTERS {
            assert!(seen.insert(character.text), "{} is listed twice", character.name);
            let entity = character.html_entity();
            assert!(entity.starts_with('&') && entity.ends_with(';') && entity.len() <= 32, "{entity}");
            assert!(!character.name.is_empty() && character.name.len() <= 100);
        }
    }

    #[test]
    fn nothing_found_and_other_requests() {
        let rows = reply("zzzz qqq");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["title"], "No character matches “zzzz qqq”");
        assert_eq!(rows[0]["subtitle"], NO_MATCH_HINT);
        let plan: serde_json::Value = serde_json::from_str(&respond(br#"{"phase":"plan","query":"x","now":0}"#)).unwrap();
        assert_eq!(plan["items"][0]["title"], ONLY_LISTS);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_lists_no_hosts_or_permissions() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert!(manifest.get("hosts").is_none() && manifest.get("permissions").is_none());
        assert_eq!(manifest["mode"], "list");
    }
}
