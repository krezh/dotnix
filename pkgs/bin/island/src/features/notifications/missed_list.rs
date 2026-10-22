use std::time::Instant;

use gpui::SpringConfig;

use super::{card, notification::Notification};
use crate::{
    island::{CONTENT_IN, SHAPE_SPRING},
    spring::Spring,
};

const ENTER_DRIFT: f32 = 12.0;
/// About five cards.
const MAX_VISIBLE_H: f32 = 480.0;

pub struct Row {
    pub notification: Notification,
    pub y: Spring,
    /// How much of it shows: 1 for all of it, folding to 0 as it leaves.
    pub fold: Spring,
    pub alpha: Spring,
    pub leaving: bool,
}

impl Row {
    pub fn height(&self) -> f32 {
        card::missed_height(&self.notification)
    }

    /// The room a leaving row still takes from those below it.
    fn folding(&self) -> f32 {
        if self.leaving {
            (self.height() + card::GAP) * self.fold.value()
        } else {
            0.0
        }
    }
}

/// Its rows are kept in the order they show, leaving ones where they were.
pub struct MissedList {
    rows: Vec<Row>,
    height: f32,
    motion: SpringConfig,
    fade_in: SpringConfig,
}

impl Default for MissedList {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            height: 0.0,
            motion: SHAPE_SPRING,
            fade_in: CONTENT_IN,
        }
    }
}

impl MissedList {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Known from the cards' heights alone, so the center never has to measure it, and dismissing
    /// while the list overflows leaves the center's size alone.
    pub fn visible_height(&self) -> f32 {
        (self.height + self.rows.iter().map(Row::folding).sum::<f32>()).min(MAX_VISIBLE_H)
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

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|row| row.leaving)
    }

    pub fn sync(&mut self, items: &[Notification], animate: bool, reveal_at: Instant) {
        let (motion, fade_in) = (self.motion, self.fade_in);
        for index in 0..self.rows.len() {
            let row = &mut self.rows[index];
            if row.leaving || items.iter().any(|item| item.id == row.notification.id) {
                continue;
            }
            row.leaving = true;
            if animate {
                row.fold.retarget(0.0, card::FOLD);
                // Those below take their new places at once, and are drawn lower by what it still
                // takes, so they rise with its edge.
                let room = row.height() + card::GAP;
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
        let mut y = 0.0;
        let mut next = Vec::with_capacity(items.len());
        for item in items {
            let existing = old.iter().position(|slot| {
                slot.as_ref()
                    .is_some_and(|row| !row.leaving && row.notification.id == item.id)
            });
            let mut row = match existing.and_then(|index| {
                keep_leaving(&mut old[..index], &mut next);
                old[index].take()
            }) {
                Some(row) => row,
                None => {
                    let mut row = Row {
                        notification: item.clone(),
                        y: Spring::new(y - ENTER_DRIFT, motion, 0.1),
                        fold: Spring::new(1.0, card::FOLD, 0.001),
                        alpha: Spring::new(0.0, fade_in, 0.002),
                        leaving: false,
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
            row.notification = item.clone();
            if animate {
                row.y.retarget(y, motion);
            } else {
                row.y.reset(y);
            }
            y += row.height() + card::GAP;
            next.push(row);
        }
        keep_leaving(&mut old, &mut next);
        self.height = (y - card::GAP).max(0.0);
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
