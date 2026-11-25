//! A small card over the panel asking for one thing, such as a network's password or whether a
//! code matches the one a device shows. The panel stays in view under it, dimmed, and takes no
//! clicks until the card is answered or put away.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{components::input::InputView, panel::Handler};

/// A switch between dots and the text itself.
pub struct Reveal {
    pub glyph: Option<PathBuf>,
    pub shown: bool,
    pub on_click: Handler,
}

/// Something answering the card changes, as a bar from where it stands to where it goes.
pub struct Change {
    pub label: SharedString,
    pub fraction: f32,
    pub color: u32,
    pub from: SharedString,
    pub to: SharedString,
    pub to_color: u32,
}

pub struct PromptView {
    pub title: SharedString,
    pub detail: SharedString,
    pub changes: Vec<Change>,
    /// The answer is under way: its button shows the moving border.
    pub busy: bool,
    /// Its send button answers the card.
    pub field: Option<InputView>,
    pub reveal: Option<Reveal>,
    /// Shown large, such as a pairing code to compare or type.
    pub code: Option<SharedString>,
    /// A button answering the card, beside the way back, such as Confirm.
    pub confirm: Option<(SharedString, Handler)>,
    pub error: Option<SharedString>,
    pub on_cancel: Handler,
}
