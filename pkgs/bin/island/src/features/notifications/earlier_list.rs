use std::{collections::HashMap, time::Instant};

use chrono::{Local, NaiveDate};

use super::{
    card,
    history::Earlier,
    rows::{self, Kind, Place, Rows},
};
use crate::components::{measure::Measure, stack};

pub const DAY_H: f32 = 22.0;

#[derive(Clone)]
pub enum Entry {
    /// Over the day's notifications.
    Day(NaiveDate),
    Card(Earlier),
}

#[derive(Clone, PartialEq, Eq)]
pub enum Key {
    Day(NaiveDate),
    Card((i64, u32)),
}

pub fn day_of(kept: &Earlier) -> NaiveDate {
    chrono::DateTime::<Local>::from(kept.notification.received).date_naive()
}

pub enum Days {}

impl Kind for Days {
    type Body = Entry;
    type Key = Key;
    type Place = ();
    type Extra = ();

    /// As the missed list.
    const MAX_VISIBLE_H: f32 = 480.0;

    fn key(entry: &Entry) -> Key {
        match entry {
            Entry::Day(day) => Key::Day(*day),
            Entry::Card(kept) => Key::Card(kept.key()),
        }
    }

    fn enter(_: &(), _: bool, _: Instant) {}

    fn assign(_: &mut rows::Row<Self>, _: ()) {}
}

/// Earlier notifications by day, newest first, each day under its heading; one dismissed slides
/// out, then those below rise into its place.
#[derive(Default)]
pub struct EarlierList {
    rows: Rows<Days>,
    items: Vec<Earlier>,
    /// How many lines each one's text takes, once measured.
    lines: HashMap<(i64, u32), usize>,
}

impl std::ops::Deref for EarlierList {
    type Target = Rows<Days>;

    fn deref(&self) -> &Rows<Days> {
        &self.rows
    }
}

impl EarlierList {
    pub fn sync(&mut self, items: Vec<Earlier>, animate: bool, reveal_at: Instant) {
        self.lines
            .retain(|key, _| items.iter().any(|kept| kept.key() == *key));
        self.items = items;
        self.layout(animate, reveal_at);
    }

    /// Shows `items` in place of what it shows now, such as another source's: rows in both slide
    /// to their new places, and the rest fade where they are, together, giving their room at once.
    pub fn swap(&mut self, items: Vec<Earlier>, now: Instant) {
        self.rows.swapping = true;
        self.sync(items, true, now);
        self.rows.swapping = false;
    }

    /// True when any one's text changed its height.
    pub fn measure(&mut self, measure: &Measure<'_>) -> bool {
        let mut measured = false;
        for kept in &self.items {
            if !self.lines.contains_key(&kept.key()) {
                let lines = card::body_lines(
                    &kept.notification.body,
                    card::LIST_W,
                    card::LIST_LINES,
                    measure,
                );
                self.lines.insert(kept.key(), lines);
                measured = true;
            }
        }
        if measured {
            self.layout(true, Instant::now());
        }
        measured
    }

    fn layout(&mut self, animate: bool, reveal_at: Instant) {
        let mut placed = Vec::new();
        let mut y = 0.0;
        let mut day = None;
        for kept in &self.items {
            let this = day_of(kept);
            if day != Some(this) {
                placed.push(Place {
                    body: Entry::Day(this),
                    y,
                    height: DAY_H,
                    room: DAY_H,
                    extra: (),
                });
                y += DAY_H;
                day = Some(this);
            }
            let height = card::missed_height(
                &kept.notification,
                self.lines.get(&kept.key()).copied().unwrap_or(1),
            );
            placed.push(Place {
                body: Entry::Card(kept.clone()),
                y,
                height,
                room: height + stack::GAP,
                extra: (),
            });
            y += height + stack::GAP;
        }
        self.rows
            .layout(placed, (y - stack::GAP).max(0.0), animate, reveal_at);
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.rows.advance(dt, now);
    }
}

#[cfg(test)]
#[path = "earlier_list_test.rs"]
mod tests;
