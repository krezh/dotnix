mod card;
mod dots;
mod history;
mod lock;
mod markdown;
mod missed_list;
mod notification;
mod server;
mod toasts;

use std::{
    cell::RefCell,
    collections::HashSet,
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    time::{Duration, Instant},
};

use gpui::{AnyElement, Window, prelude::*};
use serde::Deserialize;

use self::{
    card::Look,
    dots::Dots,
    history::{Earlier, Write},
    missed_list::MissedList,
    notification::{DEFAULT_ACTION, Notification, Reason, Signal},
    toasts::Toasts,
};
use super::{Feature, Feed, IslandInfo, Islands, Local, PanelCx, WidgetCx, parse_settings};
use crate::{
    components::tooltip::Tip,
    control::{self, Switch},
    island::{Panel, REVEAL_DELAY},
    panel::{Button, Handler, Header, PanelView, Views},
    theme::{ACCENT, OVERLAY1},
};

const KEEP: usize = 50;
/// How much longer a card stays when its time comes while the pointer is over the island.
const HOVER_GRACE: Duration = Duration::from_millis(1500);
/// So "5m" keeps up.
const REFRESH: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Settings {
    private: bool,
    private_apps: Vec<String>,
    hide_on_lock: bool,
    history_days: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            private: false,
            private_apps: Vec::new(),
            hide_on_lock: true,
            history_days: 7,
        }
    }
}

pub enum Event {
    /// New, or in place of one the app sent before.
    Show(Notification),
    Close(u32),
    Locked(bool),
    Clear,
    /// Timed out on one island; once no island shows it, it waits in the missed list, or its app
    /// hears of it when transient.
    Expired(Notification),
    /// Dismissed or acted on, on one island; it goes from the others.
    Withdrawn(u32),
    History(Vec<Earlier>),
}

#[derive(Clone, Default)]
pub struct State {
    missed: Arc<Vec<Notification>>,
    /// This session's included.
    history: Arc<Vec<Earlier>>,
    dnd: bool,
    private: bool,
    locked: bool,
    /// Counts every change, so comparing two states does not compare every notification.
    version: u64,
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

#[derive(Default)]
pub struct Cards {
    toasts: Toasts,
    missed: MissedList,
    earlier: bool,
    dots: Dots,
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

pub struct Notifications {
    feed: Feed<State, Event>,
    /// The truth the published state copies.
    store: RefCell<State>,
    signals: Sender<Signal>,
    history: Sender<Write>,
    history_days: u32,
    private_apps: Vec<String>,
    hide_on_lock: bool,
}

impl Notifications {
    /// In place of one it replaces.
    fn remember(&self, notification: &Notification) {
        let earlier = Earlier {
            session: history::session(),
            notification: notification.clone(),
        };
        let days = self.history_days;
        let _ = self.history.send(Write::Record(earlier.clone()));
        self.change(|store| {
            let history = Arc::make_mut(&mut store.history);
            history.retain(|kept| kept.key() != earlier.key() && !history::aged(kept.notification.received, days));
            history.insert(0, earlier);
        });
    }

    /// Keeps what this session added meanwhile.
    fn load_history(&self, read: Vec<Earlier>) {
        self.change(|store| {
            let mut merged = read;
            for kept in store.history.iter() {
                if !merged.iter().any(|read| read.key() == kept.key()) {
                    merged.push(kept.clone());
                }
            }
            merged.sort_by(|a, b| b.notification.received.cmp(&a.notification.received));
            store.history = Arc::new(merged);
        });
    }

    /// Without what is in the missed list, and without actions, whose apps may be long gone.
    fn earlier(state: &State) -> Vec<Earlier> {
        let session = history::session();
        state
            .history
            .iter()
            .filter(|kept| !(kept.session == session && state.missed.iter().any(|missed| missed.id == kept.notification.id)))
            .map(|kept| Earlier {
                session: kept.session,
                notification: Notification {
                    actions: Vec::new(),
                    ..kept.notification.clone()
                },
            })
            .collect()
    }

    fn forget(&self, keys: Vec<(i64, u32)>) {
        if keys.is_empty() {
            return;
        }
        self.change(|store| Arc::make_mut(&mut store.history).retain(|kept| !keys.contains(&kept.key())));
        let _ = self.history.send(Write::Forget(keys));
    }

    fn clear_earlier(&self) {
        let keys = Self::earlier(&self.store.borrow()).iter().map(Earlier::key).collect();
        self.forget(keys);
    }

    fn change(&self, change: impl FnOnce(&mut State)) {
        let mut store = self.store.borrow_mut();
        change(&mut store);
        store.version += 1;
        self.feed.publish(store.clone());
    }

    fn signal(&self, signal: Signal) {
        let _ = self.signals.send(signal);
    }

    fn hidden(&self, state: &State, notification: &Notification) -> bool {
        state.private
            || (self.hide_on_lock && state.locked)
            || self.private_apps.contains(&notification.app.to_lowercase())
    }

    fn set_dnd(&self, switch: Switch) {
        self.change(|store| store.dnd = switch.apply(store.dnd));
    }

    fn set_private(&self, switch: Switch) {
        self.change(|store| store.private = switch.apply(store.private));
    }

    fn add_missed(&self, notification: Notification) {
        let mut dropped = Vec::new();
        self.change(|store| {
            let missed = Arc::make_mut(&mut store.missed);
            match missed.iter().position(|item| item.id == notification.id) {
                Some(index) => missed[index] = notification,
                None => missed.insert(0, notification),
            }
            dropped = missed.split_off(missed.len().min(KEEP));
        });
        for notification in dropped {
            self.signal(Signal::Closed(notification.id, Reason::Expired));
        }
    }

    fn take_missed(&self, id: u32) -> Option<Notification> {
        let index = self.store.borrow().missed.iter().position(|item| item.id == id)?;
        let mut taken = None;
        self.change(|store| taken = Some(Arc::make_mut(&mut store.missed).remove(index)));
        taken
    }

    fn dismiss_missed(&self, id: u32) {
        if self.take_missed(id).is_some() {
            self.signal(Signal::Closed(id, Reason::Dismissed));
        }
    }

    fn close_missed(&self, id: u32) {
        if self.take_missed(id).is_some() {
            self.signal(Signal::Closed(id, Reason::Closed));
        }
    }

    fn invoke_missed(&self, id: u32, key: &str) {
        if self.take_missed(id).is_some() {
            self.signal(Signal::Action(id, key.to_owned()));
            self.signal(Signal::Closed(id, Reason::Dismissed));
        }
    }

    fn activate_missed(&self, id: u32) {
        let default = self.store.borrow().missed.iter().any(|item| item.id == id && item.default_action());
        if default {
            self.invoke_missed(id, DEFAULT_ACTION);
        } else {
            self.dismiss_missed(id);
        }
    }

    fn clear(&self) {
        let mut cleared = Vec::new();
        self.change(|store| cleared = std::mem::take(Arc::make_mut(&mut store.missed)));
        for notification in cleared {
            self.signal(Signal::Closed(notification.id, Reason::Dismissed));
        }
    }

    /// One that timed out settles once no island shows it ([`Event::Expired`]); any other goes from
    /// every island, and its app hears of it.
    fn dismiss_card(&self, cards: &mut Cards, card: u64, reason: Reason) {
        let Some(notification) = cards.toasts.dismiss(card) else {
            return;
        };
        if reason == Reason::Expired {
            self.feed.send(Event::Expired(notification));
        } else {
            self.feed.send(Event::Withdrawn(notification.id));
            self.signal(Signal::Closed(notification.id, reason));
        }
    }

    fn activate_card(&self, cards: &mut Cards, card: u64) {
        let target = cards
            .toasts
            .cards()
            .iter()
            .find(|shown| shown.id == card && shown.notification.default_action())
            .map(|shown| shown.notification.id);
        if let Some(notification) = target {
            self.signal(Signal::Action(notification, DEFAULT_ACTION.to_owned()));
        }
        self.dismiss_card(cards, card, Reason::Dismissed);
    }

    fn invoke_card(&self, cards: &mut Cards, card: u64, key: &str) {
        let Some(shown) = cards.toasts.cards().iter().find(|shown| shown.id == card) else {
            return;
        };
        self.signal(Signal::Action(shown.notification.id, key.to_owned()));
        if !shown.notification.resident {
            self.dismiss_card(cards, card, Reason::Dismissed);
        }
    }
}

fn invoke(notification: &Notification, handler: impl Fn(String) -> Handler) -> Vec<(String, Handler)> {
    notification
        .buttons()
        .map(|action| (action.key.clone(), handler(action.key.clone())))
        .collect()
}

impl Feature for Notifications {
    const NAME: &'static str = "notifications";
    const PANEL: Option<&'static str> = Some("notifications");
    const PANEL_REFRESH: Option<Duration> = Some(REFRESH);
    const COMMANDS: &'static [&'static str] = &["dnd", "private", "clear"];
    const USAGE: &'static str = "  dnd on|off|toggle
  private on|off|toggle                 hide notification contents
  clear                                 dismiss every notification";

    type State = State;
    type Local = Cards;
    type Event = Event;

    fn start(feed: Feed<State, Event>) -> Self {
        let settings: Settings = feed.settings();
        let (signals, outgoing) = mpsc::channel();
        feed.clone().supervise(move |feed| server::serve(feed, &outgoing));
        feed.clone().supervise(lock::follow);
        let (history, writes) = mpsc::channel();
        let days = settings.history_days;
        feed.clone().supervise(move |feed| history::work(feed, &writes, days));
        let notifications = Self {
            feed,
            store: RefCell::default(),
            signals,
            history,
            history_days: settings.history_days,
            private_apps: settings.private_apps.iter().map(|app| app.to_lowercase()).collect(),
            hide_on_lock: settings.hide_on_lock,
        };
        notifications.change(|store| store.private = settings.private);
        notifications
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        parse_settings::<Settings>(settings).map(drop)
    }

    fn sync_local(state: &State, cards: &mut Cards, island: &IslandInfo<'_>, animate: bool) {
        cards.missed.sync(&state.missed, animate && island.panel_open, Instant::now() + REVEAL_DELAY);
        cards.dots.show(state.missed.len(), animate);
    }

    fn event(&self, event: Event, islands: &mut Islands<'_, Cards>) {
        match event {
            Event::Show(notification) => {
                self.remember(&notification);
                let shown = islands.find(|cards| cards.toasts.live_card_for(notification.id).is_some());
                let (missed, dnd) = {
                    let store = self.store.borrow();
                    (store.missed.iter().any(|item| item.id == notification.id), store.dnd)
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
                    let shown: Vec<u64> = cards.toasts.cards().iter().map(|shown| shown.id).collect();
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
                if islands.find(|cards| cards.toasts.live_card_for(notification.id).is_some()).is_none() {
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

    fn elapse(&self, cards: &mut Cards, hovered: bool) {
        let now = Instant::now();
        for card in cards.toasts.expired(now) {
            if hovered {
                cards.toasts.postpone(card, now + HOVER_GRACE);
            } else {
                self.dismiss_card(cards, card, Reason::Expired);
            }
        }
    }

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let missed = state.missed.len();
        let color = if missed > 0 && !state.dnd { ACCENT } else { OVERLAY1 };
        let icon = w.local::<Cards>().map(|cards| cards.dots.element(color));
        let tip = match (missed, state.dnd) {
            (0, false) => "No missed notifications".to_owned(),
            (0, true) => "Do not disturb is on".to_owned(),
            (missed, dnd) => format!(
                "{missed} missed notification{}{}",
                if missed == 1 { "" } else { "s" },
                if dnd { " · do not disturb" } else { "" }
            ),
        };
        Some(
            w.base("bar-notifications")
                .tip(w.tips(), "bar-notifications", tip)
                .children(icon)
                .on_click(w.toggle(Panel::Feature(Self::NAME)))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        let cards = p.local::<Cards>()?;
        let buttons = vec![
            Button {
                id: "missed-private".into(),
                label: "Private".into(),
                active: state.private,
                toggle: true,
                fits: Vec::new(),
                busy: false,
                enabled: true,
                on_click: p.call::<Self>(|notifications| notifications.set_private(Switch::Toggle)),
            },
            Button {
                id: "missed-dnd".into(),
                label: "Do not disturb".into(),
                active: state.dnd,
                toggle: true,
                fits: Vec::new(),
                busy: false,
                enabled: true,
                on_click: p.call::<Self>(|notifications| notifications.set_dnd(Switch::Toggle)),
            },
            Button {
                id: "missed-clear".into(),
                label: "Clear".into(),
                active: false,
                toggle: false,
                fits: Vec::new(),
                busy: false,
                enabled: if cards.earlier { !Self::earlier(state).is_empty() } else { !cards.missed.is_empty() },
                on_click: p.on_local::<Self>(|notifications, cards| {
                    if cards.earlier {
                        notifications.clear_earlier();
                    } else {
                        notifications.clear();
                    }
                }),
            },
        ];
        let header = Header::Views(Views {
            tabs: vec!["Now".into(), "Earlier".into()],
            selected: usize::from(cards.earlier),
            on_select: p.on_local_with::<Self, usize>(|_, cards, tab| cards.earlier = tab == 1),
            buttons,
        });
        if cards.earlier {
            let entries = Self::earlier(state);
            let looks = entries
                .iter()
                .map(|kept| {
                    let key = kept.key();
                    Look {
                        countdown: None,
                        hidden: self.hidden(state, &kept.notification),
                        activate: p.call::<Self>(|_| {}),
                        dismiss: p.call::<Self>(move |notifications| notifications.forget(vec![key])),
                        invoke: Vec::new(),
                    }
                })
                .collect();
            return Some(PanelView {
                header,
                blocks: vec![card::earlier(entries, looks)],
            });
        }
        let looks = cards
            .missed
            .rows()
            .iter()
            .map(|row| {
                let id = row.notification.id;
                Look {
                    countdown: None,
                    hidden: self.hidden(state, &row.notification),
                    activate: p.call::<Self>(move |notifications| notifications.activate_missed(id)),
                    dismiss: p.call::<Self>(move |notifications| notifications.dismiss_missed(id)),
                    invoke: invoke(&row.notification, |key| {
                        p.call::<Self>(move |notifications| notifications.invoke_missed(id, &key))
                    }),
                }
            })
            .collect();
        let empty = if state.dnd { "Nothing missed · do not disturb is on" } else { "Nothing missed" };
        Some(PanelView {
            header,
            blocks: vec![card::missed(&cards.missed, looks, empty)],
        })
    }

    fn popup<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        let cards = p.local::<Cards>()?;
        let looks = cards
            .toasts
            .cards()
            .iter()
            .map(|shown| {
                let id = shown.id;
                Look {
                    countdown: shown.countdown(),
                    hidden: self.hidden(state, &shown.notification),
                    activate: p.on_local::<Self>(move |notifications, cards| notifications.activate_card(cards, id)),
                    dismiss: p.on_local::<Self>(move |notifications, cards| {
                        notifications.dismiss_card(cards, id, Reason::Dismissed);
                    }),
                    invoke: invoke(&shown.notification, |key| {
                        p.on_local::<Self>(move |notifications, cards| notifications.invoke_card(cards, id, &key))
                    }),
                }
            })
            .collect();
        Some(PanelView {
            header: Header::None,
            blocks: vec![card::toasts(&cards.toasts, looks)],
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        match args {
            ["clear"] => Ok(()),
            ["clear", ..] => Err("clear takes no arguments".to_owned()),
            _ => control::switch(&args[1..]).map(drop),
        }
    }

    fn command(&self, _: &State, args: &[&str]) {
        if args == ["clear"] {
            self.feed.send(Event::Clear);
            return;
        }
        let Ok(switch) = control::switch(&args[1..]) else {
            return;
        };
        match args.first() {
            Some(&"dnd") => self.set_dnd(switch),
            Some(&"private") => self.set_private(switch),
            _ => {}
        }
    }
}
