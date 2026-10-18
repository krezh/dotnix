use std::time::Duration;

use super::{
    Cards, Notifications, State,
    card::{self, Look},
};
use crate::{
    control::Switch,
    features::PanelCx,
    panel::{Button, Header, PanelView, Views},
};

/// So "5m" keeps up.
pub(super) const REFRESH: Duration = Duration::from_secs(30);

impl Notifications {
    pub(super) fn panel_view(&self, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
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
                enabled: if cards.earlier {
                    !Self::earlier(state).is_empty()
                } else {
                    !cards.missed.is_empty()
                },
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
                        dismiss: p
                            .call::<Self>(move |notifications| notifications.forget(vec![key])),
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
                    activate: p
                        .call::<Self>(move |notifications| notifications.activate_missed(id)),
                    dismiss: p.call::<Self>(move |notifications| notifications.dismiss_missed(id)),
                    invoke: card::invoke(&row.notification, |key| {
                        p.call::<Self>(move |notifications| notifications.invoke_missed(id, &key))
                    }),
                }
            })
            .collect();
        let empty = if state.dnd {
            "Nothing missed · do not disturb is on"
        } else {
            "Nothing missed"
        };
        Some(PanelView {
            header,
            blocks: vec![card::missed(&cards.missed, looks, empty)],
        })
    }
}
