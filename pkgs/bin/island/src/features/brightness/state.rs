use crate::{features::osd::Osd, theme::YELLOW};

/// Counts changes, so one that leaves the level where it was, such as up at full, still shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Level {
    pub(super) value: f32,
    pub(super) changes: u64,
}

pub(super) fn osd(before: &Level, after: &Level) -> Option<Osd> {
    (after.changes != before.changes).then(|| Osd {
        glyph: "brightness",
        color: YELLOW,
        caption: "",
        label: "Brightness".to_owned(),
        level: Some((after.value, false)),
    })
}
