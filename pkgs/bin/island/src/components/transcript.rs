//! A conversation: your turns on the right, the other side's on the left beside its mark, each
//! reply on a surface of its own under a faint line saying who and how long, the newest at the
//! bottom, with dots while a reply is on its way.

use std::{path::PathBuf, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, Context, FontWeight, SharedString, div, prelude::*, px,
    rgb, rgba,
};

use crate::{
    components::{
        button,
        code::{self, Code},
        icon, reveal,
        text::{self, Text},
    },
    island::Island,
    panel::Handler,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, CARD_BORDER_ALPHA, NOTIFICATION_ALPHA, OVERLAY2, SURFACE1,
        with_alpha,
    },
    ui::{self, Radius, SMALL_SIZE},
};

const MARK: f32 = 28.0;
const MARK_GLYPH: f32 = 15.0;
const TURN_GAP: f32 = 16.0;
const PIECE_GAP: f32 = 12.0;
const BUBBLE_PAD_X: f32 = 14.0;
const BUBBLE_PAD_Y: f32 = 10.0;
const REPLY_PAD: f32 = 16.0;
const OVERLINE: f32 = 12.0;
/// A turn of yours takes at most this share of the width.
const BUBBLE_SHARE: f32 = 0.7;
const DOT: f32 = 6.0;
/// How strongly the mark glows at the top of its breath while a reply is being written.
const GLOW_ALPHA: f32 = 0.55;

pub enum Piece {
    Text(Text),
    Code(Code),
}

pub enum Turn {
    Mine(Text),
    /// `overline` says who answered and how long it took; `streaming` while more is coming.
    Theirs {
        overline: SharedString,
        pieces: Vec<Piece>,
        streaming: bool,
        /// Small buttons under a finished reply, such as Copy.
        actions: Vec<(SharedString, Handler)>,
    },
    /// A reply on its way with nothing of it yet.
    Waiting {
        overline: SharedString,
    },
}

pub struct Transcript {
    pub id: SharedString,
    pub turns: Vec<Turn>,
    /// Marks the other side's turns, tinted with `color`.
    pub glyph: Option<PathBuf>,
    pub color: u32,
    /// Turns from this one on rise in as they first show; those before, already there, such as
    /// history, are shown still.
    pub appear_from: usize,
}

/// The other side's mark; while `alive` it glows in and out, as something thinking.
fn mark(id: SharedString, glyph: Option<&std::path::Path>, color: u32, alive: bool) -> AnyElement {
    let tint = rgba(with_alpha(color, ACCENT_TINT_ALPHA));
    let base = div()
        .relative()
        .flex_none()
        .size(px(MARK))
        .flex()
        .items_center()
        .justify_center()
        .child(ui::shape(Radius::Row, Some(tint.into())));
    let glow = alive.then(|| {
        div()
            .absolute()
            .size_full()
            .child(ui::shape(Radius::Row, Some(rgb(color).into())))
            .with_animation(
                SharedString::from(format!("{id}-glow")),
                Animation::new(Duration::from_millis(1600)).repeat(),
                |glow, t| {
                    let breath = 0.5 - 0.5 * (t * std::f32::consts::TAU).cos();
                    glow.opacity(breath * GLOW_ALPHA)
                },
            )
    });
    base.children(glow)
        .children(glyph.map(|path| icon::glyph(path, MARK_GLYPH, color)))
        .into_any_element()
}

fn dots(id: SharedString) -> impl IntoElement {
    div()
        .flex()
        .gap(px(4.0))
        .py(px(6.0))
        .children((0..3).map(move |index| {
            div()
                .size(px(DOT))
                .rounded_full()
                .bg(rgb(OVERLAY2))
                .with_animation(
                    SharedString::from(format!("{id}-dot-{index}")),
                    Animation::new(Duration::from_millis(1100)).repeat(),
                    move |dot, t| {
                        let phase = (t - index as f32 * 0.14).rem_euclid(1.0);
                        let lift = (phase * std::f32::consts::TAU).sin().max(0.0);
                        dot.opacity(0.35 + 0.65 * lift)
                    },
                )
        }))
}

/// Breathes while the reply arrives, as the search field's caret does.
fn caret() -> impl IntoElement {
    div()
        .w(px(2.0))
        .h(px(14.0))
        .rounded_full()
        .bg(rgb(ACCENT))
        .with_animation(
            "transcript-caret",
            Animation::new(Duration::from_millis(1100)).repeat(),
            |caret, t| {
                caret
                    .opacity(((t * std::f32::consts::TAU).cos() * 1.6).clamp(-1.0, 1.0) * 0.5 + 0.5)
            },
        )
}

/// A reply's own surface, quieter than a card, as a message is.
fn surface() -> gpui::Div {
    div().relative().child(ui::outlined(
        Radius::Panel,
        rgba(with_alpha(SURFACE1, NOTIFICATION_ALPHA)).into(),
        rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)).into(),
    ))
}

/// Who answered, in their color.
fn overline(text: SharedString, color: u32) -> impl IntoElement {
    div()
        .text_size(px(OVERLINE))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(color))
        .child(text)
}

pub fn transcript(transcript: Transcript, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let Transcript {
        id,
        turns,
        glyph,
        color,
        appear_from,
    } = transcript;
    let mut rows = Vec::with_capacity(turns.len());
    for (index, turn) in turns.into_iter().enumerate() {
        let at = SharedString::from(format!("{id}-{index}"));
        let entered = SharedString::from(format!("{id}-{index}-enter"));
        let element = match turn {
            Turn::Mine(content) => div()
                .flex()
                .justify_end()
                .child(
                    div()
                        .relative()
                        .max_w(gpui::relative(BUBBLE_SHARE))
                        .px(px(BUBBLE_PAD_X))
                        .py(px(BUBBLE_PAD_Y))
                        // Your turns in the accent, Claude's beside its own color.
                        .child(ui::shape(
                            Radius::Control,
                            Some(rgba(with_alpha(ACCENT, ACCENT_TINT_ALPHA)).into()),
                        ))
                        .child(div().relative().child(text::text(content, at))),
                )
                .into_any_element(),
            Turn::Theirs {
                overline: line,
                pieces,
                streaming,
                actions,
            } => {
                let count = pieces.len();
                let mut body = Vec::with_capacity(count);
                for (part, piece) in pieces.into_iter().enumerate() {
                    let piece_id = format!("{at}-{part}");
                    let element = match piece {
                        Piece::Text(content) => text::text(content, piece_id.into()),
                        Piece::Code(block) => code::code(&piece_id, block, hover, cx),
                    };
                    body.push(if streaming && part + 1 == count {
                        div()
                            .flex()
                            .items_end()
                            .gap(px(2.0))
                            .child(div().min_w(px(0.0)).child(element))
                            .child(caret())
                            .into_any_element()
                    } else {
                        element
                    });
                }
                let chips: Vec<AnyElement> = actions
                    .into_iter()
                    .enumerate()
                    .map(|(number, (label, handler))| {
                        button::chip(
                            SharedString::from(format!("{at}-action-{number}")),
                            label,
                            false,
                            hover,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                        .into_any_element()
                    })
                    .collect();
                div()
                    .flex()
                    .items_start()
                    .gap(px(10.0))
                    .child(mark(at.clone(), glyph.as_deref(), color, streaming))
                    .child(
                        surface().flex_1().min_w(px(0.0)).child(
                            div()
                                .relative()
                                .p(px(REPLY_PAD))
                                .flex()
                                .flex_col()
                                .gap(px(PIECE_GAP))
                                .text_size(px(SMALL_SIZE))
                                .child(overline(line, color))
                                .children(body)
                                .when(!chips.is_empty(), |reply| {
                                    reply.child(div().flex().gap(px(6.0)).children(chips))
                                }),
                        ),
                    )
                    .into_any_element()
            }
            Turn::Waiting { overline: line } => div()
                .flex()
                .items_start()
                .gap(px(10.0))
                .child(mark(at.clone(), glyph.as_deref(), color, true))
                .child(
                    surface().child(
                        div()
                            .relative()
                            .px(px(REPLY_PAD))
                            .py(px(REPLY_PAD - 2.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(overline(line, color))
                            .child(dots(at)),
                    ),
                )
                .into_any_element(),
        };
        rows.push(if index >= appear_from {
            reveal::appear(entered, element)
        } else {
            element
        });
    }
    div()
        .flex()
        .flex_col()
        .gap(px(TURN_GAP))
        .children(rows)
        .into_any_element()
}
