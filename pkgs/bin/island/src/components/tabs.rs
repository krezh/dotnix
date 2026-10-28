//! A row of tabs with a pill sliding to the selected one, each named by its label or its glyph;
//! and a smaller segmented control for a card's title, its tabs as wide as their labels in a
//! well, the pill springing to the chosen one and taking its width.

use std::{path::PathBuf, rc::Rc, time::Instant};

use gpui::{
    AnimationExt, AnyElement, BoxShadow, Context, Div, FontWeight, SharedString, SpringAnimation,
    SpringConfig, div, point, prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        button, icon,
        measure::{self, Measured},
        motion::{Next, keyed},
        tooltip::{Tip, Tooltips},
    },
    island::Island,
    spring::{self, Spring},
    theme::{ACCENT, CRUST, TEXT, with_alpha},
    ui::{self, HEADER_H, Radius, TEXT_SIZE},
};

const TAB_W: f32 = 96.0;
/// When every tab shows a glyph instead of its label.
const GLYPH_TAB_W: f32 = 48.0;
const GLYPH: f32 = 18.0;
const TAB_GAP: f32 = 4.0;
const PILL_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);

#[derive(Clone, Copy)]
struct Geometry {
    tab_w: f32,
    height: f32,
    gap: f32,
    text: f32,
}

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
    let geometry = Geometry {
        tab_w,
        height: HEADER_H,
        gap: TAB_GAP,
        text: TEXT_SIZE,
    };
    row(id, labels, selected, on_select, &geometry, hover, tips, cx)
}

const SEGMENT_H: f32 = 26.0;
const SEGMENT_PAD: f32 = 12.0;
const SEGMENT_TEXT: f32 = 12.5;
const WELL_PAD: f32 = 3.0;
const WELL_RADIUS: f32 = 11.0;
const KNOB_RADIUS: f32 = 8.0;
const WELL_ALPHA: u8 = 0x80;
const KNOB_ALPHA: u8 = 0x26;
/// The pill's top edge catching the light, so it reads as raised out of the well.
const KNOB_EDGE_ALPHA: u8 = 0x14;
const QUIET_ALPHA: u8 = 0x8f;
/// The pill slides and takes the new tab's width on the soft spring.
const SEGMENT_SPRING: SpringConfig = spring::SOFT;

/// Where the pill is and how wide, each on its own spring.
struct Knob {
    x: Spring,
    w: Spring,
    at: Instant,
    seen: bool,
    /// The control's own width, when it fills what it sits in.
    width: Measured,
}

impl Default for Knob {
    fn default() -> Self {
        Self {
            x: Spring::new(0.0, SEGMENT_SPRING, 0.01),
            w: Spring::new(0.0, SEGMENT_SPRING, 0.01),
            at: Instant::now(),
            seen: false,
            width: measure::measured(),
        }
    }
}

/// A segmented control: the tabs in a well, the pill sliding to the chosen one and taking its
/// width. Each tab is as wide as its label, as in a card's title; or, `fill`ing what it sits in,
/// they share its width equally, as on a row of its own in a narrow card. `id` must be unique
/// where it sits.
pub fn segmented(
    id: &str,
    labels: Vec<TabLabel>,
    selected: usize,
    on_select: Select,
    fill: bool,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let buttons: Vec<AnyElement> = labels
        .iter()
        .enumerate()
        .map(|(index, tab)| {
            let select = on_select.clone();
            let picked = index == selected;
            button::fill(
                SharedString::from(format!("{id}-{index}")),
                Radius::Row,
                None,
                !picked && hover,
            )
            .relative()
            .h(px(SEGMENT_H))
            .when(fill, |tab| tab.flex_1())
            .when(!fill, |tab| tab.px(px(SEGMENT_PAD)))
            .text_size(px(SEGMENT_TEXT))
            .font_weight(FontWeight::MEDIUM)
            .text_color(if picked {
                rgb(TEXT)
            } else {
                rgba(with_alpha(TEXT, QUIET_ALPHA))
            })
            .child(tab.label.clone())
            .on_click(cx.listener(move |this, _, _, cx| select(index, this, cx)))
            .into_any_element()
        })
        .collect();
    let texts: Vec<SharedString> = labels.into_iter().map(|tab| tab.label).collect();
    keyed(
        SharedString::from(format!("{id}-segmented")),
        move |knob: &mut Knob, window, cx| {
            // Each tab's width, from its label as it will be drawn.
            let style = window.text_style();
            let run = |text: &SharedString| {
                let mut run = style.to_run(text.len());
                run.font.weight = FontWeight::MEDIUM;
                run
            };
            let widths: Vec<f32> = if fill {
                // Equal shares of the width inside the well, once it has been measured.
                let inner = knob
                    .width
                    .get()
                    .map_or(0.0, |size| f32::from(size.width) - 2.0 * WELL_PAD);
                vec![inner / texts.len().max(1) as f32; texts.len()]
            } else {
                texts
                    .iter()
                    .map(|text| {
                        let line = window.text_system().layout_line(
                            text,
                            px(SEGMENT_TEXT),
                            &[run(text)],
                            None,
                        );
                        f32::from(line.width).ceil() + 2.0 * SEGMENT_PAD
                    })
                    .collect()
            };
            let x: f32 = widths.iter().take(selected).sum();
            let w = widths.get(selected).copied().unwrap_or(0.0);
            let now = Instant::now();
            // Until a filling control knows its width, the pill waits rather than growing from
            // nothing.
            let known = !fill || knob.width.get().is_some();
            if !knob.seen || !known || cx.reduce_motion() {
                knob.seen = known;
                knob.x.reset(x);
                knob.w.reset(w);
            } else {
                knob.x.retarget(x, SEGMENT_SPRING);
                knob.w.retarget(w, SEGMENT_SPRING);
                let dt = now.duration_since(knob.at).as_secs_f32().min(0.05);
                knob.x.advance(dt, now);
                knob.w.advance(dt, now);
            }
            knob.at = now;
            let pill = div()
                .absolute()
                .top(px(WELL_PAD))
                .left(px(WELL_PAD + knob.x.value()))
                .w(px(knob.w.value().max(0.0)))
                .h(px(SEGMENT_H))
                .rounded(px(KNOB_RADIUS))
                .shadow(vec![
                    BoxShadow {
                        color: rgba(with_alpha(CRUST, 0x66)).into(),
                        offset: point(px(0.0), px(1.0)),
                        blur_radius: px(3.0),
                        spread_radius: px(0.0),
                        inset: false,
                    },
                    BoxShadow {
                        color: rgba(with_alpha(0xffffff, KNOB_EDGE_ALPHA)).into(),
                        offset: point(px(0.0), px(1.0)),
                        blur_radius: px(0.0),
                        spread_radius: px(0.0),
                        inset: true,
                    },
                ])
                .bg(rgba(with_alpha(TEXT, KNOB_ALPHA)));
            let element = div()
                .relative()
                .flex()
                .when(fill, |well| well.w_full())
                .when(!fill, |well| well.flex_none())
                .p(px(WELL_PAD))
                .rounded(px(WELL_RADIUS))
                .bg(rgba(with_alpha(CRUST, WELL_ALPHA)))
                .when(fill, |well| well.child(measure::probe(&knob.width)))
                .when(known, |well| well.child(pill))
                .children(buttons)
                .into_any_element();
            let next = if knob.x.is_settled() && knob.w.is_settled() {
                Next::Rest
            } else {
                Next::Frame
            };
            (element, next)
        },
    )
    .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn row(
    id: &str,
    labels: Vec<TabLabel>,
    selected: usize,
    on_select: Select,
    geometry: &Geometry,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> Div {
    let Geometry {
        tab_w,
        height,
        gap,
        text,
    } = *geometry;
    let width = labels.len() as f32 * (tab_w + gap) - gap;
    let pill = div()
        .absolute()
        .top_0()
        .w(px(tab_w))
        .h(px(height))
        .child(ui::shape(Radius::Row, Some(ui::selected_fill().into())))
        .with_spring(
            SharedString::from(format!("{id}-pill")),
            SpringAnimation::new(PILL_SPRING).to(selected as f32 * (tab_w + gap)),
            |pill, x| pill.left(px(x)),
        );
    div()
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(height))
        .flex()
        .gap(px(gap))
        .child(pill)
        .children(labels.into_iter().enumerate().map(|(index, tab)| {
            let picked = index == selected;
            let select = on_select.clone();
            let color = if picked { ACCENT } else { TEXT };
            let tab_id = SharedString::from(format!("{id}-{index}"));
            let tab_button = button::fill(tab_id.clone(), Radius::Row, None, !picked && hover)
                .w(px(tab_w))
                .h(px(height))
                .text_size(px(text))
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
