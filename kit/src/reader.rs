//! What a news reader remembers on this Mac through Volant's extension storage: pinned stories,
//! hidden stories, hidden domains and stories already opened. Stories are identified by a short key,
//! such as a Hacker News item ID or a Lobsters short ID, taken from the discussion link.
//!
//! Every list is capped so the saved object stays well under Volant's 16 KB limit; the oldest entries
//! go first. Rows get Pin or Unpin, Hide Story and Hide Stories from a domain, plus Show Hidden Stories
//! once anything is hidden, and ask Volant to say when they are opened so they can show as read.
use serde::{Deserialize, Serialize};

use crate::{Action, Item, Reply, Request};

const MAX_PINNED: usize = 30;
const MAX_HIDDEN: usize = 200;
const MAX_DOMAINS: usize = 30;
const MAX_READ: usize = 300;
const PIN: &str = "pin";
const UNPIN: &str = "unpin";
const HIDE: &str = "hide";
const HIDE_DOMAIN: &str = "hide-domain";
const SHOW_HIDDEN: &str = "show-hidden";
const OPENED: &str = "opened";
const PINNED_LABEL: &str = "Pinned";
const READ_LABEL: &str = "Read";
const SEPARATOR: &str = " · ";
const UNKNOWN_ACTION: &str = "This extension does not know that action.";

#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
#[serde(default)]
pub struct State {
    pub pinned: Vec<String>,
    pub hidden: Vec<String>,
    #[serde(rename = "hiddenDomains")]
    pub hidden_domains: Vec<String>,
    pub read: Vec<String>,
}

/// A story row before the reader arranges it: its key, the domain it links to (none for the site's
/// own text posts), and the row itself.
pub struct Story {
    pub key: String,
    pub domain: Option<String>,
    pub item: Item,
}

impl State {
    /// The saved state, or an empty one when nothing was saved or it is unreadable.
    pub fn load(request: &Request) -> State {
        request.storage.clone().and_then(|value| serde_json::from_value(value).ok()).unwrap_or_default()
    }

    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }

    /// Drops hidden stories and domains, puts pinned stories first in the order they were pinned,
    /// labels pinned and read rows, and gives every row its actions.
    pub fn arrange(&self, stories: Vec<Story>) -> Vec<Item> {
        let visible = stories.into_iter().filter(|story| {
            !self.hidden.contains(&story.key) && !story.domain.as_ref().is_some_and(|domain| self.hidden_domains.contains(domain))
        });
        let (mut pinned, rest): (Vec<Story>, Vec<Story>) = visible.partition(|story| self.pinned.contains(&story.key));
        pinned.sort_by_key(|story| self.pinned.iter().position(|key| key == &story.key));
        pinned.into_iter().chain(rest).map(|story| self.decorate(story)).collect()
    }

    fn decorate(&self, story: Story) -> Item {
        let Story { key, domain, mut item } = story;
        let pinned = self.pinned.contains(&key);
        let mut labels = Vec::new();
        if pinned { labels.push(PINNED_LABEL); }
        if self.read.contains(&key) { labels.push(READ_LABEL); }
        if !labels.is_empty() {
            let prefix = labels.join(SEPARATOR);
            item.subtitle = Some(match item.subtitle { Some(subtitle) => format!("{prefix}{SEPARATOR}{subtitle}"), None => prefix });
        }
        item.actions = vec![
            if pinned { action(UNPIN, "Unpin Story") } else { action(PIN, "Pin Story") },
            action(HIDE, "Hide Story"),
        ];
        if let Some(domain) = domain { item.actions.push(action(HIDE_DOMAIN, &format!("Hide Stories from {domain}"))); }
        if !self.hidden.is_empty() || !self.hidden_domains.is_empty() { item.actions.push(action(SHOW_HIDDEN, "Show Hidden Stories")); }
        item.on_open = true;
        item
    }

    /// Applies an action or an opened notice to the story with `key`; `domain` is where it links.
    pub fn apply(&mut self, action: &str, key: &str, domain: Option<&str>) -> Result<(), String> {
        match action {
            PIN => remember(&mut self.pinned, key, MAX_PINNED),
            UNPIN => self.pinned.retain(|pinned| pinned != key),
            HIDE => { self.pinned.retain(|pinned| pinned != key); remember(&mut self.hidden, key, MAX_HIDDEN) }
            HIDE_DOMAIN => if let Some(domain) = domain { remember(&mut self.hidden_domains, domain, MAX_DOMAINS) },
            SHOW_HIDDEN => { self.hidden.clear(); self.hidden_domains.clear() }
            OPENED => remember(&mut self.read, key, MAX_READ),
            _ => return Err(UNKNOWN_ACTION.to_string()),
        }
        Ok(())
    }
}

/// Adds `value` once, moving it to the end if present, and keeps at most `limit` entries.
fn remember(list: &mut Vec<String>, value: &str, limit: usize) {
    list.retain(|existing| existing != value);
    list.push(value.to_string());
    if list.len() > limit { list.drain(..list.len() - limit); }
}

fn action(id: &str, title: &str) -> Action {
    Action { id: id.to_string(), title: title.to_string() }
}

/// Handles the `action` and `opened` phases for a reader: finds the row's key with `key_of` from its
/// discussion link and its domain with `domain_of` from its link, applies the change and saves.
/// Returns `None` for other phases so the reader handles plan and render itself.
pub fn handle_change(request: &Request, key_of: fn(&str) -> Option<String>, domain_of: fn(&str) -> Option<String>) -> Option<Result<Reply, String>> {
    let action = match request.phase.as_str() {
        "action" => request.action.clone()?,
        OPENED => OPENED.to_string(),
        _ => return None,
    };
    let row = request.item.as_ref()?;
    let Some(key) = row.secondary_url.as_deref().and_then(key_of) else { return Some(Err(UNKNOWN_ACTION.to_string())) };
    let domain = row.url.as_deref().and_then(domain_of);
    let mut state = State::load(request);
    Some(state.apply(&action, &key, domain.as_deref()).map(|_| Reply::Save(state.to_value())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn story(key: &str, domain: Option<&str>) -> Story {
        Story { key: key.into(), domain: domain.map(Into::into), item: Item { title: key.into(), subtitle: Some("details".into()), ..Default::default() } }
    }

    #[test]
    fn pins_lead_hidden_disappear_and_read_is_labelled() {
        let state = State { pinned: vec!["c".into(), "a".into()], hidden: vec!["b".into()], hidden_domains: vec!["spam.example".into()],
                             read: vec!["a".into(), "d".into()] };
        let items = state.arrange(vec![story("a", None), story("b", None), story("c", Some("x.example")), story("d", None), story("e", Some("spam.example"))]);
        let titles: Vec<&str> = items.iter().map(|item| item.title.as_str()).collect();
        assert_eq!(titles, ["c", "a", "d"]);
        assert_eq!(items[1].subtitle.as_deref(), Some("Pinned · Read · details"));
        assert_eq!(items[2].subtitle.as_deref(), Some("Read · details"));
        assert_eq!(items[0].actions[0].id, "unpin");
        assert_eq!(items[0].actions.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["unpin", "hide", "hide-domain", "show-hidden"]);
        assert_eq!(items[2].actions.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["pin", "hide", "show-hidden"]);
        assert!(items.iter().all(|item| item.on_open));
    }

    #[test]
    fn actions_change_state_and_lists_stay_bounded() {
        let mut state = State::default();
        state.apply("pin", "a", None).unwrap();
        state.apply("pin", "b", None).unwrap();
        state.apply("pin", "a", None).unwrap();
        assert_eq!(state.pinned, ["b", "a"]);
        state.apply("hide", "a", None).unwrap();
        assert_eq!(state.pinned, ["b"]);
        assert_eq!(state.hidden, ["a"]);
        state.apply("hide-domain", "b", Some("x.example")).unwrap();
        state.apply("show-hidden", "b", None).unwrap();
        assert!(state.hidden.is_empty() && state.hidden_domains.is_empty());
        for index in 0..400 { state.apply("opened", &index.to_string(), None).unwrap(); }
        assert_eq!(state.read.len(), MAX_READ);
        assert_eq!(state.read.first().map(String::as_str), Some("100"));
        assert!(state.apply("delete-everything", "a", None).is_err());
    }

    #[test]
    fn a_full_state_stays_far_under_the_storage_limit() {
        let mut state = State::default();
        for index in 0..1_000 {
            let key = format!("{:08}", 40_000_000 + index);
            state.apply("pin", &key, None).unwrap();
            state.apply("hide", &format!("h{key}"), None).unwrap();
            state.apply("opened", &format!("r{key}"), None).unwrap();
            state.apply("hide-domain", &key, Some(&format!("a-rather-long-domain-name-{index}.example.com"))).unwrap();
        }
        let bytes = serde_json::to_string(&state.to_value()).unwrap().len();
        assert!(bytes < 12 * 1024, "{bytes} bytes");
    }

    #[test]
    fn unreadable_saved_state_starts_empty() {
        let request: Request = serde_json::from_str(r#"{"phase":"render","query":"","now":0,"storage":{"pinned":"not a list"}}"#).unwrap();
        assert_eq!(State::load(&request), State::default());
    }
}
