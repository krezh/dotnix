//! A panel's cards: a surface of a fixed width with an optional title, holding blocks; and frames,
//! the smaller surfaces inside them, such as one app's row or one message.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, AnyElement, Context, Div, Hsla, MouseButton, Rgba, SharedString,
    SpringAnimation, div, prelude::*, px, relative, rgba,
};

use crate::{
    components::reveal,
    island::{CONTENT_IN as SELECT_SPRING, Island},
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
    /// Tinted as the chosen one of several, such as a picked answer; the tint eases in and out.
    pub selected: bool,
    pub motion: Motion,
    /// Shows only this much of a raised frame from its top, such as one folding away: its outline
    /// closes above the cut, and what it holds keeps its size and is cut off. `None` for all of it.
    pub shown_height: Option<f32>,
}

/// How a frame comes and goes, for one that is not placed and moved by its owner.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    Still,
    /// Rises in the first time it is drawn.
    Appear,
    /// Slides out, such as one just answered; its owner keeps it drawn for [`reveal::LEAVE`].
    Leave,
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
        selected,
        motion,
        shown_height,
    } = frame;
    let urgent = matches!(look, Look::Raised { urgent: true });
    let resting: Hsla = rgba(with_alpha(SURFACE1, CARD_ALPHA)).into();
    // The same quiet fill a picked tab has, so choosing reads the same everywhere.
    let chosen: Hsla = ui::selected_fill().into();
    let element = match look {
        Look::Inset => div()
            .id(id.clone())
            .relative()
            .p(px(INSET_PAD))
            .flex()
            .flex_col()
            .gap(px(INSET_GAP))
            .child(
                // Pinned to the corner: unpinned, it would start inside the padding.
                div().absolute().top_0().left_0().size_full().with_spring(
                    SharedString::from(format!("{id}-tint")),
                    SpringAnimation::new(SELECT_SPRING).to(selected),
                    move |tint, phase| {
                        let t = phase.0.clamp(0.0, 1.0);
                        tint.child(ui::shape(Radius::Control, Some(mix(resting, chosen, t))))
                    },
                ),
            )
            .when(on_click.is_some(), |frame| {
                frame.child(ui::control(Radius::Control, None, hover && !selected))
            })
            .children(children),
        Look::Raised { .. } => {
            let border: Hsla = if urgent {
                rgba(with_alpha(CRITICAL, URGENT_ALPHA)).into()
            } else {
                rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)).into()
            };
            let calm: Hsla = rgba(with_alpha(SURFACE1, NOTIFICATION_ALPHA)).into();
            // Where the outline and the hover fill are drawn.
            let shape = || match shown_height {
                Some(height) => div().absolute().top_0().left_0().w_full().h(px(height)),
                None => div().absolute().top_0().left_0().size_full(),
            };
            let content = div()
                .p(px(RAISED_PAD))
                .flex()
                .flex_col()
                .gap(px(RAISED_GAP))
                .children(children);
            let content = match shown_height {
                // Laid out at its own height, so the cut does not squeeze it.
                Some(_) => shape()
                    .overflow_hidden()
                    .child(content.absolute().top_0().left_0().w_full()),
                None => content.relative().size_full(),
            };
            div()
                .id(id.clone())
                .relative()
                .size_full()
                .child(shape().with_spring(
                    SharedString::from(format!("{id}-tint")),
                    SpringAnimation::new(SELECT_SPRING).to(selected),
                    move |tint, phase| {
                        let t = phase.0.clamp(0.0, 1.0);
                        tint.child(ui::outlined(Radius::Panel, mix(calm, chosen, t), border))
                    },
                ))
                .when(on_click.is_some(), |frame| {
                    frame.child(shape().child(ui::control(Radius::Panel, None, hover && !selected)))
                })
                .child(content)
        }
    };
    let element = element
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
        });
    match motion {
        Motion::Still => element.into_any_element(),
        Motion::Appear => reveal::appear(SharedString::from(format!("{id}-appear")), element),
        Motion::Leave => reveal::leave(SharedString::from(format!("{id}-leave")), element),
    }
}

/// Partway from one color to another.
fn mix(from: Hsla, to: Hsla, t: f32) -> Hsla {
    let (from, to) = (Rgba::from(from), Rgba::from(to));
    let at = |a: f32, b: f32| a + (b - a) * t;
    Rgba {
        r: at(from.r, to.r),
        g: at(from.g, to.g),
        b: at(from.b, to.b),
        a: at(from.a, to.a),
    }
    .into()
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
    /// As wide as a board of three small tiles, so a panel keeps one width across its views.
    Full,
    /// For text to read, such as a conversation: about 85 characters a line at reading size.
    Reading,
    /// As wide as a board of this many small tiles.
    Tiles(usize),
}

impl Size {
    pub fn width(self) -> f32 {
        match self {
            Size::Small => WIDTH,
            Size::Medium => 360.0,
            Size::Large => 2.0 * WIDTH + GAP,
            Size::Wide => 2.0 * Size::Medium.width() + GAP,
            Size::Full => 3.0 * WIDTH + 2.0 * GAP,
            Size::Reading => 760.0,
            Size::Tiles(count) => {
                let count = count.max(1) as f32;
                count * WIDTH + (count - 1.0) * GAP
            }
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
