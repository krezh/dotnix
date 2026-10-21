//! A text field that wraps onto more lines as you type, such as a message to send, with a send
//! button beside it.

use std::path::PathBuf;

use gpui::{
    AnyElement, Context, HighlightStyle, Keystroke, SharedString, StyledText, div, prelude::*, px,
    rgb, rgba,
};

use crate::{
    components::{button, energy, list},
    island::Island,
    panel::Handler,
    theme::{ACCENT, CONTROL_ALPHA, OVERLAY2, SURFACE1, with_alpha},
    ui::{self, Radius, SMALL_SIZE},
};

const LINE: f32 = 18.0;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 10.0;
const CARET: &str = "▏";

pub struct InputView {
    pub id: SharedString,
    pub text: SharedString,
    pub placeholder: SharedString,
    /// The most lines it grows to; past that the oldest scroll out of view.
    pub max_lines: usize,
    pub send_glyph: Option<PathBuf>,
    /// `None` while there is nothing to send, or sending has to wait.
    pub on_send: Option<Handler>,
    /// A beam runs round the field while what was sent is being answered.
    pub busy: bool,
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
pub fn edit(text: &mut String, keystroke: &Keystroke) -> Edit {
    let modifiers = &keystroke.modifiers;
    match keystroke.key.as_str() {
        "escape" => Edit::Cancel,
        "enter" if modifiers.shift => {
            text.push('\n');
            Edit::Changed
        }
        "enter" => Edit::Send,
        "backspace" if modifiers.control => {
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
        "u" if modifiers.control => {
            text.clear();
            Edit::Changed
        }
        _ => match list::typed(keystroke) {
            Some(typed) => {
                text.push_str(typed);
                Edit::Changed
            }
            None => Edit::Ignored,
        },
    }
}

pub fn input(view: InputView, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let empty = view.text.is_empty();
    let shown = if empty {
        SharedString::from(format!("{CARET}{}", view.placeholder))
    } else {
        SharedString::from(format!("{}{CARET}", view.text))
    };
    let caret = if empty {
        0..CARET.len()
    } else {
        view.text.len()..shown.len()
    };
    let mut highlights = vec![(
        caret,
        HighlightStyle {
            color: Some(rgb(ACCENT).into()),
            ..HighlightStyle::default()
        },
    )];
    if empty {
        highlights.push((
            CARET.len()..shown.len(),
            HighlightStyle {
                color: Some(rgb(OVERLAY2).into()),
                ..HighlightStyle::default()
            },
        ));
    }
    let field = div()
        .relative()
        .flex_1()
        .min_w(px(0.0))
        .child(ui::shape(
            Radius::Control,
            Some(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)).into()),
        ))
        .child(
            div()
                .relative()
                .px(px(PAD_X))
                .py(px(PAD_Y))
                .max_h(px(LINE * view.max_lines.max(1) as f32 + 2.0 * PAD_Y))
                .overflow_hidden()
                .flex()
                .flex_col()
                .justify_end()
                .text_size(px(SMALL_SIZE))
                .line_height(px(LINE))
                .child(StyledText::new(shown).with_highlights(highlights)),
        )
        .child(energy::border("input-busy", Radius::Control, 0.0, view.busy));
    let ready = view.on_send.is_some();
    let send = button::icon(
        SharedString::from(format!("{}-send", view.id)),
        view.send_glyph.as_deref(),
        None,
        ready,
        hover && ready,
    )
    .when(view.send_glyph.is_none(), |send| send.child("↑"))
    .when_some(view.on_send, |send, handler| {
        send.on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
    });
    div()
        .flex()
        .items_end()
        .gap(px(8.0))
        .child(field)
        .child(send)
        .into_any_element()
}

#[cfg(test)]
#[path = "input_test.rs"]
mod tests;
