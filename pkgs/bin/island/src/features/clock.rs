use std::{sync::mpsc, time::Duration};

use super::{Feature, Feed, GlyphSpec, PanelCx, Surface, WidgetCx};
use crate::{
    bar_widget::{BarWidget, Emphasis, Part},
    panel::{Block, CalendarView, Header, PanelView},
    resume,
    stage::Panel,
    supervisor::Ended,
};
use chrono::{DateTime, Local, Timelike};

const GAP: f32 = 6.0;
/// How long after the minute turns the clock moves, so it never shows the minute before.
const LATE: Duration = Duration::from_millis(5);

/// Minutes since the epoch.
fn minute() -> i64 {
    Local::now().timestamp().div_euclid(60)
}

fn until_next_minute() -> Duration {
    let now = Local::now();
    let into = Duration::from_secs(now.second().into())
        + Duration::from_nanos(now.nanosecond().min(999_999_999).into());
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

    /// Minutes since the epoch.
    type State = i64;

    type Event = ();

    fn start(feed: Feed<i64>) -> Self {
        let (wake, woken) = mpsc::channel();
        let waker = wake.clone();
        resume::listen(move || {
            let _ = waker.send(());
        });
        feed.supervise(move |feed| {
            // Held so the wait below times out, rather than fails, if the listener stops.
            let _wake = &wake;
            loop {
                let _ = woken.recv_timeout(until_next_minute());
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
}

impl Surface for Clock {
    const PANEL: Option<&'static str> = Some("calendar");

    const TAB: Option<(&'static str, u8)> = Some(("Calendar", 1));

    const GLYPHS: &'static [GlyphSpec] = &[("tab-calendar", &["calendar"])];

    type Local = Month;

    fn widget(&self, _: &str, minute: &i64, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        let now = DateTime::from_timestamp(minute * 60, 0)?.with_timezone(&Local);
        Some(
            w.base()
                .gap(GAP)
                .tip(now.format("%A, %-d %B %Y").to_string())
                .part(Part::text(now.format("%H:%M").to_string(), Emphasis::Title))
                .part(Part::text(
                    now.format("%a %-d").to_string(),
                    Emphasis::Small,
                ))
                .on_click(w.toggle(Self::panel_handle()))
                .on_right(w.toggle(Panel::Dashboard)),
        )
    }

    fn panel<'a>(&self, _: &i64, p: &PanelCx<'a>) -> Option<PanelView> {
        let offset = p.local::<Month>().map_or(0, |month| month.offset);
        Some(PanelView {
            scroll: false,
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
