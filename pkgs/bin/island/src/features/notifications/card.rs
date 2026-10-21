//! Notifications as the panel shows them: each a raised frame holding an item, its text and its
//! actions, laid out as a stack of toasts, the missed list, or earlier ones by day.

use std::time::SystemTime;

use chrono::Local;
use gpui::{SharedString, Window};

use super::{
    Cards, Event, Notifications,
    history::Earlier,
    markdown,
    missed_list::MissedList,
    notification::{DEFAULT_ACTION, Notification, Reason, Signal},
    toasts::Toasts,
};
use crate::{
    components::{
        card::{self, Look as FrameLook},
        item::{self, Item, Lead, Trail},
        text,
    },
    panel::{
        self, Block, ButtonRow, Countdown, FrameView, Handler, IconButton, Mark, Place, Placed,
        Text, TextStyle, Tone,
    },
    theme::{CRITICAL, app_color},
    ui::{CONTROL_H, TIGHT_INSET},
};

pub const MIN_W: f32 = 380.0;
pub const GAP: f32 = 8.0;
pub const FOOTER_H: f32 = 18.0;
const ICON: f32 = 36.0;
const BODY_LINES: usize = 2;
const DAY_H: f32 = 22.0;
const EMPTY_H: f32 = 52.0;
/// How tall the Earlier list can be before it scrolls, as the missed list.
const EARLIER_MAX_H: f32 = 480.0;

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
    pub hidden: bool,
    pub activate: Handler,
    pub dismiss: Handler,
    pub invoke: Vec<(String, Handler)>,
    pub countdown: Option<Countdown>,
}

pub(super) fn invoke(
    notification: &Notification,
    handler: impl Fn(String) -> Handler,
) -> Vec<(String, Handler)> {
    notification
        .buttons()
        .map(|action| (action.key.clone(), handler(action.key.clone())))
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

/// With the markup read and its lines joined, as the card shows it.
pub fn body_lines(body: &str, card_width: f32, window: &Window) -> usize {
    let width = card_width - 2.0 * card::RAISED_PAD - ICON - item::LEAD_GAP;
    text::wrapped_lines(&markdown::parse(body).text, text::BODY_SIZE, width, window).min(BODY_LINES)
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

pub fn missed_height(notification: &Notification) -> f32 {
    height(
        usize::from(!notification.body.is_empty()),
        notification.buttons().next().is_some(),
    )
}

pub fn toasts_height(heights: &[f32], footer: bool) -> f32 {
    let cards: f32 = heights.iter().sum::<f32>() + GAP * heights.len().saturating_sub(1) as f32;
    let footer = if footer { GAP + FOOTER_H } else { 0.0 };
    cards + footer + 2.0 * TIGHT_INSET
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

fn message(kind: Kind, notification: &Notification, look: Look) -> Block {
    let notification = if look.hidden {
        redacted(notification)
    } else {
        notification.clone()
    };
    let frame_id = kind.element_id("body");
    let (overline, trail) = match kind {
        Kind::Toast(_) => (
            notification.app.clone(),
            Trail::Faint(received_label(notification.received).into()),
        ),
        Kind::Missed(_) => (
            format!(
                "{} · {}",
                notification.app,
                received_label(notification.received)
            ),
            Trail::Close {
                group: frame_id.clone(),
                on_click: look.dismiss.clone(),
            },
        ),
        Kind::Earlier(..) => (
            format!(
                "{} · {}",
                notification.app,
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
    let mark = match (&notification.image, &notification.icon) {
        (Some(image), _) => Mark::Picture(image.clone(), true),
        (None, Some(app)) => Mark::Picture(app.clone(), false),
        (None, None) => Mark::Letter(notification.app.clone().into(), tint),
    };
    let item = Item {
        id: kind.element_id("item"),
        lead: Some(Lead {
            mark: Some(mark),
            tint: None,
            glow: None,
            faded: false,
            size: Some(ICON),
            on_click: None,
        }),
        overline: Some(overline.into()),
        title: markdown::parse(&notification.summary).text.into(),
        title_color: notification.critical.then_some(CRITICAL),
        detail: SharedString::default(),
        trail: vec![trail],
    };
    let lines = match kind {
        Kind::Toast(_) => BODY_LINES,
        Kind::Missed(_) | Kind::Earlier(..) => 1,
    };
    let under = if notification.body.is_empty() {
        Vec::new()
    } else {
        vec![Block::Text(body(&notification.body, lines))]
    };
    let buttons: Vec<IconButton> = notification
        .buttons()
        .map(|action| IconButton {
            id: kind.element_id(&format!("action-{}", action.key)),
            glyph: action.icon.clone(),
            rising: None,
            label: action.label.clone().into(),
            active: false,
            badge: None,
            on_click: look
                .invoke
                .iter()
                .find(|(key, _)| *key == action.key)
                .map_or_else(|| panel::on(|_, _| {}), |(_, handler)| handler.clone()),
        })
        .collect();
    let mut blocks = vec![Block::Item(item, under)];
    if !buttons.is_empty() {
        blocks.push(Block::Buttons(ButtonRow {
            buttons,
            compact: true,
            indent: ICON + item::LEAD_GAP,
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
    };
    Block::Frame(frame, blocks)
}

fn place(x: f32, y: f32, height: f32, opacity: f32) -> Place {
    Place {
        x,
        y,
        height,
        opacity,
        from_bottom: false,
    }
}

/// `looks` is in the order of `rows.rows()`. States its own size from the cards' known heights
/// rather than being measured, and scrolls past the most it shows.
pub fn missed(rows: &MissedList, looks: Vec<Look>, empty: &'static str) -> Block {
    let empty = rows.is_empty().then(|| {
        let outgoing = rows
            .rows()
            .iter()
            .map(|row| row.alpha.value().clamp(0.0, 1.0))
            .fold(0.0, f32::max);
        (place(0.0, 0.0, EMPTY_H, 1.0 - outgoing), Block::note(empty))
    });
    let mut looks: Vec<Option<Look>> = looks.into_iter().map(Some).collect();
    let all = rows.rows();
    // Leaving ones go underneath.
    let ordered = (0..all.len())
        .filter(|index| all[*index].leaving)
        .chain((0..all.len()).filter(|index| !all[*index].leaving));
    let mut blocks = Vec::new();
    for index in ordered {
        let row = &all[index];
        let alpha = row.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        if alpha > 0.001 {
            let at = place(row.x.value(), row.y.value(), row.height(), alpha);
            blocks.push((
                at,
                message(Kind::Missed(row.notification.id), &row.notification, look),
            ));
        }
    }
    let visible = if empty.is_some() {
        EMPTY_H
    } else {
        rows.visible_height()
    };
    blocks.extend(empty);
    Block::Placed(Placed {
        width: MIN_W,
        height: rows.extent().max(visible),
        visible: Some(visible),
        blocks,
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

fn day_of(notification: &Notification) -> chrono::NaiveDate {
    chrono::DateTime::<Local>::from(notification.received).date_naive()
}

/// `looks` is in the order of `entries`: each day's heading, then its notifications.
pub fn earlier(entries: Vec<Earlier>, looks: Vec<Look>) -> Block {
    if entries.is_empty() {
        return Block::Text(Text {
            min_width: Some(MIN_W),
            ..Text::new(Tone::Note, "Nothing earlier")
        });
    }
    let mut blocks = Vec::new();
    let mut y = 0.0;
    let mut day = None;
    for (kept, look) in entries.iter().zip(looks) {
        let this = day_of(&kept.notification);
        if day != Some(this) {
            blocks.push((place(0.0, y, DAY_H, 1.0), Block::heading(day_label(this))));
            y += DAY_H;
            day = Some(this);
        }
        let (session, id) = kept.key();
        let height = missed_height(&kept.notification);
        blocks.push((
            place(0.0, y, height, 1.0),
            message(Kind::Earlier(session, id), &kept.notification, look),
        ));
        y += height + GAP;
    }
    Block::Placed(Placed {
        width: MIN_W,
        height: y,
        visible: Some(y.min(EARLIER_MAX_H)),
        blocks,
    })
}

/// `looks` is in the order of `stack.cards()`. The newest is drawn on top, leaving ones
/// underneath, and the count of those that do not fit keeps to the island's bottom edge.
pub fn toasts(stack: &Toasts, looks: Vec<Look>) -> Block {
    let mut looks: Vec<Option<Look>> = looks.into_iter().map(Some).collect();
    let all = stack.cards();
    let ordered = (0..all.len())
        .filter(|index| all[*index].leaving)
        .chain((0..all.len()).rev().filter(|index| !all[*index].leaving));
    let mut blocks = Vec::new();
    for index in ordered {
        let shown = &all[index];
        let alpha = shown.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        if alpha > 0.001 {
            let at = place(shown.x.value(), shown.y.value(), shown.height(), alpha);
            blocks.push((
                at,
                message(Kind::Toast(shown.id), &shown.notification, look),
            ));
        }
    }
    let footer = stack.footer.value().clamp(0.0, 1.0);
    if footer > 0.001 {
        let at = Place {
            x: 0.0,
            y: TIGHT_INSET,
            height: FOOTER_H,
            opacity: footer,
            from_bottom: true,
        };
        blocks.push((at, Block::note(format!("+{} more", stack.footer_count()))));
    }
    Block::Placed(Placed {
        width: MIN_W,
        height: stack.height() - 2.0 * TIGHT_INSET,
        visible: None,
        blocks,
    })
}

#[cfg(test)]
#[path = "card_test.rs"]
mod tests;
