//! Where an island stands on its screen: in the bar, or taken off it by a feature, such as for
//! leaving the session.

use std::time::Duration;

/// How long an island takes to switch off; whatever comes after waits this long.
pub const SWITCH_OFF: Duration = Duration::from_millis(1200);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stance {
    Bar,
    /// Its pop-ups in the middle of the screen, taking the clicks around them, with a beam in
    /// `busy`'s color running round its edge while it is set.
    Center {
        busy: Option<u32>,
    },
    /// In the bar on a screen without the pop-ups, holding the whole screen so it can go black
    /// with them.
    Backdrop,
    /// Switched off like an old screen: in the middle it folds into a line, then a dot that flares
    /// and goes, and the screen goes black, taking `SWITCH_OFF`; in the bar it fades into the black.
    Off,
}

impl Stance {
    /// Has the island in the middle of the screen, or folding up there.
    pub fn centered(self) -> bool {
        matches!(self, Stance::Center { .. } | Stance::Off)
    }

    /// The strongest of two, so one feature taking the screen is enough.
    pub fn over(self, other: Stance) -> Stance {
        let rank = |stance: Stance| match stance {
            Stance::Bar => 0,
            Stance::Backdrop => 1,
            Stance::Center { .. } => 2,
            Stance::Off => 3,
        };
        if rank(other) > rank(self) {
            other
        } else {
            self
        }
    }
}
