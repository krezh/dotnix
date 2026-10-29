use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Deserialize;

use super::runtime::Gotify;
use crate::features::{
    Feature,
    notice::{Notice, NoticeAction, Origin},
};

const CRITICAL_PRIORITY: i64 = 8;
const RAISED_PRIORITY: i64 = 4;
/// What the notification center calls messages from here.
pub const SOURCE: &str = "Gotify";
/// Clicking it, as the notification center names it.
pub const DEFAULT_ACTION: &str = "default";
pub const OPEN_ACTION: &str = "open";
const OPEN: &str = "Open";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    pub tls: bool,
    pub authority: String,
    pub base_path: String,
}

impl Endpoint {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let trimmed = raw.trim();
        let (tls, rest) = if let Some(rest) = trimmed.strip_prefix("wss://") {
            (true, rest)
        } else if let Some(rest) = trimmed.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = trimmed.strip_prefix("ws://") {
            (false, rest)
        } else if let Some(rest) = trimmed.strip_prefix("http://") {
            (false, rest)
        } else {
            return Err(format!(
                "invalid Gotify URL '{trimmed}': expected https://, wss://, http:// or ws://"
            ));
        };
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, path.trim_matches('/')),
            None => (rest, ""),
        };
        if authority.is_empty()
            || authority.starts_with(':')
            || authority.contains(char::is_whitespace)
            || authority.contains('@')
        {
            return Err(format!("invalid Gotify host in '{trimmed}'"));
        }
        let path = path
            .strip_suffix("stream")
            .filter(|before| before.is_empty() || before.ends_with('/'))
            .unwrap_or(path)
            .trim_matches('/');
        let base_path = if path.is_empty() {
            String::new()
        } else {
            format!("/{path}")
        };
        Ok(Self {
            tls,
            authority: authority.to_owned(),
            base_path,
        })
    }

    pub fn display_url(&self) -> String {
        let scheme = if self.tls { "https" } else { "http" };
        format!("{scheme}://{}{}", self.authority, self.base_path)
    }

    pub fn stream_url(&self) -> String {
        format!("{}/stream", self.display_url())
    }

    pub fn stream_path(&self) -> String {
        format!("{}/stream", self.base_path)
    }

    pub fn messages_url(&self, limit: usize) -> String {
        format!("{}/message?limit={limit}", self.display_url())
    }

    pub fn delete_all_url(&self) -> String {
        format!("{}/message", self.display_url())
    }

    pub fn delete_url(&self, id: u64) -> String {
        format!("{}/message/{id}", self.display_url())
    }

    pub fn applications_url(&self) -> String {
        format!("{}/application", self.display_url())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RawMessage {
    pub id: u64,
    #[serde(default)]
    pub appid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub priority: i64,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub extras: Extras,
}

/// What of a message's extras island uses: <https://gotify.net/docs/msgextras>.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Extras {
    #[serde(rename = "client::notification", default)]
    pub notification: Option<ClientNotification>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ClientNotification {
    #[serde(default)]
    pub click: Option<Click>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Click {
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct PagedMessages {
    #[serde(default)]
    pub messages: Vec<RawMessage>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RawApp {
    pub id: u64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: u64,
    pub app_id: u64,
    pub app: String,
    pub title: String,
    pub body: String,
    pub priority: i64,
    pub timestamp: Option<i64>,
    /// Where clicking it goes.
    pub url: Option<String>,
}

impl Entry {
    pub fn from_raw(raw: RawMessage, apps: &HashMap<u64, String>) -> Self {
        let app = apps
            .get(&raw.appid)
            .cloned()
            .unwrap_or_else(|| "Gotify".to_owned());
        let title = if raw.title.trim().is_empty() {
            app.clone()
        } else {
            raw.title
        };
        let timestamp = chrono::DateTime::parse_from_rfc3339(&raw.date)
            .ok()
            .map(|dt| dt.timestamp());
        let url = raw
            .extras
            .notification
            .and_then(|notification| notification.click)
            .map(|click| click.url)
            .filter(|url| !url.trim().is_empty());
        Self {
            id: raw.id,
            app_id: raw.appid,
            app,
            title,
            body: raw.message,
            priority: raw.priority,
            timestamp,
            url,
        }
    }

    pub fn critical(&self) -> bool {
        self.priority >= CRITICAL_PRIORITY
    }

    /// Named in the overline when raised but not critical.
    fn detail(&self) -> Option<String> {
        (RAISED_PRIORITY..CRITICAL_PRIORITY)
            .contains(&self.priority)
            .then(|| format!("priority {}", self.priority))
    }

    pub fn key(&self) -> String {
        self.id.to_string()
    }

    /// `quiet` for one already on the server when island heard of it.
    pub fn notice(&self, quiet: bool) -> Notice {
        let received = self
            .timestamp
            .and_then(|at| u64::try_from(at).ok())
            .map_or_else(SystemTime::now, |at| {
                UNIX_EPOCH + Duration::from_secs(at)
            });
        Notice {
            origin: Origin {
                feature: Gotify::NAME.to_owned(),
                source: SOURCE.to_owned(),
                key: self.key(),
            },
            app: self.app.clone(),
            title: self.title.clone(),
            body: self.body.clone(),
            critical: self.critical(),
            detail: self.detail(),
            via: true,
            lead: false,
            actions: self
                .url
                .iter()
                .flat_map(|_| {
                    [DEFAULT_ACTION, OPEN_ACTION].map(|key| NoticeAction {
                        key: key.to_owned(),
                        label: OPEN.to_owned(),
                        opens: None,
                    })
                })
                .collect(),
            received,
            quiet,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub server: Option<String>,
    pub connected: bool,
    pub error: Option<String>,
    /// How many are on the server.
    pub messages: usize,
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
