use gpui::Keystroke;

use super::{
    runtime::{Clipboard, Request},
    state::State,
};
use crate::{
    components::list::{self, ROWS, Selection},
    control::{Command, Group, Report, Row, Takes, Value, form},
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

    pub(super) fn handle_command(&self, args: &[&str]) -> Report {
        if args != ["clipboard", "clear"] {
            return Report::default();
        }
        self.request(Request::Clear);
        Report::did(Row::said("Forgot the clipboard history"))
    }
}

pub(super) const CONTROLS: &[Command] = &[Command {
    name: "clipboard",
    about: "Forget the clipboard history",
    group: Group::System,
    takes: Takes::Forms {
        forms: &[form(
            "clear",
            Value::Nothing,
            "Forget the clipboard history",
        )],
        bare: None,
    },
    examples: &[],
}];
