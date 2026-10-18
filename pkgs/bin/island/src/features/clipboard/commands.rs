use gpui::Keystroke;

use super::{
    runtime::{Clipboard, Request},
    state::State,
};
use crate::{
    components::list::{self, ROWS, Selection},
    features::Outcome,
};

impl Clipboard {
    pub(super) fn handle_key(
        &self,
        state: &State,
        picker: &mut Selection,
        keystroke: &Keystroke,
    ) -> Outcome {
        let matching = state.matching(&picker.query);
        let count = matching.len();
        let selected = matching.get(picker.selected.min(count.saturating_sub(1)));
        match keystroke.key.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                if let Some(entry) = selected {
                    self.request(Request::Select {
                        id: entry.id,
                        paste: !keystroke.modifiers.shift,
                    });
                }
                return Outcome::Close;
            }
            "up" => picker.step(-1, count),
            "down" | "tab" => picker.step(1, count),
            "pageup" => picker.step(-(ROWS as i32), count),
            "pagedown" => picker.step(ROWS as i32, count),
            "delete" => {
                if let Some(entry) = selected {
                    self.request(Request::Delete(entry.id));
                }
            }
            "backspace" => picker.search(|query| {
                query.pop();
            }),
            "u" if keystroke.modifiers.control => picker.search(String::clear),
            _ => {
                if let Some(text) = list::typed(keystroke) {
                    picker.search(|query| query.push_str(text));
                }
            }
        }
        Outcome::Open
    }

    pub(super) fn handle_command(&self, args: &[&str]) {
        if args == ["clipboard", "clear"] {
            self.request(Request::Clear);
        }
    }
}

pub(super) fn check(args: &[&str]) -> Result<(), String> {
    match args {
        ["clipboard", "clear"] => Ok(()),
        _ => Err("expected clipboard clear".to_owned()),
    }
}
