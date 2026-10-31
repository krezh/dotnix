//! Notifications as the panel shows them: each a raised frame holding an item, its text and its
//! actions, laid out as a stack of toasts, the missed list, or earlier ones by day.

use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

use chrono::Local;
use gpui::{SharedString, SpringConfig, Window};

use super::{
    Cards, Event, Notifications,
    earlier_list::{EarlierList, Entry as EarlierEntry},
    missed_list::{self, Body, MissedList, PEEKS},
    notification::{DEFAULT_ACTION, Notification, Reason, Signal},
    toasts::Toasts,
};
use crate::{
    components::{
        card::{self, Look as FrameLook},
        item::{self, Item, Lead, Tint, Trail, TrailButton},
        text,
    },
    features::PanelCx,
    markdown,
    panel::{
        self, Block, ButtonRow, Countdown, FrameView, Handler, IconButton, Mark, Place, Placed,
        Text, TextStyle, Tone,
    },
    theme::{CRITICAL, PEACH, SAPPHIRE, SUBTEXT1, app_color},
    ui::{CONTROL_H, INSET, TIGHT_INSET},
};

/// A pop-up's width.
pub const MIN_W: f32 = 520.0;
/// The panel's lists, filling a panel 760 wide.
pub const LIST_W: f32 = 760.0 - 2.0 * INSET;
/// The most lines of what a card says, in the panel's lists and pop-ups alike.
pub const LIST_LINES: usize = 5;
pub const GAP: f32 = 8.0;
pub const FOOTER_H: f32 = 18.0;
const ICON: f32 = 36.0;
pub const BODY_LINES: usize = LIST_LINES;
/// How far in from the front card's sides each edge showing below a folded group sits, and how
/// strongly it shows, nearest first.
const PEEK_INSETS: [f32; 2] = [12.0, 26.0];
const PEEK_OPACITY: [f32; 2] = [1.0, 0.6];
/// The frame each edge is the bottom of, tall enough for its rounded corners.
const PEEK_DRAWN: f32 = 40.0;
/// How far right a dismissed card slides as it fades out.
const SLIDE: f32 = 72.0;
const EMPTY_H: f32 = 52.0;
/// How a dismissed card leaves: sliding out and fading by halfway, then the room it took closes.
pub const FOLD: SpringConfig = SpringConfig::new(260.0, 30.0, 1.0);

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
                Some((name, view)) => panel::on(move |island, cx| {
                    then(island, cx);
                    island.open_named(&name, Some(&view), false, cx);
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
pub fn body_lines(body: &str, card_width: f32, most: usize, window: &Window) -> usize {
    let width = card_width - 2.0 * card::RAISED_PAD - ICON - item::LEAD_GAP;
    text::wrapped_lines(&markdown::parse(body).text, text::BODY_SIZE, width, window).min(most)
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

pub(super) fn next_label_change(received: SystemTime) -> Option<Duration> {
    let elapsed = received.elapsed().unwrap_or_default();
    let seconds = elapsed.as_secs();
    let step = match seconds {
        ..3600 => 60,
        ..86_400 => 3600,
        _ => return None,
    };
    Some(
        Duration::from_secs(step - (seconds % step))
            .saturating_sub(Duration::from_nanos(u64::from(elapsed.subsec_nanos()))),
    )
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
    let notification = if look.hidden {
        redacted(notification)
    } else {
        notification.clone()
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
            tint: None,
            glow: None,
            faded: false,
            size: Some(ICON),
            on_click: None,
            top: false,
        }),
        overline: Some(overline.into()),
        title: markdown::parse(&notification.summary).text.into(),
        title_color: notification.critical.then_some(CRITICAL),
        wrap: false,
        detail: SharedString::default(),
        tinted,
        trail: vec![trail],
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
            id: kind.element_id(&format!("action-{}", action.key)),
            glyph: action.icon.clone(),
            color: None,
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

fn place(y: f32, height: f32, opacity: f32) -> Place {
    Place {
        inset: 0.0,
        offset: 0.0,
        y,
        height,
        opacity,
        from_bottom: false,
        drawn: None,
    }
}

/// The edges of the cards folded behind a group's front one, showing below it as it is folded,
/// `peek` of the way out, nearest first.
fn peeks(id: u32, bottom: f32, count: usize, peek: f32, opacity: f32) -> Vec<(Place, Block)> {
    let mut y = bottom;
    PEEKS
        .iter()
        .zip(PEEK_INSETS)
        .zip(PEEK_OPACITY)
        .take(count)
        .enumerate()
        .map(|(index, ((height, inset), strength))| {
            let height = height * peek;
            let at = Place {
                inset,
                offset: 0.0,
                y,
                height,
                opacity: opacity * strength * peek,
                from_bottom: false,
                drawn: Some(PEEK_DRAWN),
            };
            y += height;
            let frame = FrameView {
                id: format!("missed-{id}-peek-{index}").into(),
                look: FrameLook::Raised { urgent: false },
                on_click: None,
                on_dismiss: None,
                countdown: None,
                selected: false,
                motion: card::Motion::Still,
                shown_height: None,
            };
            (at, Block::Frame(frame, Vec::new()))
        })
        .collect()
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
            lead: None,
            overline: None,
            title: app.to_owned().into(),
            title_color: None,
            wrap: false,
            detail: SharedString::default(),
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
        (place(0.0, EMPTY_H, 1.0 - outgoing), Block::note(empty))
    });
    let mut looks: Vec<Option<RowLook>> = looks.into_iter().map(Some).collect();
    let drawn = rows.drawn_y();
    let mut order: Vec<usize> = (0..rows.rows().len()).collect();
    order.sort_by_key(|&index| std::cmp::Reverse(rows.rows()[index].depth));
    let mut blocks = Vec::new();
    for index in order {
        let row = &rows.rows()[index];
        let alpha = row.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        let height = row.clip.unwrap_or(row.height());
        let shown_height = row.clip;
        // A leaving one slides out to the right and fades before the room it took closes.
        let out = if row.leaving {
            missed_list::sliding(row.fold.value())
        } else {
            0.0
        };
        let opacity = alpha * (1.0 - out);
        if opacity <= 0.001 {
            continue;
        }
        let mut at = place(drawn[index], shown_height.unwrap_or(height), opacity);
        at.offset = out * SLIDE;
        if row.peeks > 0
            && row.peek.value() > 0.001
            && let Some(notification) = row.notification()
        {
            blocks.extend(peeks(
                notification.id,
                drawn[index] + height,
                row.peeks,
                row.peek.value().clamp(0.0, 1.0),
                1.0 - out,
            ));
        }
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
        blocks.push((at, block));
    }
    let visible = if empty.is_some() {
        EMPTY_H
    } else {
        rows.visible_height()
    };
    blocks.extend(empty);
    Block::Placed(Placed {
        width: LIST_W,
        height: rows.extent().max(visible),
        visible: Some(visible),
        blocks,
    })
}

/// How tall each line over a list is, such as a source's trouble.
const ABOVE_H: f32 = 28.0;

/// `above` opening over `list`, `open` of the way out, the list moving down to make room.
pub fn opening(list: Block, above: Vec<Block>, open: f32) -> Vec<Block> {
    let open = open.clamp(0.0, 1.0);
    if above.is_empty() || open <= 0.001 {
        return vec![list];
    }
    let Block::Placed(mut placed) = list else {
        let mut blocks = above;
        blocks.push(list);
        return blocks;
    };
    let room = (above.len() as f32 * ABOVE_H + GAP) * open;
    for (place, _) in &mut placed.blocks {
        if !place.from_bottom {
            place.y += room;
        }
    }
    let lines = above.into_iter().enumerate().map(|(index, block)| {
        let at = Place {
            inset: 0.0,
            offset: 0.0,
            y: index as f32 * ABOVE_H * open,
            height: ABOVE_H * open,
            opacity: open,
            from_bottom: false,
            drawn: Some(ABOVE_H),
        };
        (at, block)
    });
    placed.blocks.splice(0..0, lines);
    placed.height += room;
    placed.visible = placed.visible.map(|visible| visible + room);
    vec![Block::Placed(placed)]
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
pub fn earlier(list: &EarlierList, looks: Vec<Option<Look>>) -> Block {
    let empty = list.is_empty().then(|| {
        let outgoing = list
            .rows()
            .iter()
            .map(|row| 1.0 - row.out())
            .fold(0.0, f32::max);
        (
            place(0.0, EMPTY_H, 1.0 - outgoing),
            Block::note("Nothing earlier"),
        )
    });
    let mut looks: Vec<Option<Look>> = looks;
    let drawn = list.drawn_y();
    let mut blocks = Vec::new();
    for (index, row) in list.rows().iter().enumerate() {
        let out = row.out();
        let opacity = row.alpha.value().clamp(0.0, 1.0) * (1.0 - out);
        if opacity <= 0.001 {
            continue;
        }
        let mut at = place(drawn[index], row.height(), opacity);
        at.offset = out * SLIDE;
        let block = match (&row.entry, looks.get_mut(index).and_then(Option::take)) {
            (EarlierEntry::Day(day), _) => Block::heading(day_label(*day)),
            (EarlierEntry::Card(kept), Some(look)) => {
                let (session, id) = kept.key();
                message(Kind::Earlier(session, id), &kept.notification, look, None)
            }
            (EarlierEntry::Card(_), None) => continue,
        };
        blocks.push((at, block));
    }
    let visible = if empty.is_some() {
        EMPTY_H
    } else {
        list.visible_height()
    };
    blocks.extend(empty);
    Block::Placed(Placed {
        width: LIST_W,
        height: list.extent().max(visible),
        visible: Some(visible),
        blocks,
    })
}

/// `looks` is in the order of `stack.cards()`. The newest is drawn on top, and the count of those
/// that do not fit keeps to the island's bottom edge.
pub fn toasts(stack: &Toasts, looks: Vec<Look>) -> Block {
    let mut looks: Vec<Option<Look>> = looks.into_iter().map(Some).collect();
    let all = stack.cards();
    let drawn = stack.drawn_y();
    let mut blocks = Vec::new();
    for index in (0..all.len()).rev() {
        let shown = &all[index];
        let alpha = shown.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        // A leaving one slides out to the right and fades before the room it took closes.
        let out = shown.out();
        let opacity = alpha * (1.0 - out);
        if opacity > 0.001 {
            let mut at = place(drawn[index], shown.height(), opacity);
            at.offset = out * SLIDE;
            blocks.push((
                at,
                message(Kind::Toast(shown.id), &shown.notification, look, None),
            ));
        }
    }
    let footer = stack.footer.value().clamp(0.0, 1.0);
    if footer > 0.001 {
        let at = Place {
            inset: 0.0,
            offset: 0.0,
            y: TIGHT_INSET,
            height: FOOTER_H,
            opacity: footer,
            from_bottom: true,
            drawn: None,
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
