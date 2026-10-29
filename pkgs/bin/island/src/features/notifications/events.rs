use std::{collections::HashSet, sync::Arc};

use super::{
    Cards, Event, Notifications,
    notification::{Action, Expire, Notification, Posted, Reason, Signal, next_id},
};
use crate::features::{
    Islands,
    notice::{Notice, Post},
};

impl Notifications {
    pub(super) fn handle_event(&self, event: Event, islands: &mut Islands<'_, Cards>) {
        match event {
            Event::Show(notification) => self.show(notification, islands),
            Event::Close(id) => self.close(id, islands),
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
            Event::Posted(post) => self.posted(post, islands),
        }
    }

    fn show(&self, notification: Notification, islands: &mut Islands<'_, Cards>) {
        self.remember(&notification);
        let shown = islands.find(|cards| cards.toasts.live_card_for(notification.id).is_some());
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

    /// Waits with what was missed, unless a pop-up shows it already.
    fn show_quietly(&self, notification: Notification, islands: &mut Islands<'_, Cards>) {
        let shown = islands.find(|cards| cards.toasts.live_card_for(notification.id).is_some());
        if shown.is_some() {
            self.show(notification, islands);
        } else {
            self.remember(&notification);
            self.add_missed(notification);
        }
    }

    fn close(&self, id: u32, islands: &mut Islands<'_, Cards>) {
        let mut closed = false;
        islands.update_all(|cards| closed |= cards.toasts.close(id).is_some());
        if closed {
            self.signal(Signal::Closed(id, Reason::Closed));
        }
        self.close_missed(id);
    }

    fn posted(&self, post: Post, islands: &mut Islands<'_, Cards>) {
        match post {
            Post::Show(notice) => {
                let quiet = notice.quiet;
                let id = self
                    .posted_id(&notice.origin.feature, &notice.origin.key)
                    .unwrap_or_else(next_id);
                self.posted
                    .borrow_mut()
                    .insert(id, notice.origin.clone());
                let notification = from_notice(id, notice);
                if quiet {
                    self.show_quietly(notification, islands);
                } else {
                    self.show(notification, islands);
                }
            }
            Post::Keep { feature, keys } => {
                let gone: Vec<u32> = self
                    .posted
                    .borrow()
                    .iter()
                    .filter(|(_, origin)| origin.feature == feature && !keys.contains(&origin.key))
                    .map(|(id, _)| *id)
                    .collect();
                for id in gone {
                    self.withdraw(id, islands);
                }
            }
            Post::Trouble { feature, trouble } => self.change(|store| {
                let troubles = Arc::make_mut(&mut store.troubles);
                troubles.retain(|(known, _)| *known != feature);
                if let Some(trouble) = trouble {
                    troubles.push((feature, trouble));
                }
            }),
        }
    }

    /// Gone from its feature: it leaves without telling the feature back.
    fn withdraw(&self, id: u32, islands: &mut Islands<'_, Cards>) {
        self.close(id, islands);
        self.posted.borrow_mut().remove(&id);
    }
}

fn from_notice(id: u32, notice: Notice) -> Notification {
    let lead = notice.lead.then(|| notice.origin.feature.clone());
    Notification {
        id,
        app: notice.app,
        summary: notice.title,
        body: notice.body,
        critical: notice.critical,
        icon: None,
        image: None,
        lead,
        transient: false,
        resident: false,
        expire: Expire::Default,
        actions: notice
            .actions
            .into_iter()
            .map(|action| Action {
                key: action.key,
                label: action.label,
                icon: None,
                opens: action.opens,
            })
            .collect(),
        received: notice.received,
        posted: Some(Posted {
            origin: notice.origin,
            via: notice.via,
            detail: notice.detail,
        }),
    }
}
