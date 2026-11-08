//! A text field that wraps onto more lines as you type, such as a message to send, with a send
//! button inside it at its end.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{components::list, key::Key, panel::Handler};

pub const CARET: &str = "▏";

pub struct InputView {
    pub text: SharedString,
    pub placeholder: SharedString,
    /// The most lines it grows to; past that the oldest scroll out of view.
    pub max_lines: usize,
    pub send_glyph: Option<PathBuf>,
    /// `None` while there is nothing to send, or sending has to wait.
    pub on_send: Option<Handler>,
    /// A beam runs round the field while what was sent is being answered.
    pub busy: bool,
    /// Shows a dot for each character, such as for a password.
    pub secret: bool,
}

/// What a key did to the text.
#[derive(Debug, PartialEq, Eq)]
pub enum Edit {
    Changed,
    Send,
    Cancel,
    Ignored,
}

/// Enter sends and Shift+Enter starts a new line; Backspace removes a character, or a word with
/// Ctrl; Ctrl+U clears; Escape cancels.
pub fn edit(text: &mut String, key: &Key) -> Edit {
    match key.name.as_str() {
        "escape" => Edit::Cancel,
        "enter" if key.shift => {
            text.push('\n');
            Edit::Changed
        }
        "enter" => Edit::Send,
        "backspace" if key.control => {
            let kept = text.trim_end().len();
            text.truncate(kept);
            let start = text.rfind(char::is_whitespace).map_or(0, |at| at + 1);
            text.truncate(start);
            Edit::Changed
        }
        "backspace" => {
            text.pop();
            Edit::Changed
        }
        "u" if key.control => {
            text.clear();
            Edit::Changed
        }
        _ => match list::typed(key) {
            Some(typed) => {
                text.push_str(typed);
                Edit::Changed
            }
            None => Edit::Ignored,
        },
    }
}

#[cfg(test)]
#[path = "input_test.rs"]
mod tests;
