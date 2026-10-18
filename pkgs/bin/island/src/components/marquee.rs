//! Text that scrolls when it does not fit, and the text measuring it relies on.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::Range,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    Div, FontFeatures, FontWeight, HighlightStyle, SharedString, StyledText, TextRun, Window, div, font, prelude::*,
    px, rgb,
};

use crate::theme::{FONT, TEXT};

/// Between the end of one pass and the start of the next.
const GAP: f32 = 40.0;
/// Px per second.
const SPEED: f32 = 28.0;
/// At each edge of the slot.
const FADE: f32 = 18.0;
/// Fade levels; glyphs sharing a level share a text run.
const LEVELS: f32 = 24.0;
const CACHE_LIMIT: usize = 256;

type Key = (SharedString, u32, u32);

pub struct TextLayout {
    pub width: f32,
    glyphs: Vec<(Range<usize>, f32, f32)>,
}

struct Layouts {
    known: HashMap<Key, Rc<TextLayout>>,
    pending: HashSet<Key>,
    epoch: Instant,
    scrolling: bool,
}

impl Default for Layouts {
    fn default() -> Self {
        Self {
            known: HashMap::new(),
            pending: HashSet::new(),
            epoch: Instant::now(),
            scrolling: false,
        }
    }
}

/// Ambient motion does not need the display's full refresh rate (often 144–240 Hz), and each frame
/// rebuilds the whole island.
pub const TICK: Duration = Duration::from_micros(33_333);

/// Lookups made during render are shaped at the end of the frame by [`TextLayouts::resolve`], so
/// elements built without a `Window` can still lay themselves out by text.
#[derive(Clone, Default)]
pub struct TextLayouts(Rc<RefCell<Layouts>>);

impl TextLayouts {
    pub fn begin_frame(&self) {
        self.0.borrow_mut().scrolling = false;
    }

    pub fn scrolling(&self) -> bool {
        self.0.borrow().scrolling
    }

    /// The shared clock, in seconds.
    fn mark_scrolling(&self) -> f32 {
        let mut layouts = self.0.borrow_mut();
        layouts.scrolling = true;
        layouts.epoch.elapsed().as_secs_f32()
    }

    pub fn get(&self, text: &SharedString, size: f32, weight: FontWeight) -> Option<Rc<TextLayout>> {
        let key = (text.clone(), size.to_bits(), weight.0.to_bits());
        let mut layouts = self.0.borrow_mut();
        let layout = layouts.known.get(&key).cloned();
        if layout.is_none() {
            layouts.pending.insert(key);
        }
        layout
    }

    pub fn resolve(&self, window: &mut Window) -> bool {
        let mut layouts = self.0.borrow_mut();
        if layouts.pending.is_empty() {
            return false;
        }
        if layouts.known.len() > CACHE_LIMIT {
            layouts.known.clear();
        }
        for key in std::mem::take(&mut layouts.pending) {
            let layout = shape(window, &key.0, f32::from_bits(key.1), FontWeight(f32::from_bits(key.2)));
            layouts.known.insert(key, Rc::new(layout));
        }
        window.refresh();
        true
    }
}

pub fn shape(window: &Window, text: &str, size: f32, weight: FontWeight) -> TextLayout {
    let mut face = font(FONT);
    face.weight = weight;
    face.features = FontFeatures(Arc::new(vec![("tnum".to_owned(), 1)]));
    let run = TextRun {
        len: text.len(),
        font: face,
        color: rgb(TEXT).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window.text_system().shape_line(text.to_owned().into(), px(size), &[run], None);
    let x = |index: usize| f32::from(line.x_for_index(index));
    let glyphs = text
        .char_indices()
        .map(|(start, ch)| {
            let end = start + ch.len_utf8();
            (start..end, x(start), x(end))
        })
        .collect();
    TextLayout {
        width: f32::from(line.width).ceil(),
        glyphs,
    }
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The left edge fades by `left`, 0 to 1: still text starts at that edge, so it eases in as the
/// text starts to move and out as it comes to rest.
fn edge_fades(layout: &TextLayout, origin: f32, slot: f32, left: f32) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut runs: Vec<(Range<usize>, f32)> = Vec::new();
    for (range, start, end) in &layout.glyphs {
        let center = origin + (start + end) / 2.0;
        let right_side = smoothstep((slot - center) / FADE);
        let left_side = 1.0 - left * (1.0 - smoothstep(center / FADE));
        let visibility = right_side.min(left_side);
        let fade = ((1.0 - visibility) * LEVELS).round() / LEVELS;
        match runs.last_mut() {
            Some((last, last_fade)) if *last_fade == fade && last.end == range.start => last.end = range.end,
            _ => runs.push((range.clone(), fade)),
        }
    }
    runs.into_iter()
        .filter(|(_, fade)| *fade > 0.0)
        .map(|(range, fade)| {
            (
                range,
                HighlightStyle {
                    fade_out: Some(fade),
                    ..HighlightStyle::default()
                },
            )
        })
        .collect()
}

/// Text that does not fit glides in a seamless loop and dissolves into both edges instead of being
/// clipped.
pub fn marquee(layouts: &TextLayouts, text: SharedString, size: f32, weight: FontWeight, slot: f32) -> Div {
    let line_h = (size * 1.4).ceil();
    let area = div().min_w(px(0.0)).h(px(line_h)).text_size(px(size)).font_weight(weight);
    let Some(layout) = layouts.get(&text, size, weight).filter(|layout| layout.width > slot) else {
        return area.flex().items_center().child(div().min_w(px(0.0)).truncate().child(text));
    };
    // Position follows the clock rather than a per-frame animation, so motion is equally smooth at
    // any frame rate; the owner keeps frames coming at `TICK` while `scrolling()` reports true.
    let offset = (layouts.mark_scrolling() * SPEED) % (layout.width + GAP);
    passes(area, &text, &layout, slot, offset, 1.0)
}

thread_local! {
    /// When each text hovered right now started to glide.
    static GLIDES: RefCell<HashMap<SharedString, Instant>> = RefCell::default();
}

/// As [`marquee`], but text that does not fit glides only while `hovered`, from where it rests,
/// and is back at rest the moment it is not. At rest it dissolves into its right edge.
pub fn marquee_on_hover(
    layouts: &TextLayouts,
    id: &SharedString,
    text: SharedString,
    size: f32,
    weight: FontWeight,
    slot: f32,
    hovered: bool,
) -> Div {
    let line_h = (size * 1.4).ceil();
    let area = div().min_w(px(0.0)).h(px(line_h)).text_size(px(size)).font_weight(weight);
    let layout = layouts.get(&text, size, weight).filter(|layout| layout.width > slot);
    let since = GLIDES.with_borrow_mut(|glides| {
        if hovered && layout.is_some() {
            Some(*glides.entry(id.clone()).or_insert_with(Instant::now))
        } else {
            glides.remove(id);
            None
        }
    });
    let Some(layout) = layout else {
        return area.flex().items_center().child(div().min_w(px(0.0)).truncate().child(text));
    };
    let area = area.w(px(slot));
    let Some(since) = since else {
        return passes(area, &text, &layout, slot, 0.0, 0.0);
    };
    layouts.mark_scrolling();
    let travelled = since.elapsed().as_secs_f32() * SPEED;
    // The left edge starts unfaded, as at rest, and fades in as the text moves off it.
    let edge = (travelled / FADE).clamp(0.0, 1.0);
    passes(area, &text, &layout, slot, travelled % (layout.width + GAP), edge)
}

/// Two passes of the text, `offset` along the loop, so one follows straight on from the other.
fn passes(area: Div, text: &SharedString, layout: &TextLayout, slot: f32, offset: f32, left: f32) -> Div {
    let cycle = layout.width + GAP;
    area.relative().overflow_hidden().child(div().absolute().top_0().left_0().size_full().children((0..2).map(
        |pass| {
            let origin = pass as f32 * cycle - offset;
            div()
                .absolute()
                .top_0()
                .left(px(origin))
                .h_full()
                .flex()
                .items_center()
                .whitespace_nowrap()
                .child(StyledText::new(text.clone()).with_highlights(edge_fades(layout, origin, slot, left)))
        },
    )))
}
