//! What the running island answers an `island ctl` command with: whether it changed something,
//! and the state to show, which `--json` gives as it is.

use futures::channel::oneshot;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value as Json};

use super::parse::Control;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Shown {
    Switch(bool),
    /// 0–1, on the device named after it if any.
    Level {
        value: f32,
        muted: bool,
    },
    Count(u64),
    Text(String),
    /// Not known yet, such as a brightness island has not changed.
    Unknown,
    /// A row that is only its label, such as an action taken.
    Nothing,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Row {
    pub label: String,
    pub value: Shown,
    /// Such as the device a level is on.
    pub detail: String,
}

impl Row {
    pub fn new(label: impl Into<String>, value: Shown) -> Self {
        Self {
            label: label.into(),
            value,
            detail: String::new(),
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Only a label, such as what was just done.
    pub fn said(label: impl Into<String>) -> Self {
        Self::new(label, Shown::Nothing)
    }

    fn key(&self) -> String {
        self.label
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    }

    fn json(&self) -> Json {
        let detail = (!self.detail.is_empty()).then(|| Json::from(self.detail.clone()));
        match &self.value {
            Shown::Switch(on) => Json::from(*on),
            Shown::Level { value, muted } => {
                let mut map = Map::new();
                let value = (f64::from(*value) * 100.0).round() / 100.0;
                map.insert("value".to_owned(), Json::from(value));
                map.insert("muted".to_owned(), Json::from(*muted));
                if let Some(detail) = detail {
                    map.insert("device".to_owned(), detail);
                }
                Json::Object(map)
            }
            Shown::Count(count) => Json::from(*count),
            Shown::Text(text) => Json::from(text.clone()),
            Shown::Unknown => Json::Null,
            Shown::Nothing => Json::from(true),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// Shown above the rows, such as island's name and version for the overview.
    pub title: Option<String>,
    pub changed: bool,
    pub rows: Vec<Row>,
}

impl Report {
    /// The state as it is.
    pub fn shows(rows: Vec<Row>) -> Self {
        Self {
            title: None,
            changed: false,
            rows,
        }
    }

    /// What a change made the state, or what was done.
    pub fn did(row: Row) -> Self {
        Self {
            title: None,
            changed: true,
            rows: vec![row],
        }
    }

    pub fn json(&self) -> Json {
        Json::Object(
            self.rows
                .iter()
                .map(|row| (row.key(), row.json()))
                .collect(),
        )
    }
}

/// A [`Report`] now, or one that comes once slow work is done, so the UI does not wait for it.
pub enum Reply {
    Now(Report),
    Later(oneshot::Receiver<Result<Report, String>>),
}

impl From<Report> for Reply {
    fn from(report: Report) -> Self {
        Reply::Now(report)
    }
}

/// A command for the running island, with where its answer goes.
pub struct Asked {
    pub control: Control,
    pub answer: oneshot::Sender<Result<Report, String>>,
}

#[cfg(test)]
#[path = "report_test.rs"]
mod tests;
