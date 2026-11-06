//! Public Holidays for Volant: the upcoming public holidays of a country, from today on, read from
//! Nager.Date's free API on date.nager.at. The country is a preference (Volant 0.3.22 and later; the
//! United States until the owner picks another). Typed text narrows the list by name, in English or
//! the local language, and a two-letter country code typed first, such as `holidays de` or
//! `holidays fr noël`, shows that country instead.
//!
//! Each row shows the holiday's name, its date and weekday and how many days away it is, and its local
//! name when that differs. Holidays kept only in some regions carry a Regional badge. Return opens the
//! country's year on nagerholidays.com.
//!
//! The module has no clock, so "today" is the UTC date of the request's `now`. It reads this year,
//! and next year too once the next 90 days reach into it, so late in the year the list carries on
//! into January. When the first word is a country code but the rest matches nothing there, as in
//! `holidays st patrick` (ST is São Tomé and Príncipe), the whole text filters the owner's country
//! instead; both are fetched in the same round, at most four requests.
use serde::{Deserialize, Serialize};
use volant_extension_kit::{volant_extension_output, Preferences};

const API: &str = "https://date.nager.at/api/v3/PublicHolidays/";
const SITE: &str = "https://nagerholidays.com/PublicHoliday/Country/";
const PATH_SEPARATOR: &str = "/";
const COUNTRY_PREFERENCE: &str = "country";
/// The `country` preference's default; the manifest declares the same.
const DEFAULT_COUNTRY: &str = "US";
/// Every country Nager.Date covers (its /api/v3/AvailableCountries), for reading a typed code.
const SUPPORTED: &str = "AD AG AI AL AM AO AR AT AU AW AX BA BB BD BE BF BG BH BI BJ BL BM BO BQ BR BS BW BY BZ CA CC CD CF CG \
    CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG ER ES ET FI FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN \
    GP GQ GR GT GW GY HK HN HR HT HU ID IE IM IQ IS IT JE JM JP KE KH KI KM KN KR KY KZ LC LI LR LS LT LU LV LY MA MC MD ME MF MG \
    MH MK ML MN MP MQ MR MS MT MW MX MZ NA NC NE NF NG NI NL NO NR NU NZ PA PE PF PG PH PL PM PN PR PT PW PY RO RS RU RW SB SC SD \
    SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TG TK TN TO TR TT TV TZ UA UG US UY VA VC VE VG VI VN VU WF WS YE ZA \
    ZM ZW";
const LOOKAHEAD_DAYS: i64 = 90;
const SECONDS_PER_DAY: i64 = 86_400;
const MAXIMUM_ROWS: usize = 100;
const WEEKDAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const TODAY: &str = "today";
const TOMORROW: &str = "tomorrow";
const IN_PREFIX: &str = "in ";
const DAYS_SUFFIX: &str = " days";
const SEPARATOR: &str = " · ";
const REGIONAL: &str = "Regional";
const PLAN: &str = "plan";
const RENDER: &str = "render";
const OK: i64 = 200;
const NOT_FOUND: i64 = 404;
const TOO_MANY: i64 = 429;
const NO_MATCH_PREFIX: &str = "No upcoming holidays in ";
const NO_MATCH_SUFFIX: &str = " match that name.";
const NONE_PREFIX: &str = "No upcoming public holidays found for ";
const NONE_SUFFIX: &str = ".";
const UNKNOWN_PREFIX: &str = "Nager.Date has no holidays for ";
const UNKNOWN_SUFFIX: &str = ".";
const BUSY: &str = "Nager.Date is limiting requests. Try again in a minute.";
const UNAVAILABLE_PREFIX: &str = "Nager.Date is unavailable (HTTP ";
const UNAVAILABLE_SUFFIX: &str = "). Try again.";
const UNREADABLE: &str = "Nager.Date sent a reply Volant could not read.";
const NOT_UNDERSTOOD: &str = "Volant sent a request this extension does not understand.";
const ONLY_LISTS: &str = "This extension only lists.";

#[derive(Deserialize)]
struct Request {
    phase: String,
    #[serde(default)]
    query: String,
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

/// The fields of a Nager.Date holiday this extension reads.
#[derive(Deserialize)]
struct Holiday {
    date: String,
    #[serde(default, rename = "localName")]
    local_name: String,
    name: String,
    #[serde(default = "national")]
    global: bool,
}

fn national() -> bool { true }

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

/// A row badge: a short text Volant shows at the row's end (Volant 0.3.8 and later).
#[derive(Serialize, Debug, PartialEq)]
struct Badge { text: String }

/// One explaining row, for an empty list or a failure.
fn message(text: &str) -> Row {
    Row { title: text.to_string(), ..Default::default() }
}

/// One way to read the typed text: a country and the text that filters its holidays.
#[derive(Debug, PartialEq)]
struct Reading { country: String, filter: String }

/// A two-letter country Nager.Date covers, uppercased, or none.
fn supported(code: &str) -> Option<String> {
    let upper = code.to_ascii_uppercase();
    (upper.len() == 2 && upper.bytes().all(|byte| byte.is_ascii_uppercase()) && SUPPORTED.split_whitespace().any(|known| known == upper))
        .then_some(upper)
}

/// The owner's country from preferences, or the default when it is missing or not one Nager.Date covers.
fn preferred(preferences: &Preferences) -> String {
    supported(preferences.string(COUNTRY_PREFERENCE, DEFAULT_COUNTRY)).unwrap_or_else(|| DEFAULT_COUNTRY.to_string())
}

/// The readings to try, in order. A typed country code alone shows that country. A code followed by
/// more text tries that country filtered by the rest, then the owner's country filtered by everything.
/// Any other text filters the owner's country.
fn readings(query: &str, preferred: &str) -> Vec<Reading> {
    let query = query.trim();
    let (first, rest) = query.split_once(char::is_whitespace).unwrap_or((query, ""));
    let whole = Reading { country: preferred.to_string(), filter: query.to_string() };
    match supported(first) {
        Some(country) if rest.trim().is_empty() => vec![Reading { country, filter: String::new() }],
        Some(country) => vec![Reading { country, filter: rest.trim().to_string() }, whole],
        None => vec![whole],
    }
}

/// Days since 1970-01-01 for a civil date (proleptic Gregorian).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let (y, m) = if month <= 2 { (year - 1, month + 9) } else { (year, month - 3) };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * m + 2) / 5 + day - 1;
    era * 146_097 + year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year - 719_468
}

/// The civil date (year, month, day) for days since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 { shifted_month + 3 } else { shifted_month - 9 };
    (era * 400 + year_of_era + i64::from(month <= 2), month, day)
}

/// Days since 1970-01-01 for a "YYYY-MM-DD" date.
fn parse_date(text: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    ((1..=12).contains(&month) && (1..=31).contains(&day) && text.len() == 10).then(|| days_from_civil(year, month, day))
}

/// The years to read for a request time: this year, and next year once the lookahead reaches it.
pub fn years(now: i64) -> Vec<i64> {
    let today = now.div_euclid(SECONDS_PER_DAY);
    let (year, ..) = civil_from_days(today);
    let (ahead, ..) = civil_from_days(today + LOOKAHEAD_DAYS);
    if ahead > year { vec![year, ahead] } else { vec![year] }
}

/// The Nager.Date request for one country's year.
pub fn url_for(country: &str, year: i64) -> String {
    format!("{API}{year}{PATH_SEPARATOR}{country}")
}

/// Every request for a query: each reading's country for each year, without repeats, at most four.
pub fn plan_urls(query: &str, preferred: &str, now: i64) -> Vec<String> {
    let mut urls = Vec::new();
    for reading in readings(query, preferred) {
        for year in years(now) {
            let url = url_for(&reading.country, year);
            if !urls.contains(&url) { urls.push(url); }
        }
    }
    urls
}

/// What a failed request says.
fn failure(status: i64, country: &str) -> String {
    match status {
        NOT_FOUND => format!("{UNKNOWN_PREFIX}{country}{UNKNOWN_SUFFIX}"),
        TOO_MANY => BUSY.to_string(),
        _ => format!("{UNAVAILABLE_PREFIX}{status}{UNAVAILABLE_SUFFIX}"),
    }
}

/// "Thursday, November 26, 2026 · in 20 days".
fn when(days: i64, today: i64) -> String {
    let (year, month, day) = civil_from_days(days);
    let weekday = WEEKDAYS[(days + 4).rem_euclid(7) as usize];
    let distance = match days - today {
        0 => TODAY.to_string(),
        1 => TOMORROW.to_string(),
        count => format!("{IN_PREFIX}{count}{DAYS_SUFFIX}"),
    };
    format!("{weekday}, {} {day}, {year}{SEPARATOR}{distance}", MONTHS[(month - 1) as usize])
}

/// Whether a holiday matches typed text: every word appears in its English or local name.
fn matches(holiday: &Holiday, filter: &str) -> bool {
    let names = format!("{} {}", holiday.name, holiday.local_name).to_lowercase();
    filter.split_whitespace().all(|word| names.contains(word))
}

/// The upcoming holidays for one reading, or what to say when one of its requests failed. Nager.Date
/// lists a holiday once per group of regions that keep it, so a repeated name and date shows once,
/// as Regional only when every entry is.
fn reading_rows(reading: &Reading, responses: &[Response], now: i64) -> Result<Vec<Row>, String> {
    let today = now.div_euclid(SECONDS_PER_DAY);
    let filter = reading.filter.to_lowercase();
    let mut dated: Vec<(i64, i64, Holiday)> = Vec::new();
    for year in years(now) {
        let url = url_for(&reading.country, year);
        let Some(response) = responses.iter().find(|response| response.url == url) else { return Err(failure(0, &reading.country)) };
        if response.status != OK { return Err(failure(response.status, &reading.country)); }
        let holidays = serde_json::from_str::<Vec<Holiday>>(&response.body).map_err(|_| UNREADABLE.to_string())?;
        for holiday in holidays {
            let Some(days) = parse_date(&holiday.date) else { continue };
            if days < today || !matches(&holiday, &filter) { continue; }
            match dated.iter_mut().find(|(seen, _, kept)| *seen == days && kept.name == holiday.name) {
                Some((_, _, kept)) => kept.global |= holiday.global,
                None => dated.push((days, year, holiday)),
            }
        }
    }
    dated.sort_by_key(|(days, ..)| *days);
    Ok(dated.into_iter().take(MAXIMUM_ROWS).map(|(days, year, holiday)| {
        let mut subtitle = when(days, today);
        let local = holiday.local_name.trim();
        if !local.is_empty() && local != holiday.name.trim() { subtitle = format!("{subtitle}{SEPARATOR}{local}"); }
        Row {
            title: holiday.name,
            subtitle: Some(subtitle),
            url: Some(format!("{SITE}{}{PATH_SEPARATOR}{year}", reading.country)),
            badges: if holiday.global { Vec::new() } else { vec![Badge { text: REGIONAL.to_string() }] },
        }
    }).collect())
}

/// The rows for a render: the first reading with holidays to show, otherwise what the last one says.
fn render(request: &Request, preferred: &str) -> Vec<Row> {
    let readings = readings(&request.query, preferred);
    let mut last = Vec::new();
    for reading in &readings {
        match reading_rows(reading, &request.responses, request.now) {
            Ok(rows) if !rows.is_empty() => return rows,
            Ok(_) if reading.filter.is_empty() => last = vec![message(&format!("{NONE_PREFIX}{}{NONE_SUFFIX}", reading.country))],
            Ok(_) => last = vec![message(&format!("{NO_MATCH_PREFIX}{}{NO_MATCH_SUFFIX}", reading.country))],
            Err(reason) => last = vec![message(&reason)],
        }
    }
    last
}

/// Handles one request and returns the reply Volant reads.
pub fn respond(input: &[u8]) -> String {
    let Ok(request) = serde_json::from_slice::<Request>(input) else { return items(vec![message(NOT_UNDERSTOOD)]) };
    let preferred = preferred(&Preferences::from_input(input));
    match request.phase.as_str() {
        PLAN => serde_json::json!({ "fetch": plan_urls(&request.query, &preferred, request.now) }).to_string(),
        RENDER => items(render(&request, &preferred)),
        _ => items(vec![message(ONLY_LISTS)]),
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
    /// 2026-11-06T12:00:00Z, a Friday.
    const NOVEMBER: i64 = 1_793_966_400;
    /// 2026-03-02T12:00:00Z.
    const MARCH: i64 = 1_772_452_800;

    /// Trimmed, fictional holidays in the shape Nager.Date sends.
    const US_2026: &str = r#"[
        {"date":"2026-01-01","localName":"New Year's Day","name":"New Year's Day","countryCode":"US","fixed":false,"global":true,"counties":null,"launchYear":null,"types":["Public","Bank"]},
        {"date":"2026-11-11","localName":"Veterans Day","name":"Veterans Day","countryCode":"US","global":true,"types":["Public"]},
        {"date":"2026-11-06","localName":"Sample Day","name":"Sample Day","countryCode":"US","global":false,"counties":["US-CA"],"types":["Observance"]},
        {"date":"2026-11-26","localName":"Thanksgiving Day","name":"Thanksgiving Day","countryCode":"US","global":true,"types":["Public"]}]"#;
    const US_2027: &str = r#"[
        {"date":"2027-01-01","localName":"New Year's Day","name":"New Year's Day","countryCode":"US","global":true,"types":["Public"]}]"#;
    const DE_2026: &str = r#"[
        {"date":"2026-11-07","localName":"Beispieltag","name":"Example Day","countryCode":"DE","global":true,"types":["Public"]},
        {"date":"2026-12-25","localName":"Erster Weihnachtstag","name":"Christmas Day","countryCode":"DE","global":true,"types":["Public"]}]"#;
    const DE_2027: &str = "[]";

    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&respond(request.to_string().as_bytes())).unwrap()
    }

    fn render_rows(query: &str, preferences: serde_json::Value, responses: &[(&str, i64, &str)]) -> Vec<serde_json::Value> {
        let responses: Vec<serde_json::Value> = responses.iter().map(|(url, status, body)| serde_json::json!({"url":url,"status":status,"body":body})).collect();
        let value = reply(serde_json::json!({"phase":"render","query":query,"now":NOVEMBER,"preferences":preferences,"responses":responses}));
        value["items"].as_array().unwrap().clone()
    }

    fn all_responses() -> Vec<(&'static str, i64, &'static str)> {
        vec![("https://date.nager.at/api/v3/PublicHolidays/2026/US", 200, US_2026), ("https://date.nager.at/api/v3/PublicHolidays/2027/US", 200, US_2027),
             ("https://date.nager.at/api/v3/PublicHolidays/2026/DE", 200, DE_2026), ("https://date.nager.at/api/v3/PublicHolidays/2027/DE", 200, DE_2027)]
    }

    #[test]
    fn dates_round_trip_and_name_their_weekday() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(parse_date("2026-11-06"), Some(NOVEMBER / SECONDS_PER_DAY));
        assert_eq!(civil_from_days(parse_date("2000-02-29").unwrap()), (2000, 2, 29));
        assert_eq!(parse_date("2026-13-01"), None);
        assert_eq!(when(parse_date("2026-11-26").unwrap(), NOVEMBER / SECONDS_PER_DAY), "Thursday, November 26, 2026 · in 20 days");
    }

    #[test]
    fn reads_next_year_once_the_lookahead_reaches_it() {
        assert_eq!(years(MARCH), [2026]);
        assert_eq!(years(NOVEMBER), [2026, 2027]);
        assert_eq!(plan_urls("", "US", MARCH), ["https://date.nager.at/api/v3/PublicHolidays/2026/US"]);
    }

    #[test]
    fn typed_codes_pick_a_country_and_fall_back_to_the_owner_s() {
        assert_eq!(readings("de", "US"), [Reading { country: "DE".into(), filter: String::new() }]);
        assert_eq!(readings("christmas", "US"), [Reading { country: "US".into(), filter: "christmas".into() }]);
        assert_eq!(readings("st patrick", "IE"), [Reading { country: "ST".into(), filter: "patrick".into() },
            Reading { country: "IE".into(), filter: "st patrick".into() }]);
        assert_eq!(readings("zz day", "US").len(), 1, "Codes Nager.Date does not cover are text");
        assert_eq!(plan_urls("fr noël", "US", NOVEMBER).len(), 4);
    }

    #[test]
    fn every_planned_url_is_on_a_manifest_host() {
        for query in ["", "de", "fr noël", "christmas", "st patrick", "C++ & you?", "zz"] {
            for (now, country) in [(NOVEMBER, "us"), (MARCH, "gb"), (NOVEMBER, "jp")] {
                let plan = reply(serde_json::json!({"phase":"plan","query":query,"now":now,"preferences":{"country":country}}));
                let fetch = plan["fetch"].as_array().unwrap();
                assert!(!fetch.is_empty() && fetch.len() <= 4, "{query}");
                for url in fetch { assert!(manifest_permits(MANIFEST, url.as_str().unwrap()), "{url}"); }
            }
        }
    }

    #[test]
    fn the_country_preference_matches_the_manifest() {
        let manifest: serde_json::Value = serde_json::from_str(MANIFEST).unwrap();
        let preference = &manifest["preferences"][0];
        assert_eq!(preference["id"], COUNTRY_PREFERENCE);
        assert_eq!(preference["default"], DEFAULT_COUNTRY.to_lowercase());
        let choices = preference["choices"].as_array().unwrap();
        assert!(choices.len() <= 20);
        for choice in choices { assert!(supported(choice["id"].as_str().unwrap()).is_some(), "{choice}"); }
        let plan = |preferences: serde_json::Value| reply(serde_json::json!({"phase":"plan","query":"","now":MARCH,"preferences":preferences}))["fetch"][0].clone();
        assert_eq!(plan(serde_json::json!({})), "https://date.nager.at/api/v3/PublicHolidays/2026/US", "Older Volant sends no preferences");
        assert_eq!(plan(serde_json::json!({"country":"gb"})), "https://date.nager.at/api/v3/PublicHolidays/2026/GB");
        assert_eq!(plan(serde_json::json!({"country":"../x"})), "https://date.nager.at/api/v3/PublicHolidays/2026/US");
    }

    #[test]
    fn render_lists_upcoming_holidays_across_the_new_year() {
        let rows = render_rows("", serde_json::json!({"country":"us"}), &all_responses());
        let titles: Vec<&str> = rows.iter().map(|row| row["title"].as_str().unwrap()).collect();
        assert_eq!(titles, ["Sample Day", "Veterans Day", "Thanksgiving Day", "New Year's Day"]);
        assert_eq!(rows[0]["subtitle"], "Friday, November 6, 2026 · today");
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"Regional"}]));
        assert!(rows[1].get("badges").is_none());
        assert_eq!(rows[3]["subtitle"], "Friday, January 1, 2027 · in 56 days");
        assert_eq!(rows[3]["url"], "https://nagerholidays.com/PublicHoliday/Country/US/2027");
    }

    #[test]
    fn typed_text_filters_by_name_or_picks_a_country() {
        let rows = render_rows("de", serde_json::json!({}), &all_responses());
        assert_eq!(rows[0]["title"], "Example Day");
        assert_eq!(rows[0]["subtitle"], "Saturday, November 7, 2026 · tomorrow · Beispieltag");
        assert_eq!(render_rows("de weihnacht", serde_json::json!({}), &all_responses())[0]["title"], "Christmas Day", "Local names match too");
        let rows = render_rows("de thanks", serde_json::json!({}), &all_responses());
        assert_eq!(rows[0]["title"], NO_MATCH_PREFIX.to_string() + "US" + NO_MATCH_SUFFIX, "Falls back to the whole text in the owner's country");
        let rows = render_rows("THANKS", serde_json::json!({}), &all_responses());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["title"], "Thanksgiving Day");
    }

    #[test]
    fn repeated_regional_entries_show_once_and_words_match_in_any_order() {
        let body = r#"[
            {"date":"2026-12-24","localName":"Sample Eve","name":"Sample Eve","global":false,"counties":["US-A"]},
            {"date":"2026-12-24","localName":"Sample Eve","name":"Sample Eve","global":false,"counties":["US-B"]},
            {"date":"2026-12-26","localName":"Second Sample","name":"Second Sample","global":false},
            {"date":"2026-12-26","localName":"Second Sample","name":"Second Sample","global":true}]"#;
        let responses = [("https://date.nager.at/api/v3/PublicHolidays/2026/US", 200, body), ("https://date.nager.at/api/v3/PublicHolidays/2027/US", 200, "[]")];
        let rows = render_rows("", serde_json::json!({}), &responses);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["badges"], serde_json::json!([{"text":"Regional"}]));
        assert!(rows[1].get("badges").is_none(), "National in any entry is national");
        assert_eq!(render_rows("eve sample", serde_json::json!({}), &responses).len(), 1);
    }

    #[test]
    fn failures_and_empty_lists_explain_themselves() {
        let failed = render_rows("", serde_json::json!({}), &[("https://date.nager.at/api/v3/PublicHolidays/2026/US", 0, "")]);
        assert_eq!(failed[0]["title"], "Nager.Date is unavailable (HTTP 0). Try again.");
        let missing = render_rows("ad", serde_json::json!({}), &[("https://date.nager.at/api/v3/PublicHolidays/2026/AD", 404, ""),
            ("https://date.nager.at/api/v3/PublicHolidays/2027/AD", 404, "")]);
        assert_eq!(missing[0]["title"], "Nager.Date has no holidays for AD.");
        let busy = render_rows("", serde_json::json!({}), &[("https://date.nager.at/api/v3/PublicHolidays/2026/US", 429, ""),
            ("https://date.nager.at/api/v3/PublicHolidays/2027/US", 200, "[]")]);
        assert_eq!(busy[0]["title"], BUSY);
        let unreadable = render_rows("", serde_json::json!({}), &[("https://date.nager.at/api/v3/PublicHolidays/2026/US", 200, "<html>"),
            ("https://date.nager.at/api/v3/PublicHolidays/2027/US", 200, "[]")]);
        assert_eq!(unreadable[0]["title"], UNREADABLE);
        let empty = render_rows("", serde_json::json!({}), &[("https://date.nager.at/api/v3/PublicHolidays/2026/US", 200, "[]"),
            ("https://date.nager.at/api/v3/PublicHolidays/2027/US", 200, "[]")]);
        assert_eq!(empty[0]["title"], "No upcoming public holidays found for US.");
        assert_eq!(reply(serde_json::json!({"phase":"action","query":"","now":0}))["items"][0]["title"], ONLY_LISTS);
    }
}
