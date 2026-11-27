use super::{
    picker::Picker,
    runtime::{Clipboard, Request},
    state::State,
};
use crate::{
    components::list::{self, ROWS},
    control::{Command, Group, Report, Row, Takes, Value, form},
    features::Outcome,
    key::Key,
};

impl Clipboard {
    pub(super) fn handle_key(
        &self,
        state: &State,
        picker: &mut Picker,
        keystroke: &Key,
    ) -> Outcome {
        let matching = state.matching(&picker.selection.query, picker.filter);
        let layout = matching.layout;
        let selected = matching.entries.get(
            picker
                .selection
                .selected
                .min(layout.count.saturating_sub(1)),
        );
        match keystroke.name.as_str() {
            "escape" => return Outcome::Close,
            "enter" => {
                if let Some(entry) = selected {
                    self.request(Request::Select {
                        id: entry.id,
                        paste: !keystroke.shift,
                    });
                }
                return Outcome::Close;
            }
            "up" => picker.step(-1, layout),
            "down" | "tab" => picker.step(1, layout),
            "pageup" => picker.step(-(ROWS as i32), layout),
            "pagedown" => picker.step(ROWS as i32, layout),
            "delete" => {
                if let Some(entry) = selected {
                    self.request(Request::Delete(entry.id));
                }
            }
            "backspace" => picker.search(|query| {
                query.pop();
            }),
            "u" if keystroke.control => picker.search(String::clear),
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
        Report::did(Row::said(
            "Forgot the clipboard history, keeping what is pinned",
        ))
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
            "Forget the clipboard history, keeping what is pinned",
        )],
        bare: None,
    },
    examples: &[],
}];
