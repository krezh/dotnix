//! The tabs over the history, each keeping only some of it.

use super::{class::Class, entry::Entry};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Filter {
    #[default]
    All,
    Pinned,
    Text,
    Links,
    Code,
    Colors,
    Images,
    Files,
}

pub(super) const FILTERS: [Filter; 8] = [
    Filter::All,
    Filter::Pinned,
    Filter::Text,
    Filter::Links,
    Filter::Code,
    Filter::Colors,
    Filter::Images,
    Filter::Files,
];

impl Filter {
    pub(super) fn label(self) -> &'static str {
        match self {
            Filter::All => "All",
            Filter::Pinned => "Pinned",
            Filter::Text => "Text",
            Filter::Links => "Links",
            Filter::Code => "Code",
            Filter::Colors => "Colors",
            Filter::Images => "Images",
            Filter::Files => "Files",
        }
    }

    pub(super) fn admits(self, entry: &Entry) -> bool {
        match self {
            Filter::All => true,
            Filter::Pinned => entry.pinned,
            Filter::Text => entry.class == Class::Text,
            Filter::Links => entry.class == Class::Link,
            Filter::Code => entry.class == Class::Code,
            Filter::Colors => entry.class == Class::Color,
            Filter::Images => entry.class == Class::Picture,
            Filter::Files => entry.class == Class::Files,
        }
    }

    /// Shown when it keeps nothing and nothing is typed; `None` for the whole history.
    pub(super) fn empty(self) -> Option<&'static str> {
        match self {
            Filter::All => None,
            Filter::Pinned => Some("Nothing pinned yet"),
            Filter::Text => Some("No text yet"),
            Filter::Links => Some("No links yet"),
            Filter::Code => Some("No code yet"),
            Filter::Colors => Some("No colors yet"),
            Filter::Images => Some("No images yet"),
            Filter::Files => Some("No files yet"),
        }
    }
}
