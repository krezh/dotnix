//! A row in a panel: a lead, a title line ending in amounts, statuses or a dropdown, and
//! whatever it holds under the title, such as a slider. With a line over the title it reads as a
//! message: a faint line naming where it is from, then the title, larger, then what it says.

use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::{
        badge::{self, Mark},
        button,
        dropdown::{self, Dropdown},
        motion,
        popover::Popovers,
    },
    island::Island,
    panel::Handler,
    theme::{ACCENT_TINT_ALPHA, OVERLAY2, SUBTEXT1, TEXT, with_alpha},
    ui::{self, Radius, SMALL_SIZE, TEXT_SIZE},
};

/// The most a lead's glow shows, at full strength.
const GLOW_ALPHA: f32 = 96.0;
/// Keeps a changing amount, such as "50%" to "Muted", from moving what is beside it.
const AMOUNT_W: f32 = 40.0;
const LEAD: f32 = 40.0;
/// Between the lead and the text.
pub const LEAD_GAP: f32 = 12.0;
/// The line over the title, and the title under it.
pub const OVERLINE_H: f32 = 16.0;
pub const HEADLINE_H: f32 = 18.0;
/// Between the lines of a message.
pub const MESSAGE_GAP: f32 = 2.0;

/// What sits left of the title.
pub struct Lead {
    pub mark: Option<Mark>,
    /// A square of this color behind the mark.
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
    /// A dot before the text, both in the color, such as "Recording".
    Status(SharedString, u32),
    Dropdown(Dropdown),
    /// A ✕ that shows while the pointer is over `group`, such as the frame the item is in.
    Close {
        group: SharedString,
        on_click: Handler,
    },
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
    let tint = lead.tint.map(|color| {
        ui::shape(
            Radius::Control,
            Some(rgba(with_alpha(color, ACCENT_TINT_ALPHA)).into()),
        )
    });
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
            .children(tint)
            .children(glow)
            .child(mark)
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .into_any_element(),
        None => div()
            .relative()
            .size(px(size))
            .flex_none()
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
    }
}

/// `children` go under the title, beside the lead.
pub fn item(
    item: Item,
    children: Vec<AnyElement>,
    popovers: &Popovers,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let id = |part: &str| SharedString::from(format!("{}-{part}", item.id));
    let top = item.lead.as_ref().is_some_and(|lead| lead.top);
    let lead = item
        .lead
        .map(|lead| lead_element(id("lead"), lead, hover, cx));
    let trail: Vec<AnyElement> = item
        .trail
        .into_iter()
        .enumerate()
        .map(|(index, trail)| {
            trail_element(id(&format!("trail-{index}")), trail, popovers, hover, cx)
        })
        .collect();
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
                        .font_weight(FontWeight::SEMIBOLD)
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
            column.gap(px(4.0)).child(
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
        .into_any_element()
}
