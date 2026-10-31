use super::{
    Cards, Notifications, State,
    card::{self, HeadLook, Look, RowLook},
    cards::Filter,
    earlier_list::Entry as EarlierEntry,
    missed_list::{Body, Row},
    notification::Notification,
};
use std::path::PathBuf;

use crate::{
    components::{item::TrailButton, tabs},
    control::Switch,
    features::{
        PanelCx,
        notice::{self, Reply, Trouble},
    },
    panel::{
        Block, Choice, Dropdown, Header, IconButton, Item, Lead, Mark, PanelView, TabLabel, Trail,
        Views,
    },
    theme::{FLAMINGO, MAUVE, RED},
};

/// The trouble's glyph, as large as the text beside it.
const TROUBLE_GLYPH: f32 = 18.0;

/// Each kind of source among `notifications`, as the dropdown offers them, with how many each has.
fn sources<'a>(notifications: impl Iterator<Item = &'a Notification>) -> Vec<(Filter, usize)> {
    let mut found: Vec<(Filter, usize)> = Vec::new();
    for notification in notifications {
        let filter = notification
            .source()
            .map_or(Filter::Desktop, |source| Filter::Source(source.to_owned()));
        match found.iter_mut().find(|(known, _)| *known == filter) {
            Some((_, count)) => *count += 1,
            None => found.push((filter, 1)),
        }
    }
    found.sort_by_key(|(filter, _)| !matches!(filter, Filter::Desktop));
    found
}

fn label(filter: &Filter) -> String {
    match filter {
        Filter::All => "All sources".to_owned(),
        Filter::Desktop => "Desktop".to_owned(),
        Filter::Source(source) => source.clone(),
    }
}

/// The glyph a source's choice shows: island's own for all and for desktop apps, and for a
/// feature's source the one the feature marks what it posts with.
fn glyph<'a>(
    filter: &Filter,
    notifications: impl Iterator<Item = &'a Notification>,
    p: &PanelCx<'_>,
) -> Option<PathBuf> {
    match filter {
        Filter::All => p.glyph("notifications-all"),
        Filter::Desktop => p.glyph("notifications-desktop"),
        Filter::Source(source) => notifications
            .filter_map(|notification| notification.posted.as_ref())
            .find(|posted| posted.origin.source == *source)
            .and_then(|posted| card::source_glyph(&posted.origin.feature, p)),
    }
}

impl Notifications {
    fn pick<'a>(
        &self,
        cards: &Cards,
        found: Vec<(Filter, usize)>,
        notifications: impl Iterator<Item = &'a Notification> + Clone,
        p: &PanelCx<'_>,
    ) -> Dropdown {
        let total = found.iter().map(|(_, count)| count).sum();
        let mut filters = vec![(Filter::All, total)];
        filters.extend(found);
        if !filters.iter().any(|(filter, _)| *filter == cards.filter) {
            filters.push((cards.filter.clone(), 0));
        }
        let choices = filters
            .iter()
            .map(|(filter, count)| Choice {
                label: label(filter).into(),
                glyph: glyph(filter, notifications.clone(), p),
                detail: Some(count.to_string().into()),
            })
            .collect();
        let selected = filters
            .iter()
            .position(|(filter, _)| *filter == cards.filter);
        let filters: Vec<Filter> = filters.into_iter().map(|(filter, _)| filter).collect();
        Dropdown {
            id: "notifications-sources".into(),
            choices,
            selected,
            open: cards.picking,
            height: Some(tabs::STRIP_H),
            on_toggle: p.on_local::<Self>(|_, cards| cards.picking = !cards.picking),
            on_select: p.on_local_with::<Self, usize>(move |_, cards, index| {
                if let Some(filter) = filters.get(index) {
                    cards.set_filter(filter.clone());
                }
            }),
        }
    }

    /// Private and Do not disturb, then Clear on its own.
    fn icons(&self, state: &State, cards: &Cards, p: &PanelCx<'_>) -> Vec<Vec<IconButton>> {
        let private = if state.private {
            "Private · on"
        } else {
            "Private"
        };
        let dnd = if state.dnd {
            "Do not disturb · on"
        } else {
            "Do not disturb"
        };
        let earlier = cards.earlier;
        let switches = vec![
            IconButton {
                id: "notifications-private".into(),
                glyph: p.glyph("notifications-private"),
                color: Some(MAUVE),
                label: private.into(),
                active: state.private,
                badge: None,
                on_click: p.call::<Self>(|notifications| notifications.set_private(Switch::Toggle)),
            },
            IconButton {
                id: "notifications-dnd".into(),
                glyph: p.glyph("notifications-dnd"),
                color: Some(FLAMINGO),
                label: dnd.into(),
                active: state.dnd,
                badge: None,
                on_click: p.call::<Self>(|notifications| notifications.set_dnd(Switch::Toggle)),
            },
        ];
        let clear = vec![IconButton {
            id: "notifications-clear".into(),
            glyph: p.glyph("notifications-clear"),
            color: None,
            label: "Clear".into(),
            active: false,
            badge: None,
            on_click: p.on_local::<Self>(move |notifications, cards| {
                if earlier {
                    notifications.clear_earlier(&cards.filter);
                } else if cards.filter == Filter::All {
                    notifications.clear();
                } else {
                    let shown: Vec<u32> = cards
                        .missed
                        .rows()
                        .iter()
                        .filter(|row| !row.leaving)
                        .filter_map(|row| row.notification().map(|shown| shown.id))
                        .collect();
                    for id in shown {
                        notifications.dismiss_missed(id);
                    }
                }
            }),
        }];
        vec![switches, clear]
    }

    /// What is wrong with a source, in red over the list, with a button to try again.
    fn trouble(&self, feature: &str, trouble: &Trouble, p: &PanelCx<'_>) -> Block {
        let feature = feature.to_owned();
        let lead = p.glyph("notifications-offline").map(|glyph| Lead {
            mark: Some(Mark::Pulsing(Box::new(Mark::Glyph(glyph, RED)))),
            tint: None,
            glow: None,
            faded: false,
            size: Some(TROUBLE_GLYPH),
            on_click: None,
            top: false,
        });
        Block::Item(
            Item {
                id: format!("notifications-trouble-{feature}").into(),
                lead,
                overline: None,
                title: trouble.title.clone().into(),
                title_color: Some(RED),
                wrap: false,
                detail: trouble.detail.clone().into(),
                tinted: Vec::new(),
                trail: vec![Trail::Button(TrailButton {
                    label: Some("Retry now".into()),
                    glyph: None,
                    turned: false,
                    tint: Some(RED),
                    on_click: p.call::<Self>(move |_| notice::reply(&feature, Reply::Retry)),
                })],
            },
            Vec::new(),
        )
    }

    fn missed_looks(&self, state: &State, cards: &Cards, p: &PanelCx<'_>) -> Vec<RowLook> {
        cards
            .missed
            .rows()
            .iter()
            .map(|row| match &row.body {
                Body::Card(notification) => {
                    if let Some(after) = card::next_label_change(notification.received) {
                        p.redraw_in(after);
                    }
                    let id = notification.id;
                    let activate = if row.folded_front() {
                        let app = notification.app.clone();
                        p.on_local::<Self>(move |_, cards| cards.missed.toggle(&app))
                    } else {
                        p.call::<Self>(move |notifications| notifications.activate_missed(id))
                    };
                    RowLook::Card(Look {
                        lead: card::lead(notification, p),
                        corner: card::corner(notification, p),
                        countdown: None,
                        hidden: self.hidden(state, notification),
                        activate,
                        dismiss: p
                            .call::<Self>(move |notifications| notifications.dismiss_missed(id)),
                        invoke: card::invoke(notification, |key| {
                            p.call::<Self>(move |notifications| {
                                notifications.invoke_missed(id, &key)
                            })
                        }),
                    })
                }
                Body::Head { app, .. } => {
                    let ids: Vec<u32> = cards
                        .missed
                        .rows()
                        .iter()
                        .filter(|row| !row.leaving)
                        .filter_map(Row::notification)
                        .filter(|shown| shown.app == *app)
                        .map(|shown| shown.id)
                        .collect();
                    let app = app.clone();
                    RowLook::Head(HeadLook {
                        toggle: p.on_local::<Self>(move |_, cards| cards.missed.toggle(&app)),
                        clear: p.call::<Self>(move |notifications| {
                            for id in &ids {
                                notifications.dismiss_missed(*id);
                            }
                        }),
                        more: p.glyph("notifications-more"),
                        close: p.glyph("notifications-close"),
                    })
                }
            })
            .collect()
    }

    pub(super) fn panel_view(&self, state: &State, p: &PanelCx<'_>) -> Option<PanelView> {
        let cards = p.local::<Cards>()?;
        let earlier = Self::earlier(state);
        let found = if cards.earlier {
            sources(earlier.iter().map(|kept| &kept.notification))
        } else {
            sources(state.missed.iter())
        };
        let pick = if cards.earlier {
            self.pick(
                cards,
                found,
                earlier.iter().map(|kept| &kept.notification),
                p,
            )
        } else {
            self.pick(cards, found, state.missed.iter(), p)
        };
        let views = Block::Views(Views {
            lead: None,
            tabs: vec![
                TabLabel {
                    label: "Now".into(),
                    glyph: None,
                    count: (!state.missed.is_empty())
                        .then(|| state.missed.len().to_string().into()),
                },
                TabLabel {
                    label: "Earlier".into(),
                    glyph: None,
                    count: None,
                },
            ],
            selected: usize::from(cards.earlier),
            on_select: p.on_local_with::<Self, usize>(|_, cards, tab| {
                cards.earlier = tab == 1;
                cards.picking = false;
            }),
            pick: Some(pick),
            buttons: Vec::new(),
            icons: self.icons(state, cards, p),
        });
        let mut blocks = vec![views];
        let troubles: Vec<Block> = cards
            .troubles
            .shown
            .iter()
            .map(|(feature, trouble)| self.trouble(feature, trouble, p))
            .collect();
        let open = cards.troubles.open.value();
        if cards.earlier {
            let looks = cards
                .earlier_list
                .rows()
                .iter()
                .map(|row| {
                    let EarlierEntry::Card(kept) = &row.entry else {
                        return None;
                    };
                    let key = kept.key();
                    Some(Look {
                        lead: card::lead(&kept.notification, p),
                        corner: card::corner(&kept.notification, p),
                        countdown: None,
                        hidden: self.hidden(state, &kept.notification),
                        activate: p.call::<Self>(|_| {}),
                        dismiss: p
                            .call::<Self>(move |notifications| notifications.forget(vec![key])),
                        invoke: Vec::new(),
                    })
                })
                .collect();
            blocks.extend(card::opening(
                card::earlier(&cards.earlier_list, looks),
                troubles,
                open,
            ));
            return Some(PanelView {
                header: Header::Tabs,
                blocks,
                scroll: false,
            });
        }
        let empty = if state.dnd {
            "Nothing missed · do not disturb is on"
        } else {
            "Nothing missed"
        };
        let looks = self.missed_looks(state, cards, p);
        blocks.extend(card::opening(
            card::missed(&cards.missed, looks, empty),
            troubles,
            open,
        ));
        Some(PanelView {
            scroll: false,
            header: Header::Tabs,
            blocks,
        })
    }
}
