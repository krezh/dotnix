//! A word or two in a tinted pill, naming what something is or saying how it stands rather than
//! doing anything, such as "Package" or "Update failed".

use std::path::PathBuf;

use smol_str::SmolStr as SharedString;

pub struct Chip {
    pub glyph: Option<PathBuf>,
    pub label: SharedString,
    /// Its words and glyph, and its pill washed in it.
    pub color: u32,
    /// The moving border, while what it reports is under way.
    pub busy: bool,
    /// Across its whole line at a control's height, its words centred, such as how a whole
    /// activation stands.
    pub wide: bool,
}

impl Chip {
    pub fn new(label: impl Into<SharedString>, color: u32) -> Self {
        Self {
            glyph: None,
            label: label.into(),
            color,
            busy: false,
            wide: false,
        }
    }
}
