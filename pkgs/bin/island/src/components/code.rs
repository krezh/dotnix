//! Monospaced lines, plain or as a diff, such as a command about to run or the change to a file.

use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::{button, reveal},
    island::Island,
    panel::Handler,
    theme::{
        CARD_BORDER_ALPHA, CRUST, GREEN, MONO, OVERLAY1, RED, SUBTEXT1, SURFACE1, TEXT, with_alpha,
    },
    ui::{self, Radius},
};

const SIZE: f32 = 12.0;
const LINE: f32 = 19.0;
const PAD_X: f32 = 10.0;
const PAD_Y: f32 = 8.0;
const NUMBER_W: f32 = 34.0;
const HEADING_SIZE: f32 = 12.0;
const GROUND_ALPHA: u8 = 0x80;
const CHANGE_ALPHA: u8 = 0x1a;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Change {
    Same,
    Added,
    Removed,
}

pub struct CodeLine {
    pub number: Option<u32>,
    pub text: SharedString,
    pub change: Change,
}

impl CodeLine {
    pub fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            number: None,
            text: text.into(),
            change: Change::Same,
        }
    }
}

/// How the lines come in the first time they are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Motion {
    #[default]
    Still,
    /// They rise in, such as what a step returned just now; never for what was there already.
    Appear,
}

pub struct Code {
    /// Names its animations, so they play once for what it shows wherever it moves; empty to go
    /// by its place in the panel.
    pub key: SharedString,
    /// A name over the lines, such as a file's, with a fainter detail at the other end.
    pub heading: Option<(SharedString, SharedString)>,
    pub lines: Vec<CodeLine>,
    /// Long lines wrap, such as a command; otherwise they are cut at the edge, such as source.
    pub wrap: bool,
    /// A Copy chip in the heading, which gets one if it has none.
    pub copy: Option<Handler>,
    pub motion: Motion,
}

fn line(code_line: CodeLine, numbered: bool, wrap: bool) -> impl IntoElement {
    let (color, ground, sign) = match code_line.change {
        Change::Same => (TEXT, None, " "),
        Change::Added => (GREEN, Some(GREEN), "+"),
        Change::Removed => (RED, Some(RED), "−"),
    };
    let diff = ground.is_some() || numbered;
    let removed = code_line.change == Change::Removed;
    div()
        .flex()
        .px(px(if numbered { 0.0 } else { PAD_X }))
        .text_color(rgb(color))
        .when_some(ground, |row, ground| {
            row.bg(rgba(with_alpha(ground, CHANGE_ALPHA)))
        })
        .when(numbered, |row| {
            row.child(
                div()
                    .w(px(NUMBER_W))
                    .flex_none()
                    .pr(px(PAD_X))
                    .flex()
                    .justify_end()
                    .text_color(rgb(if ground.is_some() { color } else { OVERLAY1 }))
                    .children(code_line.number.map(|number| number.to_string())),
            )
        })
        .when(diff, |row| row.child(div().flex_none().w(px(SIZE)).child(sign)))
        .child(
            div()
                .min_w(px(0.0))
                .pr(px(PAD_X))
                .when(!wrap, |text| text.whitespace_nowrap().overflow_hidden())
                .when(removed, |text| text.line_through())
                .child(code_line.text),
        )
}

/// `id` names its Copy chip and its rising in, which plays once for each id.
pub fn code(id: &str, code: Code, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let id = if code.key.is_empty() { id } else { code.key.as_ref() };
    let numbered = code.lines.iter().any(|line| line.number.is_some());
    let wrap = code.wrap;
    let count = code.lines.len();
    let copy = code.copy.map(|handler| {
        button::chip(SharedString::from(format!("{id}-copy")), "Copy", false, hover)
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                handler(this, cx);
            }))
    });
    let heading = match (code.heading, copy.is_some()) {
        (Some(heading), _) => Some(heading),
        (None, true) => Some((SharedString::default(), SharedString::default())),
        (None, false) => None,
    };
    let rows: Vec<_> = code
        .lines
        .into_iter()
        .map(|code_line| line(code_line, numbered, wrap))
        .collect();
    let appear = code.motion == Motion::Appear;
    let lines_id = SharedString::from(format!("{id}-lines"));
    div()
        .relative()
        .min_w(px(0.0))
        .overflow_hidden()
        .child(ui::shape(
            Radius::Control,
            Some(rgba(with_alpha(CRUST, GROUND_ALPHA)).into()),
        ))
        .when_some(heading, |block, (name, detail)| {
            block.child(
                div()
                    .relative()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .px(px(PAD_X))
                    .py(px(6.0))
                    .border_b_1()
                    .border_color(rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)))
                    .text_size(px(HEADING_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(SUBTEXT1))
                    .child(div().min_w(px(0.0)).truncate().child(name))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(div().font_family(MONO).child(detail))
                            .children(copy),
                    ),
            )
        })
        // Just the heading until there are lines, such as a command still running.
        .when(count > 0, |block| {
            let lines = div()
                .relative()
                .py(px(PAD_Y))
                .flex()
                .flex_col()
                .font_family(MONO)
                .font_weight(FontWeight::NORMAL)
                .text_size(px(SIZE))
                .line_height(px(LINE))
                .children(rows);
            block.child(if appear {
                reveal::appear(lines_id, lines)
            } else {
                lines.into_any_element()
            })
        })
        .into_any_element()
}
