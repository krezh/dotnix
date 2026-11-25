use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use super::class::{self, Class};
use crate::{
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
    pub(super) class: Class,
    pub(super) mime: String,
    /// The start of the text, or the files' paths one per line.
    pub(super) text: String,
    /// In the whole text.
    pub(super) chars: i64,
    pub(super) size: i64,
    pub(super) dimensions: Option<(u32, u32)>,
    /// Unix seconds it was last copied or picked.
    pub(super) used: i64,
    /// Kept through clearing and past the limits.
    pub(super) pinned: bool,
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

    fn lines(&self) -> usize {
        self.text.lines().count().max(1)
    }

    /// What it holds, without when it was copied.
    fn size_words(&self) -> String {
        let characters = plural(self.chars, "character", "characters");
        match self.class {
            Class::Text if self.lines() > 1 => format!(
                "{characters} · {}",
                plural(self.lines() as i64, "line", "lines")
            ),
            Class::Code if self.lines() > 1 => format!(
                "{} · {characters}",
                plural(self.lines() as i64, "line", "lines")
            ),
            Class::Text | Class::Code | Class::Link => characters,
            Class::Color => class::color(&self.text).map_or(characters, rgb_words),
            Class::Picture => {
                let bytes = units::bytes(u64::try_from(self.size).unwrap_or(0));
                match self.dimensions {
                    Some((width, height)) => format!("{width}×{height} · {bytes}"),
                    None => bytes,
                }
            }
            Class::Files => plural(self.text.lines().count() as i64, "file", "files"),
        }
    }

    /// Under its name in the list.
    pub(super) fn detail(&self) -> String {
        let what = match self.class {
            Class::Link => class::site(&self.text).unwrap_or_default().to_owned(),
            _ => self.size_words(),
        };
        format!("{what} · {}", ago(self.used))
    }

    /// Over its preview.
    pub(super) fn about(&self) -> String {
        let what = match self.class {
            Class::Picture => format!("{} · {}", self.mime, self.size_words()),
            _ => self.size_words(),
        };
        let pinned = if self.pinned { " · pinned" } else { "" };
        format!("{what}{pinned} · {}", ago(self.used))
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

fn rgb_words(rgb: u32) -> String {
    let [_, r, g, b] = rgb.to_be_bytes();
    format!("rgb({r}, {g}, {b})")
}

fn ago(at: i64) -> String {
    units::ago(crate::time::now().saturating_sub(at))
}

fn next_ago(at: i64) -> Duration {
    let at = crate::time::at(at);
    units::next_change(
        units::Label::Ago,
        SystemTime::now().duration_since(at).unwrap_or_default(),
    )
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
