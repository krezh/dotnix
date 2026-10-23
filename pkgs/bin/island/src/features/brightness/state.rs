use crate::{
    features::osd::{DURATION, Osd},
    theme::YELLOW,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    /// Such as `DP-1`.
    pub(super) name: String,
    /// Such as the monitor's model; may be empty.
    pub(super) model: String,
    /// `None` when it did not answer.
    pub(super) level: Option<f32>,
}

/// Counts changes, so one that leaves the level where it was, such as up at full, still shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Levels {
    pub(super) screens: Vec<Screen>,
    /// The screens the last change was made on.
    pub(super) changed: Vec<String>,
    pub(super) changes: u64,
}

/// Names the screen when there are several and one changed; with all of them, shows the first.
pub(super) fn osd(before: &Levels, after: &Levels) -> Option<Osd> {
    if after.changes == before.changes {
        return None;
    }
    let first = after.changed.first()?;
    let screen = after.screens.iter().find(|screen| &screen.name == first)?;
    let named = after.screens.len() > 1 && after.changed.len() == 1;
    Some(Osd {
        glyph: "brightness",
        color: YELLOW,
        caption: if named { "Brightness" } else { "" },
        label: if named {
            screen.name.clone()
        } else {
            "Brightness".to_owned()
        },
        level: Some((screen.level?, false)),
        lasts: DURATION,
    })
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
