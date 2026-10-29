use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use chrono::{Local, NaiveDate};
use gpui::{SpringConfig, Window};

use super::{card, history::Earlier, missed_list::sliding};
use crate::{
    island::{CONTENT_IN, REVEAL_DELAY, SHAPE_SPRING},
    spring::Spring,
};

const ENTER_DRIFT: f32 = 12.0;
pub const DAY_H: f32 = 22.0;
/// How tall the list can be before it scrolls, as the missed list.
const MAX_VISIBLE_H: f32 = 480.0;
const LEAVE_STAGGER: Duration = Duration::from_millis(45);

#[derive(Clone)]
pub enum Entry {
    /// Over the day's notifications.
    Day(NaiveDate),
    Card(Earlier),
}

#[derive(Clone, PartialEq, Eq)]
enum Key {
    Day(NaiveDate),
    Card((i64, u32)),
}

impl Entry {
    fn key(&self) -> Key {
        match self {
            Entry::Day(day) => Key::Day(*day),
            Entry::Card(kept) => Key::Card(kept.key()),
        }
    }
}

pub fn day_of(kept: &Earlier) -> NaiveDate {
    chrono::DateTime::<Local>::from(kept.notification.received).date_naive()
}

pub struct Row {
    pub entry: Entry,
    pub y: Spring,
    /// 1 while it stays; as it leaves, it slides out by halfway to 0 and the room it took closes.
    pub fold: Spring,
    pub alpha: Spring,
    pub leaving: bool,
    height: f32,
    room: f32,
}

impl Row {
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

    fn folding(&self) -> f32 {
        if self.leaving {
            self.room * (self.fold.value() * 2.0).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Earlier notifications by day, newest first, each day under its heading; one dismissed slides
/// out, then those below rise into its place.
pub struct EarlierList {
    rows: Vec<Row>,
    height: f32,
    items: Vec<Earlier>,
    /// How many lines each one's text takes, once measured.
    lines: HashMap<(i64, u32), usize>,
    motion: SpringConfig,
    fade_in: SpringConfig,
}

impl Default for EarlierList {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            height: 0.0,
            items: Vec::new(),
            lines: HashMap::new(),
            motion: SHAPE_SPRING,
            fade_in: CONTENT_IN,
        }
    }
}

impl EarlierList {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|row| row.leaving)
    }

    pub fn visible_height(&self) -> f32 {
        (self.height + self.rows.iter().map(Row::folding).sum::<f32>()).min(MAX_VISIBLE_H)
    }

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

    pub fn sync(&mut self, items: Vec<Earlier>, animate: bool, reveal_at: Instant) {
        self.lines
            .retain(|key, _| items.iter().any(|kept| kept.key() == *key));
        self.items = items;
        self.layout(animate, reveal_at);
    }

    /// True when any one's text changed its height.
    pub fn measure(&mut self, window: &Window) -> bool {
        let mut measured = false;
        for kept in &self.items {
            if !self.lines.contains_key(&kept.key()) {
                let lines = card::body_lines(
                    &kept.notification.body,
                    card::LIST_W,
                    card::LIST_LINES,
                    window,
                );
                self.lines.insert(kept.key(), lines);
                measured = true;
            }
        }
        if measured {
            self.layout(true, Instant::now() + REVEAL_DELAY);
        }
        measured
    }

    fn placements(&self) -> (Vec<(Entry, f32, f32)>, f32) {
        let mut placed = Vec::new();
        let mut y = 0.0;
        let mut day = None;
        for kept in &self.items {
            let this = day_of(kept);
            if day != Some(this) {
                placed.push((Entry::Day(this), y, DAY_H));
                y += DAY_H;
                day = Some(this);
            }
            let height = card::missed_height(
                &kept.notification,
                self.lines.get(&kept.key()).copied().unwrap_or(1),
            );
            placed.push((Entry::Card(kept.clone()), y, height));
            y += height + card::GAP;
        }
        (placed, (y - card::GAP).max(0.0))
    }

    fn layout(&mut self, animate: bool, reveal_at: Instant) {
        let (motion, fade_in) = (self.motion, self.fade_in);
        let (placed, height) = self.placements();
        let now = Instant::now();
        let mut left = 0;
        for index in 0..self.rows.len() {
            let row = &mut self.rows[index];
            if row.leaving || placed.iter().any(|(entry, ..)| entry.key() == row.entry.key()) {
                continue;
            }
            row.leaving = true;
            if animate {
                row.fold
                    .retarget_at(0.0, card::FOLD, now + LEAVE_STAGGER * left);
                left += 1;
                let room = row.room;
                for below in &mut self.rows[index + 1..] {
                    below.y.shift(-room);
                }
            } else {
                row.fold.reset(0.0);
            }
        }

        let mut old: Vec<Option<Row>> = std::mem::take(&mut self.rows)
            .into_iter()
            .map(Some)
            .collect();
        let keep_leaving = |rows: &mut [Option<Row>], next: &mut Vec<Row>| {
            for slot in rows {
                if slot.as_ref().is_some_and(|row| row.leaving)
                    && let Some(row) = slot.take()
                    && !row.fold.is_settled()
                {
                    next.push(row);
                }
            }
        };
        let mut next = Vec::with_capacity(placed.len());
        for (entry, y, height) in placed {
            let key = entry.key();
            let room = match entry {
                Entry::Day(_) => DAY_H,
                Entry::Card(_) => height + card::GAP,
            };
            let existing = old.iter().position(|slot| {
                slot.as_ref()
                    .is_some_and(|row| !row.leaving && row.entry.key() == key)
            });
            let mut row = match existing.and_then(|index| {
                keep_leaving(&mut old[..index], &mut next);
                old[index].take()
            }) {
                Some(row) => row,
                None => {
                    let mut row = Row {
                        entry: entry.clone(),
                        y: Spring::new(y - ENTER_DRIFT, motion, 0.1),
                        fold: Spring::new(1.0, card::FOLD, 0.001),
                        alpha: Spring::new(0.0, fade_in, 0.002),
                        leaving: false,
                        height,
                        room,
                    };
                    if animate {
                        row.alpha.retarget_at(1.0, fade_in, reveal_at);
                    } else {
                        row.y.reset(y);
                        row.alpha.reset(1.0);
                    }
                    row
                }
            };
            row.entry = entry;
            row.height = height;
            row.room = room;
            if animate {
                row.y.retarget(y, motion);
            } else {
                row.y.reset(y);
            }
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
        }
        self.rows
            .retain(|row| !(row.leaving && row.fold.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.rows
            .iter()
            .all(|row| row.y.is_settled() && row.fold.is_settled() && row.alpha.is_settled())
    }
}

#[cfg(test)]
#[path = "earlier_list_test.rs"]
mod tests;
