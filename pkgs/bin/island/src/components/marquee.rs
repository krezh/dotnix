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

fn edge_fades(layout: &TextLayout, origin: f32, slot: f32) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut runs: Vec<(Range<usize>, f32)> = Vec::new();
    for (range, left, right) in &layout.glyphs {
        let center = origin + (left + right) / 2.0;
        let visibility = smoothstep(center.min(slot - center) / FADE);
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
    let cycle = layout.width + GAP;
    // Position follows the clock rather than a per-frame animation, so motion is equally smooth at
    // any frame rate; the owner keeps frames coming at `TICK` while `scrolling()` reports true.
    let offset = (layouts.mark_scrolling() * SPEED) % cycle;
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
                .child(StyledText::new(text.clone()).with_highlights(edge_fades(&layout, origin, slot)))
        },
    )))
}
