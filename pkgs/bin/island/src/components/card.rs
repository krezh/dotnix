//! A panel's cards: a surface of a fixed width with an optional title, holding blocks; and frames,
//! the smaller surfaces inside them, such as one app's row or one message.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Context, Div, Hsla, MouseButton, SharedString, div,
    prelude::*, px, relative, rgba,
};

use crate::{
    island::Island,
    panel::Handler,
    theme::{
        ACCENT, CARD_ALPHA, CARD_BORDER_ALPHA, CRITICAL, NOTIFICATION_ALPHA, SURFACE1, with_alpha,
    },
    ui::{self, Radius},
};

/// Inside a raised frame, and between what it holds.
pub const RAISED_PAD: f32 = 14.0;
pub const RAISED_GAP: f32 = 8.0;
const INSET_PAD: f32 = 8.0;
const INSET_GAP: f32 = 6.0;
const COUNTDOWN_H: f32 = 2.0;
/// How strongly the countdown shows; it is a quiet part of the frame's edge, not content.
const COUNTDOWN_ALPHA: u8 = 0xb3;
const URGENT_ALPHA: u8 = 0xb3;

/// Time running out, drawn along a frame's bottom edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countdown {
    /// 0–1.
    pub left: f32,
    /// `None` while paused.
    pub running: Option<Duration>,
    /// Counts each pause and resume, so the bar starts a new animation from where it is.
    pub round: u32,
}

pub enum Look {
    /// A quiet surface inset in its card, such as one of many rows in a grid.
    Inset,
    /// Its own outlined surface, filling the space it is given, such as a message; urgent ones
    /// have a red outline.
    Raised { urgent: bool },
}

pub struct FrameView {
    pub id: SharedString,
    pub look: Look,
    pub on_click: Option<Handler>,
    /// On a right click.
    pub on_dismiss: Option<Handler>,
    pub countdown: Option<Countdown>,
}

/// Lies on the border's straight stretch between the rounded corners, so it reads as the outline
/// itself.
fn countdown_bar(id: &SharedString, countdown: Countdown) -> AnyElement {
    let bar = div()
        .h_full()
        .rounded_full()
        .bg(rgba(with_alpha(ACCENT, COUNTDOWN_ALPHA)));
    let bar = match countdown.running {
        Some(left) => bar
            .w(relative(countdown.left))
            .with_animation(
                SharedString::from(format!("{id}-countdown-{}", countdown.round)),
                Animation::new(left),
                move |bar, t| bar.w(relative(countdown.left * (1.0 - t))),
            )
            .into_any_element(),
        None => bar.w(relative(countdown.left)).into_any_element(),
    };
    div()
        .absolute()
        .left(px(ui::PANEL_RADIUS))
        .right(px(ui::PANEL_RADIUS))
        .bottom_0()
        .h(px(COUNTDOWN_H))
        .child(bar)
        .into_any_element()
}

/// Its `id` also names the group hovering it reveals parts of, such as a ✕.
pub fn frame(
    frame: FrameView,
    children: Vec<AnyElement>,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let FrameView {
        id,
        look,
        on_click,
        on_dismiss,
        countdown,
    } = frame;
    let urgent = matches!(look, Look::Raised { urgent: true });
    let element = match look {
        Look::Inset => div()
            .id(id.clone())
            .relative()
            .p(px(INSET_PAD))
            .flex()
            .flex_col()
            .gap(px(INSET_GAP))
            .child(ui::shape(
                Radius::Control,
                Some(rgba(with_alpha(SURFACE1, CARD_ALPHA)).into()),
            ))
            .children(children),
        Look::Raised { .. } => {
            let border: Hsla = if urgent {
                rgba(with_alpha(CRITICAL, URGENT_ALPHA)).into()
            } else {
                rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)).into()
            };
            div()
                .id(id.clone())
                .relative()
                .size_full()
                .child(ui::outlined(
                    Radius::Panel,
                    rgba(with_alpha(SURFACE1, NOTIFICATION_ALPHA)).into(),
                    border,
                ))
                .when(on_click.is_some(), |frame| {
                    frame.child(ui::control(Radius::Panel, None, hover))
                })
                .child(
                    div()
                        .relative()
                        .size_full()
                        .p(px(RAISED_PAD))
                        .flex()
                        .flex_col()
                        .gap(px(RAISED_GAP))
                        .children(children),
                )
        }
    };
    element
        .group(id.clone())
        .when_some(on_click, |frame, handler| {
            frame
                .cursor_pointer()
                .when(hover, |frame| frame.hover(|style| style))
                .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
        })
        .when_some(on_dismiss, |frame, handler| {
            frame.on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    handler(this, cx);
                }),
            )
        })
        .when_some(countdown, |frame, countdown| {
            frame.child(countdown_bar(&id, countdown))
        })
        .into_any_element()
}

pub const WIDTH: f32 = 270.0;
pub const GAP: f32 = 12.0;
/// Samples a history chart holds.
pub const SLOTS: usize = 40;

/// A card's width is fixed when it is created; content truncates rather than stretching it.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Size {
    Small,
    Medium,
    Large,
    /// As wide as two medium cards side by side.
    Wide,
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Small => WIDTH,
            Size::Medium => 360.0,
            Size::Large => 2.0 * WIDTH + GAP,
            Size::Wide => 2.0 * Size::Medium.width() + GAP,
        }
    }
}

/// A card's heading, with a fainter detail at the other end, such as "Output … 48 kHz · stereo".
pub struct Title {
    pub text: SharedString,
    pub detail: SharedString,
}

impl Title {
    pub fn with_detail(text: impl Into<SharedString>, detail: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            detail: detail.into(),
        }
    }
}

impl From<&'static str> for Title {
    fn from(text: &'static str) -> Self {
        Self::with_detail(text, "")
    }
}

impl From<String> for Title {
    fn from(text: String) -> Self {
        Self::with_detail(text, "")
    }
}

impl From<SharedString> for Title {
    fn from(text: SharedString) -> Self {
        Self::with_detail(text, "")
    }
}

pub struct Stat {
    pub text: SharedString,
    pub color: u32,
}

pub fn surface() -> Div {
    div().relative().child(ui::outlined(
        Radius::Panel,
        rgba(with_alpha(SURFACE1, CARD_ALPHA)).into(),
        rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)).into(),
    ))
}

pub fn stat(text: impl Into<SharedString>, color: u32) -> Stat {
    Stat {
        text: text.into(),
        color,
    }
}
