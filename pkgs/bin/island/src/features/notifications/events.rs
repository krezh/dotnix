use std::collections::HashSet;

use super::{
    Cards, Event, Notifications,
    notification::{Reason, Signal},
};
use crate::features::Islands;

impl Notifications {
    pub(super) fn handle_event(&self, event: Event, islands: &mut Islands<'_, Cards>) {
        match event {
            Event::Show(notification) => {
                self.remember(&notification);
                let shown =
                    islands.find(|cards| cards.toasts.live_card_for(notification.id).is_some());
                let (missed, dnd) = {
                    let store = self.store.borrow();
                    (
                        store.missed.iter().any(|item| item.id == notification.id),
                        store.dnd,
                    )
                };
                if missed || (dnd && shown.is_none() && !notification.critical) {
                    self.add_missed(notification);
                } else {
                    islands.update_all(|cards| cards.toasts.show(notification.clone()));
                }
            }
            Event::Close(id) => {
                let mut closed = false;
                islands.update_all(|cards| closed |= cards.toasts.close(id).is_some());
                if closed {
                    self.signal(Signal::Closed(id, Reason::Closed));
                }
                self.close_missed(id);
            }
            Event::Locked(locked) => self.change(|store| store.locked = locked),
            Event::Clear => {
                let mut dismissed = HashSet::new();
                islands.update_all(|cards| {
                    let shown: Vec<u64> =
                        cards.toasts.cards().iter().map(|shown| shown.id).collect();
                    for card in shown {
                        if let Some(notification) = cards.toasts.dismiss(card) {
                            dismissed.insert(notification.id);
                        }
                    }
                });
                for id in dismissed {
                    self.signal(Signal::Closed(id, Reason::Dismissed));
                }
                self.clear();
            }
            Event::Expired(notification) => {
                if islands
                    .find(|cards| cards.toasts.live_card_for(notification.id).is_some())
                    .is_none()
                {
                    if notification.transient {
                        self.signal(Signal::Closed(notification.id, Reason::Expired));
                    } else {
                        self.add_missed(notification);
                    }
                }
            }
            Event::Withdrawn(id) => islands.update_all(|cards| {
                cards.toasts.close(id);
            }),
            Event::History(read) => self.load_history(read),
        }
    }
}
