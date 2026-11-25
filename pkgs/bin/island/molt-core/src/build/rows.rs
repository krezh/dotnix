//! A row of the build screen's lists, whatever it shows, and the groups the rows fall into.

use crate::units::Hue;

/// How a row presents its state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowLook {
    /// Raised, two lines, energy beam: work happening now.
    Active,
    /// Lightly raised with a progress bar: a transfer.
    Transfer,
    /// Dim dot: waiting, or never touched.
    Idle,
    Done,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RowContent {
    pub group: usize,
    pub look: RowLook,
    pub title: String,
    /// Store-hash prefix telling apart rows whose titles would otherwise be identical.
    pub tag: String,
    /// Secondary text beside the title on flat rows of lists built with a source column.
    pub column: String,
    /// Monospace second line of active rows.
    pub detail: String,
    pub value: String,
    /// Transfer progress, drawn as a bar on transfer rows.
    pub fraction: f64,
}

impl RowContent {
    /// What filtering matches against.
    pub fn search_text(&self) -> String {
        format!("{}\n{}", self.title, self.column).to_lowercase()
    }
}

pub struct GroupSpec {
    pub name: &'static str,
    /// Colours the section label and row markers.
    pub hue: Hue,
    /// Rows entering this group go to its top rather than its bottom.
    pub newest_first: bool,
    /// The group's tab shows only while the group has rows.
    pub optional: bool,
}

pub const INPUT_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        name: "Failed",
        hue: Hue::Red,
        newest_first: false,
        optional: true,
    },
    GroupSpec {
        name: "Fetching",
        hue: Hue::Sapphire,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Fetched",
        hue: Hue::Green,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Cached",
        hue: Hue::Overlay,
        newest_first: false,
        optional: false,
    },
];

pub const WORK_GROUPS: &[GroupSpec] = &[
    GroupSpec {
        name: "Failed",
        hue: Hue::Red,
        newest_first: false,
        optional: true,
    },
    GroupSpec {
        name: "Building",
        hue: Hue::Yellow,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Downloading",
        hue: Hue::Sapphire,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Queued",
        hue: Hue::Overlay,
        newest_first: false,
        optional: false,
    },
    GroupSpec {
        name: "Done",
        hue: Hue::Green,
        newest_first: true,
        optional: false,
    },
];

/// Store-path or message text to a readable name: `/nix/store/<hash>-molt-0.1.0.drv` → `molt-0.1.0`.
pub fn activity_name(text: &str) -> String {
    let Some(store_name) = text
        .split("/nix/store/")
        .nth(1)
        .and_then(|value| value.split(['\'', ' ']).next())
    else {
        return text.to_owned();
    };
    store_name
        .trim_end_matches(".drv")
        .split_once('-')
        .map_or(store_name, |(_, name)| name)
        .to_owned()
}
