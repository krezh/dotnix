//! A list of two-line entries divided by rules, such as commits: a summary over a fainter detail.

use gpui::{AnyElement, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    theme::{CONTROL_ALPHA, SUBTEXT1, SURFACE1, with_alpha},
    ui::SMALL_SIZE,
};

const DETAIL_SIZE: f32 = 12.0;

pub struct Line {
    pub summary: SharedString,
    pub detail: SharedString,
}

pub fn lines(lines: Vec<Line>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .children(lines.into_iter().enumerate().map(|(index, line)| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .py(px(8.0))
                .when(index > 0, |row| row.border_t_1().border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(SMALL_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(line.summary),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(DETAIL_SIZE))
                        .text_color(rgb(SUBTEXT1))
                        .child(line.detail),
                )
        }))
        .into_any_element()
}
