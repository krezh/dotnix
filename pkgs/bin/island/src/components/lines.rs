//! A list of two-line entries divided by rules, such as commits: a summary over a fainter detail;
//! and any rows divided by the same rules, such as a card's list of networks.

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

/// Each rule starts `indent` in, such as where the rows' text starts past their leads.
pub fn divided(rows: Vec<AnyElement>, indent: f32) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .relative()
                .when(index > 0, |row| {
                    row.child(
                        div()
                            .absolute()
                            .top_0()
                            .left(px(indent))
                            .right_0()
                            .h(px(1.0))
                            .bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))),
                    )
                })
                .child(row)
        }))
        .into_any_element()
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
                .when(index > 0, |row| {
                    row.border_t_1()
                        .border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)))
                })
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
