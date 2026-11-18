use super::{runtime::Switcher, state::Local};
use crate::{
    features::Outcome,
    key::{Key, Modifier},
};

impl Switcher {
    /// Tab and Shift+Tab step, § (the key above Tab) keeps to the focused app, Q closes the picked
    /// window, Esc leaves things as they were, and Enter focuses the pick.
    pub(super) fn handle_key(&self, local: &mut Local, key: &Key) -> Outcome {
        match key.name.as_str() {
            "escape" => return Outcome::Close,
            "enter" => return self.pick(local),
            "tab" if key.shift => local.step(-1),
            "tab" | "down" | "right" => local.step(1),
            "up" | "left" => local.step(-1),
            "§" | "½" | "`" | "~" => local.toggle_same(),
            "q" => {
                if let Some(address) = local.picked().map(|window| window.address.clone()) {
                    self.close(&address);
                    local.forget(&address);
                }
            }
            _ => {}
        }
        Outcome::Open
    }

    /// Opened with Alt held, letting go of it focuses the pick.
    pub(super) fn handle_release(&self, local: &Local, modifier: Modifier) -> Outcome {
        if modifier == Modifier::Alt && local.held {
            self.pick(local)
        } else {
            Outcome::Open
        }
    }

    fn pick(&self, local: &Local) -> Outcome {
        if let Some(window) = local.picked() {
            self.focus(&window.address);
        }
        Outcome::Close
    }
}
