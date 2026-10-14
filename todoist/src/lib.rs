//! Todoist for Volant: today's tasks at a keystroke, quick add in Todoist's own natural language, and
//! complete or reschedule from a row's Actions. `todo` shows today and overdue, `todo upcoming`,
//! `todo inbox` and `todo p1` show those views, `todo /<filter>` runs any Todoist filter, and any
//! other text offers to add it as a task, followed by the tasks that match it.
//!
//! Return opens a task in Todoist.app (Volant falls back to the web link when it is not installed),
//! and Command-Return opens it on app.todoist.com.
//!
//! Volant adds the owner's API token to requests to api.todoist.com itself, so this module never
//! sees it. Changes are sent only from the `action` phase, after the owner chooses a row or action.
use std::collections::HashMap;

use serde::Deserialize;
use volant_extension_kit::{encode_query, volant_extension, Action, Item, Reply, Request, Submission};

const API: &str = "https://api.todoist.com/api/v1/";
const FILTER: &str = "tasks/filter?limit=30&query=";
const QUICK_ADD: &str = "tasks/quick";
const PROJECTS: &str = "projects?limit=200";
const TASKS: &str = "tasks/";
const CLOSE: &str = "/close";
const TASK_PAGE: &str = "https://app.todoist.com/app/task/";
const TASK_APP_LINK: &str = "todoist://task?id=";
const BROWSER_TITLE: &str = "Open in Browser";
const POST: &str = "POST";
const FILTER_PREFIX: &str = "/";
const SEARCH_FILTER: &str = "search: ";
const ADD: &str = "add";
const COMPLETE: &str = "complete";
const TOMORROW: &str = "tomorrow";
const NEXT_WEEK: &str = "next-week";
const ADDED_STATE: &str = "added";
const COMPLETED_STATE: &str = "completed:";
const MOVED_STATE: &str = "moved:";
const SEPARATOR: &str = " · ";
const LABEL_PREFIX: &str = "@";
const PRIORITY_PREFIX: &str = "p";
const URGENT: i64 = 4;
const NOTHING: &str = "Todoist returned nothing.";
const UNREADABLE: &str = "Todoist sent a reply Volant could not read.";
const REFUSED: &str = "Todoist refused the API token. Update it in Settings, Extensions.";
const UNKNOWN_ACTION: &str = "This extension does not know that action.";
const NO_TASK: &str = "Volant did not say which task that was.";
const EMPTY_ADD: &str = "Type the task to add after todo.";
const ADD_HINT: &str = "Return adds it to Todoist; dates, #project, @label and p1 to p4 are understood";

/// Todoist views by the words that pick them: the filter each runs, and what an empty one says.
const VIEWS: &[(&[&str], &str, &str)] = &[
    (&["", "today"], "today | overdue", "Nothing due today. Type todo and a task to add one."),
    (&["upcoming", "week"], "7 days | overdue", "Nothing due in the next seven days."),
    (&["inbox"], "#Inbox", "Your Inbox is empty."),
    (&["p1", "urgent"], "p1", "No priority 1 tasks."),
];

/// The actions every task row offers, by id and title, and the due string each one sets.
const TASK_ACTIONS: &[(&str, &str, Option<&str>)] = &[
    (COMPLETE, "Complete Task", None),
    (TOMORROW, "Due Tomorrow", Some("tomorrow")),
    (NEXT_WEEK, "Due Next Week", Some("next monday")),
];

#[derive(Deserialize)]
struct Page {
    #[serde(default)]
    results: Vec<Task>,
}

#[derive(Deserialize)]
struct Projects {
    #[serde(default)]
    results: Vec<Project>,
}

#[derive(Deserialize)]
struct Project {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct Task {
    id: String,
    content: String,
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    priority: i64,
    due: Option<Due>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    checked: bool,
    #[serde(default)]
    is_deleted: bool,
}

#[derive(Deserialize)]
struct Due {
    date: Option<String>,
    string: Option<String>,
}

/// What the owner typed, as Todoist sees it: a view's filter, a filter they wrote, or text to add.
#[derive(Debug, PartialEq)]
pub enum Query {
    View(&'static str, &'static str),
    Filter(String),
    Text(String),
}

/// Reads the text after the keyword.
pub fn parse(query: &str) -> Query {
    let text = query.trim();
    if let Some(filter) = text.strip_prefix(FILTER_PREFIX) {
        return Query::Filter(filter.trim().to_string());
    }
    let lowered = text.to_lowercase();
    match VIEWS.iter().find(|(words, ..)| words.contains(&lowered.as_str())) {
        Some((_, filter, empty)) => Query::View(filter, empty),
        None => Query::Text(text.to_string()),
    }
}

/// The task list request for a Todoist filter.
pub fn filter_url(filter: &str) -> String {
    format!("{API}{FILTER}{}", encode_query(filter))
}

/// The project list, fetched with every task list so rows can name their project.
pub fn projects_url() -> String {
    format!("{API}{PROJECTS}")
}

fn quick_add_url() -> String {
    format!("{API}{QUICK_ADD}")
}

fn close_url(id: &str) -> String {
    format!("{API}{TASKS}{id}{CLOSE}")
}

fn update_url(id: &str) -> String {
    format!("{API}{TASKS}{id}")
}

/// The task ID in a row's link, if it is one of this extension's task links.
pub fn task_id(url: &str) -> Option<&str> {
    url.strip_prefix(TASK_PAGE).filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn add_title(text: &str) -> String {
    format!("Add “{text}”")
}

fn unavailable(status: i64) -> String {
    format!("Todoist is unavailable (HTTP {status}). Try again.")
}

fn no_matches(text: &str) -> String {
    format!("No open tasks match {text}.")
}

fn added(text: &str, due: Option<&str>) -> String {
    match due {
        Some(due) => format!("Added “{text}”, due {due}"),
        None => format!("Added “{text}”"),
    }
}

fn completed(title: &str) -> String {
    format!("Completed “{title}”")
}

fn moved(title: &str) -> String {
    format!("Rescheduled “{title}”")
}

/// "p1" for Todoist's urgent priority 4, down to "p3" for 2; normal priority shows nothing.
fn priority_label(priority: i64) -> Option<String> {
    (2..=URGENT).contains(&priority).then(|| format!("{PRIORITY_PREFIX}{}", URGENT + 1 - priority))
}

/// One row for an open task, with its priority, due date, project and labels.
fn task_row(task: &Task, projects: &HashMap<String, String>) -> Item {
    let mut details = Vec::new();
    if let Some(priority) = priority_label(task.priority) { details.push(priority); }
    if let Some(due) = task.due.as_ref().and_then(|due| due.string.clone()).filter(|s| !s.is_empty()) { details.push(due); }
    if let Some(project) = projects.get(&task.project_id) { details.push(project.clone()); }
    details.extend(task.labels.iter().map(|label| format!("{LABEL_PREFIX}{label}")));
    Item {
        title: task.content.clone(),
        subtitle: Some(details.join(SEPARATOR)).filter(|s| !s.is_empty()),
        url: Some(format!("{TASK_APP_LINK}{}", task.id)),
        secondary_url: Some(format!("{TASK_PAGE}{}", task.id)),
        secondary_title: Some(BROWSER_TITLE.to_string()),
        actions: TASK_ACTIONS.iter().map(|(id, title, _)| Action { id: id.to_string(), title: title.to_string() }).collect(),
        ..Default::default()
    }
}

/// Open tasks soonest first, then most urgent; tasks without a due date go last.
fn sorted(mut tasks: Vec<Task>) -> Vec<Task> {
    tasks.retain(|task| !task.checked && !task.is_deleted && task_id(&format!("{TASK_PAGE}{}", task.id)).is_some());
    tasks.sort_by(|a, b| {
        let due = |task: &Task| task.due.as_ref().and_then(|due| due.date.clone());
        match (due(a), due(b)) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
        .then(b.priority.cmp(&a.priority))
    });
    tasks
}

fn plan(request: &Request) -> Reply {
    let filter = match parse(&request.query) {
        Query::View(filter, _) => filter.to_string(),
        Query::Filter(filter) => filter,
        Query::Text(text) => format!("{SEARCH_FILTER}{text}"),
    };
    Reply::Fetch(vec![filter_url(&filter), projects_url()])
}

/// Project names by ID, or none if the project list failed or could not be read; rows then simply
/// leave the project out.
fn project_names(request: &Request) -> HashMap<String, String> {
    let projects = projects_url();
    request.responses.iter()
        .find(|response| response.url == projects && (200..=299).contains(&response.status))
        .and_then(|response| serde_json::from_str::<Projects>(&response.body).ok())
        .map(|list| list.results.into_iter().map(|project| (project.id, project.name)).collect())
        .unwrap_or_default()
}

/// Turns a response's status into the error the owner should see, if it failed.
fn check(status: i64) -> Result<(), String> {
    match status {
        200..=299 => Ok(()),
        401 | 403 => Err(REFUSED.to_string()),
        status => Err(unavailable(status)),
    }
}

fn render(request: &Request) -> Result<Reply, String> {
    if let Some(state) = request.state.as_deref() {
        let response = request.responses.first().ok_or(NOTHING)?;
        check(response.status)?;
        return confirmation(state, &response.body, &request.query);
    }
    let projects = projects_url();
    let response = request.responses.iter().find(|response| response.url != projects).ok_or(NOTHING)?;
    check(response.status)?;
    let page: Page = serde_json::from_str(&response.body).map_err(|_| UNREADABLE.to_string())?;
    let names = project_names(request);
    let mut items: Vec<Item> = sorted(page.results).iter().map(|task| task_row(task, &names)).collect();
    match parse(&request.query) {
        Query::Text(text) if !text.is_empty() => {
            items.insert(0, Item { title: add_title(&text), subtitle: Some(ADD_HINT.to_string()), primary_action: Some(ADD.to_string()), ..Default::default() });
            if items.len() == 1 { items.push(Item { title: no_matches(&text), ..Default::default() }); }
        }
        Query::View(_, empty) if items.is_empty() => items.push(Item { title: empty.to_string(), ..Default::default() }),
        _ => {}
    }
    Ok(Reply::Items(items))
}

/// The confirmation after a change went through.
fn confirmation(state: &str, body: &str, query: &str) -> Result<Reply, String> {
    if state == ADDED_STATE {
        let task: Option<Task> = serde_json::from_str(body).ok();
        let text = task.as_ref().map_or(query.trim(), |task| task.content.as_str());
        let due = task.as_ref().and_then(|task| task.due.as_ref()).and_then(|due| due.string.as_deref());
        return Ok(Reply::Notice(added(text, due)));
    }
    if let Some(title) = state.strip_prefix(COMPLETED_STATE) { return Ok(Reply::Notice(completed(title))); }
    if let Some(title) = state.strip_prefix(MOVED_STATE) { return Ok(Reply::Notice(moved(title))); }
    Err(UNREADABLE.to_string())
}

/// Sends the change the owner chose: adding the typed task, or completing or rescheduling a row's.
fn act(request: &Request) -> Result<Reply, String> {
    let action = request.action.as_deref().unwrap_or_default();
    if action == ADD {
        let text = request.query.trim();
        if text.is_empty() { return Err(EMPTY_ADD.to_string()); }
        let body = serde_json::json!({ "text": text });
        return Ok(Reply::Submit(vec![Submission { url: quick_add_url(), method: POST.to_string(), json: Some(body) }], ADDED_STATE.to_string()));
    }
    let (_, _, due) = TASK_ACTIONS.iter().find(|(id, ..)| *id == action).ok_or(UNKNOWN_ACTION)?;
    let row = request.item.as_ref().ok_or(NO_TASK)?;
    let id = row.secondary_url.as_deref().and_then(task_id).ok_or(NO_TASK)?;
    Ok(match due {
        None => Reply::Submit(vec![Submission { url: close_url(id), method: POST.to_string(), json: None }], format!("{COMPLETED_STATE}{}", row.title)),
        Some(due) => {
            let body = serde_json::json!({ "due_string": due });
            Reply::Submit(vec![Submission { url: update_url(id), method: POST.to_string(), json: Some(body) }], format!("{MOVED_STATE}{}", row.title))
        }
    })
}

fn handle(request: &Request) -> Result<Reply, String> {
    if request.is_plan() { Ok(plan(request)) } else if request.is_action() { act(request) } else { render(request) }
}

volant_extension!(handle);

#[cfg(test)]
mod tests {
    use super::*;
    use volant_extension_kit::{handle as run, manifest_permits};

    const MANIFEST: &str = include_str!("../manifest.json");
    const TASKS_BODY: &str = r#"{"results":[
        {"id":"6Xb","content":"Renew passport","priority":1,"due":{"date":"2026-10-20","string":"Oct 20"},"labels":[],"checked":false,"is_deleted":false},
        {"id":"6Xa","content":"Ship 0.3.3","project_id":"P2","priority":4,"due":{"date":"2026-10-14T15:00:00","string":"today at 3pm"},"labels":["work"],"checked":false,"is_deleted":false},
        {"id":"6Xc","content":"Someday idea","priority":2,"due":null,"labels":["ideas","later"],"checked":false,"is_deleted":false},
        {"id":"6Xd","content":"Already done","priority":1,"due":null,"labels":[],"checked":true,"is_deleted":false},
        {"id":"../x","content":"Odd ID","priority":1,"due":null,"labels":[],"checked":false,"is_deleted":false}],"next_cursor":null}"#;

    /// Runs the handler on a request the way Volant would, and parses the reply.
    fn reply(request: serde_json::Value) -> serde_json::Value {
        serde_json::from_str(&run(request.to_string().as_bytes(), handle)).unwrap()
    }

    const PROJECTS_BODY: &str = r#"{"results":[{"id":"P1","name":"Inbox"},{"id":"P2","name":"Volant"}],"next_cursor":null}"#;

    fn render_with(query: &str, body: &str) -> serde_json::Value {
        reply(serde_json::json!({"phase":"render","query":query,"now":0,"responses":[
            {"url":filter_url("x"),"status":200,"body":body},{"url":projects_url(),"status":200,"body":PROJECTS_BODY}]}))
    }

    #[test]
    fn queries_pick_views_filters_or_text() {
        assert_eq!(parse(""), Query::View("today | overdue", VIEWS[0].2));
        assert_eq!(parse(" Upcoming "), Query::View("7 days | overdue", VIEWS[1].2));
        assert_eq!(parse("/#Work & today"), Query::Filter("#Work & today".to_string()));
        assert_eq!(parse("buy milk tomorrow p1"), Query::Text("buy milk tomorrow p1".to_string()));
    }

    #[test]
    fn every_url_is_on_a_manifest_host() {
        for url in [filter_url("today | overdue"), projects_url(), filter_url("search: a&b #c"), quick_add_url(), close_url("6Xa"), update_url("6Xa")] {
            assert!(manifest_permits(MANIFEST, &url), "{url}");
        }
    }

    #[test]
    fn plan_fetches_the_filter_for_the_query() {
        let today = reply(serde_json::json!({"phase":"plan","query":"","now":0}));
        assert_eq!(today["fetch"][0], "https://api.todoist.com/api/v1/tasks/filter?limit=30&query=today%20%7C%20overdue");
        assert_eq!(today["fetch"][1], "https://api.todoist.com/api/v1/projects?limit=200");
        assert_eq!(reply(serde_json::json!({"phase":"plan","query":"milk","now":0}))["fetch"][0],
            "https://api.todoist.com/api/v1/tasks/filter?limit=30&query=search%3A%20milk");
    }

    #[test]
    fn tasks_render_soonest_and_most_urgent_first() {
        let items = render_with("", TASKS_BODY)["items"].as_array().unwrap().clone();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["title"], "Ship 0.3.3");
        assert_eq!(items[0]["subtitle"], "p1 · today at 3pm · Volant · @work");
        assert_eq!(items[0]["url"], "todoist://task?id=6Xa");
        assert_eq!(items[0]["secondaryURL"], "https://app.todoist.com/app/task/6Xa");
        assert_eq!(items[0]["actions"][0]["id"], "complete");
        assert_eq!(items[1]["title"], "Renew passport");
        assert_eq!(items[1]["subtitle"], "Oct 20");
        assert_eq!(items[2]["subtitle"], "p3 · @ideas · @later");
    }

    #[test]
    fn a_failed_project_list_only_drops_project_names() {
        let request = serde_json::json!({"phase":"render","query":"","now":0,"responses":[
            {"url":filter_url("x"),"status":200,"body":TASKS_BODY},{"url":projects_url(),"status":500,"body":""}]});
        assert_eq!(reply(request)["items"][0]["subtitle"], "p1 · today at 3pm · @work");
    }

    #[test]
    fn text_offers_to_add_it_above_matching_tasks() {
        let items = render_with("passport", TASKS_BODY)["items"].as_array().unwrap().clone();
        assert_eq!(items[0]["title"], "Add “passport”");
        assert_eq!(items[0]["primaryAction"], "add");
        assert_eq!(items.len(), 4);
        let empty = render_with("zzz", r#"{"results":[]}"#)["items"].as_array().unwrap().clone();
        assert_eq!(empty[1]["title"], "No open tasks match zzz.");
        let view = render_with("inbox", r#"{"results":[]}"#);
        assert_eq!(view["items"][0]["title"], "Your Inbox is empty.");
    }

    #[test]
    fn actions_send_quick_add_close_and_reschedule() {
        let add = reply(serde_json::json!({"phase":"action","action":"add","query":"buy milk tomorrow p1","now":0,"item":{"title":"Add “buy milk tomorrow p1”"}}));
        assert_eq!(add["fetch"][0], serde_json::json!({"url":"https://api.todoist.com/api/v1/tasks/quick","method":"POST","json":{"text":"buy milk tomorrow p1"}}));
        assert_eq!(add["state"], "added");
        let row = serde_json::json!({"title":"Ship 0.3.3","url":"todoist://task?id=6Xa","secondaryURL":"https://app.todoist.com/app/task/6Xa"});
        let done = reply(serde_json::json!({"phase":"action","action":"complete","query":"","now":0,"item":row}));
        assert_eq!(done["fetch"][0], serde_json::json!({"url":"https://api.todoist.com/api/v1/tasks/6Xa/close","method":"POST"}));
        let later = reply(serde_json::json!({"phase":"action","action":"tomorrow","query":"","now":0,"item":row}));
        assert_eq!(later["fetch"][0]["json"], serde_json::json!({"due_string":"tomorrow"}));
        let foreign = reply(serde_json::json!({"phase":"action","action":"complete","query":"","now":0,"item":{"title":"x","secondaryURL":"https://example.com/app/task/1"}}));
        assert_eq!(foreign["items"][0]["title"], NO_TASK);
    }

    #[test]
    fn confirmations_and_failures_read_plainly() {
        let added = r#"{"id":"7Y","content":"buy milk","priority":4,"due":{"date":"2026-10-15","string":"tomorrow"},"labels":[]}"#;
        let notice = reply(serde_json::json!({"phase":"render","query":"buy milk tomorrow p1","now":0,"state":"added","responses":[{"url":"u","status":200,"body":added}]}));
        assert_eq!(notice["notice"], "Added “buy milk”, due tomorrow");
        let done = reply(serde_json::json!({"phase":"render","query":"","now":0,"state":"completed:Ship 0.3.3","responses":[{"url":"u","status":204,"body":""}]}));
        assert_eq!(done["notice"], "Completed “Ship 0.3.3”");
        let refused = reply(serde_json::json!({"phase":"render","query":"","now":0,"responses":[{"url":"u","status":401,"body":""}]}));
        assert_eq!(refused["items"][0]["title"], REFUSED);
    }
}
