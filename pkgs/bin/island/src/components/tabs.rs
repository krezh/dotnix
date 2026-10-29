//! A row of tabs with a pill sliding to the selected one, each named by its label or its glyph;
//! or, contained in a strip of their own, labelled tabs with a count beside each label.

use std::{path::PathBuf, rc::Rc};

use gpui::{
    AnimationExt, Context, Div, FontWeight, SharedString, SpringAnimation, SpringConfig, div,
    prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        button, icon,
        marquee::TextLayouts,
        motion,
        tooltip::{Tip, Tooltips},
    },
    island::Island,
    theme::{ACCENT, OVERLAY1, SUBTEXT1, SURFACE1, TEXT, with_alpha},
    ui::{self, HEADER_H, Radius, SMALL_SIZE, STRIP_PAD, TEXT_SIZE},
};

const TAB_W: f32 = 96.0;
/// When every tab shows a glyph instead of its label.
const GLYPH_TAB_W: f32 = 48.0;
const GLYPH: f32 = 18.0;
const TAB_GAP: f32 = 4.0;
const PILL_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);

pub type Select = Rc<dyn Fn(usize, &mut Island, &mut Context<Island>)>;

/// One tab: its label, shown unless it has a glyph to show instead.
pub struct TabLabel {
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    /// Smaller, after the label in a strip, such as how many the view holds.
    pub count: Option<SharedString>,
}

const STRIP_TAB_H: f32 = 28.0;
/// A strip of tabs, edge to edge, for what sits beside it to match.
pub const STRIP_H: f32 = STRIP_TAB_H + 2.0 * STRIP_PAD;
const STRIP_TAB_GAP: f32 = 2.0;
const STRIP_TAB_PAD: f32 = 14.0;
/// What a tab takes before its text is measured.
const UNMEASURED_W: f32 = 88.0;
const COUNT_GAP: f32 = 7.0;
const COUNT_SIZE: f32 = 12.0;
const PILL_ALPHA: u8 = 0xe6;

/// As wide as its label and count, once they are measured.
fn tab_width(tab: &TabLabel, layouts: &TextLayouts) -> f32 {
    let label = layouts.get(&tab.label, SMALL_SIZE, FontWeight::SEMIBOLD);
    let count = tab
        .count
        .as_ref()
        .map(|count| layouts.get(count, COUNT_SIZE, FontWeight::SEMIBOLD));
    match (label, count) {
        (Some(label), None) => label.width + 2.0 * STRIP_TAB_PAD,
        (Some(label), Some(Some(count))) => {
            label.width + COUNT_GAP + count.width + 2.0 * STRIP_TAB_PAD
        }
        _ => UNMEASURED_W,
    }
}

/// Labelled tabs in a strip of their own, as the panel's own tabs sit, each as wide as its label,
/// the pill sliding and stretching to the selected one, and each label's count tinted while its
/// tab is.
pub fn strip(
    id: &str,
    labels: Vec<TabLabel>,
    selected: usize,
    on_select: Select,
    hover: bool,
    layouts: &TextLayouts,
    cx: &mut Context<Island>,
) -> Div {
    let widths: Vec<f32> = labels.iter().map(|tab| tab_width(tab, layouts)).collect();
    let width = widths.iter().sum::<f32>() + STRIP_TAB_GAP * labels.len().saturating_sub(1) as f32;
    let pill_x: f32 = widths[..selected.min(widths.len())]
        .iter()
        .map(|width| width + STRIP_TAB_GAP)
        .sum();
    let pill_w = widths.get(selected).copied().unwrap_or(UNMEASURED_W);
    let pill = div()
        .absolute()
        .top_0()
        .h(px(STRIP_TAB_H))
        .with_spring(
            SharedString::from(format!("{id}-pill")),
            SpringAnimation::new(PILL_SPRING).to(pill_x),
            |pill, x| pill.left(px(x)),
        )
        .child(
            div()
                .relative()
                .h(px(STRIP_TAB_H))
                .child(ui::shape(
                    Radius::Row,
                    Some(rgba(with_alpha(SURFACE1, PILL_ALPHA)).into()),
                ))
                .with_spring(
                    SharedString::from(format!("{id}-pill-w")),
                    SpringAnimation::new(PILL_SPRING).to(pill_w),
                    |pill, w| pill.w(px(w)),
                ),
        );
    let tabs = div()
        .relative()
        .w(px(width))
        .h(px(STRIP_TAB_H))
        .flex()
        .gap(px(STRIP_TAB_GAP))
        .child(pill)
        .children(labels.into_iter().zip(widths).enumerate().map(|(index, (tab, tab_w))| {
            let picked = index == selected;
            let select = on_select.clone();
            let tab_id = SharedString::from(format!("{id}-{index}"));
            let count_id = SharedString::from(format!("{id}-{index}-count"));
            button::fill(tab_id, Radius::Row, None, !picked && hover)
                .flex_none()
                .w(px(tab_w))
                .h(px(STRIP_TAB_H))
                .gap(px(COUNT_GAP))
                .text_size(px(SMALL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if picked { TEXT } else { SUBTEXT1 }))
                .child(tab.label)
                .when_some(tab.count, |tab, count| {
                    tab.child(
                        div()
                            .text_size(px(COUNT_SIZE))
                            .text_color(rgb(if picked { ACCENT } else { OVERLAY1 }))
                            .child(motion::text(count_id, count)),
                    )
                })
                .on_click(cx.listener(move |this, _, _, cx| select(index, this, cx)))
        }));
    div()
        .relative()
        .flex_none()
        .p(px(STRIP_PAD))
        .child(div().absolute().size_full().top_0().left_0().child(ui::strip()))
        .child(tabs)
}

/// `id` must be unique where the tabs sit. A tab showing its glyph names itself in a tooltip.
pub fn tabs(
    id: &str,
    labels: Vec<TabLabel>,
    selected: usize,
    on_select: Select,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> Div {
    let tab_w = if !labels.is_empty() && labels.iter().all(|tab| tab.glyph.is_some()) {
        GLYPH_TAB_W
    } else {
        TAB_W
    };
    let width = labels.len() as f32 * (tab_w + TAB_GAP) - TAB_GAP;
    let pill = div()
        .absolute()
        .top_0()
        .w(px(tab_w))
        .h(px(HEADER_H))
        .child(ui::shape(Radius::Row, Some(ui::selected_fill().into())))
        .with_spring(
            SharedString::from(format!("{id}-pill")),
            SpringAnimation::new(PILL_SPRING).to(selected as f32 * (tab_w + TAB_GAP)),
            |pill, x| pill.left(px(x)),
        );
    div()
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(HEADER_H))
        .flex()
        .gap(px(TAB_GAP))
        .child(pill)
        .children(labels.into_iter().enumerate().map(|(index, tab)| {
            let picked = index == selected;
            let select = on_select.clone();
            let color = if picked { ACCENT } else { TEXT };
            let tab_id = SharedString::from(format!("{id}-{index}"));
            let tab_button = button::fill(tab_id.clone(), Radius::Row, None, !picked && hover)
                .w(px(tab_w))
                .h(px(HEADER_H))
                .text_size(px(TEXT_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(color));
            match tab.glyph {
                Some(path) => tab_button
                    .child(icon::glyph(&path, GLYPH, color))
                    .tip(tips, tab_id, tab.label),
                None => tab_button.child(tab.label),
            }
            .on_click(cx.listener(move |this, _, _, cx| select(index, this, cx)))
        }))
}
