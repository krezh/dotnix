//! The notification cards one island shows as they arrive, newest on top, each timing out on its
//! own while the pop-up shows.

use std::time::{Duration, Instant};

use gpui::{SpringConfig, Window};

use super::{card, notification::Notification};
use crate::{
    island::{CONTENT_IN, CONTENT_OUT, REVEAL_DELAY, SHAPE_SPRING},
    spring::Spring,
};

pub const MAX_VISIBLE: usize = 3;
const ENTER_DRIFT: f32 = 12.0;
const RETURN_DRIFT: f32 = 16.0;
const LEAVE_SLIDE: f32 = 56.0;
/// How long a card shows when its app leaves it to the server.
const DURATION: Duration = Duration::from_millis(4500);

pub struct Card {
    pub id: u64,
    pub notification: Notification,
    pub lines: Option<usize>,
    pub y: Spring,
    pub x: Spring,
    pub alpha: Spring,
    pub leaving: bool,
    pub visible: bool,
    shown_before: bool,
    /// When the card times out; set once it is visible while the pop-up shows.
    pub due: Option<Instant>,
}

impl Card {
    pub fn height(&self) -> f32 {
        card::height(self.lines.unwrap_or(1), self.notification.buttons().next().is_some())
    }
}

pub struct Toasts {
    cards: Vec<Card>,
    next_id: u64,
    height: f32,
    overflow: usize,
    footer_count: usize,
    pub footer: Spring,
    /// Whether the island is showing the cards.
    shown: bool,
    motion: SpringConfig,
    fade_in: SpringConfig,
    fade_out: SpringConfig,
}

impl Default for Toasts {
    fn default() -> Self {
        Self {
            cards: Vec::new(),
            next_id: 0,
            height: card::toasts_height(&[card::height(1, false)], false),
            overflow: 0,
            footer_count: 0,
            footer: Spring::new(0.0, CONTENT_OUT, 0.002),
            shown: false,
            motion: SHAPE_SPRING,
            fade_in: CONTENT_IN,
            fade_out: CONTENT_OUT,
        }
    }
}

impl Toasts {
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn is_empty(&self) -> bool {
        self.cards.iter().all(|card| card.leaving)
    }

    fn card_mut(&mut self, id: u64) -> Option<&mut Card> {
        self.cards.iter_mut().find(|card| card.id == id)
    }

    /// The card still showing `notification`.
    pub fn live_card_for(&self, notification: u32) -> Option<u64> {
        self.cards
            .iter()
            .find(|card| !card.leaving && card.notification.id == notification)
            .map(|card| card.id)
    }

    /// Shows `notification`: in place of its card if it has one, otherwise as a new card on top.
    pub fn show(&mut self, notification: Notification) {
        match self.live_card_for(notification.id).and_then(|id| self.card_mut(id)) {
            Some(card) => {
                card.notification = notification;
                card.lines = None;
                card.due = None;
            }
            None => {
                let id = self.next_id;
                self.next_id += 1;
                self.cards.insert(
                    0,
                    Card {
                        id,
                        notification,
                        lines: None,
                        y: Spring::new(-ENTER_DRIFT, self.motion, 0.1),
                        x: Spring::new(0.0, self.fade_out, 0.1),
                        alpha: Spring::new(0.0, self.fade_in, 0.002),
                        leaving: false,
                        visible: false,
                        shown_before: false,
                        due: None,
                    },
                );
            }
        }
        self.layout();
    }

    /// Takes away the card showing `notification`, returning it.
    pub fn close(&mut self, notification: u32) -> Option<Notification> {
        self.dismiss(self.live_card_for(notification)?)
    }

    /// Takes away card `id`, returning its notification.
    pub fn dismiss(&mut self, id: u64) -> Option<Notification> {
        let fade_out = self.fade_out;
        let card = self.card_mut(id).filter(|card| !card.leaving)?;
        card.leaving = true;
        card.due = None;
        card.alpha.retarget(0.0, fade_out);
        card.x.retarget(LEAVE_SLIDE, fade_out);
        let notification = card.notification.clone();
        self.layout();
        Some(notification)
    }

    /// The cards whose time has come by `now`.
    pub fn expired(&self, now: Instant) -> Vec<u64> {
        self.cards
            .iter()
            .filter(|card| card.due.is_some_and(|due| due <= now))
            .map(|card| card.id)
            .collect()
    }

    /// Gives card `id` until `until` before it times out.
    pub fn postpone(&mut self, id: u64, until: Instant) {
        if let Some(card) = self.card_mut(id) {
            card.due = Some(until);
        }
    }

    /// The soonest a card times out.
    pub fn due(&self) -> Option<Instant> {
        self.cards.iter().filter_map(|card| card.due).min()
    }

    /// The island started or stopped showing the cards; the visible ones start timing out once
    /// it does.
    pub fn set_shown(&mut self, shown: bool) {
        self.shown = shown;
        self.arm();
    }

    fn arm(&mut self) {
        if !self.shown {
            return;
        }
        let now = Instant::now();
        for card in &mut self.cards {
            if card.visible && !card.leaving && card.due.is_none() {
                card.due = card.notification.timeout(DURATION).map(|timeout| now + timeout);
            }
        }
    }

    /// Measures the body text of new cards; true when any changed height.
    pub fn measure(&mut self, window: &Window) -> bool {
        let mut measured = false;
        for card in self.cards.iter_mut().filter(|card| card.lines.is_none()) {
            card.lines = Some(card::body_lines(&card.notification.body, card::MIN_W, window));
            measured = true;
        }
        if measured {
            self.layout();
        }
        measured
    }

    fn layout(&mut self) {
        let reveal_at = Instant::now() + REVEAL_DELAY;
        let (motion, fade_in, fade_out) = (self.motion, self.fade_in, self.fade_out);
        let mut y = 0.0;
        let mut heights = Vec::with_capacity(MAX_VISIBLE);
        let mut live = 0;

        for card in self.cards.iter_mut().filter(|card| !card.leaving) {
            live += 1;
            if heights.len() == MAX_VISIBLE {
                card.visible = false;
                card.due = None;
                card.alpha.retarget(0.0, fade_out);
                continue;
            }

            if card.shown_before && !card.visible && card.alpha.value() < 0.05 {
                card.y.reset(y + RETURN_DRIFT);
            }
            card.y.retarget(y, motion);
            if card.alpha.value() < 0.05 {
                card.alpha.retarget_at(1.0, fade_in, reveal_at);
            } else {
                card.alpha.retarget(1.0, fade_in);
            }
            card.visible = true;
            card.shown_before = true;

            let height = card.height();
            heights.push(height);
            y += height + card::GAP;
        }

        self.overflow = live - heights.len();
        if !heights.is_empty() {
            self.height = card::toasts_height(&heights, self.overflow > 0);
        }
        if self.overflow > 0 {
            self.footer_count = self.overflow;
            self.footer.retarget(1.0, fade_in);
        } else {
            self.footer.retarget(0.0, fade_out);
        }
        self.arm();
    }

    pub fn footer_count(&self) -> usize {
        self.footer_count
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for card in &mut self.cards {
            card.y.advance(dt, now);
            card.x.advance(dt, now);
            card.alpha.advance(dt, now);
        }
        self.footer.advance(dt, now);
        self.cards.retain(|card| !(card.leaving && card.alpha.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.footer.is_settled()
            && self
                .cards
                .iter()
                .all(|card| card.y.is_settled() && card.x.is_settled() && card.alpha.is_settled())
    }
}
