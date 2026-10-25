use super::{
    Cards, Notifications, State,
    card::{self, Look},
};
use crate::{
    control::Switch,
    features::PanelCx,
    panel::{Block, Button, Header, PanelView, TabLabel, Views},
};

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
        let header = Header::Tabs;
        let views = Block::Views(Views {
            lead: None,
            tabs: vec![
                TabLabel {
                    label: "Now".into(),
                    glyph: p.glyph("notifications-now"),
                },
                TabLabel {
                    label: "Earlier".into(),
                    glyph: p.glyph("notifications-earlier"),
                },
            ],
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
                        lead: card::lead(&kept.notification, p),
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
                blocks: vec![views, card::earlier(entries, looks)],
            });
        }
        let looks = cards
            .missed
            .rows()
            .iter()
            .map(|row| {
                if let Some(after) = card::next_label_change(row.notification.received) {
                    p.redraw_in(after);
                }
                let id = row.notification.id;
                Look {
                    lead: card::lead(&row.notification, p),
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
            blocks: vec![views, card::missed(&cards.missed, looks, empty)],
        })
    }
}
