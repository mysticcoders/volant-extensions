//! Weather for Volant: current conditions and a 10-day forecast from Open-Meteo. `weather` uses the
//! owner's location when the `location` permission is allowed; `weather <place>` looks the place up
//! first, then fetches its forecast in a second round. Temperatures and wind arrive in metric and are
//! converted for display when the owner's system uses US units.
use serde::Deserialize;
use volant_extension_kit::{encode_query, parse_rfc3339, volant_extension, Item, Location, Reply, Request};

const GEOCODE: &str = "https://geocoding-api.open-meteo.com/v1/search?count=1&language=en&format=json&name=";
const FORECAST: &str = "https://api.open-meteo.com/v1/forecast";
const FORECAST_FIELDS: &str = "&current=temperature_2m,apparent_temperature,weather_code,wind_speed_10m,is_day&daily=weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max&timezone=auto&forecast_days=10";
const ATTRIBUTION_URL: &str = "https://open-meteo.com/";
const ATTRIBUTION_TITLE: &str = "Weather Data by Open-Meteo";
const GEOCODE_STATE: &str = "geocode";
const FORECAST_STATE: &str = "forecast:";
const HERE: &str = "Current location";
const USAGE: &str = "Type weather and a place, such as weather paris, or allow location for local weather.";
const NOTHING: &str = "Open-Meteo returned nothing.";
const UNREADABLE: &str = "Open-Meteo sent a forecast Volant could not read.";
const SEPARATOR: &str = " · ";
const PLACE_SEPARATOR: &str = ", ";
const DEGREE: &str = "°";
const MPH: &str = "mph";
const KMH: &str = "km/h";
const TODAY: &str = "Today";
const TOMORROW: &str = "Tomorrow";
const MIDNIGHT_UTC: &str = "T00:00:00Z";
const KM_PER_MILE: f64 = 1.609_344;
const SECONDS_PER_DAY: i64 = 86_400;
const EPOCH_WEEKDAY: i64 = 4;
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const UNKNOWN_CONDITION: &str = "Unknown conditions";

/// WMO weather interpretation codes, as Open-Meteo reports them: how each reads in a row, and the
/// Volant row symbol for it by day and by night.
const CONDITIONS: &[(&[i64], &str, &str, &str)] = &[
    (&[0], "Clear", SUN, MOON),
    (&[1], "Mainly clear", CLOUD_SUN, CLOUD_MOON),
    (&[2], "Partly cloudy", CLOUD_SUN, CLOUD_MOON),
    (&[3], "Overcast", CLOUD, CLOUD),
    (&[45, 48], "Fog", FOG, FOG),
    (&[51, 53, 55], "Drizzle", DRIZZLE, DRIZZLE),
    (&[56, 57], "Freezing drizzle", DRIZZLE, DRIZZLE),
    (&[61], "Light rain", RAIN, RAIN),
    (&[63], "Rain", RAIN, RAIN),
    (&[65], "Heavy rain", HEAVY_RAIN, HEAVY_RAIN),
    (&[66, 67], "Freezing rain", SLEET, SLEET),
    (&[71], "Light snow", SNOW, SNOW),
    (&[73], "Snow", SNOW, SNOW),
    (&[75], "Heavy snow", SNOW, SNOW),
    (&[77], "Snow grains", SNOW, SNOW),
    (&[80, 81], "Rain showers", RAIN, RAIN),
    (&[82], "Heavy rain showers", HEAVY_RAIN, HEAVY_RAIN),
    (&[85, 86], "Snow showers", SNOW, SNOW),
    (&[95], "Thunderstorm", THUNDERSTORM, THUNDERSTORM),
    (&[96, 99], "Thunderstorm with hail", THUNDERSTORM, THUNDERSTORM),
];
const SUN: &str = "sun";
const MOON: &str = "moon";
const CLOUD_SUN: &str = "cloud-sun";
const CLOUD_MOON: &str = "cloud-moon";
const CLOUD: &str = "cloud";
const FOG: &str = "fog";
const DRIZZLE: &str = "drizzle";
const RAIN: &str = "rain";
const HEAVY_RAIN: &str = "heavy-rain";
const SLEET: &str = "sleet";
const SNOW: &str = "snow";
const THUNDERSTORM: &str = "thunderstorm";

#[derive(Deserialize)]
struct Places {
    #[serde(default)]
    results: Vec<Place>,
}

#[derive(Deserialize)]
struct Place {
    name: String,
    latitude: f64,
    longitude: f64,
    admin1: Option<String>,
    country: Option<String>,
}

#[derive(Deserialize)]
struct Forecast {
    current: Option<Current>,
    daily: Option<Daily>,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: Option<f64>,
    apparent_temperature: Option<f64>,
    weather_code: Option<i64>,
    wind_speed_10m: Option<f64>,
    /// 1 by day and 0 at night at the place, which picks the moon symbols.
    is_day: Option<i64>,
}

#[derive(Deserialize)]
struct Daily {
    time: Vec<String>,
    weather_code: Vec<Option<i64>>,
    temperature_2m_max: Vec<Option<f64>>,
    temperature_2m_min: Vec<Option<f64>>,
    #[serde(default)]
    precipitation_probability_max: Vec<Option<i64>>,
}

/// The forecast request for a point, in metric.
pub fn forecast_url(location: Location) -> String {
    format!("{FORECAST}?latitude={:.2}&longitude={:.2}{FORECAST_FIELDS}", location.latitude, location.longitude)
}

/// The place-name lookup for a query.
pub fn geocode_url(query: &str) -> String {
    format!("{GEOCODE}{}", encode_query(query))
}

/// "Paris, Île-de-France, France" from a lookup result, leaving out parts that repeat or are missing.
fn place_label(place: &Place) -> String {
    let mut parts = vec![place.name.clone()];
    for part in [&place.admin1, &place.country].into_iter().flatten() {
        if !parts.contains(part) { parts.push(part.clone()); }
    }
    parts.join(PLACE_SEPARATOR)
}

/// What a WMO weather code means, in words.
fn condition(code: Option<i64>) -> &'static str {
    code.and_then(|code| CONDITIONS.iter().find(|(codes, ..)| codes.contains(&code)).map(|(_, text, ..)| *text)).unwrap_or(UNKNOWN_CONDITION)
}

/// The row symbol for a WMO weather code, by day or by night; none for an unknown code.
pub fn symbol(code: Option<i64>, day: bool) -> Option<String> {
    let entry = code.and_then(|code| CONDITIONS.iter().find(|(codes, ..)| codes.contains(&code)))?;
    Some(if day { entry.2 } else { entry.3 }.to_string())
}

/// A Celsius temperature as whole degrees in the owner's units.
pub fn temperature(celsius: f64, us: bool) -> String {
    let degrees = if us { celsius * 9.0 / 5.0 + 32.0 } else { celsius };
    format!("{}{DEGREE}", degrees.round() as i64)
}

/// A km/h wind speed in the owner's units.
pub fn wind(kmh: f64, us: bool) -> String {
    if us { format!("{} {MPH}", (kmh / KM_PER_MILE).round() as i64) } else { format!("{} {KMH}", kmh.round() as i64) }
}

/// "Today", "Tomorrow", then "Thu, Oct 16" for each forecast date, which Open-Meteo gives in the
/// place's own time zone with today first.
pub fn day_label(index: usize, date: &str) -> String {
    match index {
        0 => TODAY.to_string(),
        1 => TOMORROW.to_string(),
        _ => {
            let days = parse_rfc3339(&format!("{date}{MIDNIGHT_UTC}")).map(|seconds| seconds.div_euclid(SECONDS_PER_DAY));
            let month = date.get(5..7).and_then(|m| m.parse::<usize>().ok()).and_then(|m| MONTHS.get(m.wrapping_sub(1)));
            let day = date.get(8..10).and_then(|d| d.parse::<u32>().ok());
            match (days, month, day) {
                (Some(days), Some(month), Some(day)) => format!("{}, {month} {day}", WEEKDAYS[(days + EPOCH_WEEKDAY).rem_euclid(7) as usize]),
                _ => date.to_string(),
            }
        }
    }
}

fn unavailable(status: i64) -> String {
    format!("Open-Meteo is unavailable (HTTP {status}). Try again.")
}

fn no_place(query: &str) -> String {
    format!("Open-Meteo knows no place called {query}.")
}

fn feels_like(text: String) -> String {
    format!("Feels like {text}")
}

fn wind_text(text: String) -> String {
    format!("Wind {text}")
}

fn rain_chance(percent: i64) -> String {
    format!("{percent}% chance of rain")
}

/// The first round: a forecast for the owner's location, or a lookup of the place they typed.
fn plan(request: &Request) -> Result<Reply, String> {
    let query = request.query.trim();
    if !query.is_empty() {
        return Ok(Reply::Continue(vec![geocode_url(query)], GEOCODE_STATE.to_string()));
    }
    let location = request.location.ok_or(USAGE)?;
    Ok(Reply::Continue(vec![forecast_url(location)], format!("{FORECAST_STATE}{HERE}")))
}

/// A later round: turns a place lookup into a forecast request, or a forecast into rows.
fn render(request: &Request) -> Result<Reply, String> {
    let response = request.responses.first().ok_or(NOTHING)?;
    if response.status != 200 {
        return Err(unavailable(response.status));
    }
    let state = request.state.as_deref().unwrap_or_default();
    if state == GEOCODE_STATE {
        let places: Places = serde_json::from_str(&response.body).map_err(|_| UNREADABLE.to_string())?;
        let place = places.results.first().ok_or_else(|| no_place(request.query.trim()))?;
        let location = Location { latitude: place.latitude, longitude: place.longitude };
        return Ok(Reply::Continue(vec![forecast_url(location)], format!("{FORECAST_STATE}{}", place_label(place))));
    }
    let label = state.strip_prefix(FORECAST_STATE).unwrap_or(HERE);
    let forecast: Forecast = serde_json::from_str(&response.body).map_err(|_| UNREADABLE.to_string())?;
    Ok(Reply::Items(rows(label, &forecast, request.uses_us_units())))
}

/// The current-conditions row followed by one row per forecast day.
fn rows(label: &str, forecast: &Forecast, us: bool) -> Vec<Item> {
    let mut items = Vec::new();
    if let Some(current) = &forecast.current {
        let mut title = vec![label.to_string()];
        if let Some(celsius) = current.temperature_2m { title.push(temperature(celsius, us)); }
        title.push(condition(current.weather_code).to_string());
        let mut details = Vec::new();
        if let Some(celsius) = current.apparent_temperature { details.push(feels_like(temperature(celsius, us))); }
        if let Some(kmh) = current.wind_speed_10m { details.push(wind_text(wind(kmh, us))); }
        items.push(Item {
            title: title.join(SEPARATOR),
            subtitle: Some(details.join(SEPARATOR)).filter(|s| !s.is_empty()),
            secondary_url: Some(ATTRIBUTION_URL.to_string()),
            secondary_title: Some(ATTRIBUTION_TITLE.to_string()),
            symbol: symbol(current.weather_code, current.is_day != Some(0)),
            ..Default::default()
        });
    }
    if let Some(daily) = &forecast.daily {
        for (index, date) in daily.time.iter().enumerate() {
            let high = daily.temperature_2m_max.get(index).copied().flatten();
            let low = daily.temperature_2m_min.get(index).copied().flatten();
            let mut details = Vec::new();
            if let (Some(high), Some(low)) = (high, low) { details.push(format!("{} / {}", temperature(high, us), temperature(low, us))); }
            if let Some(percent) = daily.precipitation_probability_max.get(index).copied().flatten() { details.push(rain_chance(percent)); }
            items.push(Item {
                title: format!("{}{SEPARATOR}{}", day_label(index, date), condition(daily.weather_code.get(index).copied().flatten())),
                subtitle: Some(details.join(SEPARATOR)).filter(|s| !s.is_empty()),
                symbol: symbol(daily.weather_code.get(index).copied().flatten(), true),
                ..Default::default()
            });
        }
    }
    if items.is_empty() {
        items.push(Item { title: UNREADABLE.to_string(), ..Default::default() });
    }
    items
}

fn handle(request: &Request) -> Result<Reply, String> {
    if request.is_plan() { plan(request) } else { render(request) }
}

volant_extension!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::{handle as run, manifest_permits};

    const MANIFEST: &str = include_str!("../manifest.json");

    /// Runs the handler on a request the way Volant would, and parses the reply.
    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&run(request.to_string().as_bytes(), handle)).unwrap()
    }

    const FORECAST_BODY: &str = r#"{"current":{"temperature_2m":22.9,"apparent_temperature":22.3,"weather_code":2,"wind_speed_10m":16.1},
        "daily":{"time":["2026-10-14","2026-10-15","2026-10-16"],"weather_code":[2,61,95],"temperature_2m_max":[23.3,18.0,15.0],
        "temperature_2m_min":[8.7,9.0,null],"precipitation_probability_max":[0,70,90]}}"#;

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        let here = Location { latitude: 37.8, longitude: -122.3 };
        for url in [forecast_url(here), geocode_url("São Paulo & more")] {
            assert!(manifest_permits(MANIFEST, &url), "{url}");
        }
    }

    #[test]
    fn plan_uses_location_or_looks_up_the_place() {
        let local = reply(serde_json::json!({"phase":"plan","query":"","now":0,"location":{"latitude":37.8,"longitude":-122.3}}));
        assert_eq!(local["state"], "forecast:Current location");
        assert!(local["fetch"][0].as_str().unwrap().starts_with("https://api.open-meteo.com/v1/forecast?latitude=37.80&longitude=-122.30&"));
        let place = reply(serde_json::json!({"phase":"plan","query":"new york","now":0}));
        assert_eq!(place["state"], "geocode");
        assert!(place["fetch"][0].as_str().unwrap().ends_with("&name=new%20york"));
        let neither = reply(serde_json::json!({"phase":"plan","query":" ","now":0}));
        assert_eq!(neither["items"][0]["title"], USAGE);
    }

    #[test]
    fn a_found_place_continues_to_its_forecast() {
        let body = r#"{"results":[{"name":"Paris","latitude":48.85341,"longitude":2.3488,"admin1":"Île-de-France","country":"France"}]}"#;
        let next = reply(serde_json::json!({"phase":"render","query":"paris","now":0,"state":"geocode","responses":[{"url":"u","status":200,"body":body}]}));
        assert_eq!(next["state"], "forecast:Paris, Île-de-France, France");
        assert!(next["fetch"][0].as_str().unwrap().contains("latitude=48.85&longitude=2.35"));
        let none = reply(serde_json::json!({"phase":"render","query":"zzqx","now":0,"state":"geocode","responses":[{"url":"u","status":200,"body":"{}"}]}));
        assert_eq!(none["items"][0]["title"], "Open-Meteo knows no place called zzqx.");
    }

    #[test]
    fn a_forecast_renders_in_the_owners_units() {
        let us = reply(serde_json::json!({"phase":"render","query":"","now":0,"units":"us","state":"forecast:Current location","responses":[{"url":"u","status":200,"body":FORECAST_BODY}]}));
        let items = us["items"].as_array().unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(items[0]["title"], "Current location · 73° · Partly cloudy");
        assert_eq!(items[0]["subtitle"], "Feels like 72° · Wind 10 mph");
        assert_eq!(items[0]["secondaryURL"], "https://open-meteo.com/");
        assert_eq!(items[1]["title"], "Today · Partly cloudy");
        assert_eq!(items[1]["subtitle"], "74° / 48° · 0% chance of rain");
        assert_eq!(items[2]["title"], "Tomorrow · Light rain");
        assert_eq!(items[3]["title"], "Fri, Oct 16 · Thunderstorm");
        assert_eq!(items[3]["subtitle"], "90% chance of rain");
        assert_eq!(items[0]["symbol"], "cloud-sun");
        assert_eq!(items[2]["symbol"], "rain");
        assert_eq!(items[3]["symbol"], "thunderstorm");
        let metric = reply(serde_json::json!({"phase":"render","query":"","now":0,"units":"metric","state":"forecast:Paris","responses":[{"url":"u","status":200,"body":FORECAST_BODY}]}));
        assert_eq!(metric["items"][0]["title"], "Paris · 23° · Partly cloudy");
        assert_eq!(metric["items"][0]["subtitle"], "Feels like 22° · Wind 16 km/h");
    }

    #[test]
    fn failures_become_a_single_explaining_row() {
        let busy = reply(serde_json::json!({"phase":"render","query":"","now":0,"state":"forecast:Paris","responses":[{"url":"u","status":429,"body":"{\"error\":true}"}]}));
        assert_eq!(busy["items"][0]["title"], "Open-Meteo is unavailable (HTTP 429). Try again.");
        let garbage = reply(serde_json::json!({"phase":"render","query":"","now":0,"state":"forecast:Paris","responses":[{"url":"u","status":200,"body":"<html>"}]}));
        assert_eq!(garbage["items"][0]["title"], UNREADABLE);
    }

    #[test]
    fn symbols_follow_the_code_and_the_night() {
        assert_eq!(symbol(Some(0), true).as_deref(), Some("sun"));
        assert_eq!(symbol(Some(0), false).as_deref(), Some("moon"));
        assert_eq!(symbol(Some(2), false).as_deref(), Some("cloud-moon"));
        assert_eq!(symbol(Some(82), true).as_deref(), Some("heavy-rain"));
        assert_eq!(symbol(Some(67), true).as_deref(), Some("sleet"));
        assert_eq!(symbol(Some(1234), true), None);
        let night = r#"{"current":{"temperature_2m":10.0,"weather_code":0,"is_day":0},"daily":null}"#;
        let value = reply(serde_json::json!({"phase":"render","query":"","now":0,"state":"forecast:Paris","responses":[{"url":"u","status":200,"body":night}]}));
        assert_eq!(value["items"][0]["symbol"], "moon");
    }

    #[test]
    fn days_and_units_read_naturally() {
        assert_eq!(day_label(2, "2026-10-16"), "Fri, Oct 16");
        assert_eq!(day_label(5, "2027-01-01"), "Fri, Jan 1");
        assert_eq!(temperature(-40.0, true), "-40°");
        assert_eq!(wind(100.0, true), "62 mph");
    }
}
