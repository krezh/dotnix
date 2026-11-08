//! A button that opens a short list of things to do with what it sits on, such as trusting or
//! forgetting a device. Opening grows the button into the list below it, its right edge kept, as
//! the island grows into a panel; closing folds it back.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::panel::Handler;

pub struct Action {
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    /// A switch at the row's end, on or off, for something that stays set, such as trust.
    pub switch: Option<bool>,
    /// Drawn in red, for something that cannot be taken back.
    pub danger: bool,
    pub on_click: Handler,
}

pub struct Actions {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    /// The button's tooltip.
    pub label: SharedString,
    pub actions: Vec<Action>,
    pub open: bool,
    /// Opens or closes it; also called to close it when a click lands elsewhere.
    pub on_toggle: Handler,
}
