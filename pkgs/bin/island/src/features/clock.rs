use std::{thread, time::Duration};

use chrono::{DateTime, Local, Timelike};
use gpui::{AnyElement, FontWeight, MouseButton, div, prelude::*, px};

use super::{Feature, Feed, PanelCx, WidgetCx};
use crate::{
    components::{motion, tooltip::Tip},
    island::Panel,
    panel::{Block, CalendarView, Header, PanelView},
    supervisor::Ended,
    ui::{SMALL_SIZE, TITLE_SIZE},
};

const GAP: f32 = 10.0;
/// How long after the minute turns the clock moves, so it never shows the minute before.
const LATE: Duration = Duration::from_millis(5);

/// Minutes since the epoch.
fn minute() -> i64 {
    Local::now().timestamp().div_euclid(60)
}

fn until_next_minute() -> Duration {
    let now = Local::now();
    let into = Duration::from_secs(now.second().into()) + Duration::from_nanos(now.nanosecond().min(999_999_999).into());
    Duration::from_secs(60).saturating_sub(into) + LATE
}

/// Months away from the current one.
#[derive(Default)]
pub struct Month {
    offset: i32,
}

impl super::Local for Month {
    fn panel_closed(&mut self) {
        self.offset = 0;
    }
}

pub struct Clock;

impl Feature for Clock {
    const NAME: &'static str = "clock";
    const PANEL: Option<&'static str> = Some("calendar");
    const TAB: Option<(&'static str, u8)> = Some(("Calendar", 0));

    /// Minutes since the epoch.
    type State = i64;
    type Local = Month;
    type Event = ();

    fn start(feed: Feed<i64>) -> Self {
        feed.supervise(|feed| {
            loop {
                thread::sleep(until_next_minute());
                if !feed.publish(minute()) {
                    return Ended::Unwanted;
                }
            }
        });
        Self
    }

    fn initial() -> i64 {
        minute()
    }

    fn widget(&self, _: &str, minute: &i64, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let now = DateTime::from_timestamp(minute * 60, 0)?.with_timezone(&Local);
        Some(
            w.base("bar-clock")
                .gap(px(GAP))
                .tip(w.tips(), "bar-clock", now.format("%A, %-d %B %Y").to_string())
                .child(div().text_size(px(SMALL_SIZE)).child(motion::text("date", now.format("%a %-d %b").to_string())))
                .child(
                    div()
                        .text_size(px(TITLE_SIZE))
                        .font_weight(FontWeight::BOLD)
                        .child(motion::text("time", now.format("%H:%M").to_string())),
                )
                .on_click(w.toggle(Panel::Feature(Self::NAME)))
                .on_mouse_down(MouseButton::Right, w.toggle(Panel::System))
                .into_any_element(),
        )
    }

    fn panel<'a>(&self, _: &i64, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let offset = p.local::<Month>().map_or(0, |month| month.offset);
        Some(PanelView {
            header: Header::Tabs,
            blocks: vec![Block::Calendar(CalendarView {
                offset,
                on_previous: p.on_local::<Self>(|_, month| month.offset -= 1),
                on_next: p.on_local::<Self>(|_, month| month.offset += 1),
                on_today: p.on_local::<Self>(|_, month| month.offset = 0),
            })],
        })
    }
}
