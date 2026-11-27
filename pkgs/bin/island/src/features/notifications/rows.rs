//! The motion a notification list's rows share: one that goes slides out and the rows below rise
//! into its room, new ones drift in, and a swap to another set slides or fades each row.

use std::{
    ops::Deref,
    time::{Duration, Instant},
};

use super::card;
use crate::{
    spring::Spring,
    stage::{CONTENT_IN, CONTENT_OUT, SHAPE_SPRING},
};

pub const ENTER_DRIFT: f32 = 12.0;
pub const LEAVE_STAGGER: Duration = Duration::from_millis(45);
/// The most that leave one after another; any more leave with the last of them.
pub const STAGGERED: u32 = 8;

/// How far a leaving row has slid out, 0 to 1, as its fold goes from 1 to 0: it is gone by
/// halfway, and the room it took closes after.
pub fn sliding(fold: f32) -> f32 {
    (1.0 - (fold - 0.5) * 2.0).clamp(0.0, 1.0)
}

/// What one list adds to the shared motion.
pub trait Kind: Sized {
    type Body: Clone;
    type Key: PartialEq;
    /// What a placement says beyond where it goes.
    type Place;
    /// What a row keeps beyond the shared springs.
    type Extra;

    /// How tall the list shows before it scrolls.
    const MAX_VISIBLE_H: f32;

    fn key(body: &Self::Body) -> Self::Key;

    /// How opaque a row placed so ends up.
    fn alpha(_: &Self::Place) -> f32 {
        1.0
    }

    fn enter(place: &Self::Place, animate: bool, reveal_at: Instant) -> Self::Extra;

    /// A row starting to leave, before it gives its room back.
    fn leave(_: &mut Row<Self>) {}

    /// A row placed again where `place` says; `left` is how many began leaving in this layout.
    fn kept(_: &mut Row<Self>, _: &Place<Self>, _: Kept) {}

    fn assign(row: &mut Row<Self>, place: Self::Place);

    fn advance(_: &mut Self::Extra, _: f32, _: Instant) {}

    fn settled(_: &Self::Extra) -> bool {
        true
    }
}

#[derive(Clone, Copy)]
pub struct Kept {
    pub animate: bool,
    pub now: Instant,
    pub left: u32,
}

pub struct Row<K: Kind> {
    pub body: K::Body,
    pub y: Spring,
    /// 1 while it stays; as it leaves, it slides out by halfway to 0 and the room it took closes.
    pub fold: Spring,
    pub alpha: Spring,
    pub leaving: bool,
    pub extra: K::Extra,
    height: f32,
    /// What it takes from those below it.
    pub room: f32,
}

impl<K: Kind> Deref for Row<K> {
    type Target = K::Extra;

    fn deref(&self) -> &K::Extra {
        &self.extra
    }
}

impl<K: Kind> Row<K> {
    pub fn height(&self) -> f32 {
        self.height
    }

    /// How far it has slid out, 0 to 1.
    pub fn out(&self) -> f32 {
        if self.leaving {
            sliding(self.fold.value())
        } else {
            0.0
        }
    }

    /// The room a leaving row still takes from those below it: all of it while it slides out,
    /// then closing.
    fn folding(&self) -> f32 {
        if self.leaving {
            self.room * (self.fold.value() * 2.0).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Where a row goes, as a list works it out.
pub struct Place<K: Kind> {
    pub body: K::Body,
    pub y: f32,
    pub height: f32,
    pub room: f32,
    pub extra: K::Place,
}

/// Kept in the order they show, leaving ones where they were.
pub struct Rows<K: Kind> {
    rows: Vec<Row<K>>,
    height: f32,
    /// Laying out another set in place of this one, as a swap does.
    pub swapping: bool,
}

impl<K: Kind> Default for Rows<K> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            height: 0.0,
            swapping: false,
        }
    }
}

impl<K: Kind> Rows<K> {
    pub fn rows(&self) -> &[Row<K>] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|row| row.leaving)
    }

    pub fn visible_height(&self) -> f32 {
        self.full_height().min(K::MAX_VISIBLE_H)
    }

    /// How tall it all is where the rows are going, with the room those leaving still take.
    pub fn full_height(&self) -> f32 {
        self.height + self.rows.iter().map(Row::folding).sum::<f32>()
    }

    /// Where each row is drawn, in the order of [`Self::rows`]: below every row folding away
    /// above it by what that one still takes, so they follow its edge up.
    pub fn drawn_y(&self) -> Vec<f32> {
        let mut above = 0.0;
        self.rows
            .iter()
            .map(|row| {
                let y = row.y.value() + above;
                above += row.folding();
                y
            })
            .collect()
    }

    pub fn extent(&self) -> f32 {
        self.rows
            .iter()
            .zip(self.drawn_y())
            .map(|(row, y)| y.max(row.y.target()) + row.height())
            .fold(self.height, f32::max)
    }

    /// Places the rows where `placed` says, `height` tall in all.
    pub fn layout(
        &mut self,
        placed: Vec<Place<K>>,
        height: f32,
        animate: bool,
        reveal_at: Instant,
    ) {
        let (motion, fade_in) = (SHAPE_SPRING, CONTENT_IN);
        let now = Instant::now();
        let swapping = self.swapping;
        let mut left = 0;
        for index in 0..self.rows.len() {
            let row = &mut self.rows[index];
            let key = K::key(&row.body);
            if row.leaving || placed.iter().any(|place| K::key(&place.body) == key) {
                continue;
            }
            row.leaving = true;
            if swapping {
                row.room = 0.0;
                row.fold.retarget(0.0, CONTENT_OUT);
                continue;
            }
            K::leave(row);
            if animate {
                // Several leaving at once, as when cleared, fold away one after another.
                row.fold
                    .retarget_at(0.0, card::FOLD, now + LEAVE_STAGGER * left.min(STAGGERED));
                left += 1;
                // Those below take their new places at once, and are drawn lower by what it still
                // takes, so they rise with its edge.
                let room = row.room;
                for below in &mut self.rows[index + 1..] {
                    below.y.shift(-room);
                }
            } else {
                row.fold.reset(0.0);
            }
        }

        let mut old: Vec<Option<Row<K>>> = std::mem::take(&mut self.rows)
            .into_iter()
            .map(Some)
            .collect();
        let keep_leaving = |rows: &mut [Option<Row<K>>], next: &mut Vec<Row<K>>| {
            for slot in rows {
                if slot.as_ref().is_some_and(|row| row.leaving)
                    && let Some(row) = slot.take()
                    && !row.fold.is_settled()
                {
                    next.push(row);
                }
            }
        };
        let kept = Kept { animate, now, left };
        let mut next = Vec::with_capacity(placed.len());
        for place in placed {
            let key = K::key(&place.body);
            let existing = old.iter().position(|slot| {
                slot.as_ref()
                    .is_some_and(|row| !row.leaving && K::key(&row.body) == key)
            });
            let alpha = K::alpha(&place.extra);
            let fresh = existing.is_none();
            let mut row = match existing.and_then(|index| {
                keep_leaving(&mut old[..index], &mut next);
                old[index].take()
            }) {
                Some(mut row) => {
                    K::kept(&mut row, &place, kept);
                    row
                }
                None => {
                    let mut row = Row {
                        body: place.body.clone(),
                        y: Spring::new(place.y - ENTER_DRIFT, motion, 0.1),
                        fold: Spring::new(1.0, card::FOLD, 0.001),
                        alpha: Spring::new(0.0, fade_in, 0.002),
                        leaving: false,
                        extra: K::enter(&place.extra, animate, reveal_at),
                        height: place.height,
                        room: place.room,
                    };
                    if animate {
                        row.alpha.retarget_at(alpha, fade_in, reveal_at);
                    } else {
                        row.y.reset(place.y);
                        row.alpha.reset(alpha);
                    }
                    row
                }
            };
            // From far out of sight, it rises in where it goes rather than sweeping across the list.
            if swapping && !fresh && (row.y.value() - place.y).abs() > K::MAX_VISIBLE_H {
                row.y.reset(place.y - ENTER_DRIFT);
                row.alpha.reset(0.0);
                row.alpha.retarget(alpha, fade_in);
            }
            row.body = place.body;
            row.height = place.height;
            row.room = place.room;
            if animate && fresh {
                row.y.retarget_at(place.y, motion, reveal_at);
            } else if animate {
                row.y.retarget(place.y, motion);
            } else {
                row.y.reset(place.y);
            }
            K::assign(&mut row, place.extra);
            next.push(row);
        }
        keep_leaving(&mut old, &mut next);
        self.height = height;
        self.rows = next;
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for row in &mut self.rows {
            row.y.advance(dt, now);
            row.fold.advance(dt, now);
            row.alpha.advance(dt, now);
            K::advance(&mut row.extra, dt, now);
        }
        self.rows
            .retain(|row| !(row.leaving && row.fold.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.rows.iter().all(|row| {
            row.y.is_settled()
                && row.fold.is_settled()
                && row.alpha.is_settled()
                && K::settled(&row.extra)
        })
    }
}
