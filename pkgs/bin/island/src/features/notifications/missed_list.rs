use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use gpui::{SpringConfig, Window};

use super::{card, notification::Notification};
use crate::{
    island::{CONTENT_IN, REVEAL_DELAY, SHAPE_SPRING},
    spring::Spring,
};

const ENTER_DRIFT: f32 = 12.0;
const LEAVE_STAGGER: Duration = Duration::from_millis(45);
/// How long a card coming to the front of its group waits as the one it replaces starts sliding out.
const PROMOTE_AFTER: Duration = Duration::from_millis(60);
/// About five cards.
const MAX_VISIBLE_H: f32 = 600.0;
/// A group's heading over its cards.
pub const HEAD_H: f32 = 24.0;
const HEAD_GAP: f32 = 6.0;
/// How tall each edge showing below a folded group's front card is, nearest first: one for each
/// card folded behind it, up to two.
pub const PEEKS: [f32; 2] = [8.0, 6.0];

#[derive(Clone)]
pub enum Body {
    Card(Notification),
    /// Over an app's cards, when it has more than one.
    Head {
        app: String,
        count: usize,
        via: Option<String>,
        open: bool,
    },
}

#[derive(Clone, PartialEq, Eq)]
enum Key {
    Card(u32),
    Head(String),
}

impl Body {
    fn key(&self) -> Key {
        match self {
            Body::Card(notification) => Key::Card(notification.id),
            Body::Head { app, .. } => Key::Head(app.clone()),
        }
    }
}

pub struct Row {
    pub body: Body,
    pub y: Spring,
    /// How much of it shows: 1 for all of it, folding to 0 as it leaves.
    pub fold: Spring,
    pub alpha: Spring,
    pub leaving: bool,
    /// How far behind the front card of its group it is folded; 0 for one in front.
    pub depth: usize,
    /// Drawn this tall, as the card it is folded behind.
    pub clip: Option<f32>,
    /// The edges of the cards folded behind it that show below it.
    pub peeks: usize,
    /// How far out those edges are: 1 while folded, 0 once spread out.
    pub peek: Spring,
    height: f32,
    /// What it takes from those below it.
    room: f32,
}

impl Row {
    pub fn height(&self) -> f32 {
        self.height
    }

    /// The front of a group with cards folded behind it, which opens the group when clicked.
    pub fn folded_front(&self) -> bool {
        !self.leaving && self.depth == 0 && self.peeks > 0 && self.peek.target() > 0.5
    }

    pub fn notification(&self) -> Option<&Notification> {
        match &self.body {
            Body::Card(notification) => Some(notification),
            Body::Head { .. } => None,
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

/// How far a leaving row has slid out, 0 to 1, as its fold goes from 1 to 0: it is gone by
/// halfway, and the room it took closes after.
pub fn sliding(fold: f32) -> f32 {
    (1.0 - (fold - 0.5) * 2.0).clamp(0.0, 1.0)
}

/// Where a row goes, as the layout works it out.
struct Placement {
    body: Body,
    y: f32,
    height: f32,
    room: f32,
    depth: usize,
    clip: Option<f32>,
    peeks: usize,
}

/// Its rows are kept in the order they show, leaving ones where they were. Each app's cards are
/// grouped under a heading once it has more than one, folded behind the newest until opened.
pub struct MissedList {
    rows: Vec<Row>,
    height: f32,
    items: Vec<Notification>,
    /// Apps whose cards are spread out rather than folded.
    open: HashSet<String>,
    /// How many lines each card's text takes, once measured.
    lines: HashMap<u32, usize>,
    /// Apps in the order their groups show: one moves to the top only when something new arrives
    /// for it, so dismissing never moves a group.
    order: Vec<String>,
    motion: SpringConfig,
    fade_in: SpringConfig,
}

impl Default for MissedList {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            height: 0.0,
            items: Vec::new(),
            open: HashSet::new(),
            lines: HashMap::new(),
            order: Vec::new(),
            motion: SHAPE_SPRING,
            fade_in: CONTENT_IN,
        }
    }
}

/// Those folded behind another are out of sight until spread out.
fn faded(depth: usize) -> f32 {
    if depth == 0 { 1.0 } else { 0.0 }
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
        let arrived: Vec<&str> = items
            .iter()
            .filter(|item| !self.items.iter().any(|known| known.id == item.id))
            .map(|item| item.app.as_str())
            .collect();
        self.order
            .retain(|app| items.iter().any(|item| item.app == *app));
        for app in arrived.iter().rev() {
            self.order.retain(|known| known != app);
            self.order.insert(0, (*app).to_owned());
        }
        self.items = items.to_vec();
        self.lines
            .retain(|id, _| items.iter().any(|item| item.id == *id));
        self.open
            .retain(|app| items.iter().filter(|item| item.app == *app).count() > 1);
        self.layout(animate, reveal_at);
    }

    /// Spreads an app's cards out, or folds them back behind the newest.
    pub fn toggle(&mut self, app: &str) {
        if !self.open.remove(app) {
            self.open.insert(app.to_owned());
        }
        self.layout(true, Instant::now());
    }

    /// True when any new card's text changed its height.
    pub fn measure(&mut self, window: &Window) -> bool {
        let mut measured = false;
        for item in &self.items {
            if !self.lines.contains_key(&item.id) {
                let lines = card::body_lines(&item.body, card::LIST_W, card::LIST_LINES, window);
                self.lines.insert(item.id, lines);
                measured = true;
            }
        }
        if measured {
            self.layout(true, Instant::now() + REVEAL_DELAY);
        }
        measured
    }

    fn card_height(&self, notification: &Notification) -> f32 {
        card::missed_height(
            notification,
            self.lines.get(&notification.id).copied().unwrap_or(1),
        )
    }

    /// Each app's cards together, newest first, the groups in their kept order.
    fn groups(&self) -> Vec<Vec<&Notification>> {
        let mut groups: Vec<Vec<&Notification>> = Vec::new();
        for item in &self.items {
            match groups.iter_mut().find(|group| group[0].app == item.app) {
                Some(group) => group.push(item),
                None => groups.push(vec![item]),
            }
        }
        groups.sort_by_key(|group| {
            self.order
                .iter()
                .position(|app| *app == group[0].app)
                .unwrap_or(usize::MAX)
        });
        groups
    }

    fn placements(&self) -> (Vec<Placement>, f32) {
        let mut placed = Vec::new();
        let mut y = 0.0;
        for group in self.groups() {
            let app = &group[0].app;
            let open = self.open.contains(app);
            if group.len() > 1 {
                placed.push(Placement {
                    body: Body::Head {
                        app: app.clone(),
                        count: group.len(),
                        via: group[0]
                            .posted
                            .as_ref()
                            .filter(|posted| posted.via)
                            .map(|posted| posted.origin.source.clone()),
                        open,
                    },
                    y,
                    height: HEAD_H,
                    room: HEAD_H + HEAD_GAP,
                    depth: 0,
                    clip: None,
                    peeks: 0,
                });
                y += HEAD_H + HEAD_GAP;
            }
            if open || group.len() == 1 {
                for item in group {
                    let height = self.card_height(item);
                    placed.push(Placement {
                        body: Body::Card(item.clone()),
                        y,
                        height,
                        room: height + card::GAP,
                        depth: 0,
                        clip: None,
                        peeks: 0,
                    });
                    y += height + card::GAP;
                }
                continue;
            }
            let front = self.card_height(group[0]);
            let peeks = (group.len() - 1).min(PEEKS.len());
            let below: f32 = PEEKS[..peeks].iter().sum();
            for (depth, item) in group.into_iter().enumerate() {
                placed.push(Placement {
                    body: Body::Card(item.clone()),
                    y,
                    height: front,
                    room: if depth == 0 {
                        front + below + card::GAP
                    } else {
                        0.0
                    },
                    depth,
                    clip: (depth > 0).then_some(front),
                    peeks: if depth == 0 { peeks } else { 0 },
                });
            }
            y += front + below + card::GAP;
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
            if row.leaving
                || placed
                    .iter()
                    .any(|place| place.body.key() == row.body.key())
            {
                continue;
            }
            row.leaving = true;
            // The front of a folded group gives no room back: the next card of the group takes its
            // place.
            if row.peeks > 0 && row.peek.target() > 0.5 {
                row.room = 0.0;
                row.peek.reset(0.0);
            }
            if animate {
                // Several leaving at once, as when cleared, fold away one after another.
                row.fold
                    .retarget_at(0.0, card::FOLD, now + LEAVE_STAGGER * left);
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
        for place in placed {
            let key = place.body.key();
            let existing = old.iter().position(|slot| {
                slot.as_ref()
                    .is_some_and(|row| !row.leaving && row.body.key() == key)
            });
            let alpha = faded(place.depth);
            let mut row = match existing.and_then(|index| {
                keep_leaving(&mut old[..index], &mut next);
                old[index].take()
            }) {
                Some(mut row) => {
                    let peek = if place.peeks > 0 { 1.0 } else { 0.0 };
                    // Coming to the front in place of one folding away, it shows once that one has.
                    let promoted = row.depth > 0 && place.depth == 0 && left > 0;
                    if animate && promoted {
                        // Already where the leaving one was, under it, so the edges below stay.
                        row.alpha.retarget_at(alpha, fade_in, now + PROMOTE_AFTER);
                        row.y.reset(place.y);
                        row.peek.reset(peek);
                    } else if animate {
                        row.alpha.retarget(alpha, fade_in);
                        row.peek.retarget(peek, motion);
                    } else {
                        row.alpha.reset(alpha);
                        row.peek.reset(peek);
                    }
                    row
                }
                None => {
                    let mut row = Row {
                        body: place.body.clone(),
                        y: Spring::new(place.y - ENTER_DRIFT, motion, 0.1),
                        fold: Spring::new(1.0, card::FOLD, 0.001),
                        alpha: Spring::new(0.0, fade_in, 0.002),
                        leaving: false,
                        depth: place.depth,
                        clip: place.clip,
                        peeks: place.peeks,
                        peek: Spring::new(
                            if place.peeks > 0 && !animate {
                                1.0
                            } else {
                                0.0
                            },
                            motion,
                            0.002,
                        ),
                        height: place.height,
                        room: place.room,
                    };
                    if animate {
                        row.alpha.retarget_at(alpha, fade_in, reveal_at);
                        if place.peeks > 0 {
                            row.peek.retarget_at(1.0, motion, reveal_at);
                        }
                    } else {
                        row.y.reset(place.y);
                        row.alpha.reset(alpha);
                    }
                    row
                }
            };
            row.body = place.body;
            row.depth = place.depth;
            row.clip = place.clip;
            if place.peeks > 0 {
                row.peeks = place.peeks;
            }
            row.height = place.height;
            row.room = place.room;
            if animate {
                row.y.retarget(place.y, motion);
            } else {
                row.y.reset(place.y);
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
            row.peek.advance(dt, now);
        }
        self.rows
            .retain(|row| !(row.leaving && row.fold.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.rows.iter().all(|row| {
            row.y.is_settled()
                && row.fold.is_settled()
                && row.alpha.is_settled()
                && row.peek.is_settled()
        })
    }
}
