use std::time::{Duration, SystemTime};

use chrono::Local;
use gpui::{
    Animation, AnimationExt, AnyElement, Context, FontStyle, FontWeight, HighlightStyle, Hsla, InteractiveText,
    MouseButton, ObjectFit, SharedString, StrikethroughStyle, StyledText, TextRun, UnderlineStyle, Window, div, font,
    prelude::*, px, relative, rgb, rgba,
};

use super::{
    history::Earlier,
    markdown,
    missed_list::MissedList,
    notification::{DEFAULT_ACTION, Notification},
    toasts::Toasts,
};
use crate::{
    components::{button, icon, motion},
    island::Island,
    panel::Handler,
    theme::{
        ACCENT, CARD_BORDER_ALPHA, CONTROL_ALPHA, CRITICAL, FONT, NOTIFICATION_ALPHA, ON_ACCENT, SUBTEXT1, SURFACE1,
        TEXT, app_color, with_alpha,
    },
    ui::{self, CONTROL_H, Radius, SMALL_SIZE, TEXT_SIZE, TIGHT_INSET, TITLE_SIZE},
};

pub const MIN_W: f32 = 380.0;
pub const GAP: f32 = 8.0;
pub const FOOTER_H: f32 = 18.0;
const PAD: f32 = 14.0;
const ICON: f32 = 36.0;
const ICON_GAP: f32 = 12.0;
const META_LINE: f32 = 16.0;
const SUMMARY_LINE: f32 = 18.0;
const BODY_SIZE: f32 = SMALL_SIZE;
const BODY_LINE: f32 = 17.0;
const BODY_LINES: usize = 2;
const ACTION_GAP: f32 = 8.0;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Countdown {
    /// 0–1.
    pub left: f32,
    /// `None` while paused.
    pub running: Option<Duration>,
    /// Counts each pause and resume, so the bar starts a new animation from where it is.
    pub round: u32,
}

const COUNTDOWN_H: f32 = 2.0;
/// How strongly the countdown shows; it is a quiet part of the card's edge, not content.
const COUNTDOWN_ALPHA: u8 = 0xb3;

/// Lies on the border's straight stretch between the rounded corners, so it reads as the outline
/// itself.
fn countdown_bar(kind: Kind, countdown: Countdown) -> AnyElement {
    let bar = div().h_full().rounded_full().bg(rgba(with_alpha(ACCENT, COUNTDOWN_ALPHA)));
    let bar = match countdown.running {
        Some(left) => bar
            .w(relative(countdown.left))
            .with_animation(
                kind.element_id(&format!("countdown-{}", countdown.round)),
                Animation::new(left),
                move |bar, t| bar.w(relative(countdown.left * (1.0 - t))),
            )
            .into_any_element(),
        None => bar.w(relative(countdown.left)).into_any_element(),
    };
    div()
        .absolute()
        .left(px(ui::PANEL_RADIUS))
        .right(px(ui::PANEL_RADIUS))
        .bottom_0()
        .h(px(COUNTDOWN_H))
        .child(bar)
        .into_any_element()
}

fn highlight(style: markdown::Style) -> HighlightStyle {
    let line = |shown: bool| shown.then(|| px(1.0));
    let tint: Hsla = rgba(with_alpha(SURFACE1, CONTROL_ALPHA)).into();
    let cover: Hsla = rgb(SURFACE1).into();
    HighlightStyle {
        font_weight: style.bold.then_some(FontWeight::SEMIBOLD),
        font_style: style.italic.then_some(FontStyle::Italic),
        underline: line(style.underline).map(|thickness| UnderlineStyle { thickness, ..UnderlineStyle::default() }),
        strikethrough: line(style.strike)
            .map(|thickness| StrikethroughStyle { thickness, ..StrikethroughStyle::default() }),
        color: if style.spoiler {
            Some(cover)
        } else {
            style.link.map(|_| rgb(ACCENT).into())
        },
        background_color: if style.spoiler {
            Some(cover)
        } else {
            style.code.then_some(tint)
        },
        ..HighlightStyle::default()
    }
}

fn body_text(kind: Kind, body: &str) -> AnyElement {
    let parsed = markdown::parse(body);
    let ranges = parsed.link_ranges();
    let highlights: Vec<_> = parsed.spans.iter().map(|(range, style)| (range.clone(), highlight(*style))).collect();
    let links = parsed.links;
    let styled = StyledText::new(parsed.text).with_highlights(highlights);
    if links.is_empty() {
        return styled.into_any_element();
    }
    InteractiveText::new(kind.element_id("links"), styled)
        .on_click(ranges, move |index, _, cx| {
            if let Some(url) = links.get(index) {
                cx.stop_propagation();
                cx.open_url(url);
            }
        })
        .into_any_element()
}

/// With the Markdown read and its lines joined, as the card shows it.
pub fn body_lines(body: &str, card_width: f32, window: &Window) -> usize {
    let body = markdown::parse(body).text;
    if body.is_empty() {
        return 0;
    }
    let run = TextRun {
        len: body.len(),
        font: gpui::Font {
            weight: FontWeight::MEDIUM,
            ..font(FONT)
        },
        color: rgb(TEXT).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let lines = window
        .text_system()
        .shape_text(
            SharedString::from(body),
            px(BODY_SIZE),
            &[run],
            Some(px(card_width - 2.0 * PAD - ICON - ICON_GAP)),
            None,
        )
        .map_or(1, |lines| lines.iter().map(|line| line.wrap_boundaries.len() + 1).sum());
    lines.min(BODY_LINES)
}

pub fn height(body_lines: usize, buttons: bool) -> f32 {
    let body = if body_lines == 0 {
        0.0
    } else {
        2.0 + BODY_LINE * body_lines as f32
    };
    let buttons = if buttons { ACTION_GAP + CONTROL_H } else { 0.0 };
    let text = META_LINE + 2.0 + SUMMARY_LINE + body;
    text.max(ICON) + buttons + 2.0 * PAD
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
    let minutes = received.elapsed().map_or(0, |elapsed| elapsed.as_secs() / 60);
    match minutes {
        0 => "now".to_owned(),
        1..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", minutes / 60),
        _ => chrono::DateTime::<Local>::from(received).format("%b %-d").to_string(),
    }
}

fn on(handler: Handler, stop: bool) -> impl Fn(&mut Island, &mut Context<Island>) + 'static {
    move |island, cx| {
        if stop {
            cx.stop_propagation();
        }
        handler(island, cx);
    }
}

pub fn card(kind: Kind, notification: &Notification, look: Look, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let redacted;
    let notification = if look.hidden {
        redacted = Notification {
            summary: "New notification".to_owned(),
            body: if notification.body.is_empty() { String::new() } else { "Contents hidden".to_owned() },
            actions: notification
                .actions
                .iter()
                .filter(|action| action.key == DEFAULT_ACTION)
                .cloned()
                .collect(),
            image: None,
            ..notification.clone()
        };
        &redacted
    } else {
        notification
    };
    let body_lines = match kind {
        Kind::Toast(_) => BODY_LINES,
        Kind::Missed(_) | Kind::Earlier(..) => 1,
    };
    let group = kind.element_id("group");
    let trailing = match kind {
        Kind::Toast(_) => div()
            .text_color(rgb(SUBTEXT1))
            .child(motion::text(kind.element_id("received"), received_label(notification.received)))
            .into_any_element(),
        Kind::Missed(_) | Kind::Earlier(..) => {
            let dismiss = on(look.dismiss.clone(), true);
            button::ghost(kind.element_id("dismiss"), hover)
                .opacity(0.0)
                .when(hover, |close| close.group_hover(group.clone(), |style| style.opacity(1.0)))
                .size(px(META_LINE))
                .text_color(rgb(SUBTEXT1))
                .when(hover, |close| close.hover(|style| style.text_color(rgb(TEXT))))
                .child("✕")
                .on_click(cx.listener(move |this, _, _, cx| dismiss(this, cx)))
                .into_any_element()
        }
    };
    let meta = match kind {
        Kind::Toast(_) => notification.app.clone(),
        Kind::Missed(_) => format!("{} · {}", notification.app, received_label(notification.received)),
        Kind::Earlier(..) => format!(
            "{} · {}",
            notification.app,
            chrono::DateTime::<Local>::from(notification.received).format("%H:%M")
        ),
    };
    let buttons = notification.buttons().next().is_some().then(|| {
        div()
            .mt(px(ACTION_GAP))
            .ml(px(ICON + ICON_GAP))
            .h(px(CONTROL_H))
            .flex()
            .gap(px(6.0))
            .children(notification.buttons().map(|action| {
                let invoke = look
                    .invoke
                    .iter()
                    .find(|(key, _)| *key == action.key)
                    .map(|(_, handler)| on(handler.clone(), true));
                let id = kind.element_id(&format!("action-{}", action.key));
                match &action.icon {
                    Some(glyph) => button::icon(id, Some(glyph), false, hover).size(px(CONTROL_H)),
                    None => button::button(id, action.label.clone(), false, hover),
                }
                .when_some(invoke, |button, invoke| button.on_click(cx.listener(move |this, _, _, cx| invoke(this, cx))))
            }))
            .into_any_element()
    });
    let tint = if notification.critical {
        CRITICAL
    } else {
        app_color(&notification.app)
    };
    let initial: String = notification
        .app
        .chars()
        .next()
        .into_iter()
        .flat_map(char::to_uppercase)
        .collect();
    let border = if notification.critical {
        Hsla::from(rgba(with_alpha(CRITICAL, 0xb3)))
    } else {
        Hsla::from(rgba(with_alpha(SURFACE1, CARD_BORDER_ALPHA)))
    };
    let background = ui::outlined(Radius::Panel, Hsla::from(rgba(with_alpha(SURFACE1, NOTIFICATION_ALPHA))), border);
    let activate = on(look.activate, false);
    let dismiss = on(look.dismiss, true);
    let tile = div()
        .size(px(ICON))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .relative()
        .map(|tile| match (&notification.image, &notification.icon) {
            (Some(image), _) => tile.child(icon::rounded(image, ICON, ObjectFit::Cover, ui::CONTROL_RADIUS)),
            (None, Some(app)) => tile.child(icon::picture(app, ICON, ObjectFit::Contain)),
            (None, None) => tile
                .child(ui::shape(Radius::Control, Some(rgb(tint).into())))
                .text_color(rgb(ON_ACCENT))
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(px(TITLE_SIZE))
                .child(initial),
        });
    let text = div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(META_LINE))
                .line_height(px(META_LINE))
                .flex()
                .items_center()
                .justify_between()
                .text_size(px(SMALL_SIZE))
                .text_color(rgb(SUBTEXT1))
                .child(meta)
                .child(trailing),
        )
        .child(
            div()
                .mt(px(2.0))
                .h(px(SUMMARY_LINE))
                .line_height(px(SUMMARY_LINE))
                .text_size(px(TEXT_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .when(notification.critical, |summary| summary.text_color(rgb(CRITICAL)))
                .truncate()
                .child(motion::text(kind.element_id("summary"), markdown::parse(&notification.summary).text)),
        )
        .when(!notification.body.is_empty(), |text| {
            text.child(
                div()
                    .mt(px(2.0))
                    .line_height(px(BODY_LINE))
                    .text_size(px(BODY_SIZE))
                    .line_clamp(body_lines)
                    .max_h(px(BODY_LINE * body_lines as f32))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(body_text(kind, &notification.body)),
            )
        });

    div()
        .id(kind.element_id("body"))
        .group(group)
        .relative()
        .size_full()
        .cursor_pointer()
        .when(hover, |card| card.hover(|style| style))
        .on_click(cx.listener(move |this, _, _, cx| activate(this, cx)))
        .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, _, cx| dismiss(this, cx)))
        .child(background)
        .child(ui::control(Radius::Panel, None, hover))
        .child(
            ui::base()
                .relative()
                .size_full()
                .p(px(PAD))
                .flex()
                .flex_col()
                .child(div().flex().items_center().gap(px(ICON_GAP)).child(tile).child(text))
                .children(buttons),
        )
        .when_some(look.countdown, |card, countdown| card.child(countdown_bar(kind, countdown)))
        .into_any_element()
}

/// `looks` is in the order of `rows.rows()`. States its own size from the cards' known heights
/// rather than being measured, and scrolls past the most it shows.
pub fn missed(rows: &MissedList, looks: Vec<Look>, empty: SharedString, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    if rows.is_empty() && rows.rows().is_empty() {
        return div()
            .min_w(px(MIN_W))
            .py(px(14.0))
            .flex()
            .justify_center()
            .text_size(px(SMALL_SIZE))
            .text_color(rgb(SUBTEXT1))
            .child(empty)
            .into_any_element();
    }
    let mut looks: Vec<Option<Look>> = looks.into_iter().map(Some).collect();
    let all = rows.rows();
    let ordered = (0..all.len())
        .filter(|index| all[*index].leaving)
        .chain((0..all.len()).filter(|index| !all[*index].leaving));
    let mut content = div().relative().min_w(px(MIN_W)).h(px(rows.extent()));
    for index in ordered {
        let row = &all[index];
        let alpha = row.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        if alpha <= 0.001 {
            continue;
        }
        content = content.child(
            div()
                .absolute()
                .left(px(row.x.value()))
                .top(px(row.y.value()))
                .w_full()
                .h(px(row.height()))
                .opacity(alpha)
                .child(card(Kind::Missed(row.notification.id), &row.notification, look, hover, cx)),
        );
    }
    div()
        .id("missed-list")
        .min_w(px(MIN_W))
        .h(px(rows.visible_height()))
        .overflow_y_scroll()
        .child(content)
        .into_any_element()
}

const DAY_H: f32 = 22.0;
/// How tall the Earlier list can be before it scrolls, as the missed list.
const EARLIER_MAX_H: f32 = 480.0;

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

fn earlier_extent(entries: &[Earlier]) -> f32 {
    let mut height = 0.0;
    let mut day = None;
    for kept in entries {
        let this = day_of(&kept.notification);
        if day != Some(this) {
            height += DAY_H;
            day = Some(this);
        }
        height += missed_height(&kept.notification) + GAP;
    }
    height
}

/// Up to where the list scrolls, or one line when it is empty.
pub fn earlier_height(entries: &[Earlier]) -> f32 {
    if entries.is_empty() { 0.0 } else { earlier_extent(entries).min(EARLIER_MAX_H) }
}

/// `looks` is in the order of `entries`.
pub fn earlier(entries: Vec<Earlier>, looks: Vec<Look>, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    if entries.is_empty() {
        return div()
            .min_w(px(MIN_W))
            .py(px(14.0))
            .flex()
            .justify_center()
            .text_size(px(SMALL_SIZE))
            .text_color(rgb(SUBTEXT1))
            .child("Nothing earlier")
            .into_any_element();
    }
    let mut list = div().flex().flex_col().min_w(px(MIN_W));
    let mut day = None;
    for (kept, look) in entries.iter().zip(looks) {
        let this = day_of(&kept.notification);
        if day != Some(this) {
            list = list.child(
                div()
                    .h(px(DAY_H))
                    .flex()
                    .items_center()
                    .text_size(px(SMALL_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(SUBTEXT1))
                    .child(day_label(this)),
            );
            day = Some(this);
        }
        let (session, id) = kept.key();
        list = list.child(
            div()
                .h(px(missed_height(&kept.notification)))
                .mb(px(GAP))
                .child(card(Kind::Earlier(session, id), &kept.notification, look, hover, cx)),
        );
    }
    div()
        .id("earlier-list")
        .min_w(px(MIN_W))
        .h(px(earlier_height(&entries)))
        .overflow_y_scroll()
        .child(list)
        .into_any_element()
}

/// `looks` is in the order of `stack.cards()`; `island_below` is how far the island reaches below
/// the top of the cards.
pub fn toasts(stack: &Toasts, looks: Vec<Look>, island_below: f32, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let mut looks: Vec<Option<Look>> = looks.into_iter().map(Some).collect();
    let all = stack.cards();
    let ordered = (0..all.len())
        .filter(|index| all[*index].leaving)
        .chain((0..all.len()).rev().filter(|index| !all[*index].leaving));
    let mut content = div()
        .relative()
        .min_w(px(MIN_W))
        .h(px(stack.height() - 2.0 * TIGHT_INSET));
    for index in ordered {
        let card = &all[index];
        let alpha = card.alpha.value().clamp(0.0, 1.0);
        let Some(look) = looks.get_mut(index).and_then(Option::take) else {
            continue;
        };
        if alpha <= 0.001 {
            continue;
        }
        content = content.child(
            div()
                .absolute()
                .left(px(card.x.value()))
                .top(px(card.y.value()))
                .w_full()
                .h(px(card.height()))
                .opacity(alpha)
                .child(self::card(Kind::Toast(card.id), &card.notification, look, hover, cx)),
        );
    }
    let footer = stack.footer.value().clamp(0.0, 1.0);
    if footer > 0.001 {
        content = content.child(
            div()
                .absolute()
                .left_0()
                .top(px(island_below - TIGHT_INSET - FOOTER_H))
                .w_full()
                .h(px(FOOTER_H))
                .flex()
                .justify_center()
                .items_center()
                .text_size(px(SMALL_SIZE))
                .text_color(rgb(SUBTEXT1))
                .opacity(footer)
                .child(motion::text("toasts-more", format!("+{} more", stack.footer_count()))),
        );
    }
    content.into_any_element()
}
