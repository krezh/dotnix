use gpui::Keystroke;

use super::{
    Ai,
    state::State,
    view::{Tab, View},
};
use crate::{features::Outcome, panel::Edit};

/// Typing goes to the chat while it shows; elsewhere only Escape does anything, closing the panel.
pub(super) fn key(ai: &Ai, state: &State, view: &mut View, keystroke: &Keystroke) -> Outcome {
    if view.tab != Tab::Chat {
        return if keystroke.key == "escape" {
            Outcome::Close
        } else {
            Outcome::Open
        };
    }
    match crate::components::input::edit(&mut view.draft, keystroke) {
        Edit::Cancel => Outcome::Close,
        Edit::Send => {
            if !state.chat.busy && !view.draft.trim().is_empty() {
                ai.chat_send(std::mem::take(&mut view.draft));
            }
            Outcome::Open
        }
        Edit::Changed | Edit::Ignored => Outcome::Open,
    }
}
