//! A month of days, with today marked and buttons to step between months.

use chrono::{Datelike, Duration, Local, Months, NaiveDate};
use gpui::{AnyElement, Context, FontWeight, div, prelude::*, px, rgb};

use crate::{
    components::button,
    island::Island,
    panel::Handler,
    theme::{ACCENT, ON_ACCENT, OVERLAY2, PEACH, TEXT},
    ui::{self, HEADER_H, HEADING_SIZE, Radius, SMALL_SIZE},
};

const DAY_H: f32 = 40.0;
const TODAY: f32 = 36.0;
const DAY_GAP: f32 = 4.0;

pub struct CalendarView {
    /// Months away from the current one.
    pub offset: i32,
    pub on_previous: Handler,
    pub on_next: Handler,
    pub on_today: Handler,
}

pub fn calendar(view: CalendarView, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let offset = view.offset;
    let today = Local::now().date_naive();
    let first_of_today = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let first = if offset >= 0 {
        first_of_today.checked_add_months(Months::new(offset.unsigned_abs()))
    } else {
        first_of_today.checked_sub_months(Months::new(offset.unsigned_abs()))
    }
    .unwrap_or(first_of_today);
    let start = first - Duration::days(first.weekday().num_days_from_monday().into());
    let weekend = |weekday: i64| weekday >= 5;

    let nav = |id: &'static str, glyph: &'static str, handler: Handler, cx: &mut Context<Island>| {
        button::ghost(id, hover)
            .size(px(HEADER_H))
            .text_size(px(HEADING_SIZE))
            .child(glyph)
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
    };
    let on_today = view.on_today;
    let back = (offset != 0).then(|| {
        button::ghost("calendar-today", hover)
            .h(px(HEADER_H))
            .px(px(12.0))
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(ACCENT))
            .child("Today")
            .on_click(cx.listener(move |this, _, _, cx| on_today(this, cx)))
    });

    let weekdays = div().flex().children(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().enumerate().map(
        |(index, day)| {
            div()
                .flex_1()
                .flex()
                .justify_center()
                .text_size(px(SMALL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if weekend(index as i64) { PEACH } else { TEXT }))
                .child(*day)
        },
    ));

    let weeks = (0..6).map(|week| {
        div().flex().gap(px(DAY_GAP)).children((0..7).map(|weekday| {
            let date = start + Duration::days(week * 7 + weekday);
            let in_month = date.month() == first.month();
            let is_today = date == today;
            div()
                .flex_1()
                .min_w(px(DAY_H))
                .h(px(DAY_H))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(match (is_today, in_month) {
                    (true, _) => ON_ACCENT,
                    (false, true) => TEXT,
                    (false, false) => OVERLAY2,
                }))
                .child(
                    div()
                        .relative()
                        .size(px(TODAY))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(is_today, |cell| {
                            cell.child(ui::shape(Radius::Control, Some(rgb(ACCENT).into())))
                                .font_weight(FontWeight::BOLD)
                        })
                        .child(date.day().to_string()),
                )
        }))
    });

    div()
        .flex()
        .flex_col()
        .gap(px(DAY_GAP))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb(px(6.0))
                .child(
                    div()
                        .pl(px(8.0))
                        .text_size(px(HEADING_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(first.format("%B %Y").to_string()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .children(back)
                        .child(nav("calendar-prev", "‹", view.on_previous.clone(), cx))
                        .child(nav("calendar-next", "›", view.on_next.clone(), cx)),
                ),
        )
        .child(div().pb(px(4.0)).child(weekdays))
        .children(weeks)
        .into_any_element()
}
