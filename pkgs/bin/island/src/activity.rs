//! Work a feature carries on with after its panel closes, such as a build: what island shows of it
//! in a small island of its own beside the bar, or as a ring over a fullscreen window.

use std::path::PathBuf;

use crate::panel::{Block, Handler, SharedString};

/// Where the work stands, least pressing first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Standing {
    /// Ended as it should; shows a little while, then folds away.
    Done,
    Running,
    /// Ended waiting on the user, such as a build ready to switch to; stays until its panel opens.
    NeedsYou,
    /// Stays until its panel opens.
    Failed,
}

impl Standing {
    /// Ended and stays until its panel opens.
    pub fn waits(self) -> bool {
        matches!(self, Self::NeedsYou | Self::Failed)
    }
}

pub struct Activity {
    /// Names this piece of work and how it ended, so a new one is told from the same one moving on.
    pub key: SharedString,
    pub glyph: Option<PathBuf>,
    pub color: u32,
    pub standing: Standing,
    /// How far along, 0 to 1; `None` while it cannot tell, and the ring turns.
    pub progress: Option<f32>,
    /// What it is doing, short, such as `Building 33%`.
    pub title: SharedString,
    /// Fainter after the title, such as how long it has run.
    pub detail: Option<SharedString>,
    /// Its line when several are under way, such as `NixOS · 41 of 112 · 2m 48s`.
    pub summary: SharedString,
    /// At the end of its line when several are under way, in a color, such as `33%`.
    pub aside: Option<(SharedString, u32)>,
    /// What drops from it under the pointer: where it has got to and what can be done.
    pub card: Vec<Block>,
    /// Opens its panel on the view it belongs to.
    pub on_open: Handler,
}
