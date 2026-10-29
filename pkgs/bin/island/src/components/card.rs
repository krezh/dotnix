//! A panel's cards: a surface of a fixed width with an optional title, holding blocks; frames,
//! the smaller surfaces inside them, such as one app's row or one message; and cards shared
//! between two columns of about the same height.

use std::{cell::RefCell, collections::HashMap, path::PathBuf, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, BoxShadow, Context, Div, Hsla, MouseButton, Rgba,
    SharedString, SpringAnimation, div, point, prelude::*, px, relative, rgba,
};

use crate::{
    components::{
        measure::{self, Measured},
        reveal,
    },
    island::{CONTENT_IN as SELECT_SPRING, Island},
    panel::Handler,
    theme::{
        CARD_ALPHA, CARD_BORDER_ALPHA, CRITICAL, NOTIFICATION_ALPHA, SURFACE1, with_alpha,
    },
    ui::{self, Radius},
};

/// Inside a raised frame, and between what it holds.
pub const RAISED_PAD: f32 = 14.0;
pub const RAISED_GAP: f32 = 8.0;
const INSET_PAD: f32 = 8.0;
const INSET_GAP: f32 = 6.0;
/// The countdown's tint: soft shapes of light blurred on the GPU, stronger toward its end, each
/// with how far along the frame it starts and how strongly it shows. Together they rise from
/// barely there to faint.
const COUNTDOWN_LAYERS: [(f32, u8); 2] = [(0.0, 0x1c), (0.5, 0x18)];
const COUNTDOWN_BLUR: f32 = 28.0;
/// Each shape is a line through the frame's middle, of no height, its shadow spread round it to
/// this far: a shadow is drawn only outside what casts it, so the line leaves no hole.
const COUNTDOWN_SPREAD: f32 = 18.0;
/// How far inside the frame's ends the shapes' light reaches, so the blur melts away before its
/// outline.
const COUNTDOWN_INSET_START: f32 = 10.0;
const COUNTDOWN_INSET_END: f32 = 28.0;
const URGENT_ALPHA: u8 = 0xb3;

/// Time running out, drawn as a soft blurred tint inside a frame that drains toward its start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countdown {
    /// 0–1.
    pub left: f32,
    /// `None` while paused.
    pub running: Option<Duration>,
    /// Counts each pause and resume, so the tint starts a new animation from where it is.
    pub round: u32,
    /// The tint's color, such as the app's own.
    pub color: u32,
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

/// Behind what the frame holds: light inside the glass, blurred so it has no edges at all, which
/// drains toward the frame's start as time runs out.
fn countdown_tint(id: &SharedString, countdown: Countdown) -> AnyElement {
    let tint = div()
        .absolute()
        .top_0()
        .left_0()
        .h_full()
        .flex()
        .items_center()
        .children(COUNTDOWN_LAYERS.map(|(from, alpha)| {
            div()
                .absolute()
                .h(px(0.0))
                .left(relative(from))
                .ml(px(COUNTDOWN_INSET_START + COUNTDOWN_SPREAD))
                .right(px(COUNTDOWN_INSET_END + COUNTDOWN_SPREAD))
                .shadow(vec![BoxShadow {
                    color: rgba(with_alpha(countdown.color, alpha)).into(),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(COUNTDOWN_BLUR),
                    spread_radius: px(COUNTDOWN_SPREAD),
                    inset: false,
                }])
        }));
    match countdown.running {
        Some(left) => tint
            .w(relative(countdown.left))
            .with_animation(
                SharedString::from(format!("{id}-countdown-{}", countdown.round)),
                Animation::new(left),
                move |tint, t| tint.w(relative(countdown.left * (1.0 - t))),
            )
            .into_any_element(),
        None => tint.w(relative(countdown.left)).into_any_element(),
    }
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
                .when_some(countdown, |frame, countdown| {
                    frame.child(shape().overflow_hidden().child(countdown_tint(&id, countdown)))
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
    /// Before the text, in its color, such as what the card is about.
    pub glyph: Option<(PathBuf, u32)>,
    /// A › at the end that opens more of what the card shows.
    pub open: Option<Handler>,
}

impl Title {
    pub fn with_detail(text: impl Into<SharedString>, detail: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            detail: detail.into(),
            glyph: None,
            open: None,
        }
    }

    pub fn glyph(mut self, glyph: Option<PathBuf>, color: u32) -> Self {
        self.glyph = glyph.map(|path| (path, color));
        self
    }

    pub fn opens(mut self, handler: Handler) -> Self {
        self.open = Some(handler);
        self
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

const BALANCED: usize = 2;
/// Taken for a card not yet drawn.
const UNDRAWN_H: f32 = 160.0;
/// Past this many cards, every split is too many to try, and each goes under the shorter column.
const TRIED: usize = 16;

thread_local! {
    /// Each balanced card's drawn height, by its key.
    static HEIGHTS: RefCell<HashMap<SharedString, Measured>> = RefCell::default();
}

fn drawn(key: &SharedString) -> Measured {
    HEIGHTS.with_borrow_mut(|heights| {
        heights
            .entry(key.clone())
            .or_insert_with(measure::measured)
            .clone()
    })
}

/// Which of two columns each card goes in, in order, so the taller column is as short as it can
/// be and each keeps the cards' order. It follows the heights drawn last time, so a card that grows
/// can move to the other column once it has been drawn.
pub fn balance(keys: &[SharedString]) -> Vec<usize> {
    let heights: Vec<f32> = keys
        .iter()
        .map(|key| {
            drawn(key)
                .get()
                .map_or(UNDRAWN_H, |size| f32::from(size.height))
                + GAP
        })
        .collect();
    if heights.len() > TRIED {
        let mut filled = [0.0f32; BALANCED];
        return heights
            .iter()
            .map(|height| {
                let column = usize::from(filled[1] < filled[0]);
                filled[column] += height;
                column
            })
            .collect();
    }
    // Every split, the first card always on the left; the lowest bits are the first cards, so
    // among equal splits the one keeping the earliest cards on the left wins.
    let taller = |split: u32| {
        let mut filled = [0.0f32; BALANCED];
        for (index, height) in heights.iter().enumerate() {
            filled[(split >> index & 1) as usize] += height;
        }
        filled[0].max(filled[1])
    };
    let best = (0..1u32 << heights.len().saturating_sub(1))
        .map(|split| split << 1)
        .min_by(|a, b| taller(*a).total_cmp(&taller(*b)))
        .unwrap_or(0);
    (0..heights.len())
        .map(|index| (best >> index & 1) as usize)
        .collect()
}

/// Cards shared between two columns as [`balance`] splits them, each named by its key.
pub fn balanced(cards: Vec<(SharedString, AnyElement)>) -> AnyElement {
    let keys: Vec<SharedString> = cards.iter().map(|(key, _)| key.clone()).collect();
    let mut sides: [Vec<AnyElement>; BALANCED] = Default::default();
    for ((key, element), column) in cards.into_iter().zip(balance(&keys)) {
        sides[column].push(
            div()
                .relative()
                .child(element)
                .child(measure::probe(&drawn(&key)))
                .into_any_element(),
        );
    }
    div()
        .flex()
        .items_start()
        .gap(px(GAP))
        .children(
            sides
                .into_iter()
                .map(|side| div().flex().flex_col().gap(px(GAP)).children(side)),
        )
        .into_any_element()
}
