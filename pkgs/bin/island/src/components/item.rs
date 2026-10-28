//! A row in a panel: a lead, a title line ending in amounts, statuses or a dropdown, and
//! whatever it holds under the title, such as a slider. With a line over the title it reads as a
//! message: a faint line naming where it is from, then the title, larger, then what it says.

use std::path::PathBuf;

use gpui::{
    AnyElement, BoxShadow, Context, FontWeight, SharedString, div, point, prelude::*, px, rgb,
    rgba,
};

use crate::{
    components::{
        actions::{self, Actions},
        badge::{self, Mark},
        button,
        dropdown::{self, Dropdown},
        icon, motion,
        popover::Popovers,
        tooltip::Tooltips,
    },
    island::Island,
    panel::Handler,
    theme::{CRUST, OVERLAY2, SUBTEXT1, TEXT, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, CONTROL_RADIUS, Radius, SMALL_SIZE, TEXT_SIZE},
};

/// A glyph at a row's end, such as a lock.
const MARK: f32 = 14.0;
/// A tinted lead's glow under it, in its color.
const TINT_GLOW_ALPHA: u8 = 0x4d;
const TINT_GLOW_DROP: f32 = 4.0;
const TINT_GLOW_BLUR: f32 = 14.0;
/// A glyph on a tinted lead.
const ON_TINT: u32 = CRUST;
/// Between the parts at a row's end.
const CONTROLS_GAP: f32 = 10.0;
/// The most a lead's glow shows, at full strength.
const GLOW_ALPHA: f32 = 96.0;
/// Keeps a changing amount, such as "50%" to "Muted", from moving what is beside it.
const AMOUNT_W: f32 = 40.0;
const LEAD: f32 = 40.0;
/// Between the lead and the text.
pub const LEAD_GAP: f32 = 10.0;
/// Between a row's title line and what is under it.
const LINE_GAP: f32 = 2.0;
/// A row's title: a touch larger than what is beside it, in medium weight so a list stays light.
const ROW_TITLE: f32 = 13.5;
/// The line over the title, and the title under it.
pub const OVERLINE_H: f32 = 16.0;
pub const HEADLINE_H: f32 = 18.0;
/// Between the lines of a message.
pub const MESSAGE_GAP: f32 = 2.0;

/// What sits left of the title.
pub struct Lead {
    pub mark: Option<Mark>,
    /// A solid square of this color behind the mark, which is drawn dark on it.
    pub tint: Option<u32>,
    /// A color showing through at a strength from 0 to 1, such as a level heard right now.
    pub glow: Option<(u32, f32)>,
    /// Drawn faint, such as something muted.
    pub faded: bool,
    /// The square's side, with the mark filling it; otherwise a standard square the mark sits in.
    pub size: Option<f32>,
    pub on_click: Option<Handler>,
    /// Stands at the top, beside the first lines, rather than centred on the row, such as a large
    /// lead beside a tall card.
    pub top: bool,
}

/// What sits at the end of the first line, in order.
pub enum Trail {
    Text(SharedString, u32),
    /// Quiet, such as how long ago.
    Faint(SharedString),
    /// Quiet, keeping the width of the widest of its fits so what is beside it never moves as
    /// it changes, such as a rate.
    Steady(SharedString, Vec<SharedString>),
    /// A dot before the text, both in the color, such as "Recording".
    Status(SharedString, u32),
    Dropdown(Dropdown),
    /// A ✕ that shows while the pointer is over `group`, such as the frame the item is in.
    Close {
        group: SharedString,
        on_click: Handler,
    },
    /// Switches what the row stands for, in `color` while on.
    Switch {
        on: bool,
        color: u32,
        /// Off, it is shown faint, such as one another switch drives.
        enabled: bool,
        on_click: Handler,
    },
    /// A small mark, such as a lock on a network that needs a password.
    Glyph(PathBuf, u32),
    /// A "⋯" opening further things to do with the row, such as forgetting it.
    Menu(Actions),
}

impl Trail {
    /// Controls end the row rather than the title line, so they sit centred on all of it.
    fn controls(&self) -> bool {
        matches!(self, Trail::Switch { .. } | Trail::Glyph(..) | Trail::Menu(_))
    }
}

/// A row: an optional lead, a title line, and whatever the item holds under the title.
pub struct Item {
    pub id: SharedString,
    pub lead: Option<Lead>,
    /// A faint line over the title, such as where a message is from; the trail moves onto it.
    pub overline: Option<SharedString>,
    pub title: SharedString,
    pub title_color: Option<u32>,
    /// A longer title goes onto more lines rather than ending in an ellipsis.
    pub wrap: bool,
    /// Fainter, after the title.
    pub detail: SharedString,
    pub trail: Vec<Trail>,
}

fn lead_element(id: SharedString, lead: Lead, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let (size, fill) = lead.size.map_or((LEAD, 0.8), |size| (size, 1.0));
    // A tinted lead is a solid square of its color, glowing softly, with its glyph cut out of it
    // in the panel's dark tone; faded, such as something off, it is a quiet grey square instead.
    let off = lead.faded && lead.tint.is_some();
    let tint = lead.tint.map(|color| {
        let fill = if off {
            rgba(with_alpha(TINT, TRACK_ALPHA)).into()
        } else {
            rgb(color).into()
        };
        ui::shape(Radius::Control, Some(fill))
    });
    let shadow = lead.tint.filter(|_| !off).map(|color| {
        vec![BoxShadow {
            color: rgba(with_alpha(color, TINT_GLOW_ALPHA)).into(),
            offset: point(px(0.0), px(TINT_GLOW_DROP)),
            blur_radius: px(TINT_GLOW_BLUR),
            spread_radius: px(0.0),
            inset: false,
        }]
    });
    let lead = Lead {
        mark: match (lead.mark, lead.tint) {
            (Some(Mark::Glyph(path, _)), Some(_)) if off => Some(Mark::Glyph(path, OVERLAY2)),
            (Some(Mark::Glyph(path, _)), Some(_)) => Some(Mark::Glyph(path, ON_TINT)),
            (mark, _) => mark,
        },
        ..lead
    };
    let glow = lead
        .glow
        .filter(|(_, strength)| *strength > 0.0)
        .map(|(color, strength)| {
            let alpha = (strength.clamp(0.0, 1.0) * GLOW_ALPHA) as u8;
            ui::shape(Radius::Control, Some(rgba(with_alpha(color, alpha)).into()))
        });
    // A mascot answers the pointer itself, so it goes without the grey hover behind.
    let own_hover = matches!(lead.mark, Some(Mark::Mascot(_)));
    let mark = div()
        .when(lead.faded, |mark| mark.opacity(0.4))
        .children(lead.mark.as_ref().map(|mark| badge::mark(mark, size, fill)));
    match lead.on_click {
        Some(handler) if own_hover => div()
            .id(id)
            .size(px(size))
            .flex_none()
            .cursor_pointer()
            .child(mark)
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .into_any_element(),
        Some(handler) => button::ghost(id, hover)
            .size(px(size))
            .flex_none()
            .rounded(px(CONTROL_RADIUS))
            .when_some(shadow, |lead, shadow| lead.shadow(shadow))
            .children(tint)
            .children(glow)
            .child(mark)
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .into_any_element(),
        None => div()
            .relative()
            .size(px(size))
            .flex_none()
            .rounded(px(CONTROL_RADIUS))
            .when_some(shadow, |lead, shadow| lead.shadow(shadow))
            .flex()
            .items_center()
            .justify_center()
            .children(tint)
            .children(glow)
            .child(mark)
            .into_any_element(),
    }
}

fn trail_element(
    id: SharedString,
    trail: Trail,
    popovers: &Popovers,
    tips: &Tooltips,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    match trail {
        Trail::Text(text, color) => div()
            .flex_none()
            .min_w(px(AMOUNT_W))
            .flex()
            .justify_end()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(color))
            .child(motion::text(id, text))
            .into_any_element(),
        Trail::Faint(text) => div()
            .flex_none()
            .text_color(rgb(SUBTEXT1))
            .child(motion::text(id, text))
            .into_any_element(),
        Trail::Steady(text, fits) => div()
            .flex_none()
            .text_color(rgb(SUBTEXT1))
            .child(motion::steady_at(id, text, fits, motion::Hold::End))
            .into_any_element(),
        Trail::Status(text, color) => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(6.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(color))
            .child(div().size(px(8.0)).rounded_full().bg(rgb(color)))
            .child(motion::text(id, text))
            .into_any_element(),
        Trail::Dropdown(picker) => dropdown::dropdown(picker, popovers, hover, cx),
        Trail::Close { group, on_click } => button::ghost(id, hover)
            .flex_none()
            .opacity(0.0)
            .when(hover, |close| {
                close.group_hover(group, |style| style.opacity(1.0))
            })
            .size(px(OVERLINE_H))
            .text_color(rgb(SUBTEXT1))
            .when(hover, |close| {
                close.hover(|style| style.text_color(rgb(TEXT)))
            })
            .child("✕")
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                on_click(this, cx);
            }))
            .into_any_element(),
        Trail::Switch {
            on,
            color,
            enabled,
            on_click,
        } => button::switch_control(id, on, color, enabled, on_click, cx),
        Trail::Glyph(path, color) => div()
            .flex_none()
            .child(icon::glyph(&path, MARK, color))
            .into_any_element(),
        Trail::Menu(menu) => actions::actions(menu, popovers, tips, hover, cx),
    }
}

/// `children` go under the title, beside the lead. A row with a control at its end, such as a
/// switch, ends in all of its trail, centred on the row, rather than its title line.
pub fn item(
    item: Item,
    children: Vec<AnyElement>,
    popovers: &Popovers,
    tips: &Tooltips,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let id = |part: &str| SharedString::from(format!("{}-{part}", item.id));
    let top = item.lead.as_ref().is_some_and(|lead| lead.top);
    let lead = item
        .lead
        .map(|lead| lead_element(id("lead"), lead, hover, cx));
    let at_end = item.trail.iter().any(Trail::controls);
    let mut trail: Vec<AnyElement> = item
        .trail
        .into_iter()
        .enumerate()
        .map(|(index, trail)| {
            trail_element(id(&format!("trail-{index}")), trail, popovers, tips, hover, cx)
        })
        .collect();
    let controls = at_end.then(|| {
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(CONTROLS_GAP))
            .text_size(px(SMALL_SIZE))
            .children(std::mem::take(&mut trail))
    });
    let title_color = item.title_color;
    let wrap = item.wrap;
    let column = div().flex_1().min_w(px(0.0)).flex().flex_col();
    let column = match item.overline {
        Some(overline) => column
            .gap(px(MESSAGE_GAP))
            .child(
                div()
                    .h(px(OVERLINE_H))
                    .line_height(px(OVERLINE_H))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .text_size(px(SMALL_SIZE))
                    .text_color(rgb(SUBTEXT1))
                    .child(div().min_w(px(0.0)).truncate().child(overline))
                    .children(trail),
            )
            .child(
                div()
                    .min_h(px(HEADLINE_H))
                    .line_height(px(HEADLINE_H))
                    .text_size(px(TEXT_SIZE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .when_some(title_color, |title, color| title.text_color(rgb(color)))
                    .when(!wrap, |title| title.h(px(HEADLINE_H)).truncate())
                    .child(motion::text(id("title"), item.title)),
            ),
        None => {
            let title = div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .gap(px(6.0))
                .child(
                    div()
                        .min_w(px(0.0))
                        .when(!wrap, |title| title.truncate())
                        .text_size(px(ROW_TITLE))
                        .font_weight(FontWeight::MEDIUM)
                        .when_some(title_color, |title, color| title.text_color(rgb(color)))
                        .child(motion::text(id("title"), item.title)),
                )
                .when(!item.detail.is_empty(), |line| {
                    line.child(
                        div()
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(rgb(OVERLAY2))
                            .child(motion::text(id("detail"), item.detail)),
                    )
                });
            column.gap(px(LINE_GAP)).child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .text_size(px(SMALL_SIZE))
                    .child(title)
                    .children(trail),
            )
        }
    };
    div()
        .flex()
        .when(top, |row| row.items_start())
        .when(!top, |row| row.items_center())
        .gap(px(LEAD_GAP))
        .children(lead)
        .child(column.children(children))
        .children(controls)
        .into_any_element()
}
