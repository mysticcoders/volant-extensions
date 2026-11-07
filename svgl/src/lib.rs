//! Svgl for Volant: searches svgl's library of brand logos on api.svgl.app and the Simple Icons set
//! on jsDelivr, and copies a logo's SVG. Both are fetched in the same round: svgl's search, and Simple
//! Icons' whole index (`simple-icons.json`, about 450 KB, under Volant's 512 KB fetch limit), which this
//! module searches itself. svgl's rows come first; Simple Icons' follow, exact matches first.
//!
//! Each svgl logo gets a row with its categories; Return opens the brand's website and Command-Return
//! its brand guidelines. A logo with a dark version or a wordmark gets a row for each, whose Return
//! opens that SVG on svgl.app. Each Simple Icons brand gets a row with its brand color as a badge, whose
//! Return opens its single-color SVG on cdn.jsdelivr.net. When one source fails, the other's rows still
//! show, followed by a row saying which source failed.
//!
//! Every row also carries `copy`, `{"url","type":"svg","title":"Copy SVG"}`, which a Volant release
//! with row copying (and the manifest's `clipboard-write` permission) fetches only when the owner
//! chooses Copy, putting the SVG on the clipboard. Older Volant versions ignore `copy`, so each row's
//! link still leads somewhere useful. An svgl copy URL is offered only when it is https on svgl.app
//! exactly, the host the manifest declares; svgl's list holds a few malformed links. A Simple Icons
//! copy URL is built from the brand's slug, which must be lowercase letters, digits and underscores.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use volant_extension_kit::{encode_query, is_web_link, volant_extension_output};

const API: &str = "https://api.svgl.app?search=";
const LIBRARY: &str = "https://svgl.app/";
const SIMPLE_ICONS_INDEX: &str = "https://cdn.jsdelivr.net/npm/simple-icons@16/data/simple-icons.json";
const SIMPLE_ICONS_SVG: &str = "https://cdn.jsdelivr.net/npm/simple-icons@16/icons/";
const SVG_SUFFIX: &str = ".svg";
const SVG: &str = "svg";
const COPY_SVG: &str = "Copy SVG";
const BRAND_TITLE: &str = "Open Brand Guidelines";
const DARK: &str = "Dark";
const WORDMARK: &str = "Wordmark";
const DARK_WORDMARK: &str = "Dark wordmark";
const DARK_SUFFIX: &str = " (dark)";
const WORDMARK_SUFFIX: &str = " wordmark";
const DARK_WORDMARK_SUFFIX: &str = " wordmark (dark)";
const SVG_HINT: &str = "Return opens the SVG";
const SIMPLE_ICONS_HINT: &str = "Simple Icons · single-color mark";
const HEX_PREFIX: &str = "#";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const PROMPT: &str = "Type a brand or product, such as Vercel.";
const NOTHING_FOUND_START: &str = "No logo matches “";
const NOTHING_FOUND_END: &str = "” in svgl or Simple Icons";
const SVGL_NAME: &str = "svgl";
const SIMPLE_ICONS_NAME: &str = "Simple Icons";
const UNAVAILABLE_MIDDLE: &str = " is unavailable (HTTP ";
const UNAVAILABLE_END: &str = "). Try again.";
const UNREADABLE_END: &str = " sent a reply Volant could not read.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const SEPARATOR: &str = " · ";
const LIST_SEPARATOR: &str = ", ";
const OK: i64 = 200;
const NOT_FOUND: i64 = 404;
const MAXIMUM_ROWS: usize = 100;
const MAXIMUM_SIMPLE_ICONS: usize = 20;
const MAXIMUM_QUERY: usize = 100;
const HEX_DIGITS: usize = 6;

/// Characters Simple Icons' `titleToSlug` replaces before it strips accents.
const SLUG_WORDS: [(char, &str); 4] = [('+', "plus"), ('.', "dot"), ('&', "and"), ('ß', "ss")];

/// Letters that reduce to a base letter, by Unicode decomposition or by Simple Icons' own replacements
/// (đ ħ ı ĸ ŀ ł ŧ ø).
const SLUG_FOLDS: [(char, &str); 19] = [
    ('a', "àáâãäåāăąǎǻ"), ('c', "çćĉċč"), ('d', "ďđ"), ('e', "èéêëēĕėęě"), ('g', "ĝğġģ"), ('h', "ĥħ"),
    ('i', "ìíîïĩīĭįıǐ"), ('j', "ĵ"), ('k', "ķĸ"), ('l', "ĺļľŀł"), ('n', "ñńņň"), ('o', "òóôõöōŏőøǒơ"),
    ('r', "ŕŗř"), ('s', "śŝşšș"), ('t', "ţťŧț"), ('u', "ùúûüũūŭůűųǔư"), ('w', "ŵ"), ('y', "ýÿŷ"),
    ('z', "źżž"),
];

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    url: String,
    status: i64,
    body: String,
}

/// One logo as svgl lists it. `category` and `route` (and `wordmark`) are each either a single value
/// or a list or light and dark pair.
#[derive(Deserialize)]
struct Logo {
    title: String,
    #[serde(default)]
    category: Option<OneOrMany>,
    route: Variants,
    #[serde(default)]
    wordmark: Option<Variants>,
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "brandUrl", default)]
    brand_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany { One(String), Many(Vec<String>) }

#[derive(Deserialize)]
#[serde(untagged)]
enum Variants { One(String), Themed { light: String, dark: String } }

impl Variants {
    fn light(&self) -> &str {
        match self { Variants::One(url) => url, Variants::Themed { light, .. } => light }
    }

    fn dark(&self) -> Option<&str> {
        match self { Variants::One(_) => None, Variants::Themed { dark, .. } => Some(dark) }
    }
}

/// One brand in Simple Icons' index. `slug` is present in version 16; without it the slug is derived
/// from the title as Simple Icons does.
#[derive(Deserialize)]
struct SimpleIcon {
    title: String,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    hex: String,
    #[serde(default)]
    aliases: Option<Aliases>,
}

/// A brand's other names: also known as, former names, localized names, and duplicates that share the
/// mark.
#[derive(Deserialize, Default)]
struct Aliases {
    #[serde(default)]
    aka: Vec<String>,
    #[serde(default)]
    old: Vec<String>,
    #[serde(default)]
    loc: HashMap<String, String>,
    #[serde(default)]
    dup: Vec<Duplicate>,
}

#[derive(Deserialize)]
struct Duplicate { title: String }

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
    #[serde(skip_serializing_if = "Option::is_none")]
    copy: Option<Copy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<Image>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    badges: Vec<Badge>,
}

/// What a Volant release with row copying puts on the clipboard: the file at `url`, fetched when the
/// owner chooses Copy.
#[derive(Serialize, Debug, PartialEq)]
struct Copy { url: String, #[serde(rename = "type")] kind: &'static str, title: &'static str }

/// What Volant 0.3.37 and later draw at the row's start once the owner allows row images: the same
/// SVG the row copies, checked and drawn in Volant's no-network extension helper.
#[derive(Serialize, Debug, PartialEq)]
struct Image { url: String }

/// A row badge with only text. Volant 0.3.8 and later show it in place of the extension's name.
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// Why one source gave no rows.
#[derive(Debug, PartialEq)]
enum Failure { Unavailable(i64), Unreadable }

/// The svgl search request for a query, or None when it is empty or too long.
pub fn url_for(query: &str) -> Option<String> {
    let query = query.trim();
    (!query.is_empty() && query.chars().count() <= MAXIMUM_QUERY).then(|| [API, &encode_query(query)].concat())
}

/// Whether an SVG link is one Volant may fetch for this manifest: https on svgl.app itself.
pub fn is_library_svg(url: &str) -> bool {
    url.strip_prefix(LIBRARY).is_some_and(|path| !path.is_empty() && !path.contains(['?', '#', '@']))
}

/// Simple Icons' slug for a title, as its `titleToSlug` makes it: lowercased, `+`, `.`, `&` and `ß`
/// spelled out, accents dropped, and anything but ASCII letters and digits removed.
pub fn title_to_slug(title: &str) -> String {
    let mut slug = String::new();
    for character in title.to_lowercase().chars() {
        if let Some((_, word)) = SLUG_WORDS.iter().find(|(from, _)| *from == character) {
            slug.push_str(word);
        } else if character.is_ascii_lowercase() || character.is_ascii_digit() {
            slug.push(character);
        } else if let Some((base, _)) = SLUG_FOLDS.iter().find(|(_, letters)| letters.contains(character)) {
            slug.push(*base);
        }
    }
    slug
}

/// Whether a slug is safe to put in an icon URL: lowercase ASCII letters, digits and underscores.
fn is_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// The single-color SVG on jsDelivr for a Simple Icons slug.
pub fn simple_icon_url(slug: &str) -> String {
    [SIMPLE_ICONS_SVG, slug, SVG_SUFFIX].concat()
}

fn copy(url: &str) -> Option<Copy> {
    is_library_svg(url).then(|| Copy { url: url.to_string(), kind: SVG, title: COPY_SVG })
}

/// A row for one of a logo's other SVGs, opened and copied from svgl.app.
fn variant_row(title: &str, suffix: &str, badge: &str, url: &str) -> Option<Row> {
    let copy = copy(url)?;
    Some(Row {
        title: [title, suffix].concat(),
        subtitle: Some(SVG_HINT.to_string()),
        url: Some(url.to_string()),
        image: Some(Image { url: url.to_string() }),
        copy: Some(copy),
        badges: vec![Badge { text: badge.to_string() }],
        ..Default::default()
    })
}

/// A logo's rows: the logo itself, then its dark version and wordmarks when svgl has them.
fn rows(logo: &Logo) -> Vec<Row> {
    let title = logo.title.trim();
    let categories = match &logo.category {
        Some(OneOrMany::One(category)) => category.clone(),
        Some(OneOrMany::Many(list)) => list.join(LIST_SEPARATOR),
        None => String::new(),
    };
    let website = logo.url.clone().filter(|link| is_web_link(link));
    let brand = logo.brand_url.clone().filter(|link| is_web_link(link));
    let light = logo.route.light();
    let mut details = Vec::new();
    if !categories.is_empty() { details.push(categories); }
    let main = Row {
        title: title.to_string(),
        subtitle: (!details.is_empty()).then(|| details.join(SEPARATOR)),
        url: website.or_else(|| is_library_svg(light).then(|| light.to_string())),
        secondary_title: brand.as_ref().map(|_| BRAND_TITLE.to_string()),
        secondary_url: brand,
        copy: copy(light),
        image: is_library_svg(light).then(|| Image { url: light.to_string() }),
        badges: Vec::new(),
    };
    let mut rows = vec![main];
    rows.extend(logo.route.dark().and_then(|dark| variant_row(title, DARK_SUFFIX, DARK, dark)));
    if let Some(wordmark) = &logo.wordmark {
        rows.extend(variant_row(title, WORDMARK_SUFFIX, WORDMARK, wordmark.light()));
        rows.extend(wordmark.dark().and_then(|dark| variant_row(title, DARK_WORDMARK_SUFFIX, DARK_WORDMARK, dark)));
    }
    rows
}

/// svgl's rows for its search response; a 404 is svgl's answer for no match.
fn svgl_rows(response: Option<&Response>) -> Result<Vec<Row>, Failure> {
    let Some(response) = response else { return Err(Failure::Unavailable(0)) };
    match response.status {
        OK => {}
        NOT_FOUND => return Ok(Vec::new()),
        status => return Err(Failure::Unavailable(status)),
    }
    let logos = serde_json::from_str::<Vec<Logo>>(&response.body).map_err(|_| Failure::Unreadable)?;
    Ok(logos.iter().filter(|logo| !logo.title.trim().is_empty()).flat_map(rows).collect())
}

/// How well a brand matches a lowercased query and its slug: 0 for an exact title, slug or alias, 1
/// for a title or slug starting with it, 2 for one containing it, 3 for an alias containing it, or
/// None.
fn rank(icon: &SimpleIcon, slug: &str, query: &str, query_slug: &str) -> Option<u8> {
    let title = icon.title.to_lowercase();
    let aliases = icon.aliases.as_ref().map(|aliases| {
        aliases.aka.iter().chain(&aliases.old).chain(aliases.loc.values()).chain(aliases.dup.iter().map(|dup| &dup.title))
            .map(|alias| alias.to_lowercase()).collect::<Vec<_>>()
    }).unwrap_or_default();
    let slug_matches = |test: fn(&str, &str) -> bool| !query_slug.is_empty() && test(slug, query_slug);
    if title == query || slug_matches(|a, b| a == b) || aliases.iter().any(|alias| alias == query) { return Some(0); }
    if title.starts_with(query) || slug_matches(|a, b| a.starts_with(b)) { return Some(1); }
    if title.contains(query) || slug_matches(|a, b| a.contains(b)) { return Some(2); }
    aliases.iter().any(|alias| alias.contains(query)).then_some(3)
}

/// A Simple Icons brand's row: its single-color SVG to open and copy, with its brand color as a badge.
fn simple_icon_row(icon: &SimpleIcon, slug: &str) -> Row {
    let url = simple_icon_url(slug);
    let hex = icon.hex.trim();
    let badges = (hex.len() == HEX_DIGITS && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| Badge { text: [HEX_PREFIX, hex].concat() }).into_iter().collect();
    Row {
        title: icon.title.trim().to_string(),
        subtitle: Some(SIMPLE_ICONS_HINT.to_string()),
        url: Some(url.clone()),
        image: Some(Image { url: url.clone() }),
        copy: Some(Copy { url, kind: SVG, title: COPY_SVG }),
        badges,
        ..Default::default()
    }
}

/// Simple Icons' rows for a query, searched in its index response: best matches first, then in the
/// index's alphabetical order, at most `MAXIMUM_SIMPLE_ICONS`.
fn simple_icons_rows(response: Option<&Response>, query: &str) -> Result<Vec<Row>, Failure> {
    let Some(response) = response else { return Err(Failure::Unavailable(0)) };
    if response.status != OK { return Err(Failure::Unavailable(response.status)); }
    let icons = serde_json::from_str::<Vec<SimpleIcon>>(&response.body).map_err(|_| Failure::Unreadable)?;
    let query = query.to_lowercase();
    let query_slug = title_to_slug(&query);
    let mut found: Vec<(u8, &SimpleIcon, String)> = icons.iter().filter(|icon| !icon.title.trim().is_empty()).filter_map(|icon| {
        let slug = icon.slug.clone().filter(|slug| is_slug(slug)).unwrap_or_else(|| title_to_slug(&icon.title));
        if !is_slug(&slug) { return None; }
        rank(icon, &slug, &query, &query_slug).map(|rank| (rank, icon, slug))
    }).collect();
    found.sort_by_key(|(rank, _, _)| *rank);
    Ok(found.iter().take(MAXIMUM_SIMPLE_ICONS).map(|(_, icon, slug)| simple_icon_row(icon, slug)).collect())
}

fn failure_message(source: &str, failure: &Failure) -> Row {
    let title = match failure {
        Failure::Unavailable(status) => [source, UNAVAILABLE_MIDDLE, &status.to_string(), UNAVAILABLE_END].concat(),
        Failure::Unreadable => [source, UNREADABLE_END].concat(),
    };
    message(title)
}

/// Both sources' rows: svgl's, then Simple Icons', then a row for each source that failed. With no
/// rows and no failure, says nothing matched.
fn render(request: &Request, query: &str, svgl_url: &str) -> Vec<Row> {
    let response = |url: &str| request.responses.iter().find(|response| response.url == url);
    let mut failures = Vec::new();
    let logos = svgl_rows(response(svgl_url)).unwrap_or_else(|failure| {
        failures.push(failure_message(SVGL_NAME, &failure));
        Vec::new()
    });
    let icons = simple_icons_rows(response(SIMPLE_ICONS_INDEX), query).unwrap_or_else(|failure| {
        failures.push(failure_message(SIMPLE_ICONS_NAME, &failure));
        Vec::new()
    });
    if logos.is_empty() && icons.is_empty() && failures.is_empty() {
        return vec![message([NOTHING_FOUND_START, query, NOTHING_FOUND_END].concat())];
    }
    let room = MAXIMUM_ROWS - icons.len() - failures.len();
    let mut found: Vec<Row> = logos.into_iter().take(room).collect();
    found.extend(icons);
    found.extend(failures);
    found
}

fn message(title: String) -> Row {
    Row { title, ..Default::default() }
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD.to_string())]) };
    let Some(url) = url_for(&request.query) else { return items(vec![message(PROMPT.to_string())]) };
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": [url, SIMPLE_ICONS_INDEX] }).to_string(),
        RENDER => items(render(&request, request.query.trim(), &url)),
        _ => items(vec![message(ONLY_LISTS.to_string())]),
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

    /// Fictional logos in the shapes svgl's API returns: a plain route, light and dark routes with
    /// themed wordmarks, a single wordmark, and a malformed link.
    const BODY: &str = r#"[
        {"id":1,"title":"Acme","category":["Hosting","Acme"],"route":{"light":"https://svgl.app/library/acme.svg","dark":"https://svgl.app/library/acme_dark.svg"},
         "wordmark":{"light":"https://svgl.app/library/acme_wordmark.svg","dark":"https://svgl.app/library/acme_wordmark_dark.svg"},
         "brandUrl":"https://acme.example/brand","url":"https://acme.example/"},
        {"id":2,"title":"Plainco","category":"Software","route":"https://svgl.app/library/plainco.svg","wordmark":"https://svgl.app/library/plainco-wordmark.svg","url":"https://plainco.example"},
        {"id":3,"title":"Broken","category":"Software","route":"https://svgl.applibrary/broken.svg","url":"https://broken.example"}]"#;

    /// svgl's real answer for "apple", trimmed to its first logo.
    const SVGL_APPLE: &str = r#"[{"id":213,"title":"Apple","category":"Software","route":{"light":"https://svgl.app/library/apple.svg","dark":"https://svgl.app/library/apple_dark.svg"},"url":"https://www.apple.com"}]"#;

    /// Real entries from simple-icons@16.36.0's `data/simple-icons.json`, trimmed to a few brands and
    /// shortened sources. Ferrari N.V. and Sat.1 carry slugs `titleToSlug` would not make; CBC and
    /// Biome carry aliases. Apple's entry has its `slug` removed, to cover indexes that leave it out.
    const INDEX: &str = r#"[
        {"title":".NET","slug":"dotnet","hex":"512BD4","source":"https://dotnet.microsoft.com","license":{"type":"CC0-1.0"}},
        {"title":"1&1","slug":"1and1","hex":"003D8F","source":"https://www.united-internet.de/en/newsroom/media-center/logos.html"},
        {"title":"Airbnb","slug":"airbnb","hex":"FF5A5F","source":"https://www.airbnb.com"},
        {"title":"Airbus","slug":"airbus","hex":"00205B","source":"https://brand.airbus.com/en/asset-library/airbus-logo","guidelines":"https://brand.airbus.com/en/asset-library/airbus-logo"},
        {"title":"Apple","hex":"000000","source":"https://www.apple.com"},
        {"title":"Biome","slug":"biome","hex":"60A5FA","source":"https://biomejs.dev","aliases":{"old":["Rome"]}},
        {"title":"C++","slug":"cplusplus","hex":"00599C","source":"https://isocpp.org"},
        {"title":"CBC","slug":"cbc","hex":"E60505","source":"https://www.cbc.ca","aliases":{"aka":["Canadian Broadcasting Company"],"dup":[{"title":"Radio Canada","hex":"FF0000","source":"https://radio-canada.ca"}],"loc":{"fr-CA":"Société Radio-Canada"}}},
        {"title":"Citroën","slug":"citroen","hex":"DA291C","source":"https://www.stellantis.com/en/brands/citroen"},
        {"title":"Coca-Cola","slug":"cocacola","hex":"D00013","source":"https://commons.wikimedia.org/wiki/File:Coca-Cola_logo.svg"},
        {"title":"Ferrari N.V.","slug":"ferrarinv","hex":"EB2E2C","source":"https://corporate.ferrari.com"},
        {"title":"Nike","slug":"nike","hex":"111111","source":"https://www.nike.com"},
        {"title":"Nikon","slug":"nikon","hex":"FFE100","source":"https://www.nikon.com","guidelines":"https://www.nikon.com/usage/group-info"},
        {"title":"Sat.1","slug":"sat1","hex":"047DA3","source":"https://www.prosiebensat1.com/presse/downloads/logos"},
        {"title":"ŠKODA","slug":"skoda","hex":"0E3A2F","source":"https://www.skoda-connect.com"}]"#;

    const SVGL_NOT_FOUND: &str = r#"{"error":"SVG not found"}"#;

    fn run(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render_both(query: &str, svgl: (i64, &str), index: (i64, &str)) -> Vec<serde_json::Value> {
        let url = url_for(query).unwrap();
        let responses = serde_json::json!([{"url":url,"status":svgl.0,"body":svgl.1},{"url":SIMPLE_ICONS_INDEX,"status":index.0,"body":index.1}]);
        run(serde_json::json!({"phase":"render","query":query,"now":0,"responses":responses}))["items"].as_array().unwrap().clone()
    }

    fn render(query: &str, status: i64, body: &str) -> Vec<serde_json::Value> {
        render_both(query, (status, body), (OK, "[]"))
    }

    fn titles(rows: &[serde_json::Value]) -> Vec<&str> {
        rows.iter().map(|row| row["title"].as_str().unwrap()).collect()
    }

    #[test]
    fn every_planned_url_is_on_the_manifest_hosts() {
        for query in ["vercel", "visual studio", "C++ & co", "ünïcode"] {
            let plan = run(serde_json::json!({"phase":"plan","query":query,"now":0}));
            let fetch = plan["fetch"].as_array().unwrap();
            assert_eq!(fetch.len(), 2, "One round: svgl's search and Simple Icons' index");
            for url in fetch { assert!(manifest_permits(MANIFEST, url.as_str().unwrap()), "{url}"); }
        }
        assert_eq!(url_for(" Visual Studio "), Some("https://api.svgl.app?search=Visual%20Studio".into()));
        assert_eq!(url_for("  "), None);
        assert_eq!(url_for(&"a".repeat(101)), None);
        let plan = run(serde_json::json!({"phase":"plan","query":"vercel","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://api.svgl.app?search=vercel", "https://cdn.jsdelivr.net/npm/simple-icons@16/data/simple-icons.json"]));
        assert_eq!(run(serde_json::json!({"phase":"plan","query":"","now":0}))["items"][0]["title"], PROMPT);
    }

    #[test]
    fn copy_urls_are_on_the_declared_svg_hosts() {
        assert!(is_library_svg("https://svgl.app/library/vercel.svg"));
        assert!(manifest_permits(MANIFEST, "https://svgl.app/library/vercel.svg"));
        assert!(!is_library_svg("https://svgl.applibrary/vercel.svg"));
        assert!(!is_library_svg("http://svgl.app/library/vercel.svg"));
        assert!(!is_library_svg("https://evil.example/svgl.app/x.svg"));
        assert!(!is_library_svg("https://svgl.app/"));
        assert_eq!(simple_icon_url("nike"), "https://cdn.jsdelivr.net/npm/simple-icons@16/icons/nike.svg");
        assert!(manifest_permits(MANIFEST, &simple_icon_url("backstage_casting")));
        assert!(is_slug("backstage_casting"));
        assert!(!is_slug("../x") && !is_slug("") && !is_slug("Nike") && !is_slug("a?b"));
    }

    #[test]
    fn titles_become_simple_icons_slugs() {
        for (title, slug) in [
            ("Coca-Cola", "cocacola"), (".NET", "dotnet"), ("C++", "cplusplus"), ("Mercedes", "mercedes"),
            ("1&1", "1and1"), ("Citroën", "citroen"), ("ŠKODA", "skoda"), ("Żabka", "zabka"), ("tado°", "tado"),
            ("Bakaláři", "bakalari"), ("Aldi Süd", "aldisud"), ("Île-de-France Mobilités", "iledefrancemobilites"),
            ("L'Équipe", "lequipe"), ("Straße", "strasse"), ("Łódź", "lodz"), ("Ferrari N.V.", "ferrarindotvdot"),
            ("1.1.1.1", "1dot1dot1dot1"), ("/e/", "e"), ("F#", "f"), ("São Paulo Metro", "saopaulometro"),
            ("Citroe\u{308}n", "citroen"),
        ] {
            assert_eq!(title_to_slug(title), slug, "{title}");
        }
    }

    #[test]
    fn simple_icons_rows_carry_the_svg_and_brand_color() {
        let rows = render_both("nike", (NOT_FOUND, SVGL_NOT_FOUND), (OK, INDEX));
        assert_eq!(titles(&rows), ["Nike"]);
        assert_eq!(rows[0]["subtitle"], SIMPLE_ICONS_HINT);
        assert_eq!(rows[0]["url"], "https://cdn.jsdelivr.net/npm/simple-icons@16/icons/nike.svg");
        assert_eq!(rows[0]["copy"], serde_json::json!({"url":"https://cdn.jsdelivr.net/npm/simple-icons@16/icons/nike.svg","type":"svg","title":"Copy SVG"}));
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"#111111"}]));
        assert_eq!(rows[0]["image"], serde_json::json!({"url":"https://cdn.jsdelivr.net/npm/simple-icons@16/icons/nike.svg"}), "Each row previews the SVG it copies");
        assert_eq!(titles(&render_both("airbus", (NOT_FOUND, SVGL_NOT_FOUND), (OK, INDEX))), ["Airbus"]);
        assert_eq!(titles(&render_both("air", (NOT_FOUND, SVGL_NOT_FOUND), (OK, INDEX))), ["Airbnb", "Airbus"]);
    }

    #[test]
    fn simple_icons_matches_titles_slugs_and_aliases_exact_first() {
        let search = |query: &str| render_both(query, (NOT_FOUND, SVGL_NOT_FOUND), (OK, INDEX));
        assert_eq!(titles(&search("Nik")), ["Nike", "Nikon"]);
        assert_eq!(titles(&search("nikon")), ["Nikon"]);
        assert_eq!(titles(&search("coca cola")), ["Coca-Cola"], "Matched through the slug");
        assert_eq!(titles(&search("C++")), ["C++"]);
        assert_eq!(titles(&search(".net")), [".NET"]);
        assert_eq!(titles(&search("skoda")), ["ŠKODA"]);
        assert_eq!(titles(&search("rome")), ["Biome"], "A former name");
        assert_eq!(titles(&search("radio canada")), ["CBC"], "A duplicate's title");
        assert_eq!(titles(&search("société")), ["CBC"], "A localized name");
        let apple = search("apple");
        assert_eq!(apple[0]["copy"]["url"], "https://cdn.jsdelivr.net/npm/simple-icons@16/icons/apple.svg", "A missing slug comes from the title");
        let sat = search("sat.1");
        assert_eq!(sat[0]["url"], "https://cdn.jsdelivr.net/npm/simple-icons@16/icons/sat1.svg", "The index's slug wins");
        assert_eq!(search("a")[0]["title"], "Airbnb", "Titles starting with the query lead");
        for row in search("a").iter().chain(&search("o")) {
            assert!(manifest_permits(MANIFEST, row["copy"]["url"].as_str().unwrap()), "{row}");
        }
    }

    #[test]
    fn svgl_rows_lead_and_simple_icons_follow() {
        let rows = render_both("apple", (OK, SVGL_APPLE), (OK, INDEX));
        assert_eq!(titles(&rows), ["Apple", "Apple (dark)", "Apple"]);
        assert_eq!(rows[0]["url"], "https://www.apple.com");
        assert_eq!(rows[0]["copy"]["url"], "https://svgl.app/library/apple.svg");
        assert_eq!(rows[2]["subtitle"], SIMPLE_ICONS_HINT);
        assert_eq!(rows[2]["badges"][0]["text"], "#000000");
        let many: Vec<serde_json::Value> = (0..200).map(|i| serde_json::json!({"title":format!("Logo {i}"),"route":format!("https://svgl.app/library/{i}.svg")})).collect();
        let capped = render_both("apple", (OK, &serde_json::to_string(&many).unwrap()), (OK, INDEX));
        assert_eq!(capped.len(), MAXIMUM_ROWS);
        assert_eq!(capped.last().unwrap()["title"], "Apple", "Simple Icons keeps its rows when svgl has many");
    }

    #[test]
    fn logos_and_their_variants_carry_copy() {
        let rows = render("acme", 200, BODY);
        assert_eq!(titles(&rows), ["Acme", "Acme (dark)", "Acme wordmark", "Acme wordmark (dark)", "Plainco", "Plainco wordmark", "Broken"]);
        assert_eq!(rows[0]["subtitle"], "Hosting, Acme");
        assert_eq!(rows[0]["url"], "https://acme.example/", "Older Volant opens the brand's website");
        assert_eq!(rows[0]["secondaryURL"], "https://acme.example/brand");
        assert_eq!(rows[0]["secondaryTitle"], BRAND_TITLE);
        assert_eq!(rows[0]["copy"], serde_json::json!({"url":"https://svgl.app/library/acme.svg","type":"svg","title":"Copy SVG"}));
        assert_eq!(rows[1]["url"], "https://svgl.app/library/acme_dark.svg");
        assert_eq!(rows[1]["copy"]["url"], "https://svgl.app/library/acme_dark.svg");
        assert_eq!(rows[1]["badges"], serde_json::json!([{"text":"Dark"}]));
        assert_eq!(rows[3]["badges"][0]["text"], "Dark wordmark");
        assert_eq!(rows[4]["subtitle"], "Software");
        assert!(rows[4].get("secondaryURL").is_none());
        assert!(rows[6].get("copy").is_none(), "A malformed SVG link is never offered for copying");
        for row in &rows {
            if let Some(url) = row["copy"]["url"].as_str() { assert!(manifest_permits(MANIFEST, url), "{url}"); }
        }
    }

    #[test]
    fn one_source_failing_keeps_the_other() {
        let rows = render_both("apple", (OK, SVGL_APPLE), (0, ""));
        assert_eq!(titles(&rows), ["Apple", "Apple (dark)", "Simple Icons is unavailable (HTTP 0). Try again."]);
        let rows = render_both("nike", (503, ""), (OK, INDEX));
        assert_eq!(titles(&rows), ["Nike", "svgl is unavailable (HTTP 503). Try again."]);
        let rows = render_both("nike", (OK, "<html>"), (OK, "<html>"));
        assert_eq!(titles(&rows), ["svgl sent a reply Volant could not read.", "Simple Icons sent a reply Volant could not read."]);
        let missing = run(serde_json::json!({"phase":"render","query":"nike","now":0,"responses":[]}));
        assert_eq!(missing["items"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn nothing_found_and_bad_requests() {
        assert_eq!(render_both("mercedes", (NOT_FOUND, SVGL_NOT_FOUND), (OK, INDEX))[0]["title"], "No logo matches “mercedes” in svgl or Simple Icons");
        assert_eq!(render("zzz", 200, "[]")[0]["title"], "No logo matches “zzz” in svgl or Simple Icons");
        assert!(respond(b"{").contains(NOT_UNDERSTOOD));
        assert_eq!(run(serde_json::json!({"phase":"action","query":"x","now":0}))["items"][0]["title"], ONLY_LISTS);
    }

    #[test]
    fn manifest_asks_to_copy_and_show_images_and_reach_three_hosts() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["permissions"], serde_json::json!(["clipboard-write", "row-images"]));
        assert_eq!(manifest["hosts"], serde_json::json!(["api.svgl.app", "svgl.app", "cdn.jsdelivr.net"]));
    }
}
