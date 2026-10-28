//! A few values as short columns with a label under each, one picked out, such as each day's data
//! used this week with today highlighted.

use gpui::{
    AnimationExt, AnyElement, SharedString, SpringAnimation, SpringConfig, div, prelude::*, px,
    rgb, rgba,
};

use crate::{
    components::motion,
    theme::{OVERLAY2, with_alpha},
    ui::{self, Radius},
};

const TALL: f32 = 30.0;
/// So a day with nothing still shows where it is.
const LEAST: f32 = 3.0;
const BAR_W: f32 = 14.0;
const GAP: f32 = 4.0;
const LABEL: f32 = 10.5;
const QUIET_ALPHA: u8 = 0x52;
const GROW: SpringConfig = crate::spring::SOFT_GROW;

pub struct Bars {
    pub id: SharedString,
    /// Label and value, in order.
    pub values: Vec<(SharedString, f32)>,
    pub picked: Option<usize>,
    pub color: u32,
    pub picked_color: u32,
}

pub fn bars(bars: Bars) -> AnyElement {
    let most = bars
        .values
        .iter()
        .map(|(_, value)| *value)
        .fold(0.0_f32, f32::max)
        .max(f32::EPSILON);
    div()
        .flex()
        .items_end()
        .gap(px(GAP))
        .children(bars.values.into_iter().enumerate().map(|(index, (label, value))| {
            let picked = bars.picked == Some(index);
            let fill = if picked {
                rgba(with_alpha(bars.picked_color, 0xff))
            } else {
                rgba(with_alpha(bars.color, QUIET_ALPHA))
            };
            let height = LEAST.max(value / most * TALL);
            div()
                .w(px(BAR_W))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(4.0))
                .child(
                    div()
                        .h(px(TALL))
                        .w_full()
                        .flex()
                        .items_end()
                        .child(
                            div()
                                .w_full()
                                .relative()
                                .child(ui::shape(Radius::Badge, Some(fill.into())))
                                .with_spring(
                                    SharedString::from(format!("{}-{index}", bars.id)),
                                    SpringAnimation::new(GROW).to(height),
                                    |bar, height| bar.h(px(height.max(0.0))),
                                ),
                        ),
                )
                .child(
                    div()
                        .text_size(px(LABEL))
                        .text_color(rgb(OVERLAY2))
                        .child(motion::text(
                            SharedString::from(format!("{}-{index}-label", bars.id)),
                            label,
                        )),
                )
        }))
        .into_any_element()
}
