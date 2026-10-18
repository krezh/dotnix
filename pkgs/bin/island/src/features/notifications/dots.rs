//! The bar's missed-notifications indicator: one full-size dot, or a dot for each notification
//! missed, in a 2×2 grid and then a 3×3 one of smaller dots, each moving into its place as the
//! count changes. Every dot lands on whole pixels, so they are all drawn alike.

use std::time::Instant;

use gpui::{Div, div, prelude::*, px, rgb};

use crate::{island::SHAPE_SPRING, spring::Spring};

/// The square the dots share.
pub const BOX: f32 = 16.0;
const CENTRE: f32 = BOX / 2.0;
/// One on its own is full size; they only get smaller as more join it.
const ALONE: f32 = 10.0;
/// 2×2: 6 px dots 4 apart.
const PAIR: f32 = 6.0;
const PAIR_STEP: f32 = 10.0;
/// 3×3: 4 px dots 2 apart.
const TRIO: f32 = 4.0;
const TRIO_STEP: f32 = 6.0;
pub const MOST: usize = 9;

/// Left, top and size of each dot for `count`, in whole pixels.
pub fn layout(count: usize) -> Vec<(f32, f32, f32)> {
    let two = |column: f32, row: f32| (column * PAIR_STEP, row * PAIR_STEP, PAIR);
    let three = |column: f32, row: f32| (column * TRIO_STEP, row * TRIO_STEP, TRIO);
    let middle = (BOX - PAIR) / 2.0;
    match count {
        0 | 1 => vec![(CENTRE - ALONE / 2.0, CENTRE - ALONE / 2.0, ALONE)],
        2 => vec![(0.0, middle, PAIR), (PAIR_STEP, middle, PAIR)],
        3 => vec![two(0.0, 0.0), two(1.0, 0.0), (middle, PAIR_STEP, PAIR)],
        4 => vec![two(0.0, 0.0), two(1.0, 0.0), two(0.0, 1.0), two(1.0, 1.0)],
        5 => vec![
            three(0.0, 0.0),
            three(2.0, 0.0),
            three(1.0, 1.0),
            three(0.0, 2.0),
            three(2.0, 2.0),
        ],
        6 => vec![
            three(0.0, 0.0),
            three(2.0, 0.0),
            three(0.0, 1.0),
            three(2.0, 1.0),
            three(0.0, 2.0),
            three(2.0, 2.0),
        ],
        7 => vec![
            three(0.0, 0.0),
            three(2.0, 0.0),
            three(0.0, 1.0),
            three(1.0, 1.0),
            three(2.0, 1.0),
            three(0.0, 2.0),
            three(2.0, 2.0),
        ],
        8 => vec![
            three(0.0, 0.0),
            three(1.0, 0.0),
            three(2.0, 0.0),
            three(0.0, 1.0),
            three(2.0, 1.0),
            three(0.0, 2.0),
            three(1.0, 2.0),
            three(2.0, 2.0),
        ],
        _ => (0..3)
            .flat_map(|row| (0..3).map(move |column| three(column as f32, row as f32)))
            .collect(),
    }
}

/// A dot's centre and size.
struct Slot {
    x: Spring,
    y: Spring,
    size: Spring,
}

impl Slot {
    fn new() -> Self {
        Self {
            x: Spring::new(CENTRE, SHAPE_SPRING, 0.01),
            y: Spring::new(CENTRE, SHAPE_SPRING, 0.01),
            size: Spring::new(0.0, SHAPE_SPRING, 0.01),
        }
    }
}

pub struct Dots {
    slots: Vec<Slot>,
    count: Option<usize>,
}

impl Default for Dots {
    fn default() -> Self {
        Self {
            slots: (0..MOST).map(|_| Slot::new()).collect(),
            count: None,
        }
    }
}

impl Dots {
    /// Moves the dots towards `count`'s layout; the first time, or without `animate`, they are
    /// simply there.
    pub fn show(&mut self, count: usize, animate: bool) {
        let count = count.min(MOST);
        if self.count == Some(count) {
            return;
        }
        let jump = !animate || self.count.is_none();
        self.count = Some(count);
        let spots = layout(count);
        for (index, slot) in self.slots.iter_mut().enumerate() {
            let (x, y, size) = spots
                .get(index)
                .map_or((CENTRE, CENTRE, 0.0), |&(left, top, size)| {
                    (left + size / 2.0, top + size / 2.0, size)
                });
            if jump {
                slot.x.reset(x);
                slot.y.reset(y);
                slot.size.reset(size);
                continue;
            }
            // A dot that was gone grows from the middle.
            if slot.size.value() < 0.2 {
                slot.x.reset(CENTRE);
                slot.y.reset(CENTRE);
            }
            slot.x.retarget(x, SHAPE_SPRING);
            slot.y.retarget(y, SHAPE_SPRING);
            slot.size.retarget(size, SHAPE_SPRING);
        }
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for slot in &mut self.slots {
            slot.x.advance(dt, now);
            slot.y.advance(dt, now);
            slot.size.advance(dt, now);
        }
    }

    pub fn is_settled(&self) -> bool {
        self.slots
            .iter()
            .all(|slot| slot.x.is_settled() && slot.y.is_settled() && slot.size.is_settled())
    }

    pub fn element(&self, color: u32) -> Div {
        div()
            .relative()
            .size(px(BOX))
            .flex_none()
            .children(self.slots.iter().filter_map(|slot| {
                let size = slot.size.value().max(0.0).round();
                (size >= 1.0).then(|| {
                    div()
                        .absolute()
                        .left(px((slot.x.value() - size / 2.0).round()))
                        .top(px((slot.y.value() - size / 2.0).round()))
                        .size(px(size))
                        .rounded_full()
                        .bg(rgb(color))
                })
            }))
    }
}

#[cfg(test)]
#[path = "dots_test.rs"]
mod tests;
