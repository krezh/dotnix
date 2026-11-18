//! Notifications as the panel shows them: each a raised frame holding an item, its text and its
//! actions, laid out as a stack of toasts, the missed list, or earlier ones by day.

use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::Local;

use super::{
    Cards, Event, Notifications,
    earlier_list::{EarlierList, Entry as EarlierEntry},
    missed_list::{Body, MissedList},
    notification::{DEFAULT_ACTION, Notification, Reason, Signal},
    toasts::Toasts,
};
use crate::{
    components::measure::Measure,
    components::{
        card::{self, Look as FrameLook},
        item::{self, Item, Lead, Tint, Trail, TrailButton},
        stack::{self, Card as StackCard, Note, Peeks, Scroll, Stack},
        text,
    },
    features::PanelCx,
    markdown,
    panel::SharedString,
    panel::{
        self, Block, ButtonRow, Countdown, FrameView, Handler, IconButton, Mark, Text, TextStyle,
        Tone,
    },
    spring::SpringConfig,
    theme::{CRITICAL, PEACH, SAPPHIRE, SUBTEXT1, app_color},
    ui::{CONTROL_H, INSET, TIGHT_INSET},
    units,
};

/// A pop-up's width.
pub const MIN_W: f32 = 520.0;
/// The panel's lists, filling a panel 760 wide.
pub const LIST_W: f32 = 760.0 - 2.0 * INSET;
/// The most lines of what a card says, in the panel's lists and pop-ups alike.
pub const LIST_LINES: usize = 5;
const ICON: f32 = 36.0;
pub const BODY_LINES: usize = LIST_LINES;
/// How a dismissed card leaves: fading by halfway, then the room it took closes.
pub const FOLD: SpringConfig = SpringConfig::new(260.0, 32.2, 1.0);

#[derive(Clone, Copy)]
pub enum Kind {
    Toast(u64),
    Missed(u32),
    /// Session and id.
    Earlier(i64, u32),
}

impl Kind {
    fn element_id(self, part: &str) -> SharedString {
        match self {
            Kind::Toast(id) => format!("toast-{id}-{part}").into(),
            Kind::Missed(id) => format!("missed-{id}-{part}").into(),
            Kind::Earlier(session, id) => format!("earlier-{session}-{id}-{part}").into(),
        }
    }
}

pub struct Look {
    /// What another feature lends to lead it, in place of the app's icon.
    pub lead: Option<Mark>,
    /// At the lead's corner, the mark of where it came from.
    pub corner: Option<PathBuf>,
    pub hidden: bool,
    pub activate: Handler,
    pub dismiss: Handler,
    pub invoke: Vec<(String, Handler)>,
    pub countdown: Option<Countdown>,
}

/// The mark the feature `notification` names lends, if any.
pub(super) fn lead(notification: &Notification, p: &PanelCx<'_>) -> Option<Mark> {
    p.lead(notification.lead.as_deref()?)
}

/// The glyph a feature marks what it posts with, by its name.
pub(super) fn source_glyph(feature: &str, p: &PanelCx<'_>) -> Option<PathBuf> {
    p.glyph(&format!("notice-{feature}"))
}

/// The mark of the source it came through, for one that names it.
pub(super) fn corner(notification: &Notification, p: &PanelCx<'_>) -> Option<PathBuf> {
    let posted = notification.posted.as_ref().filter(|posted| posted.via)?;
    source_glyph(&posted.origin.feature, p)
}

/// An action that opens a panel opens it too, after doing what `handler` does.
pub(super) fn invoke(
    notification: &Notification,
    handler: impl Fn(String) -> Handler,
) -> Vec<(String, Handler)> {
    notification
        .buttons()
        .map(|action| {
            let then = handler(action.key.clone());
            let handler = match action.opens.clone() {
                Some((name, view)) => panel::on(move |ui| {
                    ui.run(&then);
                    ui.open_named(&name, Some(&view), false);
                }),
                None => then,
            };
            (action.key.clone(), handler)
        })
        .collect()
}

impl Notifications {
    /// One that timed out settles once no island shows it; any other goes from every island.
    pub(super) fn dismiss_card(&self, cards: &mut Cards, card: u64, reason: Reason) {
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

    pub(super) fn activate_card(&self, cards: &mut Cards, card: u64) {
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

    pub(super) fn invoke_card(&self, cards: &mut Cards, card: u64, key: &str) {
        let Some(shown) = cards.toasts.cards().iter().find(|shown| shown.id == card) else {
            return;
        };
        self.signal(Signal::Action(shown.notification.id, key.to_owned()));
        if !shown.notification.resident {
            self.dismiss_card(cards, card, Reason::Dismissed);
        }
    }
}

/// With the markup read and its lines joined, as the card shows it, up to `most`.
pub fn body_lines(body: &str, card_width: f32, most: usize, measure: &Measure<'_>) -> usize {
    let width = card_width - 2.0 * card::RAISED_PAD - ICON - item::LEAD_GAP;
    measure
        .wrapped_lines(&markdown::parse(body).text, text::BODY_SIZE, width)
        .min(most)
}

pub fn height(body_lines: usize, buttons: bool) -> f32 {
    let body = if body_lines == 0 {
        0.0
    } else {
        item::MESSAGE_GAP + text::BODY_LINE * body_lines as f32
    };
    let buttons = if buttons {
        card::RAISED_GAP + CONTROL_H
    } else {
        0.0
    };
    let text = item::OVERLINE_H + item::MESSAGE_GAP + item::HEADLINE_H + body;
    text.max(ICON) + buttons + 2.0 * card::RAISED_PAD
}

/// `lines` as measured for its text, which the panel's lists show up to [`LIST_LINES`] of.
pub fn missed_height(notification: &Notification, lines: usize) -> f32 {
    height(
        if notification.body.is_empty() {
            0
        } else {
            lines.clamp(1, LIST_LINES)
        },
        notification.buttons().next().is_some(),
    )
}

pub fn toasts_height(heights: &[f32], footer: bool) -> f32 {
    stack::height(heights, footer) + 2.0 * TIGHT_INSET
}

fn received_label(received: SystemTime) -> String {
    let minutes = received
        .elapsed()
        .map_or(0, |elapsed| elapsed.as_secs() / 60);
    match minutes {
        0 => "now".to_owned(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", minutes / 60),
        _ => chrono::DateTime::<Local>::from(received)
            .format("%b %-d")
            .to_string(),
    }
}

pub(super) fn next_label_change(received: SystemTime) -> Option<Duration> {
    let elapsed = received.elapsed().unwrap_or_default();
    (elapsed.as_secs() < 86_400).then(|| units::next_change(units::Label::Ago, elapsed))
}

/// The markup as styled text.
fn body(body: &str, lines: usize) -> Text {
    let parsed = markdown::parse(body);
    let links = parsed.link_ranges().into_iter().zip(parsed.links).collect();
    let spans = parsed
        .spans
        .into_iter()
        .map(|(range, style)| {
            let style = TextStyle {
                bold: style.bold,
                italic: style.italic,
                underline: style.underline,
                strike: style.strike,
                code: style.code,
                covered: style.spoiler,
                link: style.link.is_some(),
            };
            (range, style)
        })
        .collect();
    Text {
        spans,
        links,
        lines: Some(lines),
        ..Text::new(Tone::Body, parsed.text)
    }
}

/// What a private notification shows in place of what it says.
fn redacted(notification: &Notification) -> Notification {
    Notification {
        summary: "New notification".to_owned(),
        body: if notification.body.is_empty() {
            String::new()
        } else {
            "Contents hidden".to_owned()
        },
        actions: notification
            .actions
            .iter()
            .filter(|action| action.key == DEFAULT_ACTION)
            .cloned()
            .collect(),
        image: None,
        ..notification.clone()
    }
}

fn message(
    kind: Kind,
    notification: &Notification,
    look: Look,
    shown_height: Option<f32>,
) -> Block {
    let hidden;
    let notification = if look.hidden {
        hidden = redacted(notification);
        &hidden
    } else {
        notification
    };
    let frame_id = kind.element_id("body");
    let from = match &notification.posted {
        Some(posted) if posted.via => format!("{} via {}", notification.app, posted.origin.source),
        _ => notification.app.clone(),
    };
    let tinted = if notification.critical {
        vec![Tint::Overline(" · critical".into(), CRITICAL)]
    } else {
        notification
            .posted
            .as_ref()
            .and_then(|posted| posted.detail.as_ref())
            .map(|detail| Tint::Overline(format!(" · {detail}").into(), PEACH))
            .into_iter()
            .collect()
    };
    let (overline, trail) = match kind {
        Kind::Toast(_) => (
            from,
            Trail::Faint(received_label(notification.received).into()),
        ),
        Kind::Missed(_) => (
            format!("{from} · {}", received_label(notification.received)),
            Trail::Close {
                group: frame_id.clone(),
                on_click: look.dismiss.clone(),
            },
        ),
        Kind::Earlier(..) => (
            format!(
                "{from} · {}",
                chrono::DateTime::<Local>::from(notification.received).format("%H:%M")
            ),
            Trail::Close {
                group: frame_id.clone(),
                on_click: look.dismiss.clone(),
            },
        ),
    };
    let tint = if notification.critical {
        CRITICAL
    } else {
        app_color(&notification.app)
    };
    let mark = match (look.lead, &notification.image, &notification.icon) {
        (Some(lent), _, _) => lent,
        (None, Some(image), _) => Mark::Picture(image.clone(), true),
        (None, None, Some(app)) => Mark::Picture(app.clone(), false),
        (None, None, None) => Mark::Letter(notification.app.clone().into(), tint),
    };
    let mark = match look.corner {
        Some(glyph) => Mark::Marked(Box::new(mark), glyph, SAPPHIRE),
        None => mark,
    };
    let item = Item {
        id: kind.element_id("item"),
        lead: Some(Lead {
            mark: Some(mark),
            size: Some(ICON),
            ..Default::default()
        }),
        overline: Some(overline.into()),
        title: markdown::parse(&notification.summary).text.into(),
        title_color: notification.critical.then_some(CRITICAL),
        tinted,
        trail: vec![trail],
        ..Default::default()
    };
    let lines = match kind {
        Kind::Toast(_) => BODY_LINES,
        Kind::Missed(_) | Kind::Earlier(..) => LIST_LINES,
    };
    let under = if notification.body.is_empty() {
        Vec::new()
    } else {
        vec![Block::Text(body(&notification.body, lines))]
    };
    let buttons: Vec<IconButton> = notification
        .buttons()
        .map(|action| IconButton {
            glyph: action.icon.clone(),
            color: None,
            label: action.label.clone().into(),
            active: false,
            badge: None,
            on_click: look
                .invoke
                .iter()
                .find(|(key, _)| *key == action.key)
                .map_or_else(|| panel::on(|_| {}), |(_, handler)| handler.clone()),
        })
        .collect();
    let mut blocks = vec![Block::Item(item, under)];
    if !buttons.is_empty() {
        blocks.push(Block::Buttons(ButtonRow {
            fill: false,
            buttons,
            compact: true,
            indent: ICON + item::LEAD_GAP,
            menu: None,
            end: false,
        }));
    }
    let frame = FrameView {
        id: frame_id,
        look: FrameLook::Raised {
            urgent: notification.critical,
        },
        on_click: Some(look.activate),
        on_dismiss: Some(look.dismiss),
        countdown: look.countdown,
        selected: false,
        // Placed and moved by the toast stack itself.
        motion: card::Motion::Still,
        shown_height,
    };
    Block::Frame(frame, blocks)
}

/// What a group's heading does, and the glyphs on its buttons.
pub struct HeadLook {
    pub toggle: Handler,
    pub clear: Handler,
    pub more: Option<PathBuf>,
    pub close: Option<PathBuf>,
}

/// A missed row's handlers, as its kind needs.
pub enum RowLook {
    Card(Look),
    Head(HeadLook),
}

/// The app's name, where it came through, and how many it has, with a button spreading them out
/// or folding them back and a ✕ clearing them.
fn head(app: &str, count: usize, via: Option<&str>, open: bool, look: HeadLook) -> Block {
    let toggle = if open {
        "Fold".to_owned()
    } else {
        format!("Show {count}")
    };
    Block::Item(
        Item {
            id: format!("missed-head-{app}").into(),
            title: app.to_owned().into(),
            tinted: via
                .map(|source| Tint::Title(format!("via {source}").into(), SAPPHIRE))
                .into_iter()
                .chain([Tint::Title(format!("· {count}").into(), SUBTEXT1)])
                .collect(),
            trail: vec![
                Trail::Button(TrailButton {
                    label: Some(toggle.into()),
                    glyph: look.more,
                    turned: open,
                    tint: None,
                    on_click: look.toggle,
                }),
                Trail::Button(TrailButton {
                    label: None,
                    glyph: look.close,
                    turned: false,
                    tint: None,
                    on_click: look.clear,
                }),
            ],
            ..Default::default()
        },
        Vec::new(),
    )
}

/// `looks` is in the order of `rows.rows()`. States its own size from the cards' known heights
/// rather than being measured, and scrolls past the most it shows. Cards folded behind another
/// are drawn first, the furthest back first, so the front one covers them.
pub fn missed(rows: &MissedList, looks: Vec<RowLook>, empty: &'static str) -> Block {
    let empty = rows.is_empty().then(|| {
        let outgoing = rows
            .rows()
            .iter()
            .map(|row| row.fold.value().clamp(0.0, 1.0))
            .fold(0.0, f32::max);
        Note {
            text: empty.into(),
            opacity: 1.0 - outgoing,
        }
    });
    let mut looks: Vec<Option<RowLook>> = looks.into_iter().map(Some).collect();
    let drawn = rows.drawn_y();
    let mut order: Vec<usize> = (0..rows.rows().len()).collect();
    order.sort_by_key(|&index| std::cmp::Reverse(rows.rows()[index].depth));
    let mut cards = Vec::new();
    for index in order {
        let row = &rows.rows()[index];
        let alpha = row.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        let shown_height = row.clip;
        // A leaving one slides out to the right and fades before the room it took closes.
        let out = row.out();
        let peeks = row
            .notification()
            .filter(|_| row.peeks > 0)
            .map(|notification| Peeks {
                id: format!("missed-{}", notification.id).into(),
                count: row.peeks,
                out: row.peek.value().clamp(0.0, 1.0),
                opacity: 1.0 - out,
            });
        let block = match (&row.body, look) {
            (Body::Card(notification), RowLook::Card(look)) => message(
                Kind::Missed(notification.id),
                notification,
                look,
                shown_height,
            ),
            (
                Body::Head {
                    app,
                    count,
                    via,
                    open,
                },
                RowLook::Head(look),
            ) => head(app, *count, via.as_deref(), *open, look),
            _ => continue,
        };
        cards.push(StackCard {
            y: drawn[index],
            height: shown_height.unwrap_or(row.height()),
            opacity: alpha * (1.0 - out),
            peeks,
            block,
        });
    }
    stack::stack(Stack {
        width: LIST_W,
        cards,
        height: rows.extent(),
        scroll: Some(Scroll {
            visible: rows.visible_height(),
            still: rows.full_height(),
        }),
        empty,
        footer: None,
    })
}

fn day_label(day: chrono::NaiveDate) -> String {
    let today = Local::now().date_naive();
    match (today - day).num_days() {
        0 => "Today".to_owned(),
        1 => "Yesterday".to_owned(),
        _ => day.format("%A %-d %B").to_string(),
    }
}

/// `looks` is in the order of `list.rows()`, `None` for a day's heading. One dismissed slides out
/// and fades before those below rise into its place.
pub fn earlier(list: &EarlierList, mut looks: Vec<Option<Look>>) -> Block {
    let empty = list.is_empty().then(|| {
        let outgoing = list
            .rows()
            .iter()
            .map(|row| 1.0 - row.out())
            .fold(0.0, f32::max);
        Note {
            text: "Nothing earlier".into(),
            opacity: 1.0 - outgoing,
        }
    });
    let drawn = list.drawn_y();
    let mut cards = Vec::new();
    for (index, row) in list.rows().iter().enumerate() {
        let block = match (&row.entry, looks.get_mut(index).and_then(Option::take)) {
            (EarlierEntry::Day(day), _) => Block::heading(day_label(*day)),
            (EarlierEntry::Card(kept), Some(look)) => {
                let (session, id) = kept.key();
                message(Kind::Earlier(session, id), &kept.notification, look, None)
            }
            (EarlierEntry::Card(_), None) => continue,
        };
        cards.push(StackCard {
            y: drawn[index],
            height: row.height(),
            opacity: row.alpha.value().clamp(0.0, 1.0) * (1.0 - row.out()),
            peeks: None,
            block,
        });
    }
    stack::stack(Stack {
        width: LIST_W,
        cards,
        height: list.extent(),
        scroll: Some(Scroll {
            visible: list.visible_height(),
            still: list.full_height(),
        }),
        empty,
        footer: None,
    })
}

/// `looks` is in the order of `stack.cards()`. The newest is drawn on top, and the count of those
/// that do not fit keeps to the island's bottom edge.
pub fn toasts(stack: &Toasts, looks: Vec<Look>) -> Block {
    let mut looks: Vec<Option<Look>> = looks.into_iter().map(Some).collect();
    let all = stack.cards();
    let drawn = stack.drawn_y();
    let mut cards = Vec::new();
    for index in (0..all.len()).rev() {
        let shown = &all[index];
        let alpha = shown.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        // A leaving one rises back the way it came and fades before the room it took closes.
        cards.push(StackCard {
            y: drawn[index] - shown.lift(),
            height: shown.height(),
            opacity: alpha * (1.0 - shown.out()),
            peeks: None,
            block: message(Kind::Toast(shown.id), &shown.notification, look, None),
        });
    }
    stack::stack(Stack {
        width: MIN_W,
        cards,
        height: stack.height() - 2.0 * TIGHT_INSET,
        scroll: None,
        empty: None,
        footer: Some(Note {
            text: format!("+{} more", stack.footer_count()).into(),
            opacity: stack.footer.value().clamp(0.0, 1.0),
        }),
    })
}

#[cfg(test)]
#[path = "card_test.rs"]
mod tests;
