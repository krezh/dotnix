//! Text in a panel, in one of a few tones: plain, or styled in ranges with clickable links, held
//! to a number of lines.

use std::ops::Range;

use gpui::{
    AnyElement, FontStyle, FontWeight, HighlightStyle, Hsla, InteractiveText, SharedString,
    StrikethroughStyle, StyledText, TextRun, UnderlineStyle, Window, div, font, prelude::*, px,
    rgb, rgba,
};

use crate::{
    components::motion,
    theme::{ACCENT, CONTROL_ALPHA, FONT, SUBTEXT1, SURFACE1, TEXT, with_alpha},
    ui::SMALL_SIZE,
};

const CAPTION_SIZE: f32 = 12.0;
/// A body line, so wrapped text keeps an even rhythm.
pub const BODY_LINE: f32 = 17.0;
pub const BODY_SIZE: f32 = SMALL_SIZE;

pub enum Tone {
    /// Names what follows.
    Heading,
    /// A short line in a color.
    Caption(u32),
    /// Says something about the whole block, such as that it is empty.
    Note,
    /// Running text, such as a message.
    Body,
}

#[derive(Clone, Copy, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub code: bool,
    /// Hidden until looked at closely, such as a spoiler.
    pub covered: bool,
    pub link: bool,
}

pub struct Text {
    pub text: SharedString,
    pub tone: Tone,
    pub spans: Vec<(Range<usize>, Style)>,
    /// Clicking these ranges opens their address.
    pub links: Vec<(Range<usize>, String)>,
    /// Longer text ends in an ellipsis.
    pub lines: Option<usize>,
    pub min_width: Option<f32>,
}

impl Text {
    pub fn new(tone: Tone, text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone,
            spans: Vec::new(),
            links: Vec::new(),
            lines: None,
            min_width: None,
        }
    }
}

fn highlight(style: Style) -> HighlightStyle {
    let line = |shown: bool| shown.then(|| px(1.0));
    let tint: Hsla = rgba(with_alpha(SURFACE1, CONTROL_ALPHA)).into();
    let cover: Hsla = rgb(SURFACE1).into();
    HighlightStyle {
        font_weight: style.bold.then_some(FontWeight::SEMIBOLD),
        font_style: style.italic.then_some(FontStyle::Italic),
        underline: line(style.underline).map(|thickness| UnderlineStyle {
            thickness,
            ..UnderlineStyle::default()
        }),
        strikethrough: line(style.strike).map(|thickness| StrikethroughStyle {
            thickness,
            ..StrikethroughStyle::default()
        }),
        color: if style.covered {
            Some(cover)
        } else {
            style.link.then(|| rgb(ACCENT).into())
        },
        background_color: if style.covered {
            Some(cover)
        } else {
            style.code.then_some(tint)
        },
        ..HighlightStyle::default()
    }
}

/// How many lines `text` takes at `size` in `width`.
pub fn wrapped_lines(text: &str, size: f32, width: f32, window: &Window) -> usize {
    if text.is_empty() {
        return 0;
    }
    let run = TextRun {
        len: text.len(),
        font: gpui::Font {
            weight: FontWeight::MEDIUM,
            ..font(FONT)
        },
        color: rgb(TEXT).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_text(
            SharedString::from(text.to_owned()),
            px(size),
            &[run],
            Some(px(width)),
            None,
        )
        .map_or(1, |lines| {
            lines
                .iter()
                .map(|line| line.wrap_boundaries.len() + 1)
                .sum()
        })
}

/// Styled text, with its links clickable.
fn styled(id: SharedString, text: Text) -> AnyElement {
    let highlights: Vec<_> = text
        .spans
        .into_iter()
        .map(|(range, style)| (range, highlight(style)))
        .collect();
    let styled = StyledText::new(text.text).with_highlights(highlights);
    if text.links.is_empty() {
        return styled.into_any_element();
    }
    let (ranges, addresses): (Vec<_>, Vec<_>) = text.links.into_iter().unzip();
    InteractiveText::new(id, styled)
        .on_click(ranges, move |index, _, cx| {
            if let Some(address) = addresses.get(index) {
                cx.stop_propagation();
                cx.open_url(address);
            }
        })
        .into_any_element()
}

/// `id` keeps its morphing text and links apart from any other's.
pub fn text(text: Text, id: SharedString) -> AnyElement {
    let min_width = text.min_width;
    let element = match text.tone {
        Tone::Heading => div()
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(SUBTEXT1))
            .child(text.text),
        Tone::Caption(color) => div()
            .text_size(px(CAPTION_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(color))
            .child(motion::text(id, text.text)),
        Tone::Note => div()
            .w_full()
            .py(px(14.0))
            .flex()
            .justify_center()
            .text_size(px(SMALL_SIZE))
            .text_color(rgb(SUBTEXT1))
            .child(motion::text(id, text.text)),
        Tone::Body => {
            let lines = text.lines;
            div()
                .line_height(px(BODY_LINE))
                .text_size(px(BODY_SIZE))
                .when_some(lines, |body, lines| {
                    body.line_clamp(lines)
                        .max_h(px(BODY_LINE * lines as f32))
                        .overflow_hidden()
                        .text_ellipsis()
                })
                .child(styled(id, text))
        }
    };
    element
        .when_some(min_width, |element, width| element.min_w(px(width)))
        .into_any_element()
}
