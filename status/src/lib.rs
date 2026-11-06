//! Service Status for Volant: the overall status of Claude, OpenAI, GitHub and Cloudflare, with a row
//! for each unresolved incident and maintenance in progress, read from each service's public
//! Statuspage summary (`/api/v2/summary.json`). Typed text narrows the list to services, components and
//! incidents whose names contain it, so `status actions` shows how GitHub Actions is doing. Return opens
//! the service's status page or the incident.
//!
//! Every page is fetched in one round, and each is read on its own: a page that fails or sends
//! something unreadable shows that one service as unavailable and leaves the others as they are.
//! Claude's page is read from status.claude.com, since status.anthropic.com redirects and Volant
//! refuses redirects.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{age, parse_rfc3339, volant_extension_output};

const SUMMARY_PATH: &str = "/api/v2/summary.json";
const INCIDENT_PATH: &str = "/incidents/";

/// The services in the order they are listed: display name and status page origin.
const SERVICES: &[(&str, &str)] = &[
    ("Claude", "https://status.claude.com"),
    ("OpenAI", "https://status.openai.com"),
    ("GitHub", "https://www.githubstatus.com"),
    ("Cloudflare", "https://www.cloudflarestatus.com"),
];

const PLAN: &str = "plan";
const RENDER: &str = "render";
const OK: i64 = 200;
const MAXIMUM_ROWS: usize = 100;
const SEPARATOR: &str = " · ";
const TIME: &str = "time";
const OPERATIONAL: &str = "operational";
const QUIET_IMPACTS: &[&str] = &["", "none", "maintenance"];
const IN_PROGRESS: &str = "in_progress";
const VERIFYING: &str = "verifying";
const FINISHED: &[&str] = &["resolved", "postmortem", "completed"];
const INCIDENT: &str = "incident";
const INCIDENTS: &str = "incidents";
const AFFECTED: &str = "affected";
const MAINTENANCE: &str = "Maintenance";
const IMPACT_SUFFIX: &str = " impact";
const UNAVAILABLE: &str = "Status unavailable";
const HTTP_PREFIX: &str = " (HTTP ";
const HTTP_SUFFIX: &str = ")";
const UNREADABLE: &str = "Status page sent a reply Volant could not read";
const NO_MATCH: &str = "No service, component or incident matches that name.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    now: i64,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    url: String,
    status: i64,
    #[serde(default)]
    body: String,
}

/// The parts of a Statuspage summary this extension reads. OpenAI's page has no `incidents` or
/// `scheduled_maintenances` while nothing is open, so both default to empty.
#[derive(Deserialize)]
struct Summary {
    status: Overall,
    #[serde(default)]
    components: Vec<Component>,
    #[serde(default)]
    incidents: Vec<Incident>,
    #[serde(default)]
    scheduled_maintenances: Vec<Incident>,
}

#[derive(Deserialize)]
struct Overall { description: String }

#[derive(Deserialize)]
struct Component {
    name: String,
    status: String,
    #[serde(default)]
    group: bool,
}

#[derive(Deserialize)]
struct Incident {
    id: String,
    name: String,
    status: String,
    #[serde(default)]
    impact: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
}

#[derive(Serialize, Default, Debug, PartialEq)]
struct Row {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
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

impl Badge {
    fn text(text: String) -> Badge { Badge { symbol: None, text } }
}

/// One explaining row, for an empty list or a failure.
fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The summary URL for a status page origin.
pub fn summary_url(page: &str) -> String {
    format!("{page}{SUMMARY_PATH}")
}

/// Every summary to fetch, one per service, all in the first round.
pub fn plan_urls() -> Vec<String> {
    SERVICES.iter().map(|(_, page)| summary_url(page)).collect()
}

/// "degraded_performance" as "Degraded performance".
fn humanize(value: &str) -> String {
    let spaced = value.replace('_', " ");
    let mut characters = spaced.chars();
    characters.next().map_or_else(String::new, |first| first.to_uppercase().chain(characters).collect())
}

/// "1 incident" or "3 incidents".
fn count(number: usize, one: &str, many: &str) -> String {
    format!("{number} {}", if number == 1 { one } else { many })
}

/// Whether an incident or maintenance is still open: unresolved incidents, and maintenance under way
/// rather than only scheduled.
fn is_open(incident: &Incident, maintenance: bool) -> bool {
    if maintenance { return incident.status == IN_PROGRESS || incident.status == VERIFYING; }
    !FINISHED.contains(&incident.status.as_str())
}

/// One service's summary as fetched: the parsed page, or what to say instead.
enum Fetched {
    Read(Summary),
    Failed(String),
}

/// Reads one service's response, so a failure stays with that service.
fn fetched(response: Option<&Response>) -> Fetched {
    let Some(response) = response else { return Fetched::Failed(format!("{UNAVAILABLE}{HTTP_PREFIX}0{HTTP_SUFFIX}")) };
    if response.status != OK { return Fetched::Failed(format!("{UNAVAILABLE}{HTTP_PREFIX}{}{HTTP_SUFFIX}", response.status)); }
    match serde_json::from_str::<Summary>(&response.body) {
        Ok(summary) => Fetched::Read(summary),
        Err(_) => Fetched::Failed(UNREADABLE.to_string()),
    }
}

/// The service row: its overall status, with open incidents and affected components as badges.
fn service_row(name: &str, page: &str, summary: &Summary, open: usize, maintenance: usize) -> Row {
    let affected = summary.components.iter().filter(|component| !component.group && component.status != OPERATIONAL).count();
    let mut badges = Vec::new();
    if open > 0 { badges.push(Badge::text(count(open, INCIDENT, INCIDENTS))); }
    if maintenance > 0 { badges.push(Badge::text(MAINTENANCE.to_string())); }
    if affected > 0 { badges.push(Badge::text(format!("{affected} {AFFECTED}"))); }
    Row { title: name.to_string(), subtitle: Some(summary.status.description.clone()), url: Some(page.to_string()), badges }
}

/// A row for an open incident or maintenance: the service and its stage, with its impact (for an
/// incident) and age as badges. Return opens it on the service's status page.
fn incident_row(service: &str, page: &str, incident: &Incident, maintenance: bool, now: i64) -> Row {
    let mut badges = Vec::new();
    if let Some(impact) = incident.impact.as_deref().filter(|impact| !QUIET_IMPACTS.contains(impact)) {
        badges.push(Badge::text(format!("{}{IMPACT_SUFFIX}", humanize(impact))));
    }
    let started = incident.started_at.as_deref().or(incident.created_at.as_deref()).and_then(parse_rfc3339);
    if let Some(started) = started { badges.push(Badge { symbol: Some(TIME), text: age(now, started) }); }
    let link = if incident.id.chars().all(|c| c.is_ascii_alphanumeric()) && !incident.id.is_empty() {
        format!("{page}{INCIDENT_PATH}{}", incident.id)
    } else {
        page.to_string()
    };
    Row {
        title: incident.name.clone(),
        subtitle: Some(if maintenance {
            format!("{service}{SEPARATOR}{MAINTENANCE}{SEPARATOR}{}", humanize(&incident.status))
        } else {
            format!("{service}{SEPARATOR}{}", humanize(&incident.status))
        }),
        url: Some(link),
        badges,
    }
}

/// A row for a component whose name matched the typed text.
fn component_row(service: &str, page: &str, component: &Component) -> Row {
    Row {
        title: component.name.clone(),
        subtitle: Some(format!("{service}{SEPARATOR}{}", humanize(&component.status))),
        url: Some(page.to_string()),
        ..Default::default()
    }
}

/// The rows for one service. With no text: the service and its open incidents. With text: the same
/// when the service's name matches, otherwise its matching components and incidents.
fn service_rows(name: &str, page: &str, fetched: &Fetched, filter: &str, now: i64) -> Vec<Row> {
    let matches = |text: &str| text.to_lowercase().contains(filter);
    let whole = filter.is_empty() || matches(name);
    let summary = match fetched {
        Fetched::Read(summary) => summary,
        Fetched::Failed(reason) if whole => {
            return vec![Row { title: name.to_string(), subtitle: Some(reason.clone()), url: Some(page.to_string()), ..Default::default() }];
        }
        Fetched::Failed(_) => return Vec::new(),
    };
    let incidents: Vec<&Incident> = summary.incidents.iter().filter(|incident| is_open(incident, false)).collect();
    let maintenances: Vec<&Incident> = summary.scheduled_maintenances.iter().filter(|incident| is_open(incident, true)).collect();
    let mut rows = Vec::new();
    if whole { rows.push(service_row(name, page, summary, incidents.len(), maintenances.len())); }
    let open = incidents.iter().map(|incident| (*incident, false)).chain(maintenances.iter().map(|incident| (*incident, true)));
    for (incident, maintenance) in open {
        if whole || matches(&incident.name) { rows.push(incident_row(name, page, incident, maintenance, now)); }
    }
    if !whole {
        rows.extend(summary.components.iter().filter(|component| matches(&component.name)).map(|component| component_row(name, page, component)));
    }
    rows
}

/// The rows for a render: every service in order, each read from its own response.
fn render(request: &Request) -> Vec<Row> {
    let filter = request.query.trim().to_lowercase();
    let mut rows = Vec::new();
    for (name, page) in SERVICES {
        let url = summary_url(page);
        let response = request.responses.iter().find(|response| response.url == url);
        rows.extend(service_rows(name, page, &fetched(response), &filter, request.now));
    }
    if rows.is_empty() { return vec![message(NO_MATCH)]; }
    rows.truncate(MAXIMUM_ROWS);
    rows
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": plan_urls() }).to_string(),
        RENDER => items(render(&request)),
        _ => items(vec![message(ONLY_LISTS)]),
    }
}

fn items(rows: Vec<Row>) -> String {
    serde_json::json!({ "items": rows }).to_string()
}

/// Whether a link is one of this extension's status pages; used by tests to check every row link.
#[cfg(test)]
fn is_status_link(url: &str) -> bool {
    SERVICES.iter().any(|(_, page)| url == *page || url.starts_with(&format!("{page}{INCIDENT_PATH}")))
}

volant_extension_output!(respond);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::manifest_permits;

    const MANIFEST: &str = include_str!("../manifest.json");
    const NOW: i64 = 1_793_000_000;

    /// A trimmed, fictional summary in the shape Statuspage sends: one open incident, one resolved,
    /// one maintenance under way and one only scheduled.
    const BUSY: &str = r#"{"page":{"id":"p","name":"Fictional","url":"https://example.com"},
        "status":{"indicator":"minor","description":"Partially Degraded Service"},
        "components":[
            {"id":"a","name":"Actions","status":"degraded_performance","group":false},
            {"id":"b","name":"Git Operations","status":"operational","group":false},
            {"id":"c","name":"North America","status":"major_outage","group":true},
            {"id":"d","name":"Webhooks","status":"partial_outage"}],
        "incidents":[
            {"id":"abc123","name":"Delayed Actions runs","status":"investigating","impact":"minor",
             "started_at":"2026-10-25T10:00:00.000Z","created_at":"2026-10-25T10:00:00.000Z","shortlink":"https://stspg.io/x","incident_updates":[]},
            {"id":"old1","name":"Old outage","status":"resolved","impact":"major","created_at":"2026-10-20T10:00:00Z"}],
        "scheduled_maintenances":[
            {"id":"m1","name":"Database upgrade","status":"in_progress","impact":"maintenance","started_at":"2026-10-25T11:00:00Z"},
            {"id":"m2","name":"Next week's upgrade","status":"scheduled","impact":"maintenance"}]}"#;
    const CALM: &str = r#"{"page":{"id":"q"},"status":{"indicator":"none","description":"All Systems Operational"},
        "components":[{"id":"e","name":"Chat Completions","status":"operational","position":0}]}"#;

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn responses(bodies: [(i64, &str); 4]) -> serde_json::Value {
        let urls = plan_urls();
        serde_json::Value::Array(urls.iter().zip(bodies).map(|(url, (status, body))| serde_json::json!({"url":url,"status":status,"body":body})).collect())
    }

    fn render_rows(query: &str, bodies: [(i64, &str); 4]) -> Vec<serde_json::Value> {
        let value = reply(serde_json::json!({"phase":"render","query":query,"now":NOW,"responses":responses(bodies)}));
        value["items"].as_array().unwrap().clone()
    }

    #[test]
    fn plans_one_summary_per_service_on_manifest_hosts() {
        let plan = reply(serde_json::json!({"phase":"plan","query":"","now":0}));
        let urls: Vec<&str> = plan["fetch"].as_array().unwrap().iter().map(|url| url.as_str().unwrap()).collect();
        assert_eq!(urls, ["https://status.claude.com/api/v2/summary.json", "https://status.openai.com/api/v2/summary.json",
            "https://www.githubstatus.com/api/v2/summary.json", "https://www.cloudflarestatus.com/api/v2/summary.json"]);
        for query in ["", "actions", "C++ & you?", "github"] {
            let plan = reply(serde_json::json!({"phase":"plan","query":query,"now":0}));
            let fetch = plan["fetch"].as_array().unwrap();
            assert!(fetch.len() <= 4);
            for url in fetch { assert!(manifest_permits(MANIFEST, url.as_str().unwrap()), "{url}"); }
        }
    }

    #[test]
    fn manifest_declares_exactly_the_status_hosts() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let hosts: Vec<&str> = manifest["hosts"].as_array().unwrap().iter().map(|host| host.as_str().unwrap()).collect();
        assert_eq!(hosts, ["status.claude.com", "status.openai.com", "www.githubstatus.com", "www.cloudflarestatus.com"]);
        assert_eq!(manifest["mode"], "list");
    }

    #[test]
    fn lists_each_service_with_open_incidents_and_maintenance() {
        let rows = render_rows("", [(200, CALM), (200, CALM), (200, BUSY), (200, CALM)]);
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Claude", "OpenAI", "GitHub", "Delayed Actions runs", "Database upgrade", "Cloudflare"]);
        assert_eq!(rows[0]["subtitle"], "All Systems Operational");
        assert_eq!(rows[0]["url"], "https://status.claude.com");
        assert!(rows[0].get("badges").is_none());
        assert_eq!(rows[2]["subtitle"], "Partially Degraded Service");
        assert_eq!(rows[2]["badges"], serde_json::json!([{"text":"1 incident"},{"text":"Maintenance"},{"text":"2 affected"}]),
            "Groups are not counted as affected components");
        assert_eq!(rows[3]["subtitle"], "GitHub · Investigating");
        assert_eq!(rows[3]["url"], "https://www.githubstatus.com/incidents/abc123");
        assert_eq!(rows[3]["badges"], serde_json::json!([{"text":"Minor impact"},{"symbol":"time","text":"21h ago"}]));
        assert_eq!(rows[4]["subtitle"], "GitHub · Maintenance · In progress");
        assert_eq!(rows[4]["badges"], serde_json::json!([{"symbol":"time","text":"20h ago"}]), "Maintenance has no impact badge");
        for row in &rows { assert!(is_status_link(row["url"].as_str().unwrap()), "{row}"); }
    }

    #[test]
    fn one_failed_service_does_not_hide_the_others() {
        let rows = render_rows("", [(200, CALM), (0, ""), (200, "<html>"), (503, "")]);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0]["subtitle"], "All Systems Operational");
        assert_eq!(rows[1]["title"], "OpenAI");
        assert_eq!(rows[1]["subtitle"], "Status unavailable (HTTP 0)");
        assert_eq!(rows[2]["subtitle"], UNREADABLE);
        assert_eq!(rows[3]["subtitle"], "Status unavailable (HTTP 503)");
        assert_eq!(rows[3]["url"], "https://www.cloudflarestatus.com");
        let missing = reply(serde_json::json!({"phase":"render","query":"","now":NOW,"responses":[]}));
        assert_eq!(missing["items"].as_array().unwrap().len(), 4, "No responses at all still lists every service");
    }

    #[test]
    fn typed_text_filters_services_components_and_incidents() {
        let bodies = [(200, CALM), (200, CALM), (200, BUSY), (503, "")];
        let rows = render_rows("ACTIONS", bodies);
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Delayed Actions runs", "Actions"]);
        assert_eq!(rows[1]["subtitle"], "GitHub · Degraded performance");
        let rows = render_rows("git", bodies);
        assert_eq!(rows[0]["title"], "GitHub", "A matching service shows whole");
        assert_eq!(rows.len(), 3);
        let rows = render_rows("cloud", bodies);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["subtitle"], "Status unavailable (HTTP 503)", "A failed service still shows when its name matches");
        let rows = render_rows("chat", bodies);
        assert_eq!(rows.len(), 2, "Components from every service match");
        assert_eq!(rows[0]["subtitle"], "Claude · Operational");
        assert_eq!(render_rows("zzz", bodies)[0]["title"], NO_MATCH);
    }

    #[test]
    fn rows_are_capped_and_other_phases_explain_themselves() {
        let many: Vec<String> = (0..150).map(|index| format!(r#"{{"name":"Region {index}","status":"operational"}}"#)).collect();
        let body = format!(r#"{{"status":{{"description":"All Systems Operational"}},"components":[{}]}}"#, many.join(","));
        let rows = render_rows("region", [(200, &body), (200, &body), (200, CALM), (200, CALM)]);
        assert_eq!(rows.len(), MAXIMUM_ROWS);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"","now":0}))["items"][0]["title"], ONLY_LISTS);
        assert_eq!(serde_json::from_str::<serde_json::Value>(&respond(b"nonsense")).unwrap()["items"][0]["title"], NOT_UNDERSTOOD);
    }
}
