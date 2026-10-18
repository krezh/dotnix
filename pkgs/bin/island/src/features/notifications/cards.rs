use std::time::{Duration, Instant};

use gpui::Window;

use super::{
    Notifications, State, dots::Dots, missed_list::MissedList, notification::Reason, toasts::Toasts,
};
use crate::{
    features::{IslandInfo, Local},
    island::REVEAL_DELAY,
};

/// How much longer a card stays when its time comes while the pointer is over the island.
const HOVER_GRACE: Duration = Duration::from_millis(1500);

#[derive(Default)]
pub struct Cards {
    pub(super) toasts: Toasts,
    pub(super) missed: MissedList,
    pub(super) earlier: bool,
    pub(super) dots: Dots,
}

impl Local for Cards {
    fn panel_closed(&mut self) {
        self.earlier = false;
    }

    fn advance(&mut self, dt: f32, now: Instant) {
        self.toasts.advance(dt, now);
        self.missed.advance(dt, now);
        self.dots.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.toasts.is_settled() && self.missed.is_settled() && self.dots.is_settled()
    }

    fn popup(&self) -> bool {
        !self.toasts.is_empty()
    }

    fn popup_shown(&mut self, shown: bool) {
        self.toasts.set_shown(shown);
    }

    fn pointer(&mut self, over: bool) {
        self.toasts.set_pointer(over, Instant::now());
    }

    fn due(&self) -> Option<Instant> {
        self.toasts.due()
    }

    fn measure(&mut self, window: &Window) -> bool {
        self.toasts.measure(window)
    }
}

pub(super) fn sync(state: &State, cards: &mut Cards, island: &IslandInfo<'_>, animate: bool) {
    cards.missed.sync(
        &state.missed,
        animate && island.panel_open,
        Instant::now() + REVEAL_DELAY,
    );
    cards.dots.show(state.missed.len(), animate);
}

impl Notifications {
    pub(super) fn elapse_cards(&self, cards: &mut Cards, hovered: bool) {
        let now = Instant::now();
        for card in cards.toasts.expired(now) {
            if hovered {
                cards.toasts.postpone(card, now + HOVER_GRACE);
            } else {
                self.dismiss_card(cards, card, Reason::Expired);
            }
        }
    }
}
