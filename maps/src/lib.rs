//! Maps for Volant: places and addresses matching the typed text, found with OpenStreetMap's
//! Nominatim search on nominatim.openstreetmap.org. Each row shows the place's name, its address and
//! its kind as a badge. Return opens the place in Apple Maps (through its maps.apple.com link, which
//! macOS hands to Maps.app); Command-Return opens it on openstreetmap.org.
//!
//! Nominatim's usage policy allows at most one request a second and no searching as you type. The
//! manifest's search mode waits for text, Volant runs it only once typing pauses, and every run plans
//! exactly one request. Search data © OpenStreetMap contributors, under the ODbL.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{encode_query, volant_extension_output};

const SEARCH: &str = "https://nominatim.openstreetmap.org/search?format=jsonv2&limit=10&q=";
const APPLE_MAPS: &str = "https://maps.apple.com/?q=";
const APPLE_MAPS_POSITION: &str = "&ll=";
const OSM_SITE: &str = "https://www.openstreetmap.org/";
const OSM_MARKER: &str = "https://www.openstreetmap.org/?mlat=";
const OSM_MARKER_LONGITUDE: &str = "&mlon=";
const OSM_TITLE: &str = "Open in OpenStreetMap";
const OSM_TYPES: &[&str] = &["node", "way", "relation"];
const COORDINATE_SEPARATOR: &str = ",";
const ADDRESS_SEPARATOR: &str = ", ";
const PATH_SEPARATOR: &str = "/";
const GENERIC_TYPE: &str = "yes";
const MAXIMUM_BADGE: usize = 32;
const PLAN: &str = "plan";
const RENDER: &str = "render";
const OK: i64 = 200;
const FORBIDDEN: i64 = 403;
const TOO_MANY: i64 = 429;
const PROMPT: &str = "Type a place or address to search OpenStreetMap.";
const NOTHING: &str = "OpenStreetMap returned nothing.";
const NO_PLACES: &str = "No places match that search.";
const UNREADABLE: &str = "OpenStreetMap sent a reply Volant could not read.";
const REFUSED: &str = "OpenStreetMap's search is busy or refused this request. Try again in a minute.";
const UNAVAILABLE_PREFIX: &str = "OpenStreetMap search is unavailable (HTTP ";
const UNAVAILABLE_SUFFIX: &str = "). Try again.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
    #[serde(default)]
    responses: Vec<Response>,
}

#[derive(Deserialize)]
struct Response { status: i64, body: String }

/// The fields of a Nominatim `jsonv2` result this extension reads.
#[derive(Deserialize)]
struct Place {
    #[serde(default)]
    name: String,
    #[serde(default)]
    display_name: String,
    lat: String,
    lon: String,
    #[serde(default)]
    osm_type: String,
    #[serde(default)]
    osm_id: Option<i64>,
    #[serde(default)]
    category: String,
    #[serde(default, rename = "type")]
    kind: String,
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

/// A row badge: a short text Volant shows at the row's end (Volant 0.3.8 and later).
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// One explaining row, for a prompt, an empty result or a failure.
fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// The Nominatim search for the typed text.
pub fn url_for(query: &str) -> String {
    format!("{SEARCH}{}", encode_query(query.trim()))
}

/// What a failed search says: Nominatim refusing or throttling, or its own status.
fn failure(status: i64) -> String {
    match status {
        FORBIDDEN | TOO_MANY => REFUSED.to_string(),
        _ => format!("{UNAVAILABLE_PREFIX}{status}{UNAVAILABLE_SUFFIX}"),
    }
}

/// "bus_stop" as "Bus stop".
fn humanize(value: &str) -> String {
    let spaced = value.replace('_', " ");
    let mut characters = spaced.chars();
    characters.next().map_or_else(String::new, |first| first.to_uppercase().chain(characters).collect())
}

/// The place's kind for its badge: its OSM type, or its category when the type only says "yes" (as
/// with `building=yes`).
fn kind(place: &Place) -> Option<String> {
    let raw = if place.kind.is_empty() || place.kind == GENERIC_TYPE { &place.category } else { &place.kind };
    let text: String = humanize(raw).chars().take(MAXIMUM_BADGE).collect();
    (!text.is_empty()).then_some(text)
}

/// A coordinate from Nominatim's string, accepted only when it is a finite number in range, so a
/// link never carries anything else.
fn coordinate(text: &str, limit: f64) -> Option<&str> {
    text.parse::<f64>().ok().filter(|value| value.is_finite() && value.abs() <= limit).map(|_| text)
}

/// The place on openstreetmap.org: its object page, or a marker at its position.
fn osm_link(place: &Place, latitude: &str, longitude: &str) -> String {
    match place.osm_id {
        Some(id) if OSM_TYPES.contains(&place.osm_type.as_str()) => format!("{OSM_SITE}{}{PATH_SEPARATOR}{id}", place.osm_type),
        _ => format!("{OSM_MARKER}{latitude}{OSM_MARKER_LONGITUDE}{longitude}"),
    }
}

/// One row for a place: its name, the rest of its address, its kind as a badge, Apple Maps on
/// Return and OpenStreetMap on Command-Return. Places without a valid position are skipped.
fn row(place: Place) -> Option<Row> {
    let latitude = coordinate(&place.lat, 90.0)?;
    let longitude = coordinate(&place.lon, 180.0)?;
    let title = if place.name.trim().is_empty() {
        place.display_name.split(ADDRESS_SEPARATOR).next().unwrap_or_default().trim().to_string()
    } else {
        place.name.trim().to_string()
    };
    if title.is_empty() { return None; }
    let address = place.display_name.strip_prefix(title.as_str()).map_or(place.display_name.as_str(), |rest| rest.trim_start_matches(ADDRESS_SEPARATOR));
    let apple = format!("{APPLE_MAPS}{}{APPLE_MAPS_POSITION}{latitude}{COORDINATE_SEPARATOR}{longitude}", encode_query(&title));
    Some(Row {
        subtitle: (!address.trim().is_empty()).then(|| address.trim().to_string()),
        url: Some(apple),
        secondary_url: Some(osm_link(&place, latitude, longitude)),
        secondary_title: Some(OSM_TITLE.to_string()),
        badges: kind(&place).map(|text| Badge { text }).into_iter().collect(),
        title,
    })
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    match request.phase.as_str() {
        PLAN if request.query.trim().is_empty() => items(vec![message(PROMPT)]),
        PLAN => serde_json::json!({ "fetch": [url_for(&request.query)] }).to_string(),
        RENDER => items(render(&request)),
        _ => items(vec![message(ONLY_LISTS)]),
    }
}

fn render(request: &Request) -> Vec<Row> {
    let Some(response) = request.responses.first() else { return vec![message(NOTHING)] };
    if response.status != OK { return vec![message(&failure(response.status))]; }
    let Ok(places) = serde_json::from_str::<Vec<Place>>(&response.body) else { return vec![message(UNREADABLE)] };
    let rows: Vec<Row> = places.into_iter().filter_map(row).collect();
    if rows.is_empty() { return vec![message(NO_PLACES)]; }
    rows
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

    /// Trimmed, fictional results in the shape Nominatim's jsonv2 format sends.
    const PLACES: &str = r#"[
        {"place_id":1,"licence":"Data © OpenStreetMap contributors, ODbL 1.0.","osm_type":"way","osm_id":5013364,
         "lat":"48.8582599","lon":"2.2945006","category":"man_made","type":"tower","place_rank":30,"importance":0.6,
         "addresstype":"man_made","name":"Fictional Tower","display_name":"Fictional Tower, 5, Example Avenue, Paris, France",
         "boundingbox":["48.85","48.86","2.29","2.30"]},
        {"place_id":2,"osm_type":"node","osm_id":42,"lat":"51.3336000","lon":"-116.2350000","category":"building","type":"yes",
         "name":"","display_name":"12 Sample Street, Springfield, Canada"},
        {"place_id":3,"osm_type":"","lat":"10.5","lon":"20.25","category":"place","type":"bus_stop","name":"Stop & Go","display_name":"Elsewhere"},
        {"place_id":4,"osm_type":"node","osm_id":7,"lat":"999","lon":"0","category":"x","type":"y","name":"Off the map","display_name":"Nowhere"}]"#;

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render_body(status: i64, body: &str) -> Vec<serde_json::Value> {
        let value = reply(serde_json::json!({"phase":"render","query":"tower","now":0,"responses":[{"url":"u","status":status,"body":body}]}));
        value["items"].as_array().unwrap().clone()
    }

    #[test]
    fn searches_nominatim_with_the_encoded_text() {
        assert_eq!(url_for(" eiffel tower "), "https://nominatim.openstreetmap.org/search?format=jsonv2&limit=10&q=eiffel%20tower");
        assert_eq!(url_for("Café & Bar #1"), "https://nominatim.openstreetmap.org/search?format=jsonv2&limit=10&q=Caf%C3%A9%20%26%20Bar%20%231");
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["paris", "1600 Pennsylvania Ave NW, Washington", "Café & Bar #1", "東京駅", "a/b?c=d"] {
            let plan = reply(serde_json::json!({"phase":"plan","query":query,"now":0}));
            let fetch = plan["fetch"].as_array().unwrap();
            assert_eq!(fetch.len(), 1, "Nominatim's policy: one request per run");
            assert!(manifest_permits(MANIFEST, fetch[0].as_str().unwrap()), "{query}");
        }
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["hosts"], serde_json::json!(["nominatim.openstreetmap.org"]));
        assert_eq!(manifest["mode"], "search");
    }

    #[test]
    fn an_empty_query_asks_for_text_without_fetching() {
        let plan = reply(serde_json::json!({"phase":"plan","query":"  ","now":0}));
        assert!(plan.get("fetch").is_none());
        assert_eq!(plan["items"][0]["title"], PROMPT);
    }

    #[test]
    fn render_builds_place_rows_with_apple_maps_and_openstreetmap_links() {
        let rows = render_body(200, PLACES);
        assert_eq!(rows.len(), 3, "A place without a valid position is skipped");
        assert_eq!(rows[0]["title"], "Fictional Tower");
        assert_eq!(rows[0]["subtitle"], "5, Example Avenue, Paris, France");
        assert_eq!(rows[0]["url"], "https://maps.apple.com/?q=Fictional%20Tower&ll=48.8582599,2.2945006");
        assert_eq!(rows[0]["secondaryURL"], "https://www.openstreetmap.org/way/5013364");
        assert_eq!(rows[0]["secondaryTitle"], OSM_TITLE);
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"Tower"}]));
        assert_eq!(rows[1]["title"], "12 Sample Street", "An unnamed place takes the first part of its address");
        assert_eq!(rows[1]["subtitle"], "Springfield, Canada");
        assert_eq!(rows[1]["badges"][0]["text"], "Building", "type=yes falls back to the category");
        assert_eq!(rows[2]["url"], "https://maps.apple.com/?q=Stop%20%26%20Go&ll=10.5,20.25");
        assert_eq!(rows[2]["secondaryURL"], "https://www.openstreetmap.org/?mlat=10.5&mlon=20.25");
        assert_eq!(rows[2]["badges"][0]["text"], "Bus stop");
    }

    #[test]
    fn empty_results_and_failures_explain_themselves() {
        assert_eq!(render_body(200, "[]")[0]["title"], NO_PLACES);
        assert_eq!(render_body(200, "<html>")[0]["title"], UNREADABLE);
        assert_eq!(render_body(403, "")[0]["title"], REFUSED);
        assert_eq!(render_body(429, "")[0]["title"], REFUSED);
        assert_eq!(render_body(0, "")[0]["title"], "OpenStreetMap search is unavailable (HTTP 0). Try again.");
        let none = reply(serde_json::json!({"phase":"render","query":"x","now":0,"responses":[]}));
        assert_eq!(none["items"][0]["title"], NOTHING);
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
