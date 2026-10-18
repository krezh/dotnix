//! A search field over a list of results, with an optional preview of the selected one beside it,
//! such as apps to launch or clipboard history to paste.

use std::{cell::Cell, path::PathBuf, rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, AnyElement, Context, FontWeight, ScrollWheelEvent, SharedString, SpringAnimation,
    SpringConfig, div, prelude::*, px, rgb,
};

use crate::{
    components::{button, card, icon, list::ROWS},
    island::Island,
    panel::Handler,
    theme::{ACCENT, OVERLAY2, SUBTEXT1},
    ui::{self, CARD_PAD, HEADING_SIZE, Radius, SMALL_SIZE, TITLE_SIZE},
};

const WIDTH: f32 = 560.0;
/// The list beside a preview pane.
const LIST_W: f32 = 400.0;
const PANE_W: f32 = 460.0;
const PANE_GAP: f32 = 10.0;
const FIELD_H: f32 = 48.0;
const CARET_H: f32 = 22.0;
const ROW_H: f32 = 48.0;
const ROW_GAP: f32 = 2.0;
const ICON: f32 = 30.0;
const PREVIEW_W: f32 = 56.0;
const PREVIEW_H: f32 = 40.0;
const SELECTION_SPRING: SpringConfig = SpringConfig::new(560.0, 44.0, 1.0);

pub struct SearchRow {
    pub id: SharedString,
    pub name: SharedString,
    pub detail: SharedString,
    pub icon: Option<PathBuf>,
    /// The icon is a picture to preview, drawn wider than an app icon.
    pub preview: bool,
    pub selected: bool,
    pub on_click: Handler,
    /// Hovering a row selects it.
    pub on_hover: Handler,
}

pub type Step = Rc<dyn Fn(i32, &mut Island, &mut Context<Island>)>;

pub enum Preview {
    Text(SharedString),
    Picture(PathBuf),
    Empty,
}

pub struct SearchView {
    pub query: SharedString,
    pub placeholder: SharedString,
    /// Shown when there are no rows and nothing is typed.
    pub empty: SharedString,
    pub rows: Vec<SearchRow>,
    pub on_step: Step,
    pub preview: Option<(Preview, SharedString)>,
}

/// Breathes rather than blinks: fully lit most of the cycle, easing out and back.
fn caret() -> impl IntoElement {
    div()
        .flex_none()
        .w(px(2.0))
        .h(px(CARET_H))
        .rounded_full()
        .bg(rgb(ACCENT))
        .with_animation(
            "launcher-caret",
            Animation::new(Duration::from_millis(1100)).repeat(),
            |caret, t| caret.opacity(((t * std::f32::consts::TAU).cos() * 1.6).clamp(-1.0, 1.0) * 0.5 + 0.5),
        )
}

thread_local! {
    /// Pixels scrolled over a list that do not yet add up to a whole row.
    static SCROLLED: Cell<f32> = const { Cell::new(0.0) };
}

/// Down is positive; touchpads add up across events.
fn rows_scrolled(event: &ScrollWheelEvent) -> i32 {
    let total = SCROLLED.get() - f32::from(event.delta.pixel_delta(px(ROW_H)).y);
    let rows = (total / ROW_H).trunc();
    SCROLLED.set(total - rows * ROW_H);
    rows as i32
}

fn preview_pane(preview: Preview, detail: SharedString, height: f32) -> AnyElement {
    let content = match preview {
        Preview::Text(text) => div()
            .text_size(px(SMALL_SIZE))
            .child(text)
            .into_any_element(),
        Preview::Picture(path) => gpui::img(path)
            .size_full()
            .object_fit(gpui::ObjectFit::Contain)
            .into_any_element(),
        Preview::Empty => div().into_any_element(),
    };
    card::surface()
        .w(px(PANE_W))
        .h(px(height))
        .flex_none()
        .p(px(CARD_PAD))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().flex_1().min_h(px(0.0)).overflow_hidden().child(content))
        .child(
            div()
                .flex_none()
                .truncate()
                .text_size(px(SMALL_SIZE))
                .text_color(rgb(SUBTEXT1))
                .child(detail),
        )
        .into_any_element()
}

/// Always the same size: typing only changes what fills the [`ROWS`] slots, so the island
/// never reshapes mid-search.
pub fn search(view: SearchView, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let field = card::surface()
        .flex_none()
        .h(px(FIELD_H))
        .px(px(16.0))
        .flex()
        .items_center()
        .gap(px(1.0))
        .text_size(px(HEADING_SIZE))
        .map(|field| {
            if view.query.is_empty() {
                field.child(caret()).child(div().pl(px(4.0)).text_color(rgb(OVERLAY2)).child(view.placeholder.clone()))
            } else {
                field
                    .child(div().min_w(px(0.0)).truncate().child(view.query.clone()))
                    .child(caret())
            }
        });

    let selected = view.rows.iter().position(|row| row.selected);
    let empty = view.rows.is_empty();
    let rows = view.rows.into_iter().map(|row| {
        let handler = row.on_click;
        let entered = row.on_hover;
        let slot = if row.preview {
            div().w(px(PREVIEW_W)).h(px(PREVIEW_H)).flex().items_center().justify_center().when_some(
                row.icon.as_deref(),
                |slot, path| slot.child(gpui::img(path.to_path_buf()).size_full().object_fit(gpui::ObjectFit::Contain)),
            )
        } else {
            div()
                .size(px(ICON))
                .when_some(row.icon.as_deref(), |slot, path| slot.child(icon::image(path, ICON)))
        };
        button::fill(row.id, Radius::Row, None, hover && !row.selected)
            .flex_none()
            .h(px(ROW_H))
            .px(px(10.0))
            .justify_start()
            .gap(px(12.0))
            .child(slot.flex_none())
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(TITLE_SIZE)).font_weight(FontWeight::SEMIBOLD).truncate().child(row.name))
                    .when(!row.detail.is_empty(), |text| {
                        text.child(div().text_size(px(SMALL_SIZE)).text_color(rgb(SUBTEXT1)).truncate().child(row.detail))
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    entered(this, cx);
                }
            }))
    });

    let on_step = view.on_step;
    let width = if view.preview.is_some() { LIST_W } else { WIDTH };
    let list_h = ROWS as f32 * ROW_H + (ROWS - 1) as f32 * ROW_GAP;
    let list = div()
        .id("launcher-list")
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(list_h))
        .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
            let step = rows_scrolled(event);
            if step != 0 {
                on_step(step, this, cx);
            }
        }))
        .when_some(selected, |list, index| {
            let top = px(index as f32 * (ROW_H + ROW_GAP));
            list.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .h(px(ROW_H))
                    .child(ui::shape(Radius::Row, Some(ui::selected_fill().into())))
                    .with_spring("launcher-selection", SpringAnimation::new(SELECTION_SPRING).to(top), |pill, top| {
                        pill.top(top)
                    }),
            )
        })
        .child(div().relative().flex().flex_col().gap(px(ROW_GAP)).children(rows))
        .when(empty, |list| {
            list.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(SUBTEXT1))
                    .child(if view.query.is_empty() {
                        view.empty.clone()
                    } else {
                        SharedString::from(format!("Nothing matches “{}”", view.query))
                    }),
            )
        });

    let body = match view.preview {
        Some((preview, detail)) => div()
            .flex()
            .gap(px(PANE_GAP))
            .child(list)
            .child(preview_pane(preview, detail, list_h))
            .into_any_element(),
        None => list.into_any_element(),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(field)
        .child(body)
        .into_any_element()
}
