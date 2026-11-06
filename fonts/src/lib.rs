//! Google Fonts for Volant: finds a font family by name and opens its specimen on Google Fonts, or its
//! page on Fontsource for the open-source fonts Google does not host. Google's own Fonts API needs a
//! key in the URL, which Volant cannot add, so the list comes from Fontsource's public API on
//! api.fontsource.org, which mirrors every Google font with its weights, styles and subsets.
//!
//! The full list is about 540 KB, over Volant's 512 KB limit for one response, so it is read by
//! category, which `?category=` accepts as a comma-separated list. All reads four pieces at once (Sans
//! Serif; Display; Serif and Monospace; Handwriting, Other and Icons), each well under the limit; the
//! dropdown's other entries read one category. Typed text then filters the families here: the name
//! itself, then names starting with it, then a word starting with it, then names containing it.
//!
//! Each row shows the category, weights and number of subsets, with badges for variable fonts and
//! italics. Return opens the specimen; Command-Return opens the font on Fontsource. The best match
//! also gets a row whose Return copies `npm install @fontsource/<id>`. All with nothing typed asks for
//! a name without fetching; a category with nothing typed lists its fonts alphabetically.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{encode_query, volant_extension_output};

const API: &str = "https://api.fontsource.org/v1/fonts?category=";
const SPECIMEN: &str = "https://fonts.google.com/specimen/";
const FONTSOURCE: &str = "https://fontsource.org/fonts/";
const FONTSOURCE_TITLE: &str = "Open on Fontsource";
const GOOGLE: &str = "google";
const INSTALL: &str = "npm install @fontsource/";
const INSTALL_HINT: &str = "Return copies the install command";
const VARIABLE: &str = "Variable";
const ITALIC: &str = "Italic";
const ITALIC_STYLE: &str = "italic";
const WEIGHT: &str = " weight";
const WEIGHTS: &str = " weights";
const SUBSET: &str = " subset";
const SUBSETS: &str = " subsets";
const EN_DASH: &str = "–";
const SEPARATOR: &str = " · ";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const PROMPT: &str = "Type a font name, such as Inter or Lora.";
const PROMPT_DETAIL: &str = "Or choose a category from the menu at the end of the field to browse it";
const NOTHING_FOUND_START: &str = "No font matches “";
const NOTHING_FOUND_END: &str = "”";
const NOTHING_FOUND_HINT: &str = "Fontsource lists every Google font and other open-source fonts";
const NOTHING_IN_CATEGORY_HINT: &str = "Choose All from the menu at the end of the field to search every category";
const UNREADABLE: &str = "Fontsource sent a list Volant could not read.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const OK: i64 = 200;
const MAXIMUM_ROWS: usize = 100;

/// Fontsource's category keys and how rows name them.
const CATEGORY_NAMES: [(&str, &str); 7] = [
    ("sans-serif", "Sans serif"), ("serif", "Serif"), ("display", "Display"), ("handwriting", "Handwriting"),
    ("monospace", "Monospace"), ("other", "Other"), ("icons", "Icons"),
];

/// What All reads: four comma-separated category lists, each under Volant's 512 KB response limit.
const ALL_CATEGORIES: [&str; 4] = ["sans-serif", "display", "serif,monospace", "handwriting,other,icons"];

/// The dropdown's single-category entries; their ids are Fontsource's category keys.
const CATEGORY_FILTERS: [&str; 5] = ["sans-serif", "serif", "display", "handwriting", "monospace"];

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    filter: Option<String>,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response { status: i64, body: String }

/// One family as Fontsource lists it.
#[derive(Deserialize, Debug, Clone)]
pub struct Font {
    pub id: String,
    pub family: String,
    #[serde(default)]
    pub subsets: Vec<String>,
    #[serde(default)]
    pub weights: Vec<u32>,
    #[serde(default)]
    pub styles: Vec<String>,
    #[serde(default)]
    pub variable: bool,
    #[serde(default)]
    pub category: String,
    #[serde(rename = "type", default)]
    pub kind: String,
}

#[derive(Serialize, Default, Debug, PartialEq)]
struct Row {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "secondaryURL")]
    secondary_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "secondaryTitle")]
    secondary_title: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

/// A row badge with only text. Volant 0.3.8 and later show it in place of the extension's name.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// The URLs for a dropdown filter: one category, or All's four pieces. No filter (Volant before 0.3.9)
/// or an unknown one means All.
pub fn urls_for(filter: Option<&str>) -> Vec<String> {
    match filter.filter(|id| CATEGORY_FILTERS.contains(id)) {
        Some(category) => vec![[API, category].concat()],
        None => ALL_CATEGORIES.iter().map(|categories| [API, categories].concat()).collect(),
    }
}

/// Whether a font id is safe in a URL path and a shell command: lowercase letters, digits and hyphens.
fn is_safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The Google Fonts specimen page of a family: its name with spaces as `+`.
pub fn specimen_url(family: &str) -> String {
    [SPECIMEN, &family.split(' ').map(encode_query).collect::<Vec<_>>().join("+")].concat()
}

/// Where Return takes a font: its Google Fonts specimen when Google hosts it, otherwise Fontsource.
pub fn page_url(font: &Font) -> String {
    if font.kind == GOOGLE { specimen_url(&font.family) } else { [FONTSOURCE, &font.id].concat() }
}

/// How well a family matches the lowercase `query`, lower being better, or None.
pub fn rank(font: &Font, query: &str) -> Option<u8> {
    if query.is_empty() { return Some(0); }
    let family = font.family.to_lowercase();
    if family == query || font.id == query { return Some(0); }
    if family.starts_with(query) { return Some(1); }
    if family.split_whitespace().any(|word| word.starts_with(query)) { return Some(2); }
    if family.contains(query) || font.id.contains(&query.replace(' ', "-")) { return Some(3); }
    None
}

/// "9 weights (100–900)" or "1 weight (400)".
fn weights(font: &Font) -> Option<String> {
    let (first, last) = (font.weights.iter().min()?, font.weights.iter().max()?);
    let count = font.weights.len();
    let span = if first == last { first.to_string() } else { [first.to_string(), last.to_string()].join(EN_DASH) };
    Some(format!("{count}{} ({span})", if count == 1 { WEIGHT } else { WEIGHTS }))
}

/// A font's row: its family, what it offers, and badges for variable fonts and italics.
fn row(font: &Font) -> Row {
    let mut details = Vec::new();
    let category = CATEGORY_NAMES.iter().find(|(key, _)| *key == font.category).map_or(font.category.as_str(), |(_, name)| *name);
    if !category.is_empty() { details.push(category.to_string()); }
    if let Some(weights) = weights(font) { details.push(weights); }
    let count = font.subsets.len();
    if count > 0 { details.push(format!("{count}{}", if count == 1 { SUBSET } else { SUBSETS })); }
    let mut badges = Vec::new();
    if font.variable { badges.push(Badge { text: VARIABLE.to_string() }); }
    if font.styles.iter().any(|style| style == ITALIC_STYLE) { badges.push(Badge { text: ITALIC.to_string() }); }
    let google = font.kind == GOOGLE;
    Row {
        title: font.family.clone(),
        subtitle: Some(details.join(SEPARATOR)),
        url: Some(page_url(font)),
        secondary_url: google.then(|| [FONTSOURCE, &font.id].concat()),
        secondary_title: google.then(|| FONTSOURCE_TITLE.to_string()),
        badges,
    }
}

/// The row whose Return copies the font's Fontsource install command.
fn install_row(font: &Font) -> Row {
    Row { title: [INSTALL, &font.id].concat(), subtitle: Some(INSTALL_HINT.to_string()), ..Default::default() }
}

/// The fonts in every response matching `query`, best first, then alphabetically.
fn matching(fonts: Vec<Font>, query: &str) -> Vec<Font> {
    let mut ranked: Vec<(u8, Font)> = fonts.into_iter().filter(|font| is_safe_id(&font.id) && !font.family.trim().is_empty())
        .filter_map(|font| rank(&font, query).map(|quality| (quality, font))).collect();
    ranked.sort_by(|(a, x), (b, y)| a.cmp(b).then_with(|| x.family.to_lowercase().cmp(&y.family.to_lowercase())));
    ranked.dedup_by(|(_, a), (_, b)| a.id == b.id);
    ranked.into_iter().map(|(_, font)| font).collect()
}

fn render(request: &Request, query: &str, filter: Option<&str>) -> Vec<Row> {
    let mut fonts = Vec::new();
    let mut failed = None;
    for response in &request.responses {
        if response.status != OK { failed = Some(response.status); continue; }
        match serde_json::from_str::<Vec<Font>>(&response.body) {
            Ok(list) => fonts.extend(list),
            Err(_) => return vec![message(UNREADABLE.to_string(), None)],
        }
    }
    let found = matching(fonts, query);
    let Some(first) = found.first() else {
        if let Some(status) = failed { return vec![message(format!("Fontsource is unavailable (HTTP {status}). Try again."), None)]; }
        let hint = if filter.is_some() { NOTHING_IN_CATEGORY_HINT } else { NOTHING_FOUND_HINT };
        return vec![message([NOTHING_FOUND_START, query, NOTHING_FOUND_END].concat(), Some(hint))];
    };
    let mut rows = vec![row(first)];
    if !query.is_empty() { rows.push(install_row(first)); }
    rows.extend(found.iter().skip(1).map(row));
    rows.truncate(MAXIMUM_ROWS);
    rows
}

fn message(title: String, subtitle: Option<&str>) -> Row {
    Row { title, subtitle: subtitle.map(str::to_string), ..Default::default() }
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD.to_string(), None)]) };
    let filter = request.filter.as_deref().filter(|id| CATEGORY_FILTERS.contains(id));
    let query = request.query.trim().to_lowercase();
    match request.phase.as_str() {
        PLAN if filter.is_none() && query.is_empty() => items(vec![message(PROMPT.to_string(), Some(PROMPT_DETAIL))]),
        PLAN => serde_json::json!({ "fetch": urls_for(filter) }).to_string(),
        RENDER => items(render(&request, &query, filter)),
        _ => items(vec![message(ONLY_LISTS.to_string(), None)]),
    }
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::manifest_permits;

    const MANIFEST: &str = include_str!("../manifest.json");

    /// Fictional families in the shape Fontsource's list returns.
    const SANS: &str = r#"[
        {"id":"interstate-sans","family":"Interstate Sans","subsets":["latin","latin-ext","cyrillic"],"weights":[100,200,300,400,500,600,700,800,900],
         "styles":["normal","italic"],"defSubset":"latin","variable":true,"lastModified":"2025-01-01","category":"sans-serif","license":"OFL-1.1","type":"google"},
        {"id":"inter-plain","family":"Inter Plain","subsets":["latin"],"weights":[400],"styles":["normal"],"defSubset":"latin","variable":false,
         "lastModified":"2024-01-01","category":"sans-serif","license":"OFL-1.1","type":"other"},
        {"id":"bad id","family":"Unsafe","subsets":[],"weights":[400],"styles":["normal"],"variable":false,"category":"sans-serif","type":"google"}]"#;
    const SERIF: &str = r#"[{"id":"lora-like","family":"Lora Like","subsets":["latin","vietnamese"],"weights":[400,700],"styles":["normal"],
        "variable":false,"category":"serif","type":"google"},{"id":"mono-printer","family":"Mono Printer","subsets":["latin"],"weights":[400],
        "styles":["normal","italic"],"variable":false,"category":"monospace","type":"google"}]"#;

    fn run(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn rows(query: &str, filter: Option<&str>, bodies: &[(i64, &str)]) -> Vec<serde_json::Value> {
        let urls = urls_for(filter);
        let responses: Vec<serde_json::Value> = bodies.iter().zip(urls.iter().chain(std::iter::repeat(&urls[0])))
            .map(|((status, body), url)| serde_json::json!({"url":url,"status":status,"body":body})).collect();
        run(serde_json::json!({"phase":"render","query":query,"now":0,"filter":filter,"responses":responses}))["items"].as_array().unwrap().clone()
    }

    #[test]
    fn every_planned_url_is_on_the_manifest_host() {
        for filter in [None, Some("all"), Some("sans-serif"), Some("serif"), Some("display"), Some("handwriting"), Some("monospace"), Some("bogus")] {
            let urls = urls_for(filter);
            assert!(!urls.is_empty() && urls.len() <= 4);
            for url in urls { assert!(manifest_permits(MANIFEST, &url), "{url}"); }
        }
        assert_eq!(urls_for(None), [
            "https://api.fontsource.org/v1/fonts?category=sans-serif", "https://api.fontsource.org/v1/fonts?category=display",
            "https://api.fontsource.org/v1/fonts?category=serif,monospace", "https://api.fontsource.org/v1/fonts?category=handwriting,other,icons"]);
        assert_eq!(urls_for(Some("monospace")), ["https://api.fontsource.org/v1/fonts?category=monospace"]);
        let plan = run(serde_json::json!({"phase":"plan","query":"inter","now":0}));
        assert_eq!(plan["fetch"].as_array().unwrap().len(), 4);
        let prompt = run(serde_json::json!({"phase":"plan","query":"","now":0,"filter":"all"}));
        assert_eq!(prompt["items"][0]["title"], PROMPT);
        let browse = run(serde_json::json!({"phase":"plan","query":"","now":0,"filter":"serif"}));
        assert_eq!(browse["fetch"], serde_json::json!(["https://api.fontsource.org/v1/fonts?category=serif"]));
    }

    #[test]
    fn pages_open_on_google_fonts_or_fontsource() {
        assert_eq!(specimen_url("Abhaya Libre"), "https://fonts.google.com/specimen/Abhaya+Libre");
        assert_eq!(specimen_url("Noto Sans JP"), "https://fonts.google.com/specimen/Noto+Sans+JP");
        assert_eq!(specimen_url("M PLUS 1p"), "https://fonts.google.com/specimen/M+PLUS+1p");
    }

    #[test]
    fn rows_show_what_a_font_offers_and_the_install_command() {
        let found = rows("inter", None, &[(200, SANS), (200, "[]"), (200, SERIF), (200, "[]")]);
        let titles: Vec<&str> = found.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Inter Plain", "npm install @fontsource/inter-plain", "Interstate Sans", "Mono Printer"], "Containing the text ranks last");
        assert_eq!(found[0]["subtitle"], "Sans serif · 1 weight (400) · 1 subset");
        assert_eq!(found[0]["url"], "https://fontsource.org/fonts/inter-plain", "Fonts Google does not host open on Fontsource");
        assert!(found[0].get("secondaryURL").is_none());
        assert!(found[1].get("url").is_none(), "Without a link, Return copies the title");
        assert_eq!(found[2]["subtitle"], "Sans serif · 9 weights (100–900) · 3 subsets");
        assert_eq!(found[2]["url"], "https://fonts.google.com/specimen/Interstate+Sans");
        assert_eq!(found[2]["secondaryURL"], "https://fontsource.org/fonts/interstate-sans");
        assert_eq!(found[2]["secondaryTitle"], FONTSOURCE_TITLE);
        assert_eq!(found[2]["badges"], serde_json::json!([{"text":"Variable"},{"text":"Italic"}]));
        let printer = rows("printer", None, &[(200, SANS), (200, SERIF)]);
        assert_eq!(printer[0]["subtitle"], "Monospace · 1 weight (400) · 1 subset");
        assert_eq!(printer[0]["badges"], serde_json::json!([{"text":"Italic"}]));
    }

    #[test]
    fn a_category_with_nothing_typed_lists_alphabetically_without_an_install_row() {
        let found = rows("", Some("sans-serif"), &[(200, SANS)]);
        let titles: Vec<&str> = found.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Inter Plain", "Interstate Sans"], "The unsafe id is left out");
    }

    #[test]
    fn ranking_prefers_the_name_itself() {
        let font = |family: &str, id: &str| Font { id: id.into(), family: family.into(), subsets: vec![], weights: vec![], styles: vec![], variable: false, category: String::new(), kind: String::new() };
        assert_eq!(rank(&font("Lora", "lora"), "lora"), Some(0));
        assert_eq!(rank(&font("Lora Italic", "lora-italic"), "lora"), Some(1));
        assert_eq!(rank(&font("Noto Sans", "noto-sans"), "sans"), Some(2));
        assert_eq!(rank(&font("Overpass", "overpass"), "pass"), Some(3));
        assert_eq!(rank(&font("Overpass", "overpass"), "zzz"), None);
    }

    #[test]
    fn failures_and_nothing_found() {
        let down = rows("inter", Some("serif"), &[(0, "")]);
        assert_eq!(down[0]["title"], "Fontsource is unavailable (HTTP 0). Try again.");
        let partial = rows("lora", None, &[(503, ""), (200, SERIF)]);
        assert_eq!(partial[0]["title"], "Lora Like", "Pieces that load still show");
        let unreadable = rows("inter", Some("serif"), &[(200, "<html>")]);
        assert_eq!(unreadable[0]["title"], UNREADABLE);
        let none = rows("qqqq", Some("serif"), &[(200, SERIF)]);
        assert_eq!(none[0]["title"], "No font matches “qqqq”");
        assert_eq!(none[0]["subtitle"], NOTHING_IN_CATEGORY_HINT);
        assert_eq!(rows("qqqq", None, &[(200, SERIF)])[0]["subtitle"], NOTHING_FOUND_HINT);
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
    }

    #[test]
    fn manifest_declares_the_categories_as_filters() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let ids: Vec<&str> = manifest["filters"].as_array().unwrap().iter().map(|f| f["id"].as_str().unwrap()).collect();
        assert_eq!(ids[0], "all");
        assert_eq!(&ids[1..], CATEGORY_FILTERS);
        assert_eq!(manifest["hosts"], serde_json::json!(["api.fontsource.org"]));
    }
}
