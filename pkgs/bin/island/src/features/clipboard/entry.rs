use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    panel::Preview,
    units::{self, plural},
    uri,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Text,
    Image,
    Files,
}

impl Kind {
    pub(super) fn word(self) -> &'static str {
        match self {
            Kind::Text => "text",
            Kind::Image => "image",
            Kind::Files => "files",
        }
    }

    pub(super) fn parse(word: &str) -> Option<Self> {
        [Kind::Text, Kind::Image, Kind::Files]
            .into_iter()
            .find(|kind| kind.word() == word)
    }
}

/// Without its data.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Entry {
    pub(super) id: i64,
    pub(super) kind: Kind,
    pub(super) mime: String,
    /// The start of the text, or the files' paths one per line.
    pub(super) text: String,
    /// In the whole text.
    pub(super) chars: i64,
    pub(super) size: i64,
    pub(super) dimensions: Option<(u32, u32)>,
    /// Unix seconds it was last copied or picked.
    pub(super) used: i64,
    pub(super) preview: Option<PathBuf>,
}

impl Entry {
    pub(super) fn title(&self) -> String {
        match self.kind {
            Kind::Text => self.text.split_whitespace().collect::<Vec<_>>().join(" "),
            Kind::Image => format!(
                "Picture ({})",
                self.mime.trim_start_matches("image/").to_uppercase()
            ),
            Kind::Files => self
                .text
                .lines()
                .map(|path| path.rsplit('/').next().unwrap_or(path))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }

    pub(super) fn detail(&self) -> String {
        let what = match self.kind {
            Kind::Text => plural(self.chars, "character", "characters"),
            Kind::Image => match self.dimensions {
                Some((width, height)) => format!(
                    "{width}×{height} · {}",
                    units::bytes(u64::try_from(self.size).unwrap_or(0))
                ),
                None => units::bytes(u64::try_from(self.size).unwrap_or(0)),
            },
            Kind::Files => plural(self.text.lines().count() as i64, "file", "files"),
        };
        format!("{what} · {}", ago(self.used))
    }

    pub(super) fn preview(&self) -> Preview {
        match (&self.preview, self.kind) {
            (Some(picture), Kind::Image | Kind::Files) => Preview::Picture(picture.clone()),
            (_, Kind::Text | Kind::Files) => Preview::Text(self.text.clone().into()),
            (None, Kind::Image) => Preview::Empty,
        }
    }

    pub(super) fn about(&self) -> String {
        match self.kind {
            Kind::Text => format!(
                "{} · {} · {}",
                plural(self.chars, "character", "characters"),
                plural(self.text.lines().count().max(1) as i64, "line", "lines"),
                ago(self.used)
            ),
            Kind::Image => format!("{} · {}", self.mime, self.detail()),
            Kind::Files => self.detail(),
        }
    }

    pub(super) fn matches(&self, query: &str) -> bool {
        query.is_empty()
            || self.text.to_lowercase().contains(query)
            || self.title().to_lowercase().contains(query)
    }

    pub(super) fn next_change(&self) -> Duration {
        next_ago(self.used)
    }
}

fn ago(at: i64) -> String {
    units::ago(crate::time::now().saturating_sub(at))
}

fn next_ago(at: i64) -> Duration {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let now_secs = i64::try_from(now.as_secs()).unwrap_or(i64::MAX);
    let elapsed = u64::try_from(now_secs.saturating_sub(at)).unwrap_or(0);
    let step = match elapsed {
        ..3600 => 60,
        ..86_400 => 3600,
        _ => 86_400,
    };
    Duration::from_secs(step - (elapsed % step))
        .saturating_sub(Duration::from_nanos(u64::from(now.subsec_nanos())))
}

pub(super) fn files(uris: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(uris)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(uri::local_path)
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "clipboard_test.rs"]
mod tests;
