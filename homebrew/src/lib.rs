//! Homebrew for Volant: look up a formula or cask by its exact name on formulae.brew.sh. Homebrew
//! publishes no search API and its full lists are far over Volant's page limit, so the module asks for
//! the formula and the cask of that name and shows whichever exist: what it is, its version, its
//! installs in the last 30 days and its site, plus a row whose Return copies its install command.
//! Return on the first row opens its formulae.brew.sh page; Command-Return opens its homepage.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{domain, volant_extension_output};

const API: &str = "https://formulae.brew.sh/api/";
const PAGE: &str = "https://formulae.brew.sh/";
const FORMULA: &str = "formula";
const CASK: &str = "cask";
const JSON: &str = ".json";
const INSTALL: &str = "brew install ";
const INSTALL_CASK: &str = "brew install --cask ";
const INSTALL_HINT: &str = "Return copies the install command";
const HOMEPAGE_TITLE: &str = "Open Homepage";
const SITE: &str = "site";
const DEPRECATED: &str = "Deprecated";
const DISABLED: &str = "Disabled";
const MONTH: &str = "30d";
const INSTALLS: &str = " installs";
const SEPARATOR: &str = " · ";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const PROMPT: &str = "Type a formula or cask name, such as wget.";
const UNREADABLE: &str = "Homebrew sent a reply Volant could not read.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";
const MAXIMUM_NAME: usize = 100;
const OK: i64 = 200;
const NOT_FOUND: i64 = 404;

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response { url: String, status: i64, body: String }

#[derive(Deserialize)]
struct Formula {
    name: String,
    desc: Option<String>,
    homepage: Option<String>,
    versions: Option<Versions>,
    #[serde(default)]
    deprecated: bool,
    #[serde(default)]
    disabled: bool,
    analytics: Option<Analytics>,
}

#[derive(Deserialize)]
struct Versions { stable: Option<String> }

#[derive(Deserialize)]
struct Cask {
    token: String,
    #[serde(default)]
    name: Vec<String>,
    desc: Option<String>,
    homepage: Option<String>,
    version: Option<String>,
    #[serde(default)]
    deprecated: bool,
    #[serde(default)]
    disabled: bool,
    analytics: Option<Analytics>,
}

#[derive(Deserialize)]
struct Analytics { install: std::collections::HashMap<String, std::collections::HashMap<String, i64>> }

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

/// A row badge: a symbol Volant draws, if any, and a short text. Volant 0.3.8 and later show them.
#[derive(Serialize, Debug, PartialEq)]
struct Badge {
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<&'static str>,
    text: String,
}

/// What both kinds of package share once read: the facts a row shows and how to install it.
struct Package {
    kind: &'static str,
    name: String,
    full_name: Option<String>,
    desc: Option<String>,
    version: Option<String>,
    homepage: Option<String>,
    installs: Option<i64>,
    deprecated: bool,
    disabled: bool,
    install: String,
}

fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The package name to look up: trimmed, lowercased, with spaces as hyphens, as Homebrew names them.
/// None when it is empty, too long or uses characters no Homebrew name has.
pub fn name_for(query: &str) -> Option<String> {
    let name = query.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join("-");
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "@+._-".contains(c);
    (!name.is_empty() && name.len() <= MAXIMUM_NAME && name.chars().all(allowed) && !name.starts_with('.')).then_some(name)
}

/// The formula and cask requests for a name.
pub fn urls_for(name: &str) -> [String; 2] {
    [format!("{API}{FORMULA}/{name}{JSON}"), format!("{API}{CASK}/{name}{JSON}")]
}

/// 56458 as "56,458".
fn thousands(value: i64) -> String {
    let digits = value.abs().to_string();
    let grouped = digits.as_bytes().rchunks(3).rev().map(|chunk| std::str::from_utf8(chunk).unwrap_or_default()).collect::<Vec<_>>().join(",");
    if value < 0 { format!("-{grouped}") } else { grouped }
}

fn installs(analytics: Option<Analytics>, name: &str) -> Option<i64> {
    analytics?.install.get(MONTH)?.get(name).copied()
}

fn formula(body: &str) -> Option<Package> {
    let formula: Formula = serde_json::from_str(body).ok()?;
    Some(Package {
        kind: FORMULA,
        install: format!("{INSTALL}{}", formula.name),
        installs: installs(formula.analytics, &formula.name),
        full_name: None,
        desc: formula.desc,
        version: formula.versions.and_then(|versions| versions.stable),
        homepage: formula.homepage,
        deprecated: formula.deprecated,
        disabled: formula.disabled,
        name: formula.name,
    })
}

fn cask(body: &str) -> Option<Package> {
    let cask: Cask = serde_json::from_str(body).ok()?;
    Some(Package {
        kind: CASK,
        install: format!("{INSTALL_CASK}{}", cask.token),
        installs: installs(cask.analytics, &cask.token),
        full_name: cask.name.into_iter().next(),
        desc: cask.desc,
        version: cask.version,
        homepage: cask.homepage,
        deprecated: cask.deprecated,
        disabled: cask.disabled,
        name: cask.token,
    })
}

/// A package's two rows: what it is, and its install command, which Return copies.
fn rows(package: Package) -> [Row; 2] {
    let mut details = Vec::new();
    if package.disabled { details.push(DISABLED.to_string()); } else if package.deprecated { details.push(DEPRECATED.to_string()); }
    details.push(package.kind.to_string());
    if let Some(full) = package.full_name.filter(|full| !full.eq_ignore_ascii_case(&package.name)) { details.push(full); }
    if let Some(desc) = package.desc { details.push(desc); }
    if let Some(version) = package.version { details.push(version); }
    let mut badges = Vec::new();
    if let Some(count) = package.installs { badges.push(Badge { symbol: None, text: format!("{}{INSTALLS}", thousands(count)) }); }
    let homepage = package.homepage.filter(|link| link.starts_with("https://") || link.starts_with("http://"));
    if let Some(site) = homepage.as_deref().and_then(domain) { badges.push(Badge { symbol: Some(SITE), text: site }); }
    [
        Row {
            title: package.name.clone(),
            subtitle: Some(details.join(SEPARATOR)),
            url: Some(format!("{PAGE}{}/{}", package.kind, package.name)),
            secondary_title: homepage.as_ref().map(|_| HOMEPAGE_TITLE.to_string()),
            secondary_url: homepage,
            badges,
        },
        Row { title: package.install, subtitle: Some(INSTALL_HINT.to_string()), ..Default::default() },
    ]
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    let Some(name) = name_for(&request.query) else {
        let text = if request.query.trim().is_empty() { PROMPT } else { "Homebrew names use letters, digits and - . _ + @ only." };
        return items(vec![message(text)]);
    };
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": urls_for(&name) }).to_string(),
        RENDER => items(render(&request, &name)),
        _ => items(vec![message(ONLY_LISTS)]),
    }
}

fn render(request: &Request, name: &str) -> Vec<Row> {
    let [formula_url, cask_url] = urls_for(name);
    let mut found = Vec::new();
    let mut failed = None;
    for response in &request.responses {
        let read: fn(&str) -> Option<Package> = if response.url == formula_url { formula } else if response.url == cask_url { cask } else { continue };
        match response.status {
            OK => match read(&response.body) {
                Some(package) => found.extend(rows(package)),
                None => return vec![message(UNREADABLE)],
            },
            NOT_FOUND => {}
            status => failed = Some(status),
        }
    }
    if !found.is_empty() { return found; }
    match failed {
        Some(status) => vec![message(&format!("Homebrew is unavailable (HTTP {status}). Try again."))],
        None => vec![message(&format!("No formula or cask is named {name}. Homebrew has no search, so type the exact name."))],
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

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render(query: &str, formula: (i64, &str), cask: (i64, &str)) -> Vec<serde_json::Value> {
        let name = name_for(query).unwrap();
        let [formula_url, cask_url] = urls_for(&name);
        let value = reply(serde_json::json!({"phase":"render","query":query,"now":0,"responses":[
            {"url":formula_url,"status":formula.0,"body":formula.1},{"url":cask_url,"status":cask.0,"body":cask.1}]}));
        value["items"].as_array().unwrap().clone()
    }

    #[test]
    fn names_are_normalized_and_checked() {
        assert_eq!(name_for("  Wget "), Some("wget".into()));
        assert_eq!(name_for("visual studio code"), Some("visual-studio-code".into()));
        assert_eq!(name_for("python@3.13"), Some("python@3.13".into()));
        assert_eq!(name_for("../etc"), None);
        assert_eq!(name_for("a/b"), None);
        assert_eq!(name_for(""), None);
        assert_eq!(name_for(&"a".repeat(101)), None);
    }

    #[test]
    fn plans_ask_for_the_formula_and_the_cask_on_the_manifest_host() {
        for query in ["wget", "visual studio code", "python@3.13", "c++"] {
            for url in urls_for(&name_for(query).unwrap()) { assert!(manifest_permits(MANIFEST, &url), "{url}"); }
        }
        let plan = reply(serde_json::json!({"phase":"plan","query":"wget","now":0}));
        assert_eq!(plan["fetch"], serde_json::json!(["https://formulae.brew.sh/api/formula/wget.json", "https://formulae.brew.sh/api/cask/wget.json"]));
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":" ","now":0}))["items"][0]["title"], PROMPT);
        assert!(reply(serde_json::json!({"phase":"plan","query":"a/b","now":0}))["items"][0]["title"].as_str().unwrap().contains("letters"));
    }

    #[test]
    fn a_formula_shows_its_facts_and_install_command() {
        let body = r#"{"name":"wget","desc":"Internet file retriever","homepage":"https://www.gnu.org/software/wget/",
            "versions":{"stable":"1.25.0"},"deprecated":false,"disabled":false,"analytics":{"install":{"30d":{"wget":56458,"wget --HEAD":119}}}}"#;
        let items = render("wget", (200, body), (404, "{}"));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["title"], "wget");
        assert_eq!(items[0]["subtitle"], "formula · Internet file retriever · 1.25.0");
        assert_eq!(items[0]["url"], "https://formulae.brew.sh/formula/wget");
        assert_eq!(items[0]["secondaryURL"], "https://www.gnu.org/software/wget/");
        assert_eq!(items[0]["badges"], serde_json::json!([{"text":"56,458 installs"},{"symbol":"site","text":"gnu.org"}]));
        assert_eq!(items[1]["title"], "brew install wget");
        assert!(items[1].get("url").is_none(), "Without a link, Return copies the title");
    }

    #[test]
    fn a_name_that_is_both_shows_both_and_casks_say_so() {
        let formula = r#"{"name":"docker","desc":"Pack, ship and run any application as a lightweight container","versions":{"stable":"28.0"}}"#;
        let cask = r#"{"token":"docker","name":["Docker Desktop"],"desc":"App to build and share containerised applications","homepage":"https://www.docker.com/products/docker-desktop/","version":"4.40","deprecated":true}"#;
        let items = render("docker", (200, formula), (200, cask));
        let titles: Vec<&str> = items.iter().map(|item| item["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["docker", "brew install docker", "docker", "brew install --cask docker"]);
        assert_eq!(items[2]["subtitle"], "Deprecated · cask · Docker Desktop · App to build and share containerised applications · 4.40");
        assert_eq!(items[2]["url"], "https://formulae.brew.sh/cask/docker");
        assert!(items[0].get("secondaryURL").is_none(), "No homepage, no second link");
    }

    #[test]
    fn missing_packages_and_failures_explain_themselves() {
        let missing = render("nosuchthing", (404, ""), (404, ""));
        assert!(missing[0]["title"].as_str().unwrap().starts_with("No formula or cask is named nosuchthing"));
        let down = render("wget", (0, ""), (404, ""));
        assert!(down[0]["title"].as_str().unwrap().contains("HTTP 0"));
        let unreadable = render("wget", (200, "<html>"), (404, ""));
        assert_eq!(unreadable[0]["title"], UNREADABLE);
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(thousands(12), "12");
    }
}
