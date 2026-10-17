//! Tabs within a panel, in the tab strip's style: fixed-width tabs, the selected one in the accent
//! over the selection pill, which slides to the tab picked.

use std::rc::Rc;

use gpui::{AnimationExt, Context, Div, FontWeight, SharedString, SpringAnimation, SpringConfig, div, prelude::*, px, rgb};

use crate::{
    components::button,
    island::Island,
    theme::{ACCENT, TEXT},
    ui::{self, HEADER_H, Radius, TEXT_SIZE},
};

const TAB_W: f32 = 96.0;
const TAB_GAP: f32 = 4.0;
const PILL_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);

/// Picks the tab at an index.
pub type Select = Rc<dyn Fn(usize, &mut Island, &mut Context<Island>)>;

/// `labels` as tabs, `selected` picked; `id` names them, and must be unique where they sit.
pub fn tabs(
    id: &str,
    labels: Vec<SharedString>,
    selected: usize,
    on_select: Select,
    hover: bool,
    cx: &mut Context<Island>,
) -> Div {
    let width = labels.len() as f32 * (TAB_W + TAB_GAP) - TAB_GAP;
    let pill = div()
        .absolute()
        .top_0()
        .w(px(TAB_W))
        .h(px(HEADER_H))
        .child(ui::shape(Radius::Row, Some(ui::selected_fill().into())))
        .with_spring(
            SharedString::from(format!("{id}-pill")),
            SpringAnimation::new(PILL_SPRING).to(selected as f32 * (TAB_W + TAB_GAP)),
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
        .children(labels.into_iter().enumerate().map(|(index, label)| {
            let picked = index == selected;
            let select = on_select.clone();
            button::fill(SharedString::from(format!("{id}-{index}")), Radius::Row, None, !picked && hover)
                .w(px(TAB_W))
                .h(px(HEADER_H))
                .text_size(px(TEXT_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if picked { ACCENT } else { TEXT }))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| select(index, this, cx)))
        }))
}
