use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::Window;

use super::{
    Notifications, State,
    dots::Dots,
    earlier_list::EarlierList,
    history::Earlier,
    missed_list::MissedList,
    notification::{Notification, Reason},
    toasts::Toasts,
};
use crate::{
    features::{IslandInfo, Local, notice::Trouble},
    island::{REVEAL_DELAY, SHAPE_SPRING},
    spring::Spring,
};

/// How much longer a card stays when its time comes while the pointer is over the island.
const HOVER_GRACE: Duration = Duration::from_millis(1500);

/// Which notifications the panel's lists show, by where they are from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    /// Desktop apps', over D-Bus.
    Desktop,
    /// The source a feature posts under.
    Source(String),
}

impl Filter {
    pub fn shows(&self, notification: &Notification) -> bool {
        match self {
            Filter::All => true,
            Filter::Desktop => notification.posted.is_none(),
            Filter::Source(source) => notification.source() == Some(source.as_str()),
        }
    }
}

/// What sources are having trouble with, over the list, opening and closing as it comes and goes.
pub struct Troubles {
    pub(super) open: Spring,
    /// The last there were, kept while they close.
    pub(super) shown: Arc<Vec<(String, Trouble)>>,
}

impl Default for Troubles {
    fn default() -> Self {
        Self {
            open: Spring::new(0.0, SHAPE_SPRING, 0.002),
            shown: Arc::default(),
        }
    }
}

impl Troubles {
    fn sync(&mut self, troubles: &Arc<Vec<(String, Trouble)>>, animate: bool) {
        let target = if troubles.is_empty() {
            0.0
        } else {
            self.shown = troubles.clone();
            1.0
        };
        if animate {
            self.open.retarget(target, SHAPE_SPRING);
        } else {
            self.open.reset(target);
        }
    }
}

#[derive(Default)]
pub struct Cards {
    pub(super) troubles: Troubles,
    pub(super) toasts: Toasts,
    pub(super) missed: MissedList,
    pub(super) earlier: bool,
    pub(super) dots: Dots,
    pub(super) filter: Filter,
    /// The sources dropdown is open.
    pub(super) picking: bool,
    pub(super) earlier_list: EarlierList,
    /// Every missed one, before the filter.
    all: Arc<Vec<Notification>>,
    /// Every earlier one, before the filter.
    all_earlier: Vec<Earlier>,
}

impl Cards {
    pub(super) fn set_filter(&mut self, filter: Filter) {
        self.picking = false;
        if self.filter == filter {
            return;
        }
        self.filter = filter;
        self.show_missed(true);
        self.show_earlier(true);
    }

    fn show_missed(&mut self, animate: bool) {
        let shown: Vec<Notification> = self
            .all
            .iter()
            .filter(|notification| self.filter.shows(notification))
            .cloned()
            .collect();
        self.missed
            .sync(&shown, animate, Instant::now() + REVEAL_DELAY);
    }

    fn show_earlier(&mut self, animate: bool) {
        let shown: Vec<Earlier> = self
            .all_earlier
            .iter()
            .filter(|kept| self.filter.shows(&kept.notification))
            .cloned()
            .collect();
        self.earlier_list
            .sync(shown, animate, Instant::now() + REVEAL_DELAY);
    }
}

impl Local for Cards {
    fn panel_closed(&mut self) {
        self.earlier = false;
        self.picking = false;
    }

    fn advance(&mut self, dt: f32, now: Instant) {
        self.toasts.advance(dt, now);
        self.missed.advance(dt, now);
        self.earlier_list.advance(dt, now);
        self.dots.advance(dt, now);
        self.troubles.open.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.toasts.is_settled()
            && self.missed.is_settled()
            && self.earlier_list.is_settled()
            && self.dots.is_settled()
            && self.troubles.open.is_settled()
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
        let mut measured = self.toasts.measure(window);
        measured |= self.missed.measure(window);
        measured |= self.earlier_list.measure(window);
        measured
    }
}

pub(super) fn sync(state: &State, cards: &mut Cards, island: &IslandInfo<'_>, animate: bool) {
    cards.all = state.missed.clone();
    if let Filter::Source(source) = &cards.filter
        && !state.missed.iter().any(|item| item.source() == Some(source))
        && !state
            .history
            .iter()
            .any(|kept| kept.notification.source() == Some(source))
    {
        cards.filter = Filter::All;
    }
    cards.show_missed(animate && island.panel_open);
    cards
        .troubles
        .sync(&state.troubles, animate && island.panel_open);
    cards.dots.show(state.missed.len(), animate);
    cards.all_earlier = Notifications::earlier(state);
    cards.show_earlier(animate && island.panel_open);
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
