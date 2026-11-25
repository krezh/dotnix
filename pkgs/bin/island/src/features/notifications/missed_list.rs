use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use super::{
    card,
    notification::Notification,
    rows::{self, Kept, Kind, Place, Rows},
};
use crate::{
    components::{
        measure::Measure,
        stack::{self, PEEKS},
    },
    spring::Spring,
    stage::{CONTENT_IN, SHAPE_SPRING},
};

/// How long a card coming to the front of its group waits as the one it replaces starts sliding out.
const PROMOTE_AFTER: Duration = Duration::from_millis(60);
/// A group's heading over its cards.
pub const HEAD_H: f32 = 24.0;
const HEAD_GAP: f32 = 6.0;

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
pub enum Key {
    Card(u32),
    Head(String),
}

/// Where a card sits in its group.
#[derive(Clone, Copy)]
pub struct Stacked {
    /// How far behind the front card of its group it is folded; 0 for one in front.
    depth: usize,
    /// Drawn this tall, as the card it is folded behind.
    clip: Option<f32>,
    /// The edges of the cards folded behind it that show below it.
    peeks: usize,
}

pub struct Folded {
    pub depth: usize,
    pub clip: Option<f32>,
    pub peeks: usize,
    /// How far out those edges are: 1 while folded, 0 once spread out.
    pub peek: Spring,
}

/// Those folded behind another are out of sight until spread out.
fn faded(depth: usize) -> f32 {
    if depth == 0 { 1.0 } else { 0.0 }
}

pub enum Groups {}

impl Kind for Groups {
    type Body = Body;
    type Key = Key;
    type Place = Stacked;
    type Extra = Folded;

    /// About five cards.
    const MAX_VISIBLE_H: f32 = 600.0;

    fn key(body: &Body) -> Key {
        match body {
            Body::Card(notification) => Key::Card(notification.id),
            Body::Head { app, .. } => Key::Head(app.clone()),
        }
    }

    fn alpha(place: &Stacked) -> f32 {
        faded(place.depth)
    }

    fn enter(place: &Stacked, animate: bool, reveal_at: Instant) -> Folded {
        let mut peek = Spring::new(
            if place.peeks > 0 && !animate { 1.0 } else { 0.0 },
            SHAPE_SPRING,
            0.002,
        );
        if animate && place.peeks > 0 {
            peek.retarget_at(1.0, SHAPE_SPRING, reveal_at);
        }
        Folded {
            depth: place.depth,
            clip: place.clip,
            peeks: place.peeks,
            peek,
        }
    }

    /// The front of a folded group gives no room back: the next card of the group takes its
    /// place.
    fn leave(row: &mut Row) {
        if row.peeks > 0 && row.peek.target() > 0.5 {
            row.room = 0.0;
            row.extra.peek.reset(0.0);
        }
    }

    fn kept(row: &mut Row, place: &Place<Self>, kept: Kept) {
        let alpha = faded(place.extra.depth);
        let peek = if place.extra.peeks > 0 { 1.0 } else { 0.0 };
        // Coming to the front in place of one folding away, it shows once that one has.
        let promoted = row.depth > 0 && place.extra.depth == 0 && kept.left > 0;
        if kept.animate && promoted {
            // Already where the leaving one was, under it, so the edges below stay.
            row.alpha
                .retarget_at(alpha, CONTENT_IN, kept.now + PROMOTE_AFTER);
            row.y.reset(place.y);
            row.extra.peek.reset(peek);
        } else if kept.animate {
            row.alpha.retarget(alpha, CONTENT_IN);
            row.extra.peek.retarget(peek, SHAPE_SPRING);
        } else {
            row.alpha.reset(alpha);
            row.extra.peek.reset(peek);
        }
    }

    fn assign(row: &mut Row, place: Stacked) {
        row.extra.depth = place.depth;
        row.extra.clip = place.clip;
        if place.peeks > 0 {
            row.extra.peeks = place.peeks;
        }
    }

    fn advance(folded: &mut Folded, dt: f32, now: Instant) {
        folded.peek.advance(dt, now);
    }

    fn settled(folded: &Folded) -> bool {
        folded.peek.is_settled()
    }
}

pub type Row = rows::Row<Groups>;

impl Row {
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
}

/// Each app's cards are grouped under a heading once it has more than one, folded behind the
/// newest until opened.
#[derive(Default)]
pub struct MissedList {
    rows: Rows<Groups>,
    items: Vec<Notification>,
    /// Apps whose cards are spread out rather than folded.
    open: HashSet<String>,
    /// How many lines each card's text takes, once measured.
    lines: HashMap<u32, usize>,
    /// Apps in the order their groups show: one moves to the top only when something new arrives
    /// for it, so dismissing never moves a group.
    order: Vec<String>,
}

/// Its visible height is known from the cards' heights alone, so the center never has to measure
/// it, and dismissing while the list overflows leaves the center's size alone.
impl std::ops::Deref for MissedList {
    type Target = Rows<Groups>;

    fn deref(&self) -> &Rows<Groups> {
        &self.rows
    }
}

impl MissedList {
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

    /// Shows `items` in place of what it shows now, such as another source's: cards in both slide
    /// to their new places, and the rest fade where they are, together, giving their room at once.
    pub fn swap(&mut self, items: &[Notification], now: Instant) {
        self.rows.swapping = true;
        self.sync(items, true, now);
        self.rows.swapping = false;
    }

    /// Spreads an app's cards out, or folds them back behind the newest.
    pub fn toggle(&mut self, app: &str) {
        if !self.open.remove(app) {
            self.open.insert(app.to_owned());
        }
        self.layout(true, Instant::now());
    }

    /// True when any new card's text changed its height.
    pub fn measure(&mut self, measure: &Measure<'_>) -> bool {
        let mut measured = false;
        for item in &self.items {
            if !self.lines.contains_key(&item.id) {
                let lines = card::body_lines(&item.body, card::LIST_W, card::LIST_LINES, measure);
                self.lines.insert(item.id, lines);
                measured = true;
            }
        }
        if measured {
            self.layout(true, Instant::now());
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

    fn placements(&self) -> (Vec<Place<Groups>>, f32) {
        const ALONE: Stacked = Stacked {
            depth: 0,
            clip: None,
            peeks: 0,
        };
        let mut placed = Vec::new();
        let mut y = 0.0;
        for group in self.groups() {
            let app = &group[0].app;
            let open = self.open.contains(app);
            if group.len() > 1 {
                placed.push(Place {
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
                    extra: ALONE,
                });
                y += HEAD_H + HEAD_GAP;
            }
            if open || group.len() == 1 {
                for item in group {
                    let height = self.card_height(item);
                    placed.push(Place {
                        body: Body::Card(item.clone()),
                        y,
                        height,
                        room: height + stack::GAP,
                        extra: ALONE,
                    });
                    y += height + stack::GAP;
                }
                continue;
            }
            let front = self.card_height(group[0]);
            let peeks = (group.len() - 1).min(PEEKS.len());
            let below: f32 = PEEKS[..peeks].iter().sum();
            for (depth, item) in group.into_iter().enumerate() {
                placed.push(Place {
                    body: Body::Card(item.clone()),
                    y,
                    height: front,
                    room: if depth == 0 {
                        front + below + stack::GAP
                    } else {
                        0.0
                    },
                    extra: Stacked {
                        depth,
                        clip: (depth > 0).then_some(front),
                        peeks: if depth == 0 { peeks } else { 0 },
                    },
                });
            }
            y += front + below + stack::GAP;
        }
        (placed, (y - stack::GAP).max(0.0))
    }

    fn layout(&mut self, animate: bool, reveal_at: Instant) {
        let (placed, height) = self.placements();
        self.rows.layout(placed, height, animate, reveal_at);
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.rows.advance(dt, now);
    }
}
