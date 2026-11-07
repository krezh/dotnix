//! A small flexbox tree, shaped like GPUI's `div()`, laid out with taffy and painted into sprites.
//! Panels are built from it, so a component reads much as it does for GPUI.

use std::{
    borrow::Cow,
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    ops::Range,
    path::PathBuf,
    process::Command,
    rc::Rc,
    thread,
};

use taffy::{
    AlignItems, AvailableSpace, CompactLength, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, NodeId, Position, Rect, Size, Style,
    TaffyTree, compute_leaf_layout,
};

use super::{
    paint::{Fit, Framing, Painter},
    panel::{
        Built, Chart, Countdown, Ease, Easing, Hit, Marquee, MorphText, Pulse, PulseKind, Reveal,
        Rise, Scrolling, Shown, Sliding, noted,
    },
    renderer::{Gpu, Sprite, overlap},
    widget::{color as solid, tinted},
    wrap::cut_styles,
};
use crate::{
    components::{
        chart::{Phase, Series},
        popover::SHADOW_SPREAD,
        search::Scroll as Rows,
        slider::OnChange,
        text::Style as TextStyle,
    },
    drawing::Drawing,
    panel::{Handler, SharedString},
    picture::Picture,
    spring::SpringConfig,
    stage::CONTENT_IN,
    theme::{ACCENT, CONTROL_ALPHA, FONT, MONO, SHADOW_ALPHA, SURFACE1, TEXT},
    ui::TEXT_SIZE,
};

pub const MEDIUM: u16 = 500;
pub const SEMIBOLD: u16 = 600;
/// Past any child's index, so a node's eases are named apart from its children.
const EASES: usize = 1 << 20;
const EDGE_RAMP: f32 = 24.0;
const STRIPE_W: f32 = 1.9;
const STRIPE_EVERY: f32 = 5.5;
const DASH: f32 = 2.5;
const DASH_EVERY: f32 = 5.5;
pub const BOLD: u16 = 700;
/// GPUI's line height when nothing sets one.
const LEADING: f32 = 1.618;

#[derive(Clone, Copy, Default)]
struct Inherited {
    size: Option<f32>,
    weight: Option<u16>,
    color: Option<u32>,
    leading: Option<f32>,
    /// A line height in pixels, over `leading`.
    line: Option<f32>,
    nowrap: Option<bool>,
    ellipsis: Option<bool>,
    mono: Option<bool>,
    strike: Option<bool>,
}

#[derive(Clone, Copy)]
struct Words {
    size: f32,
    weight: u16,
    color: u32,
    leading: f32,
    /// A line height in pixels, over `leading`.
    fixed: Option<f32>,
    nowrap: bool,
    ellipsis: bool,
    mono: bool,
    strike: bool,
    /// Lines come out on whole pixels of the screen.
    scale: f32,
}

impl Words {
    fn base(scale: f32) -> Self {
        Self {
            scale,
            size: TEXT_SIZE,
            weight: MEDIUM,
            color: TEXT,
            leading: LEADING,
            fixed: None,
            nowrap: false,
            ellipsis: false,
            mono: false,
            strike: false,
        }
    }

    fn family(self) -> &'static str {
        if self.mono { MONO } else { FONT }
    }

    fn fit(self, room: Option<f32>, most: Option<usize>) -> Fit {
        Fit {
            room,
            wraps: !self.nowrap,
            ellipsis: self.ellipsis,
            most,
        }
    }

    fn line(self) -> f32 {
        snap(self.fixed.unwrap_or(self.size * self.leading), self.scale)
    }

    fn with(self, own: Inherited) -> Self {
        Self {
            scale: self.scale,
            size: own.size.unwrap_or(self.size),
            weight: own.weight.unwrap_or(self.weight),
            color: own.color.unwrap_or(self.color),
            leading: own.leading.unwrap_or(self.leading),
            fixed: own.line.or(if own.leading.is_some() {
                None
            } else {
                self.fixed
            }),
            nowrap: own.nowrap.unwrap_or(self.nowrap),
            ellipsis: own.ellipsis.unwrap_or(self.ellipsis),
            mono: own.mono.unwrap_or(self.mono),
            strike: own.strike.unwrap_or(self.strike),
        }
    }
}

enum Content {
    None,
    Text {
        text: SharedString,
        lines: Option<usize>,
        moves: bool,
    },
    Glyph(PathBuf, u32),
    Picture(Picture, Framing),
    Drawing(String, Drawing),
    Chart(Vec<Series>, Option<Phase>),
    Countdown(Box<Countdown>),
    Sprites(Vec<Sprite>),
    Marquee {
        text: SharedString,
        slot: Option<f32>,
        on_hover: bool,
    },
    Caret,
    Pulse(PulseKind, [f32; 4], f32),
    Stripes([f32; 4]),
    Gridlines(usize, [f32; 4]),
    Styled {
        text: SharedString,
        styles: Vec<(Range<usize>, TextStyle)>,
        links: Vec<(Range<usize>, String)>,
        lines: Option<usize>,
    },
}

/// Answers the pointer in a panel.
#[derive(Default)]
struct Answers {
    on_click: Option<Handler>,
    on_right: Option<Handler>,
    hover: Option<f32>,
    tip: Option<SharedString>,
    on_change: Option<OnChange>,
    on_hover: Option<Handler>,
    on_rows: Option<Rows>,
    anchor: Option<SharedString>,
    immediate: bool,
    blocks: bool,
    lit: Option<u32>,
    shown_in: Option<SharedString>,
    pulsing: bool,
}

pub struct Node {
    style: Style,
    words: Inherited,
    fill: Option<([f32; 4], f32)>,
    ring: Option<([f32; 4], f32)>,
    top_line: Option<[f32; 4]>,
    opacity: f32,
    clip: bool,
    content: Content,
    name: Option<u64>,
    children: Vec<Node>,
    answers: Answers,
    scroll: Option<SharedString>,
    rounding: f32,
    /// What scrolls fades out under its top and bottom.
    soft: bool,
    /// The most out of view its soft edges show for, however far it can scroll now.
    soft_reach: f32,
    /// The furthest it scrolls, however much it holds.
    scroll_reach: f32,
    eases: Vec<(Easing, f32, SpringConfig)>,
    /// Takes up room but shows nothing.
    hidden: bool,
    /// Casts a shadow with corners this round.
    shadow: Option<f32>,
    /// Comes into view, or with `true`, goes.
    rises: Option<bool>,
    /// Runs a beam round itself while this is true.
    busy: Option<bool>,
}

pub fn div() -> Node {
    Node {
        style: Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            position: Position::Relative,
            ..Style::default()
        },
        words: Inherited::default(),
        fill: None,
        ring: None,
        top_line: None,
        opacity: 1.0,
        clip: false,
        content: Content::None,
        name: None,
        children: Vec::new(),
        answers: Answers::default(),
        scroll: None,
        rounding: 0.0,
        soft: false,
        soft_reach: f32::INFINITY,
        scroll_reach: f32::INFINITY,
        eases: Vec::new(),
        hidden: false,
        shadow: None,
        rises: None,
        busy: None,
    }
}

pub fn text(text: impl Into<SharedString>) -> Node {
    let mut node = div();
    node.content = Content::Text {
        text: text.into(),
        lines: None,
        moves: false,
    };
    node
}

/// Words that morph into what they say next.
pub fn moving(words: impl Into<SharedString>) -> Node {
    let mut node = text(words);
    if let Content::Text { moves, .. } = &mut node.content {
        *moves = true;
    }
    node
}

/// Held to `lines`, the last ending in an ellipsis.
pub fn clamped(words: impl Into<SharedString>, lines: usize) -> Node {
    let mut node = text(words);
    if let Content::Text { lines: most, .. } = &mut node.content {
        *most = Some(lines);
    }
    node
}

pub fn glyph(path: impl Into<PathBuf>, side: f32, tint: u32) -> Node {
    let mut node = div().size(side).flex_none();
    node.content = Content::Glyph(path.into(), tint);
    node
}

pub fn picture(picture: Picture, side: f32) -> Node {
    let mut node = div().size(side).flex_none();
    node.content = Content::Picture(picture, Framing::Contain);
    node
}

/// Own drawing in a square, in the part of the atlas named `key`.
pub fn drawing(key: impl Into<String>, drawing: Drawing) -> Node {
    let mut node = div().size(drawing.side).flex_none();
    node.content = Content::Drawing(key.into(), drawing);
    node
}

/// Fills what it is given.
pub fn chart(series: Vec<Series>, phase: Option<Phase>) -> Node {
    let mut node = div();
    node.content = Content::Chart(series, phase);
    node
}

pub fn countdown(countdown: Countdown) -> Node {
    let mut node = div().absolute().inset(0.0);
    node.content = Content::Countdown(Box::new(countdown));
    node
}

/// As much of the text as fits `slot`, gliding if it does not fit.
pub fn marquee(words: impl Into<SharedString>, slot: f32) -> Node {
    let mut node = div();
    node.content = Content::Marquee {
        text: words.into(),
        slot: Some(slot),
        on_hover: false,
    };
    node
}

/// As wide as it is given, centred if it fits and otherwise gliding while the pointer is on what
/// holds it.
pub fn marquee_on_hover(words: impl Into<SharedString>) -> Node {
    let mut node = div();
    node.content = Content::Marquee {
        text: words.into(),
        slot: None,
        on_hover: true,
    };
    node
}

/// Text in ranges of styles, its links clickable.
pub fn styled(
    words: impl Into<SharedString>,
    styles: Vec<(Range<usize>, TextStyle)>,
    links: Vec<(Range<usize>, String)>,
    lines: Option<usize>,
) -> Node {
    let mut node = div();
    node.content = Content::Styled {
        text: words.into(),
        styles,
        links,
        lines,
    };
    node
}

/// `words`, centred in the width of the widest of `fits`, so what it sits in never resizes as it
/// changes.
pub fn steady(words: impl Into<SharedString>, fits: Vec<SharedString>) -> Node {
    div()
        .flex_none()
        .items_center()
        .nowrap()
        .children(fits.into_iter().map(|fit| {
            let mut unseen = text(fit).h(0.0);
            unseen.hidden = true;
            unseen
        }))
        .child(moving(words))
}

/// A shape following the clock.
pub fn pulse(kind: PulseKind, tint: [f32; 4], radius: f32) -> Node {
    let mut node = div();
    node.content = Content::Pulse(kind, tint, radius);
    node
}

/// Diagonal hatching over all of it.
pub fn stripes(tint: [f32; 4]) -> Node {
    let mut node = div();
    node.content = Content::Stripes(tint);
    node
}

/// `count` dashed lines down it, spread evenly.
pub fn gridlines(count: usize, tint: [f32; 4]) -> Node {
    let mut node = div();
    node.content = Content::Gridlines(count, tint);
    node
}

/// Where typing goes, breathing rather than blinking.
pub fn caret(height: f32) -> Node {
    let mut node = div().w(2.0).h(height).flex_none();
    node.content = Content::Caret;
    node
}

/// Sprites of its own, relative to its corner.
pub fn sprites(sprites: Vec<Sprite>) -> Node {
    let mut node = div();
    node.content = Content::Sprites(sprites);
    node
}

fn length(value: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::length(value)
}

impl Node {
    pub fn flex(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self
    }

    pub fn col(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }

    pub fn wrap(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Row;
        self.style.flex_wrap = FlexWrap::Wrap;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.style.gap = Size {
            width: LengthPercentage::length(gap),
            height: LengthPercentage::length(gap),
        };
        self
    }

    pub fn gap_y(mut self, gap: f32) -> Self {
        self.style.gap.height = LengthPercentage::length(gap);
        self
    }

    pub fn items_center(mut self) -> Self {
        self.style.align_items = AlignItems::CENTER;
        self
    }

    pub fn items_start(mut self) -> Self {
        self.style.align_items = AlignItems::FLEX_START;
        self
    }

    pub fn items_end(mut self) -> Self {
        self.style.align_items = AlignItems::END;
        self
    }

    pub fn items_baseline(mut self) -> Self {
        self.style.align_items = AlignItems::BASELINE;
        self
    }

    pub fn justify_center(mut self) -> Self {
        self.style.justify_content = JustifyContent::CENTER;
        self
    }

    pub fn justify_between(mut self) -> Self {
        self.style.justify_content = JustifyContent::SPACE_BETWEEN;
        self
    }

    pub fn justify_end(mut self) -> Self {
        self.style.justify_content = JustifyContent::FLEX_END;
        self
    }

    pub fn justify_start(mut self) -> Self {
        self.style.justify_content = JustifyContent::FLEX_START;
        self
    }

    pub fn flex_1(mut self) -> Self {
        self.style.flex_grow = 1.0;
        self.style.flex_shrink = 1.0;
        self.style.flex_basis = Dimension::percent(0.0);
        self
    }

    pub fn flex_none(mut self) -> Self {
        self.style.flex_grow = 0.0;
        self.style.flex_shrink = 0.0;
        self
    }

    pub fn grow(mut self, by: f32) -> Self {
        self.style.flex_grow = by;
        self
    }

    pub fn w(mut self, width: f32) -> Self {
        self.style.size.width = Dimension::length(width);
        self
    }

    pub fn h(mut self, height: f32) -> Self {
        self.style.size.height = Dimension::length(height);
        self
    }

    pub fn size(self, side: f32) -> Self {
        self.w(side).h(side)
    }

    pub fn w_full(mut self) -> Self {
        self.style.size.width = Dimension::percent(1.0);
        self
    }

    pub fn h_full(mut self) -> Self {
        self.style.size.height = Dimension::percent(1.0);
        self
    }

    pub fn w_share(mut self, share: f32) -> Self {
        self.style.size.width = Dimension::percent(share);
        self
    }

    pub fn size_full(self) -> Self {
        self.w_full().h_full()
    }

    pub fn min_w(mut self, width: f32) -> Self {
        self.style.min_size.width = length(width);
        self
    }

    pub fn min_h(mut self, height: f32) -> Self {
        self.style.min_size.height = length(height);
        self
    }

    pub fn max_w(mut self, width: f32) -> Self {
        self.style.max_size.width = length(width);
        self
    }

    pub fn max_h(mut self, height: f32) -> Self {
        self.style.max_size.height = length(height);
        self
    }

    pub fn p(self, all: f32) -> Self {
        self.px(all).py(all)
    }

    pub fn px(self, side: f32) -> Self {
        self.pl(side).pr(side)
    }

    pub fn py(self, side: f32) -> Self {
        self.pt(side).pb(side)
    }

    pub fn pl(mut self, side: f32) -> Self {
        self.style.padding.left = LengthPercentage::length(side);
        self
    }

    pub fn pr(mut self, side: f32) -> Self {
        self.style.padding.right = LengthPercentage::length(side);
        self
    }

    pub fn pt(mut self, side: f32) -> Self {
        self.style.padding.top = LengthPercentage::length(side);
        self
    }

    pub fn pb(mut self, side: f32) -> Self {
        self.style.padding.bottom = LengthPercentage::length(side);
        self
    }

    pub fn ml(mut self, side: f32) -> Self {
        self.style.margin.left = length(side);
        self
    }

    pub fn mr(mut self, side: f32) -> Self {
        self.style.margin.right = length(side);
        self
    }

    pub fn mx(self, side: f32) -> Self {
        self.ml(side).mr(side)
    }

    pub fn mt(mut self, side: f32) -> Self {
        self.style.margin.top = length(side);
        self
    }

    pub fn absolute(mut self) -> Self {
        self.style.position = Position::Absolute;
        self
    }

    pub fn inset(mut self, all: f32) -> Self {
        self.style.inset = Rect {
            left: length(all),
            right: length(all),
            top: length(all),
            bottom: length(all),
        };
        self
    }

    pub fn left(mut self, at: f32) -> Self {
        self.style.inset.left = length(at);
        self
    }

    pub fn right(mut self, at: f32) -> Self {
        self.style.inset.right = length(at);
        self
    }

    pub fn top(mut self, at: f32) -> Self {
        self.style.inset.top = length(at);
        self
    }

    pub fn bottom(mut self, at: f32) -> Self {
        self.style.inset.bottom = length(at);
        self
    }

    pub fn text_size(mut self, size: f32) -> Self {
        self.words.size = Some(size);
        self
    }

    pub fn weight(mut self, weight: u16) -> Self {
        self.words.weight = Some(weight);
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.words.color = Some(color);
        self
    }

    pub fn leading(mut self, leading: f32) -> Self {
        self.words.leading = Some(leading);
        self
    }

    pub fn line(mut self, height: f32) -> Self {
        self.words.line = Some(height);
        self
    }

    pub fn mono(mut self) -> Self {
        self.words.mono = Some(true);
        self
    }

    /// Struck through.
    pub fn strike(mut self) -> Self {
        self.words.strike = Some(true);
        self
    }

    pub fn mb(mut self, value: f32) -> Self {
        self.style.margin.bottom = LengthPercentageAuto::length(value);
        self
    }

    pub fn max_w_share(mut self, share: f32) -> Self {
        self.style.max_size.width = LengthPercentageAuto::percent(share);
        self
    }

    pub fn nowrap(mut self) -> Self {
        self.words.nowrap = Some(true);
        self
    }

    /// Known by `id` wherever it sits, so what it holds keeps its text and motion as it moves.
    pub fn named(mut self, id: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        id.hash(&mut hasher);
        self.name = Some(hasher.finish());
        self
    }

    /// A picture filling its box, what spills over cut off.
    pub fn cover(mut self) -> Self {
        if let Content::Picture(_, framing) = &mut self.content {
            *framing = Framing::Cover;
        }
        self
    }

    /// On one line, ending in an ellipsis if it does not fit.
    pub fn truncate(mut self) -> Self {
        self.words.nowrap = Some(true);
        self.words.ellipsis = Some(true);
        self.clip = true;
        self
    }

    pub fn bg(mut self, color: [f32; 4], radius: f32) -> Self {
        self.fill = Some((color, radius));
        self
    }

    pub fn outline(mut self, color: [f32; 4], radius: f32) -> Self {
        self.ring = Some((color, radius));
        self
    }

    /// A one-pixel rule along its top.
    pub fn rule(mut self, color: [f32; 4]) -> Self {
        self.top_line = Some(color);
        self.style.border.top = LengthPercentage::length(1.0);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn clip(mut self) -> Self {
        self.clip = true;
        self
    }

    /// Casts a soft shadow round it, as what floats over a panel does.
    pub fn casts(mut self, radius: f32) -> Self {
        self.shadow = Some(radius);
        self
    }

    /// A beam runs round it while `busy`.
    pub fn beams(mut self, busy: bool) -> Self {
        self.busy = Some(busy);
        self
    }

    /// Fades in and settles into place the first time it is drawn.
    pub fn rises(mut self) -> Self {
        self.rises = Some(false);
        self
    }

    /// Fades out as it slides aside, from the first time it is drawn.
    pub fn leaves(mut self) -> Self {
        self.rises = Some(true);
        self
    }

    /// This share of what holds it, easing there when the share changes.
    pub fn springy(self, share: f32) -> Self {
        self.springy_with(share, CONTENT_IN)
    }

    pub fn springy_with(self, share: f32, config: SpringConfig) -> Self {
        self.w_share(share)
            .eases(Easing::Wide { whole: 0.0 }, share, config)
    }

    /// Eases to `value` when it changes, starting there.
    pub fn eases(mut self, easing: Easing, value: f32, config: SpringConfig) -> Self {
        self.eases.push((easing, value, config));
        self
    }

    /// What scrolls under its top and bottom fades away, more the more lies past each.
    pub fn soft_edges(mut self) -> Self {
        self.soft = true;
        self
    }

    /// As [`Self::soft_edges`], showing for no more than `reach` out of view.
    pub fn soft_within(mut self, reach: f32) -> Self {
        self.soft_reach = reach;
        self
    }

    /// Scrolls no further than `reach`.
    pub fn scrolls_within(mut self, reach: f32) -> Self {
        self.scroll_reach = reach;
        self
    }

    pub fn scrolls(mut self, key: impl Into<SharedString>) -> Self {
        self.scroll = Some(key.into());
        self.clip = true;
        self
    }

    pub fn child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn when(self, condition: bool, then: impl FnOnce(Self) -> Self) -> Self {
        if condition { then(self) } else { self }
    }

    pub fn when_some<T>(self, value: Option<T>, then: impl FnOnce(Self, T) -> Self) -> Self {
        match value {
            Some(value) => then(self, value),
            None => self,
        }
    }

    pub fn no_shrink(mut self) -> Self {
        self.style.flex_shrink = 0.0;
        self
    }

    pub fn left_share(mut self, share: f32) -> Self {
        self.style.inset.left = LengthPercentageAuto::percent(share);
        self
    }

    /// A picture's corners.
    pub fn rounded(mut self, radius: f32) -> Self {
        self.rounding = radius;
        self
    }

    pub fn on_click(mut self, handler: Handler) -> Self {
        self.answers.on_click = Some(handler);
        self
    }

    pub fn on_right(mut self, handler: Handler) -> Self {
        self.answers.on_right = Some(handler);
        self
    }

    /// Tinted, at this radius, while the pointer is on it.
    /// Words in the color it hands down turn to `tint` under the pointer.
    pub fn lit(mut self, tint: u32) -> Self {
        self.answers.lit = Some(tint);
        self
    }

    /// Unseen until the pointer is in the rect anchored as `group`.
    /// Fades in and out for as long as it shows.
    pub fn pulsing(mut self) -> Self {
        self.answers.pulsing = true;
        self
    }

    pub fn shown_in(mut self, group: SharedString) -> Self {
        self.answers.shown_in = Some(group);
        self
    }

    /// Shows no hover.
    pub fn unlit(mut self) -> Self {
        self.answers.hover = None;
        self
    }

    pub fn hover(mut self, radius: f32) -> Self {
        self.answers.hover = Some(radius);
        self
    }

    pub fn tip(mut self, text: impl Into<SharedString>) -> Self {
        self.answers.tip = Some(text.into());
        self
    }

    /// Acts as the button goes down: what opens or picks from a list does.
    pub fn on_press(mut self, handler: Handler) -> Self {
        self.answers.on_click = Some(handler);
        self.answers.immediate = true;
        self
    }

    /// Nothing under it shows a hover or a tip.
    pub fn blocks(mut self) -> Self {
        self.answers.blocks = true;
        self
    }

    /// Told when the pointer comes onto it.
    pub fn on_hover(mut self, handler: Handler) -> Self {
        self.answers.on_hover = Some(handler);
        self
    }

    /// Told how many rows it scrolled.
    pub fn on_rows(mut self, rows: Rows) -> Self {
        self.answers.on_rows = Some(rows);
        self
    }

    /// Notes where it comes out, for something opening from it.
    pub fn anchor(mut self, key: impl Into<SharedString>) -> Self {
        self.answers.anchor = Some(key.into());
        self
    }

    /// Follows the pointer, as a slider does.
    pub fn slides(mut self, on_change: OnChange) -> Self {
        self.answers.on_change = Some(on_change);
        self
    }
}

struct Context {
    text: SharedString,
    words: Words,
    lines: Option<usize>,
    styles: Option<Rc<Vec<(Range<usize>, TextStyle)>>>,
}

struct Laid {
    id: NodeId,
    words: Words,
    children: Vec<Laid>,
}

/// On a whole pixel of the screen, halves going toward zero.
fn snap(value: f32, scale: f32) -> f32 {
    let pixels = value * scale;
    (pixels.abs() - 0.5).ceil().copysign(pixels) / scale
}

/// With every length on whole pixels of the screen.
fn snapped(style: &Style, scale: f32) -> Style {
    let side = |side: Dimension| {
        if side.tag() == CompactLength::LENGTH_TAG {
            Dimension::length(snap(side.value(), scale))
        } else {
            side
        }
    };
    let space = |length: LengthPercentage| {
        let raw = length.into_raw();
        if raw.tag() == CompactLength::LENGTH_TAG {
            LengthPercentage::length(snap(raw.value(), scale))
        } else {
            length
        }
    };
    let place = |length: LengthPercentageAuto| {
        let raw = length.into_raw();
        if raw.tag() == CompactLength::LENGTH_TAG {
            LengthPercentageAuto::length(snap(raw.value(), scale))
        } else {
            length
        }
    };
    let mut out = style.clone();
    out.size.width = side(out.size.width);
    out.size.height = side(out.size.height);
    for size in [&mut out.min_size, &mut out.max_size] {
        size.width = place(size.width);
        size.height = place(size.height);
    }
    out.gap.width = space(out.gap.width);
    out.gap.height = space(out.gap.height);
    out.padding.left = space(out.padding.left);
    out.padding.right = space(out.padding.right);
    out.padding.top = space(out.padding.top);
    out.padding.bottom = space(out.padding.bottom);
    for edges in [&mut out.margin, &mut out.inset] {
        edges.left = place(edges.left);
        edges.right = place(edges.right);
        edges.top = place(edges.top);
        edges.bottom = place(edges.bottom);
    }
    out
}

fn tree(node: &Node, parent: Words, taffy: &mut TaffyTree<Context>) -> Laid {
    let words = parent.with(node.words);
    let style = snapped(&node.style, words.scale);
    let leaf = match &node.content {
        Content::Text { text, lines, .. } => Some((text, lines, None)),
        Content::Styled {
            text,
            styles,
            lines,
            ..
        } => Some((text, lines, Some(Rc::new(styles.clone())))),
        _ => None,
    };
    if let Some((text, lines, styles)) = leaf {
        let context = Context {
            text: text.clone(),
            words,
            lines: *lines,
            styles,
        };
        let id = taffy
            .new_leaf_with_context(style, context)
            .expect("taffy never fails to add a node");
        return Laid {
            id,
            words,
            children: Vec::new(),
        };
    }
    let children: Vec<Laid> = node
        .children
        .iter()
        .map(|child| tree(child, words, taffy))
        .collect();
    let ids: Vec<NodeId> = children.iter().map(|child| child.id).collect();
    let id = taffy
        .new_with_children(style, &ids)
        .expect("taffy never fails to add a node");
    Laid {
        id,
        words,
        children,
    }
}

/// What it measures, rounded up to whole pixels of the screen.
fn measured(
    context: &Context,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
    painter: &mut Painter,
    scale: f32,
) -> Size<f32> {
    let size = natural_size(context, known, available, painter, scale);
    Size {
        width: (size.width * scale).ceil() / scale,
        height: (size.height * scale).ceil() / scale,
    }
}

fn natural_size(
    context: &Context,
    known: Size<Option<f32>>,
    available: Size<AvailableSpace>,
    painter: &mut Painter,
    scale: f32,
) -> Size<f32> {
    let words = context.words;
    let style = (words.size, words.weight);
    let line = words.line();
    let room = known.width.or(match available.width {
        AvailableSpace::Definite(width) => Some(width),
        _ => None,
    });
    let fit = words.fit(room, context.lines);
    let (width, rows) = if let Some(styles) = &context.styles {
        let (text, styles) = short_styled(painter, &context.text, styles, style, fit, scale);
        let rich = painter.rich(&text, &styles, words.size, words.weight, fit.wrap(), scale);
        (rich.width, rich.rows.len())
    } else {
        let family = words.family();
        let short = painter.shortened(family, &context.text, style, fit, scale);
        let shown = short.as_deref().unwrap_or(&context.text);
        let rows = painter.row_count(family, shown, style, fit, scale);
        let widest = painter.widest(family, shown, style, scale);
        (fit.wrap().map_or(widest, |wrap| wrap.min(widest)), rows)
    };
    let rows = context.lines.map_or(rows, |most| rows.min(most));
    Size {
        width: known.width.unwrap_or(width.ceil()),
        height: known.height.unwrap_or(rows.max(1) as f32 * line),
    }
}

/// `text` and `styles` as they show once cut to `fit`.
fn short_styled<'a>(
    painter: &mut Painter,
    text: &'a str,
    styles: &'a [(Range<usize>, TextStyle)],
    (size, weight): (f32, u16),
    fit: Fit,
    scale: f32,
) -> (Cow<'a, str>, Cow<'a, [(Range<usize>, TextStyle)]>) {
    match painter.shortened(FONT, text, (size, weight), fit, scale) {
        Some(short) => {
            let styles = cut_styles(styles, short.len());
            (Cow::Owned(short), Cow::Owned(styles))
        }
        None => (Cow::Borrowed(text), Cow::Borrowed(styles)),
    }
}

/// What of `rect` shows through `clip`, which is left, top, right and bottom.
fn seen_part([x, y, w, h]: [f32; 4], clip: Option<[f32; 4]>) -> [f32; 4] {
    let [left, top, right, bottom] = clip.map_or([x, y, x + w, y + h], |clip| {
        overlap([x, y, x + w, y + h], clip)
    });
    [left, top, (right - left).max(0.0), (bottom - top).max(0.0)]
}

#[derive(Clone, Copy)]
enum RuleAt {
    Under,
    Through,
}

const UNDERLINE_DROP: f32 = 0.618;

/// The name of child `index` of the node named `path`.
fn branch(path: u64, index: usize) -> u64 {
    (path ^ (index as u64 + 1)).wrapping_mul(0x0100_0000_01b3)
}

fn open_address(address: &str) {
    let address = address.to_owned();
    thread::spawn(move || {
        let _ = Command::new("xdg-open").arg(address).status();
    });
}

/// Paints into a [`Built`], from its corner.
pub struct Painting<'a> {
    pub painter: &'a mut Painter,
    pub gpu: &'a Gpu,
    pub scale: f32,
    pub built: Built,
    pub offsets: &'a HashMap<SharedString, f32>,
    /// How wide what holds the node being painted is.
    /// Where what holds the node being painted is.
    pub holder_rect: [f32; 4],
    /// The hit whose words light up, and the color they rest in.
    pub lighting: Option<(usize, u32)>,
    /// What each place showed the last time.
    pub before: &'a HashMap<u64, Shown>,
    /// Names the node being painted by where it is in the tree.
    pub path: u64,
    /// Where text sits in what holds it, 0 at the start and 1 at the end.
    pub align: f32,
    pub paints: u64,
}

impl Painting<'_> {
    /// Lays `node` out within `room` and returns how big it came out; what it paints goes in the
    /// built panel.
    pub fn paint(&mut self, node: &Node, room: Option<f32>, origin: (f32, f32)) -> (f32, f32) {
        let mut taffy: TaffyTree<Context> = TaffyTree::new();
        taffy.disable_rounding();
        let laid = tree(node, Words::base(self.scale), &mut taffy);
        let (painter, scale) = (&mut *self.painter, self.scale);
        let space = Size {
            width: room.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite),
            height: AvailableSpace::MaxContent,
        };
        let done =
            taffy.compute_layout_with_measure(laid.id, space, |inputs, _, context, style| {
                compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known, available| match context.as_deref() {
                        Some(context) => measured(context, known, available, painter, scale),
                        None => Size::ZERO,
                    },
                )
            });
        if done.is_err() {
            return (0.0, 0.0);
        }
        let size = taffy
            .layout(laid.id)
            .map_or((0.0, 0.0), |layout| (layout.size.width, layout.size.height));
        self.paints += 1;
        self.path = branch(0, self.paints as usize);
        self.node(node, &laid, &taffy, origin, 1.0, None);
        size
    }

    /// How big `node` comes out, without painting it.
    pub fn measure(&mut self, node: &Node, room: Option<f32>) -> (f32, f32) {
        let mut taffy: TaffyTree<Context> = TaffyTree::new();
        taffy.disable_rounding();
        let laid = tree(node, Words::base(self.scale), &mut taffy);
        let (painter, scale) = (&mut *self.painter, self.scale);
        let space = Size {
            width: room.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite),
            height: AvailableSpace::MaxContent,
        };
        let done =
            taffy.compute_layout_with_measure(laid.id, space, |inputs, _, context, style| {
                compute_leaf_layout(
                    inputs,
                    style,
                    |_, _| 0.0,
                    |known, available| match context.as_deref() {
                        Some(context) => measured(context, known, available, painter, scale),
                        None => Size::ZERO,
                    },
                )
            });
        if done.is_err() {
            return (0.0, 0.0);
        }
        taffy
            .layout(laid.id)
            .map_or((0.0, 0.0), |layout| (layout.size.width, layout.size.height))
    }

    fn marquee(
        &mut self,
        text: &str,
        slot: Option<f32>,
        on_hover: bool,
        words: Words,
        rect: [f32; 4],
        clip: Option<[f32; 4]>,
        fade: f32,
    ) {
        let slot = slot.unwrap_or(rect[2]);
        let line = self
            .painter
            .line(text, words.size, words.weight, self.scale);
        let top = rect[1] + (rect[3] - words.size * 1.2) / 2.0;
        if line.width <= slot {
            let left = if on_hover {
                (rect[0] + (rect[2] - line.width) / 2.0).round()
            } else {
                rect[0]
            };
            self.painter.text(
                self.gpu,
                &line,
                (left, top),
                solid(words.color),
                self.scale,
                &mut self.built.sprites,
            );
            return;
        }
        let mut sprites = Vec::new();
        self.painter.text(
            self.gpu,
            &line,
            (rect[0], top),
            solid(words.color),
            self.scale,
            &mut sprites,
        );
        self.built.marquees.push(Marquee {
            key: self.path,
            hover: on_hover.then_some(self.holder_rect),
            rect: [rect[0], rect[1], slot, rect[3]],
            width: line.width,
            sprites,
            clip,
            fade,
        });
    }

    /// Notes what this place shows; if it changed, it turns into it, and nothing more is needed.
    fn morphed(
        &mut self,
        text: &SharedString,
        words: Words,
        rect: [f32; 4],
        clip: Option<[f32; 4]>,
        fade: f32,
    ) -> bool {
        let shown = noted(self.before.get(&self.path), text);
        let from = shown.from.clone();
        self.built.texts.insert(self.path, shown);
        let Some((old, since)) = from else {
            return false;
        };
        let line = self
            .painter
            .line_in(words.family(), text, words.size, words.weight, self.scale);
        if line.width > rect[2] + 0.5 {
            return false;
        }
        let top = rect[1] + (words.line() - words.size * 1.2) / 2.0;
        let (start, slots, end) = self.painter.morph_parts(
            self.gpu,
            words.family(),
            (&old, text),
            (words.size, words.weight),
            solid(words.color),
            top,
            self.scale,
        );
        let mut rest = Vec::new();
        self.painter.text(
            self.gpu,
            &line,
            (rect[0], top),
            solid(words.color),
            self.scale,
            &mut rest,
        );
        self.built.morphs.push(MorphText {
            rect,
            align: self.align,
            since,
            start,
            slots,
            end,
            rest,
            clip,
            fade,
        });
        true
    }

    fn styled(
        &mut self,
        text: &str,
        styles: &[(Range<usize>, TextStyle)],
        links: &[(Range<usize>, String)],
        most: Option<usize>,
        words: Words,
        room: f32,
        (rect, clip): ([f32; 4], Option<[f32; 4]>),
    ) {
        let fit = words.fit(Some(room), most);
        let (text, styles) = short_styled(
            self.painter,
            text,
            styles,
            (words.size, words.weight),
            fit,
            self.scale,
        );
        let (text, styles) = (text.as_ref(), styles.as_ref());
        let rich = self.painter.rich(
            text,
            styles,
            words.size,
            words.weight,
            fit.wrap(),
            self.scale,
        );
        let shown = most.map_or(rich.rows.len(), |most| most.min(rich.rows.len()));
        let line = words.line();
        for (index, row) in rich.rows[..shown].iter().enumerate() {
            let box_top = rect[1] + index as f32 * line;
            let top = box_top + (line - words.size * 1.2) / 2.0;
            for (from, wide, style) in &row.marks {
                let style = styles[*style].1;
                let x = rect[0] + from;
                if style.code || style.covered {
                    let ground = if style.covered {
                        solid(SURFACE1)
                    } else {
                        tinted(SURFACE1, CONTROL_ALPHA)
                    };
                    let left = (x * self.scale).round() / self.scale;
                    let right = ((x + wide) * self.scale).round() / self.scale;
                    self.built.sprites.push(Sprite::shape(
                        [left, box_top, right - left, line],
                        ground,
                        0.0,
                        2.0,
                    ));
                }
            }
            self.painter.rich_row(
                self.gpu,
                row,
                (rect[0], top),
                solid(words.color),
                self.scale,
                &mut self.built.sprites,
            );
            for (from, wide, style) in &row.marks {
                let index = *style;
                let style = styles[index].1;
                let x = rect[0] + from;
                let tint = solid(if style.link { ACCENT } else { words.color });
                if style.underline {
                    let at = self.rule_at(words, box_top, RuleAt::Under);
                    self.rule(x, at, *wide, tint);
                }
                if style.strike {
                    let at = self.rule_at(words, box_top, RuleAt::Through);
                    self.rule(x, at, *wide, tint);
                }
                let range = &styles[index].0;
                if let Some((_, address)) =
                    links.iter().find(|(link, _)| link.contains(&range.start))
                {
                    let address = address.clone();
                    self.built.hits.push(Hit {
                        rect: seen_part([x, box_top, *wide, line], clip),
                        on_click: Some(Rc::new(move |_| open_address(&address))),
                        ..Hit::default()
                    });
                }
            }
        }
    }

    /// Where a line under or through text is, down from a row that starts at `top`.
    fn rule_at(&mut self, words: Words, top: f32, at: RuleAt) -> f32 {
        let (ascent, descent) = self
            .painter
            .reach_of(words.family(), words.size, words.weight);
        let baseline = (words.line() - ascent - descent) / 2.0 + ascent;
        top + match at {
            RuleAt::Under => baseline + descent * UNDERLINE_DROP,
            RuleAt::Through => (ascent * 0.5 + baseline) * 0.5,
        }
    }

    /// A line of one device pixel or more, on whole pixels.
    fn rule(&mut self, x: f32, y: f32, width: f32, tint: [f32; 4]) {
        let scale = self.scale;
        let wide = (width * scale).round().max(1.0) / scale;
        let thick = scale.round().max(1.0) / scale;
        self.built.sprites.push(Sprite::shape(
            [snap(x, scale), snap(y, scale), wide, thick],
            tint,
            0.0,
            2.0,
        ));
    }

    fn words(&mut self, text: &str, words: Words, lines: Option<usize>, room: f32, rect: [f32; 4]) {
        let tint = solid(words.color);
        let top = |row: usize| {
            rect[1] + row as f32 * words.line() + (words.line() - words.size * 1.2) / 2.0
        };
        let rows = self.painter.fitted(
            words.family(),
            text,
            (words.size, words.weight),
            words.fit(Some(room), lines),
            self.scale,
        );
        let first = self.built.sprites.len();
        for (row, line) in rows.iter().enumerate() {
            self.painter.text(
                self.gpu,
                line,
                (rect[0], top(row)),
                tint,
                self.scale,
                &mut self.built.sprites,
            );
            if words.strike {
                let box_top = rect[1] + row as f32 * words.line();
                let at = self.rule_at(words, box_top, RuleAt::Through);
                self.rule(rect[0], at, line.width, tint);
            }
        }
        if let Some((hit, rest)) = self.lighting
            && rest == words.color
            && let Some((_, sprites)) = &mut self.built.hits[hit].lit
        {
            sprites.push((first, self.built.sprites.len()));
        }
    }

    fn node(
        &mut self,
        node: &Node,
        laid: &Laid,
        taffy: &TaffyTree<Context>,
        (ox, oy): (f32, f32),
        fade: f32,
        clip: Option<[f32; 4]>,
    ) {
        let Ok(layout) = taffy.layout(laid.id) else {
            return;
        };
        if node.hidden {
            return;
        }
        if let Some(name) = node.name {
            self.path = name;
        }
        let (raw_x, raw_y) = (ox + layout.location.x, oy + layout.location.y);
        let (x, y) = (snap(raw_x, self.scale), snap(raw_y, self.scale));
        let w = snap(raw_x + layout.size.width, self.scale) - x;
        let h = snap(raw_y + layout.size.height, self.scale) - y;
        let rect = [x, y, w, h];
        let fade = fade * node.opacity;
        let first = self.built.sprites.len();
        let eases_before = self.built.eases.len();
        if let Some(radius) = node.shadow {
            self.built.sprites.push(Sprite::shadow(
                [
                    x - SHADOW_SPREAD,
                    y - SHADOW_SPREAD,
                    w + 2.0 * SHADOW_SPREAD,
                    h + 2.0 * SHADOW_SPREAD,
                ],
                [0.0, 0.0, 0.0, SHADOW_ALPHA],
                radius,
                SHADOW_SPREAD,
                0.0,
            ));
        }
        if let Some((tint, radius)) = node.fill {
            self.built
                .sprites
                .push(Sprite::shape(rect, tint, radius, 2.0));
        }
        if let Some((tint, radius)) = node.ring {
            self.built
                .sprites
                .push(Sprite::ring(rect, tint, radius, 2.0));
        }
        if let Some(tint) = node.top_line {
            self.built
                .sprites
                .push(Sprite::shape([x, y, w, 1.0], tint, 0.0, 2.0));
        }
        let hover = node.answers.hover.map(|radius| {
            self.built
                .sprites
                .push(Sprite::shape(rect, [0.0; 4], radius, 2.0));
            self.built.sprites.len() - 1
        });
        match &node.content {
            Content::None => {}
            Content::Text { text, lines, moves } => {
                if !*moves || !self.morphed(text, laid.words, rect, clip, fade) {
                    self.words(text, laid.words, *lines, layout.size.width, rect);
                }
            }
            Content::Glyph(path, tint) => self.painter.glyph_icon(
                self.gpu,
                path,
                rect,
                solid(*tint),
                self.scale,
                &mut self.built.sprites,
            ),
            Content::Picture(picture, framing) => {
                let start = self.built.sprites.len();
                self.painter.picture(
                    self.gpu,
                    picture,
                    *framing,
                    rect,
                    self.scale,
                    &mut self.built.sprites,
                );
                for sprite in &mut self.built.sprites[start..] {
                    *sprite = sprite.rounded(node.rounding);
                }
            }
            Content::Drawing(key, drawing) => self.painter.drawing(
                self.gpu,
                key,
                drawing,
                (x, y),
                self.scale,
                &mut self.built.sprites,
            ),
            Content::Chart(series, phase) => self.built.charts.push(Chart {
                rect,
                series: series.clone(),
                phase: *phase,
                clip,
                fade,
            }),
            Content::Countdown(countdown) => {
                let mut countdown = countdown.as_ref().clone();
                countdown.rect = rect;
                countdown.opacity = fade;
                self.built.countdowns.push(countdown);
            }
            Content::Sprites(own) => self
                .built
                .sprites
                .extend(own.iter().map(|sprite| sprite.moved(x, y))),
            Content::Marquee {
                text,
                slot,
                on_hover,
            } => self.marquee(text, *slot, *on_hover, laid.words, rect, clip, fade),
            Content::Caret => self.built.carets.push((rect, clip)),
            Content::Pulse(kind, tint, radius) => self.built.pulses.push(Pulse {
                rect,
                tint: *tint,
                radius: *radius,
                kind: *kind,
                clip,
                fade,
            }),
            Content::Stripes(tint) => {
                let own = [x, y, x + w, y + h];
                let mut at = x - h;
                while at < x + w {
                    self.built
                        .sprites
                        .push(Sprite::line((at, y + h), (at + h, y), STRIPE_W, *tint).within(own));
                    at += STRIPE_EVERY;
                }
            }
            Content::Gridlines(count, tint) if *count >= 2 => {
                let segments = (*count - 1) as f32;
                for index in 0..*count {
                    let at = x + w * (index as f32 / segments);
                    let mut down = y;
                    while down < y + h {
                        let end = (down + DASH).min(y + h);
                        self.built.sprites.push(Sprite::shape(
                            [at - 0.5, down, 1.0, end - down],
                            *tint,
                            0.0,
                            2.0,
                        ));
                        down += DASH_EVERY;
                    }
                }
            }
            Content::Gridlines(..) => {}
            Content::Styled {
                text,
                styles,
                links,
                lines,
            } => self.styled(
                text,
                styles,
                links,
                *lines,
                laid.words,
                layout.size.width,
                (rect, clip),
            ),
        }
        let answers = &node.answers;
        if let Some(key) = &answers.anchor {
            self.built.anchors.insert(key.clone(), rect);
        }
        let mut hit_at = None;
        if answers.on_click.is_some()
            || answers.on_right.is_some()
            || answers.hover.is_some()
            || answers.tip.is_some()
            || answers.on_hover.is_some()
            || answers.on_rows.is_some()
            || answers.lit.is_some()
            || answers.blocks
        {
            hit_at = Some(self.built.hits.len());
            self.built.hits.push(Hit {
                rect: seen_part(rect, clip),
                hover,
                on_click: answers.on_click.clone(),
                on_right: answers.on_right.clone(),
                on_hover: answers.on_hover.clone(),
                on_rows: answers.on_rows.clone(),
                tip: answers.tip.clone(),
                immediate: answers.immediate,
                blocks: answers.blocks,
                from: first,
                to: first,
                lit: answers.lit.map(|tint| (solid(tint), Vec::new())),
            });
        }
        let own_clip = node.clip.then(|| {
            let mine = [x, y, x + w, y + h];
            clip.map_or(mine, |outer| overlap(mine, outer))
        });
        let inner = own_clip.or(clip);
        let mut offset = 0.0;
        let mut reached = None;
        let mut scrolled = None;
        if let Some(key) = &node.scroll {
            let bottom = laid
                .children
                .iter()
                .filter_map(|child| taffy.layout(child.id).ok())
                .map(|child| child.location.y + child.size.height)
                .fold(0.0, f32::max);
            let room = (bottom - h).max(0.0);
            let reach = room.min(node.scroll_reach).max(0.0);
            offset = self.offsets.get(key).copied().unwrap_or(0.0).min(reach);
            reached = Some(room);
            scrolled = Some(self.built.scrolling.len());
            self.built.scrolling.push(Scrolling {
                key: key.clone(),
                rect,
                reach,
                from: self.built.sprites.len(),
                to: 0,
            });
        }
        let lighting = self.lighting;
        if answers.lit.is_some() {
            self.lighting = hit_at.map(|hit| (hit, laid.words.color));
        }
        let holder_rect = std::mem::replace(&mut self.holder_rect, rect);
        let align = self.align;
        let path = self.path;
        if node.style.flex_direction == FlexDirection::Column {
            if node.style.align_items == AlignItems::CENTER {
                self.align = 0.5;
            }
        } else {
            let justify = node.style.justify_content;
            self.align = if justify == JustifyContent::FLEX_END {
                1.0
            } else if justify == JustifyContent::CENTER {
                0.5
            } else {
                0.0
            };
        }
        let inside = self.built.sprites.len();
        for (index, (child, laid)) in node.children.iter().zip(&laid.children).enumerate() {
            self.path = branch(path, index);
            self.node(child, laid, taffy, (raw_x, raw_y - offset), fade, inner);
        }
        self.path = path;
        self.align = align;
        self.lighting = lighting;
        self.holder_rect = holder_rect;
        if let Some(at) = scrolled {
            self.built.scrolling[at].to = self.built.sprites.len();
        }
        if let Some(reach) = reached.filter(|_| node.soft) {
            let reach = reach.min(node.soft_reach).max(0.0);
            let above = offset.min(reach);
            let shown = |past: f32| (past / EDGE_RAMP).clamp(0.0, 1.0);
            let edges = [y, shown(above), y + h, shown(reach - above)];
            if edges[1] > 0.0 || edges[3] > 0.0 {
                for sprite in &mut self.built.sprites[inside..] {
                    *sprite = sprite.fading(edges);
                }
            }
        }
        let count = self.built.sprites.len();
        if let Some(group) = &node.answers.shown_in {
            self.built.reveals.push(Reveal {
                group: group.clone(),
                from: first,
                to: count,
            });
        }
        if node.answers.pulsing {
            self.built.pulsing.push((first, count));
        }
        if let Some(at) = hit_at {
            self.built.hits[at].to = count;
        }
        if let Some(key) = &answers.anchor {
            self.built.anchored.insert(key.clone(), (first, count));
        }
        for (index, (easing, value, config)) in node.eases.iter().enumerate() {
            let easing = match *easing {
                Easing::Wide { .. } => Easing::Wide {
                    whole: self.holder_rect[2],
                },
                Easing::Along => Easing::Shift {
                    x: self.holder_rect[2],
                    y: 0.0,
                },
                Easing::Stretch { .. } => Easing::Stretch { left: x },
                easing => easing,
            };
            self.built.eases.push(Ease {
                key: branch(path, EASES + index),
                from: first,
                to: count,
                easing,
                value: *value,
                config: *config,
                follows: None,
            });
        }
        if let Some(on_change) = &answers.on_change {
            let eases = &mut self.built.eases[eases_before..];
            let fill = eases
                .iter()
                .find(|ease| matches!(ease.easing, Easing::Wide { .. }))
                .map(|ease| (ease.key, ease.value));
            for ease in eases {
                ease.follows = Some(rect);
            }
            self.built.sliding.push(Sliding {
                key: path,
                rect,
                seen: seen_part(rect, clip),
                on_change: on_change.clone(),
                fill,
                from: first,
                to: count,
            });
        }
        if let Some(busy) = node.busy {
            self.built.beams.push((path, busy, rect));
        }
        if let Some(leaving) = node.rises {
            self.built.rises.push(Rise {
                key: path,
                from: first,
                to: count,
                leaving,
            });
        }
        for sprite in &mut self.built.sprites[first..count] {
            if node.opacity < 1.0 {
                *sprite = sprite.faded(node.opacity);
            }
            if let Some(clip) = own_clip {
                *sprite = sprite.within(clip);
            }
        }
    }
}

#[cfg(test)]
#[path = "node_test.rs"]
mod tests;
