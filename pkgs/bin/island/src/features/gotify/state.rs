use std::collections::HashMap;

use serde::Deserialize;

const CRITICAL_PRIORITY: i64 = 8;

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
        Self {
            id: raw.id,
            app_id: raw.appid,
            app,
            title,
            body: raw.message,
            priority: raw.priority,
            timestamp,
        }
    }

    pub fn critical(&self) -> bool {
        self.priority >= CRITICAL_PRIORITY
    }
}

pub fn urgency(priority: i64) -> u8 {
    match priority {
        ..=3 => 0,
        4..=7 => 1,
        _ => 2,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub server: Option<String>,
    pub connected: bool,
    pub busy: bool,
    pub error: Option<String>,
    pub messages: Vec<Entry>,
}

impl State {
    pub fn configured(&self) -> bool {
        self.server.is_some()
    }

    pub fn has_critical(&self) -> bool {
        self.messages.iter().any(Entry::critical)
    }

    pub fn summary(&self) -> String {
        if !self.configured() {
            return "Gotify · not configured".to_owned();
        }
        if let Some(error) = &self.error {
            return format!("Gotify · {error}");
        }
        match self.messages.len() {
            0 => "Gotify · no messages".to_owned(),
            1 => "Gotify · 1 message".to_owned(),
            count => format!("Gotify · {count} messages"),
        }
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
