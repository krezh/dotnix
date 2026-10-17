use std::time::Instant;

use gpui::SpringConfig;

use super::{card, notification::Notification};
use crate::{
    island::{CONTENT_IN, CONTENT_OUT, SHAPE_SPRING},
    spring::Spring,
};

const ENTER_DRIFT: f32 = 12.0;
const LEAVE_SLIDE: f32 = 56.0;
/// About five cards.
const MAX_VISIBLE_H: f32 = 480.0;

pub struct Row {
    pub notification: Notification,
    pub y: Spring,
    pub x: Spring,
    pub alpha: Spring,
    pub leaving: bool,
}

impl Row {
    pub fn height(&self) -> f32 {
        card::missed_height(&self.notification)
    }
}

pub struct MissedList {
    rows: Vec<Row>,
    height: f32,
    motion: SpringConfig,
    fade_in: SpringConfig,
    fade_out: SpringConfig,
}

impl Default for MissedList {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            height: 0.0,
            motion: SHAPE_SPRING,
            fade_in: CONTENT_IN,
            fade_out: CONTENT_OUT,
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
        self.height.min(MAX_VISIBLE_H)
    }

    pub fn extent(&self) -> f32 {
        self.rows
            .iter()
            .map(|row| row.y.value().max(row.y.target()) + row.height())
            .fold(self.height, f32::max)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.iter().all(|row| row.leaving)
    }

    pub fn sync(&mut self, items: &[Notification], animate: bool, reveal_at: Instant) {
        let (motion, fade_in, fade_out) = (self.motion, self.fade_in, self.fade_out);
        for row in self.rows.iter_mut().filter(|row| !row.leaving) {
            if !items.iter().any(|item| item.id == row.notification.id) {
                row.leaving = true;
                if animate {
                    row.alpha.retarget(0.0, fade_out);
                    row.x.retarget(LEAVE_SLIDE, fade_out);
                } else {
                    row.alpha.reset(0.0);
                }
            }
        }

        let mut y = 0.0;
        let mut next = Vec::with_capacity(items.len());
        for item in items {
            let existing = self
                .rows
                .iter()
                .position(|row| !row.leaving && row.notification.id == item.id);
            let mut row = match existing {
                Some(index) => self.rows.remove(index),
                None => {
                    let mut row = Row {
                        notification: item.clone(),
                        y: Spring::new(y - ENTER_DRIFT, motion, 0.1),
                        x: Spring::new(0.0, fade_out, 0.1),
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
        self.height = (y - card::GAP).max(0.0);
        self.rows.retain(|row| row.leaving && !row.alpha.is_settled());
        self.rows.extend(next);
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for row in &mut self.rows {
            row.y.advance(dt, now);
            row.x.advance(dt, now);
            row.alpha.advance(dt, now);
        }
        self.rows
            .retain(|row| !(row.leaving && row.alpha.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.rows
            .iter()
            .all(|row| row.y.is_settled() && row.x.is_settled() && row.alpha.is_settled())
    }
}
