//! Notifications: island's notification server. New notifications pop up as cards on the focused
//! island; ones that time out unseen, or arrive during do not disturb, wait in the notifications
//! panel. A dot in the bar shows whether any are waiting, and `island ctl dnd` and
//! `island ctl private` switch do not disturb and hidden contents.

mod card;
mod lock;
mod missed_list;
mod notification;
mod server;
mod toasts;

use std::{
    cell::RefCell,
    hash::{DefaultHasher, Hash, Hasher},
    sync::{
        Arc,
        mpsc::{self, Sender},
    },
    time::{Duration, Instant},
};

use gpui::{AnyElement, Window, div, prelude::*, px, rgb};
use serde::Deserialize;

use self::{
    card::Look,
    missed_list::MissedList,
    notification::{DEFAULT_ACTION, Notification, Reason, Signal},
    toasts::Toasts,
};
use super::{Feature, Feed, IslandInfo, Islands, Local, PanelCx, WidgetCx, parse_settings};
use crate::{
    components::tooltip::Tip,
    control::{self, Switch},
    island::{Panel, REVEAL_DELAY},
    panel::{Block, Button, Handler, Header, PanelView},
    theme::{ACCENT, OVERLAY1},
};

/// How many missed notifications are kept; older ones are dropped.
const KEEP: usize = 50;
/// How much longer a card stays when its time comes while the pointer is over the island.
const HOVER_GRACE: Duration = Duration::from_millis(1500);
/// How often the open panel redraws, so "5m" keeps up.
const REFRESH: Duration = Duration::from_secs(30);

/// `[notifications]` in the config file.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Settings {
    /// Hide every notification's contents to begin with.
    private: bool,
    /// Apps whose contents are always hidden.
    private_apps: Vec<String>,
    /// Hide contents while the session is locked.
    hide_on_lock: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            private: false,
            private_apps: Vec::new(),
            hide_on_lock: true,
        }
    }
}

pub enum Event {
    /// An app sent a notification, new or in place of one it sent before.
    Show(Notification),
    /// An app took its notification back.
    Close(u32),
    /// The session was locked or unlocked.
    Locked(bool),
}

/// The missed notifications, newest first, and the switches every island shares.
#[derive(Clone, Default)]
pub struct State {
    missed: Arc<Vec<Notification>>,
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

/// An island's cards: those popping up, and the missed list in its panel.
#[derive(Default)]
pub struct Cards {
    toasts: Toasts,
    missed: MissedList,
}

impl Local for Cards {
    fn advance(&mut self, dt: f32, now: Instant) {
        self.toasts.advance(dt, now);
        self.missed.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.toasts.is_settled() && self.missed.is_settled()
    }

    fn popup(&self) -> bool {
        !self.toasts.is_empty()
    }

    fn popup_shown(&mut self, shown: bool) {
        self.toasts.set_shown(shown);
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
    private_apps: Vec<String>,
    hide_on_lock: bool,
}

impl Notifications {
    fn change(&self, change: impl FnOnce(&mut State)) {
        let mut store = self.store.borrow_mut();
        change(&mut store);
        store.version += 1;
        self.feed.publish(store.clone());
    }

    fn signal(&self, signal: Signal) {
        let _ = self.signals.send(signal);
    }

    /// Whether `notification`'s contents are hidden.
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

    /// Takes `id` out of the missed list.
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

    /// Takes `card` away for `reason`. One that timed out waits in the missed list unless its app
    /// called it transient; the app hears of the others.
    fn dismiss_card(&self, cards: &mut Cards, card: u64, reason: Reason) {
        let Some(notification) = cards.toasts.dismiss(card) else {
            return;
        };
        if reason == Reason::Expired && !notification.transient {
            self.add_missed(notification);
        } else {
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

/// A shape key for a block, from anything that decides its size.
fn shape(of: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    of.hash(&mut hasher);
    hasher.finish()
}

/// Each action button's handler, by its key, from `handler` for a key.
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
    const COMMANDS: &'static [&'static str] = &["dnd", "private"];
    const USAGE: &'static str = "  dnd on|off|toggle
  private on|off|toggle                 hide notification contents";

    type State = State;
    type Local = Cards;
    type Event = Event;

    fn start(feed: Feed<State, Event>) -> Self {
        let settings: Settings = feed.settings();
        let (signals, outgoing) = mpsc::channel();
        feed.clone().supervise(move |feed| server::serve(feed, &outgoing));
        feed.clone().supervise(lock::follow);
        let notifications = Self {
            feed,
            store: RefCell::default(),
            signals,
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
    }

    fn event(&self, event: Event, islands: &mut Islands<'_, Cards>) {
        match event {
            Event::Show(notification) => {
                let shown = islands.find(|cards| cards.toasts.live_card_for(notification.id).is_some());
                let (missed, dnd) = {
                    let store = self.store.borrow();
                    (store.missed.iter().any(|item| item.id == notification.id), store.dnd)
                };
                if missed || (dnd && shown.is_none() && !notification.critical) {
                    self.add_missed(notification);
                } else if let Some(island) = shown.or_else(|| islands.focused()) {
                    islands.update(island, |cards| cards.toasts.show(notification));
                }
            }
            Event::Close(id) => {
                islands.update_all(|cards| {
                    if cards.toasts.close(id).is_some() {
                        self.signal(Signal::Closed(id, Reason::Closed));
                    }
                });
                self.close_missed(id);
            }
            Event::Locked(locked) => self.change(|store| store.locked = locked),
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
        let color = if missed > 0 { ACCENT } else { OVERLAY1 };
        let icon = div().size(px(10.0)).rounded_full();
        let icon = if state.dnd {
            icon.border_2().border_color(rgb(color))
        } else {
            icon.bg(rgb(color))
        };
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
                .child(icon)
                .on_click(w.toggle(Panel::Feature(Self::NAME)))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let cards = p.local::<Cards>()?;
        let mut buttons = vec![
            Button {
                id: "missed-private".into(),
                label: "Private".into(),
                active: state.private,
                toggle: true,
                on_click: p.call::<Self>(|notifications| notifications.set_private(Switch::Toggle)),
            },
            Button {
                id: "missed-dnd".into(),
                label: "Do not disturb".into(),
                active: state.dnd,
                toggle: true,
                on_click: p.call::<Self>(|notifications| notifications.set_dnd(Switch::Toggle)),
            },
        ];
        if !cards.missed.is_empty() {
            buttons.push(Button {
                id: "missed-clear".into(),
                label: "Clear".into(),
                active: false,
                toggle: false,
                on_click: p.call::<Self>(Self::clear),
            });
        }
        let looks = cards
            .missed
            .rows()
            .iter()
            .map(|row| {
                let id = row.notification.id;
                Look {
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
        let rows = &cards.missed;
        Some(PanelView {
            header: Header::Title("Notifications".into(), buttons),
            blocks: vec![Block::Custom(
                shape((rows.is_empty(), rows.target_height().to_bits())),
                Box::new(move |frame, _, cx| card::missed(rows, looks, empty.into(), frame.hover, cx)),
            )],
        })
    }

    fn popup<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let cards = p.local::<Cards>()?;
        let looks = cards
            .toasts
            .cards()
            .iter()
            .map(|shown| {
                let id = shown.id;
                Look {
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
        let stack = &cards.toasts;
        Some(PanelView {
            header: Header::None,
            blocks: vec![Block::Custom(
                shape(stack.height().to_bits()),
                Box::new(move |frame, below, cx| card::toasts(stack, looks, below, frame.hover, cx)),
            )],
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        control::switch(&args[1..]).map(drop)
    }

    fn command(&self, _: &State, args: &[&str]) {
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
