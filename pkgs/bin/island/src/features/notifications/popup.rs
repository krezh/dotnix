use super::{
    Cards, Notifications, State,
    card::{self, Look},
    notification::Reason,
};
use crate::{
    features::PanelCx,
    panel::{Header, PanelView},
};

impl Notifications {
    pub(super) fn popup_view(&self, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
        let cards = p.local::<Cards>()?;
        let looks = cards
            .toasts
            .cards()
            .iter()
            .map(|shown| {
                if let Some(after) = card::next_label_change(shown.notification.received) {
                    p.redraw_in(after);
                }
                let id = shown.id;
                Look {
                    lead: card::lead(&shown.notification, p),
                    corner: card::corner(&shown.notification, p),
                    countdown: shown.countdown(),
                    hidden: self.hidden(state, &shown.notification),
                    activate: p.on_local::<Self>(move |notifications, cards| {
                        notifications.activate_card(cards, id)
                    }),
                    dismiss: p.on_local::<Self>(move |notifications, cards| {
                        notifications.dismiss_card(cards, id, Reason::Dismissed);
                    }),
                    invoke: card::invoke(&shown.notification, |key| {
                        p.on_local::<Self>(move |notifications, cards| {
                            notifications.invoke_card(cards, id, &key)
                        })
                    }),
                }
            })
            .collect();
        Some(PanelView {
            scroll: false,
            header: Header::None,
            blocks: vec![card::toasts(&cards.toasts, looks)],
        })
    }
}
