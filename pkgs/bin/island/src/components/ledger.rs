//! A list of named entries divided by rules, each beside one or more aligned bars with an optional
//! mark, continuation, value and note; entries may fold to one summary line, and long lists scroll
//! under soft edges.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{components::badge::Mark, panel::Handler, spring::SpringConfig};

const MAX_ROWS_H: f32 = 320.0;

pub const TURN_SPRING: SpringConfig = SpringConfig::new(420.0, 41.0, 1.0);

pub struct Bar {
    pub label: SharedString,
    pub label_color: u32,
    pub fraction: f32,
    pub ahead: Option<f32>,
    pub mark: Option<f32>,
    pub color: u32,
    pub value: SharedString,
    pub value_color: u32,
    pub note: SharedString,
    pub note_color: u32,
    pub appear: bool,
}

pub struct Folded {
    pub label: SharedString,
    pub value: SharedString,
    pub note: SharedString,
}

pub struct Entry {
    pub key: SharedString,
    pub mark: Mark,
    pub tint: Option<u32>,
    /// Left-edge highlight pill; `RED` also softly tints the entry's background.
    pub accent: Option<u32>,
    pub faded: bool,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub chip: Option<(SharedString, u32)>,
    pub bars: Vec<Bar>,
    pub folded: Option<Folded>,
    pub expanded: bool,
    pub on_toggle: Option<Handler>,
}

pub struct Columns {
    pub entry: SharedString,
    pub label: SharedString,
    pub bar: SharedString,
    pub mark_key: Option<SharedString>,
    pub ahead_key: Option<SharedString>,
    pub value: SharedString,
    pub note: SharedString,
}

pub struct Ledger {
    pub columns: Option<Columns>,
    pub chevron: Option<PathBuf>,
    pub entries: Vec<Entry>,
}

impl Entry {
    fn line_count(&self) -> usize {
        (self.bars.len() + usize::from(self.folded.is_some())).max(1)
    }

    fn estimated_height(&self) -> f32 {
        16.0 + self.line_count() as f32 * 22.0
    }
}

impl Ledger {
    pub fn estimated_rows_height(&self) -> f32 {
        self.entries
            .iter()
            .map(Entry::estimated_height)
            .sum::<f32>()
    }

    pub fn overflows(&self) -> bool {
        self.estimated_rows_height() > MAX_ROWS_H
    }
}
