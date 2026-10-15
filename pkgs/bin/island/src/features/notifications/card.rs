//! A notification card, and the two stacks of them: the cards popping up as notifications arrive and
//! the missed list in the panel.

use std::time::SystemTime;

use chrono::Local;
use gpui::{
    AnyElement, Context, FontWeight, Hsla, MouseButton, ObjectFit, SharedString, TextRun, Window, div, font,
    prelude::*, px, rgb, rgba,
};

use super::{
    missed_list::MissedList,
    notification::{DEFAULT_ACTION, Notification},
    toasts::Toasts,
};
use crate::{
    components::{button, icon},
    island::Island,
    panel::Handler,
    theme::{
        CARD_BORDER_ALPHA, CRITICAL, FONT, NOTIFICATION_ALPHA, ON_ACCENT, SUBTEXT1, SURFACE1, TEXT, app_color,
        with_alpha,
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

/// Which stack a card is in, and its id there.
#[derive(Clone, Copy)]
pub enum Kind {
    Toast(u64),
    Missed(u32),
}

impl Kind {
    fn element_id(self, part: &str) -> SharedString {
        match self {
            Kind::Toast(id) => format!("toast-{id}-{part}").into(),
            Kind::Missed(id) => format!("missed-{id}-{part}").into(),
        }
    }
}

/// How a card is drawn and what clicking it does.
pub struct Look {
    /// Whether its contents are hidden.
    pub hidden: bool,
    pub activate: Handler,
    pub dismiss: Handler,
    /// Each action button's handler, by its key.
    pub invoke: Vec<(String, Handler)>,
}

pub fn body_lines(body: &str, card_width: f32, window: &Window) -> usize {
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
            SharedString::from(body.to_owned()),
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
    let text = META_LINE + 2.0 + SUMMARY_LINE + body + buttons;
    text.max(ICON) + 2.0 * PAD
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
        Kind::Missed(_) => 1,
    };
    let group = kind.element_id("group");
    let trailing = match kind {
        Kind::Toast(_) => div()
            .text_color(rgb(SUBTEXT1))
            .child(received_label(notification.received))
            .into_any_element(),
        Kind::Missed(_) => {
            let dismiss = on(look.dismiss.clone(), true);
            div()
                .id(kind.element_id("dismiss"))
                .opacity(0.0)
                .when(hover, |close| close.group_hover(group.clone(), |style| style.opacity(1.0)))
                .size(px(META_LINE))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.0))
                .cursor_pointer()
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
    };
    let buttons = notification.buttons().next().is_some().then(|| {
        div()
            .mt(px(ACTION_GAP))
            .h(px(CONTROL_H))
            .flex()
            .gap(px(6.0))
            .children(notification.buttons().map(|action| {
                let invoke = look
                    .invoke
                    .iter()
                    .find(|(key, _)| *key == action.key)
                    .map(|(_, handler)| on(handler.clone(), true));
                button::button(kind.element_id(&format!("action-{}", action.key)), action.label.clone(), false, hover)
                    .when_some(invoke, |button, invoke| {
                        button.on_click(cx.listener(move |this, _, _, cx| invoke(this, cx)))
                    })
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

    div()
        .id(kind.element_id("body"))
        .group(group)
        .relative()
        .size_full()
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| activate(this, cx)))
        .on_mouse_down(MouseButton::Right, cx.listener(move |this, _, _, cx| dismiss(this, cx)))
        .child(background)
        .child(
            ui::base()
                .relative()
                .size_full()
                .p(px(PAD))
                .flex()
                .items_start()
                .gap(px(ICON_GAP))
                .child(
                    div()
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
                                .child(ui::shape(Radius::Control, Some(rgb(tint).into()), None))
                                .text_color(rgb(ON_ACCENT))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_size(px(TITLE_SIZE))
                                .child(initial),
                        }),
                )
                .child(
                    div()
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
                                .child(notification.summary.clone()),
                        )
                        .when(!notification.body.is_empty(), |text| {
                            text.child(
                                div()
                                    .mt(px(2.0))
                                    .line_height(px(BODY_LINE))
                                    .text_size(px(BODY_SIZE))
                                    .line_clamp(body_lines)
                                    .text_ellipsis()
                                    .child(notification.body.clone()),
                            )
                        })
                        .children(buttons),
                ),
        )
        .into_any_element()
}

/// The missed list; `looks` holds each row's look, in the order of `rows.rows()`.
pub fn missed(rows: &MissedList, looks: Vec<Look>, empty: SharedString, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    if rows.is_empty() && rows.rows().is_empty() {
        return div()
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
    content.into_any_element()
}

/// The cards popping up; `looks` holds each card's look, in the order of `stack.cards()`, and
/// `island_below` is how far the island reaches below the top of the cards.
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
                .child(format!("+{} more", stack.footer_count())),
        );
    }
    content.into_any_element()
}
