use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::panel::Preview;

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
                Some((width, height)) => format!("{width}×{height} · {}", bytes(self.size)),
                None => bytes(self.size),
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
}

fn plural(count: i64, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn bytes(size: i64) -> String {
    match size {
        size if size < 1024 => format!("{size} B"),
        size if size < 1024 * 1024 => format!("{:.0} kB", size as f64 / 1024.0),
        size => format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
    }
}

/// "just now", "4 min ago", "2 h ago", "3 d ago".
fn ago(at: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        });
    match now.saturating_sub(at) {
        seconds if seconds < 60 => "just now".to_owned(),
        seconds if seconds < 3600 => format!("{} min ago", seconds / 60),
        seconds if seconds < 86_400 => format!("{} h ago", seconds / 3600),
        seconds => format!("{} d ago", seconds / 86_400),
    }
}

pub(super) fn files(uris: &[u8]) -> Vec<PathBuf> {
    String::from_utf8_lossy(uris)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|uri| {
            uri.strip_prefix("file://")
                .map(|path| path.strip_prefix("localhost").unwrap_or(path))
        })
        .map(|path| path.and_then(unescape).map(PathBuf::from))
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn unescape(text: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.bytes();
    while let Some(byte) = rest.next() {
        if byte == b'%' {
            let high = (rest.next()? as char).to_digit(16)?;
            let low = (rest.next()? as char).to_digit(16)?;
            bytes.push((high * 16 + low) as u8);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(test)]
#[path = "clipboard_test.rs"]
mod tests;
