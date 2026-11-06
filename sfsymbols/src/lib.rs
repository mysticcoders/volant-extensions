//! SF Symbols for Volant: searches the names of Apple's SF Symbols offline, so the name you need for
//! `Image(systemName:)` is a Return away. The manifest lists no hosts, so Volant sends one `render`
//! request and shows the rows. Each row's title is a symbol name, so Return copies it; its subtitle
//! gives the symbol's categories, the iOS and macOS versions that introduced it, any older names it
//! replaced, the language of a localized variant, and what a restricted symbol may only refer to.
//!
//! The list is `symbols.tsv`, written by `generate.py` from the metadata macOS ships in its CoreGlyphs
//! bundle: names, releases, categories and search keywords only, never the glyphs. Volant draws only
//! its own small set of row icons, so rows cannot show the symbols themselves.
//!
//! Typed text matches names first (exact, then a name starting with it, then a dotted part starting
//! with it, then anywhere in a name), then Apple's search keywords, categories and former names, with
//! every word required. Spaces count as dots, so `heart fill` finds `heart.fill`. Localized variants
//! such as `character.ar` come after the rest, and shorter names first within each kind of match. An
//! exact name adds rows with its SwiftUI and UIKit code, ready to copy.
use serde::{Deserialize, Serialize};
use volant_extension_kit::volant_extension_output;

const SYMBOLS: &str = include_str!("symbols.tsv");
const RELEASE_LINE: &str = "@release";
const CATEGORY_LINE: &str = "@category";
const COMMENT: char = '#';
const DIRECTIVE: char = '@';
const FIELD: char = '\t';
const LIST: char = ',';
const DOT: &str = ".";
const IOS: &str = "iOS ";
const MACOS: &str = "macOS ";
const RENDER: &str = "render";
const PROMPT: &str = "Type part of a symbol name, such as heart or arrow.right.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const NO_MATCH_START: &str = "No SF Symbol matches “";
const NO_MATCH_END: &str = "”";
const NO_MATCH_HINT: &str = "Names use dots between words, such as arrow.right.circle";
const FORMERLY: &str = "Formerly ";
const LOCALIZED: &str = "Localized for ";
const RESTRICTED_START: &str = "Use only to refer to ";
const RESTRICTED_GENERIC: &str = "restricted";
const RESTRICTED_BADGE: &str = "Restricted";
const RESTRICTED_FALLBACK: &str = "Restricted by Apple";
const SWIFTUI_START: &str = "Image(systemName: \"";
const SWIFTUI_END: &str = "\")";
const SWIFTUI_HINT: &str = "SwiftUI · Return copies the code";
const UIKIT_START: &str = "UIImage(systemName: \"";
const UIKIT_END: &str = "\")";
const UIKIT_HINT: &str = "UIKit · Return copies the code";
const APPKIT_START: &str = "NSImage(systemSymbolName: \"";
const APPKIT_END: &str = "\", accessibilityDescription: nil)";
const APPKIT_HINT: &str = "AppKit · Return copies the code";
const SEPARATOR: &str = " · ";
const LIST_SEPARATOR: &str = ", ";
const MAXIMUM_ROWS: usize = 100;
const LOCALIZED_PENALTY: u8 = 10;

/// The last part of a localized variant's name and the script or language it is drawn for.
const LOCALES: &[(&str, &str)] = &[
    ("ar", "Arabic"), ("hi", "Hindi"), ("rtl", "right-to-left text"), ("he", "Hebrew"), ("ja", "Japanese"), ("th", "Thai"),
    ("zh", "Chinese"), ("bn", "Bangla"), ("gu", "Gujarati"), ("kn", "Kannada"), ("ko", "Korean"), ("ml", "Malayalam"),
    ("mni", "Manipuri"), ("mr", "Marathi"), ("or", "Odia"), ("pa", "Punjabi"), ("sat", "Santali"), ("te", "Telugu"),
    ("si", "Sinhala"), ("ta", "Tamil"), ("el", "Greek"), ("ru", "Russian"), ("km", "Khmer"), ("my", "Burmese"),
];

/// One symbol from `symbols.tsv`, borrowing its text.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol<'a> {
    pub name: &'a str,
    pub release: &'a str,
    pub categories: &'a str,
    pub keywords: &'a str,
    pub restriction: &'a str,
    pub formerly: &'a str,
}

/// The releases and category titles declared at the top of `symbols.tsv`.
struct Tables<'a> {
    releases: Vec<(&'a str, &'a str, &'a str)>,
    categories: Vec<(&'a str, &'a str)>,
}

/// Every symbol and the tables in `text`, in file order. Lines that are not well formed are skipped.
fn parse(text: &str) -> (Tables<'_>, Vec<Symbol<'_>>) {
    let mut tables = Tables { releases: Vec::new(), categories: Vec::new() };
    let mut symbols = Vec::new();
    for line in text.lines().filter(|line| !line.is_empty() && !line.starts_with(COMMENT)) {
        let fields: Vec<&str> = line.split(FIELD).collect();
        if line.starts_with(DIRECTIVE) {
            match fields.as_slice() {
                [RELEASE_LINE, key, versions @ ..] => {
                    let version = |prefix: &str| versions.iter().find_map(|v| v.strip_prefix(prefix)).unwrap_or_default();
                    tables.releases.push((key, version(IOS), version(MACOS)));
                }
                [CATEGORY_LINE, key, title] => tables.categories.push((key, title)),
                _ => {}
            }
            continue;
        }
        if let [name, release, categories, keywords, restriction, formerly] = fields.as_slice() {
            symbols.push(Symbol { name, release, categories, keywords, restriction, formerly });
        }
    }
    (tables, symbols)
}

impl Symbol<'_> {
    /// The language a localized variant is drawn for, when its last part names one and the name
    /// without it is a symbol too.
    fn locale(&self, names: &std::collections::HashSet<&str>) -> Option<&'static str> {
        let (base, last) = self.name.rsplit_once('.')?;
        let language = LOCALES.iter().find(|(suffix, _)| *suffix == last).map(|(_, language)| *language)?;
        names.contains(base).then_some(language)
    }

    /// How well the normalized `query` matches, lower being better, or None when it does not.
    fn rank(&self, query: &str, words: &[&str], tables: &Tables) -> Option<u8> {
        let formerly = || self.formerly.split(LIST);
        if self.name == query || formerly().any(|old| old == query) { return Some(0); }
        if self.name.starts_with(query) { return Some(1); }
        if self.name.split('.').any(|part| part.starts_with(query)) && self.name.contains(&[DOT, query].concat()) { return Some(2); }
        if self.name.contains(query) { return Some(3); }
        let categories: Vec<String> = self.categories.split(LIST)
            .filter_map(|key| tables.categories.iter().find(|(k, _)| *k == key).map(|(_, title)| title.to_lowercase())).collect();
        let haystack = [self.name, self.keywords, self.formerly, &categories.join(" ")].join(" ");
        words.iter().all(|word| haystack.contains(word)).then_some(4)
    }
}

/// The symbols matching `query`, best first, at most `MAXIMUM_ROWS`, each with its locale if localized.
fn find<'a, 'b>(query: &str, tables: &Tables, symbols: &'b [Symbol<'a>]) -> Vec<(&'b Symbol<'a>, Option<&'static str>)> {
    let lower = query.trim().to_lowercase();
    let words: Vec<&str> = lower.split(|c: char| c.is_whitespace() || c == '.').filter(|word| !word.is_empty()).collect();
    if words.is_empty() { return Vec::new(); }
    let normalized = words.join(DOT);
    let names: std::collections::HashSet<&str> = symbols.iter().map(|symbol| symbol.name).collect();
    let mut found: Vec<(u8, usize, usize, &Symbol, Option<&'static str>)> = symbols.iter().enumerate().filter_map(|(index, symbol)| {
        let rank = symbol.rank(&normalized, &words, tables)?;
        let locale = symbol.locale(&names);
        let rank = if locale.is_some() { rank + LOCALIZED_PENALTY } else { rank };
        Some((rank, symbol.name.len(), index, symbol, locale))
    }).collect();
    found.sort_by_key(|(rank, length, index, ..)| (*rank, *length, *index));
    found.into_iter().take(MAXIMUM_ROWS).map(|(.., symbol, locale)| (symbol, locale)).collect()
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

/// A symbol's row: its name as the title, so Return copies it, and what is known about it.
fn row(symbol: &Symbol, locale: Option<&str>, tables: &Tables) -> Row {
    let mut details: Vec<String> = symbol.categories.split(LIST)
        .filter_map(|key| tables.categories.iter().find(|(k, _)| *k == key).map(|(_, title)| title.to_string())).collect();
    if let Some((_, ios, macos)) = tables.releases.iter().find(|(key, ..)| *key == symbol.release) {
        details.push([[IOS, ios].concat(), [MACOS, macos].concat()].join(LIST_SEPARATOR));
    }
    if !symbol.formerly.is_empty() { details.push([FORMERLY, &symbol.formerly.split(LIST).collect::<Vec<_>>().join(LIST_SEPARATOR)].concat()); }
    if let Some(language) = locale { details.push([LOCALIZED, language].concat()); }
    let mut badges = Vec::new();
    match symbol.restriction {
        "" => {}
        RESTRICTED_GENERIC => { details.push(RESTRICTED_FALLBACK.to_string()); badges.push(Badge { text: RESTRICTED_BADGE.to_string() }); }
        feature => { details.push([RESTRICTED_START, feature].concat()); badges.push(Badge { text: RESTRICTED_BADGE.to_string() }); }
    }
    Row { title: symbol.name.to_string(), subtitle: Some(details.join(SEPARATOR)), badges }
}

/// Rows with the code that draws `name` in SwiftUI, UIKit and AppKit, each copied by Return.
fn code_rows(name: &str) -> [Row; 3] {
    let code = |start: &str, end: &str, hint: &str| Row { title: [start, name, end].concat(), subtitle: Some(hint.to_string()), ..Default::default() };
    [code(SWIFTUI_START, SWIFTUI_END, SWIFTUI_HINT), code(UIKIT_START, UIKIT_END, UIKIT_HINT), code(APPKIT_START, APPKIT_END, APPKIT_HINT)]
}

fn rows(query: &str) -> Vec<Row> {
    let (tables, symbols) = parse(SYMBOLS);
    let found = find(query, &tables, &symbols);
    let normalized = query.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(DOT);
    let mut rows = Vec::new();
    for (index, (symbol, locale)) in found.iter().enumerate() {
        rows.push(row(symbol, *locale, &tables));
        if index == 0 && symbol.name == normalized { rows.extend(code_rows(symbol.name)); }
    }
    rows.truncate(MAXIMUM_ROWS);
    rows
}

fn message(title: String, subtitle: Option<&str>) -> Vec<Row> {
    vec![Row { title, subtitle: subtitle.map(str::to_string), ..Default::default() }]
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(message(NOT_UNDERSTOOD.to_string(), None)) };
    if request.phase != RENDER { return items(message(ONLY_LISTS.to_string(), None)); }
    if request.query.trim().is_empty() { return items(message(PROMPT.to_string(), None)); }
    let found = rows(&request.query);
    if found.is_empty() {
        return items(message([NO_MATCH_START, request.query.trim(), NO_MATCH_END].concat(), Some(NO_MATCH_HINT)));
    }
    items(found)
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = include_str!("../manifest.json");

    /// A fictional slice of `symbols.tsv` in its exact format.
    const SAMPLE: &str = "# comment\n@release\t2019\tiOS 13.0\tmacOS 10.15\ttvOS 13.0\n@release\t2024\tiOS 18.0\tmacOS 15.0\n\
        @category\thealth\tHealth\n@category\tshapes\tShapes\n\
        star\t2019\tshapes\tfavorite,rating\t\t\n\
        star.fill\t2019\tshapes\tfavorite\t\tstar.filled\n\
        star.ar\t2024\tshapes\t\t\t\n\
        megastar.circle\t2024\tshapes\t\t\t\n\
        wavy.thing\t2024\thealth\tsparkle\tApple’s Example feature\t\n\
        broken line without fields\n";

    fn titles(query: &str) -> Vec<String> {
        let (tables, symbols) = parse(SAMPLE);
        find(query, &tables, &symbols).into_iter().map(|(symbol, _)| symbol.name.to_string()).collect()
    }

    fn reply(query: &str) -> Vec<serde_json::Value> {
        let request = serde_json::json!({"phase":"render","query":query,"now":0});
        let reply: serde_json::Value = serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap();
        reply["items"].as_array().unwrap().clone()
    }

    #[test]
    fn the_sample_parses_into_tables_and_symbols() {
        let (tables, symbols) = parse(SAMPLE);
        assert_eq!(tables.releases, [("2019", "13.0", "10.15"), ("2024", "18.0", "15.0")]);
        assert_eq!(tables.categories, [("health", "Health"), ("shapes", "Shapes")]);
        assert_eq!(symbols.len(), 5, "The malformed line is skipped");
        assert_eq!(symbols[1].formerly, "star.filled");
    }

    #[test]
    fn names_rank_before_keywords_and_localized_variants_come_last() {
        assert_eq!(titles("star"), ["star", "star.fill", "megastar.circle", "star.ar"]);
        assert_eq!(titles("star fill"), ["star.fill"]);
        assert_eq!(titles("star.filled"), ["star.fill"], "A former name finds the symbol");
        assert_eq!(titles("favorite"), ["star", "star.fill"]);
        assert_eq!(titles("health"), ["wavy.thing"], "Category titles match");
        assert_eq!(titles("circle"), ["megastar.circle"]);
        assert!(titles("nothing here").is_empty());
        assert!(titles("  ").is_empty());
    }

    #[test]
    fn rows_describe_categories_releases_and_restrictions() {
        let (tables, symbols) = parse(SAMPLE);
        let names: std::collections::HashSet<&str> = symbols.iter().map(|symbol| symbol.name).collect();
        assert_eq!(row(&symbols[1], None, &tables).subtitle.unwrap(), "Shapes · iOS 13.0, macOS 10.15 · Formerly star.filled");
        assert_eq!(symbols[2].locale(&names), Some("Arabic"));
        assert_eq!(symbols[3].locale(&names), None);
        assert_eq!(row(&symbols[2], Some("Arabic"), &tables).subtitle.unwrap(), "Shapes · iOS 18.0, macOS 15.0 · Localized for Arabic");
        let restricted = row(&symbols[4], None, &tables);
        assert_eq!(restricted.subtitle.unwrap(), "Health · iOS 18.0, macOS 15.0 · Use only to refer to Apple’s Example feature");
        assert_eq!(restricted.badges, [Badge { text: "Restricted".into() }]);
    }

    #[test]
    fn the_embedded_list_finds_real_symbols() {
        let rows = reply("heart");
        assert_eq!(rows[0]["title"], "heart");
        assert!(rows[0]["subtitle"].as_str().unwrap().contains("iOS 13.0, macOS 10.15"));
        assert_eq!(rows[1]["title"], "Image(systemName: \"heart\")");
        assert_eq!(rows[2]["title"], "UIImage(systemName: \"heart\")");
        assert_eq!(rows[3]["title"], "NSImage(systemSymbolName: \"heart\", accessibilityDescription: nil)");
        assert!(rows.len() <= MAXIMUM_ROWS);
        assert!(rows.iter().all(|row| row.get("url").is_none()), "Without a link, Return copies the title");
        assert_eq!(reply("arrow right circle")[0]["title"], "arrow.right.circle");
        assert_eq!(reply("heart.fill")[0]["title"], "heart.fill");
        let restricted = reply("airplay.audio");
        assert_eq!(restricted[0]["badges"], serde_json::json!([{"text":"Restricted"}]));
        assert!(restricted[0]["subtitle"].as_str().unwrap().ends_with("Use only to refer to Apple’s AirPlay"), "{}", restricted[0]["subtitle"]);
        let (_, symbols) = parse(SYMBOLS);
        assert!(symbols.len() > 5000, "The generated list has {} symbols", symbols.len());
        assert!(symbols.iter().all(|symbol| symbol.name.len() <= 300 && !symbol.name.contains(' ')));
    }

    #[test]
    fn prompts_nothing_found_and_other_requests() {
        assert_eq!(reply("")[0]["title"], PROMPT);
        let none = reply("qqqzzzxx");
        assert_eq!(none[0]["title"], "No SF Symbol matches “qqqzzzxx”");
        assert_eq!(none[0]["subtitle"], NO_MATCH_HINT);
        let plan: serde_json::Value = serde_json::from_str(&respond(br#"{"phase":"plan","query":"x","now":0}"#)).unwrap();
        assert_eq!(plan["items"][0]["title"], ONLY_LISTS);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_lists_no_hosts_or_permissions() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert!(manifest.get("hosts").is_none() && manifest.get("permissions").is_none());
        assert_eq!(manifest["mode"], "search");
    }
}
