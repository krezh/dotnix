//! The island's shape and the fades and slides of what is in it, whatever draws them.

use std::time::Instant;

use super::{COLLAPSED_H, COLLAPSED_LAYER, CONTENT_IN, CONTENT_OUT, LAYERS, SHAPE_SPRING};
use crate::spring::Spring;

/// Between two panels as they pass: wide enough that the leaving one is past the island's edge
/// before the spring slows at the end, or its edge lingers.
const SLIDE_GAP: f32 = 72.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub w: f32,
    pub h: f32,
}

pub struct Morph {
    pub w: Spring,
    pub h: Spring,
    pub layers: [Spring; LAYERS],
    /// How far each panel is to the side as it passes, in lengths of `slide_width`; 0 is in place.
    pub slides: [Spring; LAYERS],
    /// Far enough for the wider of two passing panels to clear the other.
    pub slide_width: f32,
    pub chrome: Spring,
    /// Each panel's measured shape, with the key of what it was measured showing.
    pub frozen: [Option<(u64, Shape)>; LAYERS],
    /// Has taken a shape, so changes from here on animate.
    pub sized: bool,
}

fn fade_in(spring: &mut Spring) {
    if spring.target() != 1.0 {
        spring.retarget(1.0, CONTENT_IN);
    }
}

impl Morph {
    pub fn new() -> Self {
        let shape_spring = |value| Spring::new(value, SHAPE_SPRING, 0.1);
        let mut layers = [Spring::new(0.0, CONTENT_OUT, 0.002); LAYERS];
        layers[COLLAPSED_LAYER] = Spring::new(1.0, CONTENT_IN, 0.002);
        Self {
            w: shape_spring(0.0),
            h: shape_spring(COLLAPSED_H),
            layers,
            slides: [Spring::new(0.0, SHAPE_SPRING, 0.001); LAYERS],
            slide_width: 0.0,
            chrome: Spring::new(0.0, CONTENT_OUT, 0.002),
            frozen: [None; LAYERS],
            sized: false,
        }
    }

    /// Moves towards `shape`, or takes it at once the first time; true if that changed anything.
    pub fn fit(&mut self, shape: Option<Shape>) -> bool {
        let Some(shape) = shape else {
            return false;
        };
        if !self.sized {
            self.w.reset(shape.w);
            self.h.reset(shape.h);
            self.sized = true;
            return true;
        }
        if (self.w.target() - shape.w).abs() < 0.5 && (self.h.target() - shape.h).abs() < 0.5 {
            return false;
        }
        self.w.retarget(shape.w, SHAPE_SPRING);
        self.h.retarget(shape.h, SHAPE_SPRING);
        true
    }

    /// Fades `active` in and every other layer out.
    pub fn fade(&mut self, active: Option<usize>) {
        for (index, layer) in self.layers.iter_mut().enumerate() {
            if Some(index) == active {
                fade_in(layer);
            } else {
                layer.retarget(0.0, CONTENT_OUT);
            }
        }
    }

    pub fn show_chrome(&mut self) {
        fade_in(&mut self.chrome);
    }

    pub fn hide_chrome(&mut self) {
        self.chrome.retarget(0.0, CONTENT_OUT);
    }

    /// Moves one panel out sideways as another comes in from the other side, both whole, in the
    /// order their tabs sit. Each carries on from wherever it is, if it was already moving.
    pub fn slide(&mut self, out: usize, into: usize, rightwards: bool) {
        let from_side = if rightwards { 1.0 } else { -1.0 };
        let width = |index: usize| self.frozen[index].map_or(0.0, |(_, shape)| shape.w);
        self.slide_width = width(out).max(width(into)).max(self.w.target()) + SLIDE_GAP;
        self.slides[out].retarget(-from_side, SHAPE_SPRING);
        if self.slides[into].is_settled() {
            self.slides[into].reset(from_side);
        }
        self.slides[into].retarget(0.0, SHAPE_SPRING);
        self.layers[into].reset(1.0);
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.w.advance(dt, now);
        self.h.advance(dt, now);
        self.chrome.advance(dt, now);
        for layer in self.layers.iter_mut().chain(&mut self.slides) {
            layer.advance(dt, now);
        }
        // Gone to the side, it comes back to where it opens next.
        for slide in &mut self.slides {
            if slide.is_settled() {
                slide.reset(0.0);
            }
        }
    }

    pub fn is_settled(&self) -> bool {
        self.w.is_settled()
            && self.h.is_settled()
            && self.layers.iter().all(Spring::is_settled)
            && self.slides.iter().all(Spring::is_settled)
            && self.chrome.is_settled()
    }
}
