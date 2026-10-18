use std::time::{Duration, Instant};

use gpui::{SpringConfig, Window};

use super::{card, notification::Notification};
use crate::{
    island::{CONTENT_IN, CONTENT_OUT, REVEAL_DELAY, SHAPE_SPRING},
    panel::Countdown,
    spring::Spring,
};

pub const MAX_VISIBLE: usize = 3;
const ENTER_DRIFT: f32 = 12.0;
const RETURN_DRIFT: f32 = 16.0;
const LEAVE_SLIDE: f32 = 56.0;
/// How long a card shows when its app leaves it to the server.
const DURATION: Duration = Duration::from_millis(4500);
/// The least time a card gets once the pointer leaves, so one paused near its end does not vanish
/// the moment the pointer does.
const RESUME_AT_LEAST: Duration = Duration::from_millis(1000);

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
    /// When the card times out; set once it is visible while the pop-up shows, and cleared while
    /// the countdown is paused.
    pub due: Option<Instant>,
    lifetime: Option<Duration>,
    paused: Option<Duration>,
    /// Counts each pause and resume, so its countdown bar animates afresh from where it is.
    round: u32,
    /// The part of its time left, and the time that is, when this round of counting down started.
    /// Fixed for the round: the bar's animation times itself from its first frame, so it must be
    /// given the same start and length on every redraw.
    round_from: (f32, Duration),
}

impl Card {
    pub fn height(&self) -> f32 {
        card::height(
            self.lines.unwrap_or(1),
            self.notification.buttons().next().is_some(),
        )
    }

    /// `None` while it is not counting down, such as a critical one, which stays until dismissed.
    pub fn countdown(&self) -> Option<Countdown> {
        let lifetime = self.lifetime?;
        let (left, running) = match (self.paused, self.due) {
            (Some(left), _) => (fraction(left, lifetime), None),
            (None, Some(_)) => (self.round_from.0, Some(self.round_from.1)),
            (None, None) => return None,
        };
        Some(Countdown {
            left,
            running,
            round: self.round,
        })
    }

    fn count_from(&mut self, now: Instant, left: Duration) {
        let lifetime = self.lifetime.unwrap_or(left);
        self.due = Some(now + left);
        self.paused = None;
        self.round_from = (fraction(left, lifetime), left);
        self.round += 1;
    }

    fn stop(&mut self) {
        self.due = None;
        self.paused = None;
        self.lifetime = None;
    }
}

fn fraction(left: Duration, lifetime: Duration) -> f32 {
    if lifetime.is_zero() {
        return 0.0;
    }
    (left.as_secs_f32() / lifetime.as_secs_f32()).clamp(0.0, 1.0)
}

pub struct Toasts {
    cards: Vec<Card>,
    next_id: u64,
    height: f32,
    overflow: usize,
    footer_count: usize,
    pub footer: Spring,
    shown: bool,
    /// Over the island, which pauses the countdowns.
    pointer: bool,
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
            pointer: false,
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

    pub fn live_card_for(&self, notification: u32) -> Option<u64> {
        self.cards
            .iter()
            .find(|card| !card.leaving && card.notification.id == notification)
            .map(|card| card.id)
    }

    /// In place of its card if it has one, otherwise as a new card on top.
    pub fn show(&mut self, notification: Notification) {
        match self
            .live_card_for(notification.id)
            .and_then(|id| self.card_mut(id))
        {
            Some(card) => {
                card.notification = notification;
                card.lines = None;
                card.stop();
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
                        lifetime: None,
                        paused: None,
                        round: 0,
                        round_from: (1.0, Duration::ZERO),
                    },
                );
            }
        }
        self.layout();
    }

    pub fn close(&mut self, notification: u32) -> Option<Notification> {
        self.dismiss(self.live_card_for(notification)?)
    }

    pub fn dismiss(&mut self, id: u64) -> Option<Notification> {
        let fade_out = self.fade_out;
        let card = self.card_mut(id).filter(|card| !card.leaving)?;
        card.leaving = true;
        card.stop();
        card.alpha.retarget(0.0, fade_out);
        card.x.retarget(LEAVE_SLIDE, fade_out);
        let notification = card.notification.clone();
        self.layout();
        Some(notification)
    }

    pub fn expired(&self, now: Instant) -> Vec<u64> {
        self.cards
            .iter()
            .filter(|card| card.due.is_some_and(|due| due <= now))
            .map(|card| card.id)
            .collect()
    }

    pub fn postpone(&mut self, id: u64, until: Instant) {
        if let Some(card) = self.card_mut(id) {
            let now = Instant::now();
            card.count_from(now, until.saturating_duration_since(now));
        }
    }

    pub fn due(&self) -> Option<Instant> {
        self.cards.iter().filter_map(|card| card.due).min()
    }

    /// Visible cards start timing out once the island shows them.
    pub fn set_shown(&mut self, shown: bool) {
        self.shown = shown;
        self.arm();
    }

    /// Countdowns pause while the pointer is over, and carry on from where they were once it leaves.
    pub fn set_pointer(&mut self, over: bool, now: Instant) {
        self.pointer = over;
        for card in &mut self.cards {
            match (over, card.due, card.paused) {
                (true, Some(due), _) => {
                    card.paused = Some(due.saturating_duration_since(now));
                    card.due = None;
                    card.round += 1;
                }
                (false, _, Some(left)) => card.count_from(now, left.max(RESUME_AT_LEAST)),
                _ => {}
            }
        }
    }

    fn arm(&mut self) {
        if !self.shown {
            return;
        }
        let now = Instant::now();
        for card in &mut self.cards {
            if card.visible && !card.leaving && card.lifetime.is_none() {
                let Some(timeout) = card.notification.timeout(DURATION) else {
                    continue;
                };
                card.lifetime = Some(timeout);
                if self.pointer {
                    card.paused = Some(timeout);
                } else {
                    card.count_from(now, timeout);
                }
            }
        }
    }

    /// True when any new card's body changed height.
    pub fn measure(&mut self, window: &Window) -> bool {
        let mut measured = false;
        for card in self.cards.iter_mut().filter(|card| card.lines.is_none()) {
            card.lines = Some(card::body_lines(
                &card.notification.body,
                card::MIN_W,
                window,
            ));
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
                card.stop();
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
        self.cards
            .retain(|card| !(card.leaving && card.alpha.is_settled()));
    }

    pub fn is_settled(&self) -> bool {
        self.footer.is_settled()
            && self
                .cards
                .iter()
                .all(|card| card.y.is_settled() && card.x.is_settled() && card.alpha.is_settled())
    }
}

#[cfg(test)]
#[path = "toasts_test.rs"]
mod tests;
