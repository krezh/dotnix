//! A row of tabs with a pill sliding to the selected one, each named by its label or its glyph.

use std::{path::PathBuf, rc::Rc};

use gpui::{
    AnimationExt, Context, Div, FontWeight, SharedString, SpringAnimation, SpringConfig, div,
    prelude::*, px, rgb,
};

use crate::{
    components::{
        button, icon,
        tooltip::{Tip, Tooltips},
    },
    island::Island,
    theme::{ACCENT, TEXT},
    ui::{self, HEADER_H, Radius, TEXT_SIZE},
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
