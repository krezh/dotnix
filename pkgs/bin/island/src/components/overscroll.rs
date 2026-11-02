//! Something that scrolls giving a little past its ends: a touchpad pulling on at the top or bottom
//! stretches what it holds that way, ever less the further it goes, and lets it spring back once
//! the pull stops. A wheel stops at the ends.

use std::{
    cell::RefCell,
    collections::HashMap,
    time::{Duration, Instant},
};

use gpui::{
    Div, ScrollDelta, ScrollEvent, ScrollHandle, SharedString, Stateful, div, point, prelude::*, px,
};

use crate::{
    components::motion::{Next, keyed},
    island::SHAPE_SPRING,
    spring::{Spring, rubber, unrubber},
};

/// The furthest a pull stretches it.
pub const REACH: f32 = 48.0;
/// How long after the last scroll a pull counts as let go: a touchpad never says when it lifts.
pub const LET_GO: Duration = Duration::from_millis(80);

struct Over {
    /// How far past an end it is pulled: above 0 past the top.
    pull: f32,
    last: Option<Instant>,
    stretch: Spring,
    at: Option<Instant>,
    /// Its own, when what scrolls is given none.
    scroll: ScrollHandle,
}

impl Over {
    fn new() -> Self {
        Self {
            pull: 0.0,
            last: None,
            stretch: Spring::new(0.0, SHAPE_SPRING, 0.1),
            at: None,
            scroll: ScrollHandle::new(),
        }
    }

    /// Held where the pull puts it; let go, it springs back.
    fn advance(&mut self, now: Instant, still: bool) -> Next {
        let dt = self
            .at
            .map_or(0.0, |at| now.duration_since(at).as_secs_f32().min(0.05));
        self.at = Some(now);
        if still {
            self.pull = 0.0;
            self.stretch.reset(0.0);
            return Next::Rest;
        }
        if self.pull != 0.0 {
            if self
                .last
                .is_some_and(|last| now.duration_since(last) < LET_GO)
            {
                self.stretch.reset(rubber(self.pull, REACH));
                return Next::After(LET_GO);
            }
            self.pull = 0.0;
            self.stretch.retarget(0.0, SHAPE_SPRING);
            return Next::Frame;
        }
        self.stretch.advance(dt, now);
        if self.stretch.is_settled() {
            Next::Rest
        } else {
            Next::Frame
        }
    }
}

thread_local! {
    static OVERS: RefCell<HashMap<SharedString, Over>> = RefCell::default();
}

/// After a scroll by `delta` has moved what scrolls to `offset`, which may be past either end of
/// `0` to `-most`: the pull it leaves, and where what scrolls stays. A pull gives way before what
/// it holds scrolls again.
pub fn settle(pull: f32, offset: f32, delta: f32, most: f32) -> (f32, f32) {
    if pull != 0.0 {
        let edge = if pull > 0.0 { 0.0 } else { -most };
        let next = pull + delta;
        return if next * pull > 0.0 {
            (next, edge)
        } else {
            (0.0, (edge + next).clamp(-most, 0.0))
        };
    }
    if offset > 0.0 {
        (offset, 0.0)
    } else if offset < -most {
        (offset + most, -most)
    } else {
        (0.0, offset)
    }
}

/// `scrolling` holding `content`, giving past its ends; `key` must be unique among everything that
/// scrolls. It follows `scroll` if given, otherwise a handle of its own.
pub fn overscroll(
    key: impl Into<SharedString>,
    scrolling: Stateful<Div>,
    scroll: Option<&ScrollHandle>,
    content: impl IntoElement,
) -> Stateful<Div> {
    let key = key.into();
    let scroll = scroll.cloned().unwrap_or_else(|| {
        OVERS.with_borrow_mut(|overs| {
            overs
                .entry(key.clone())
                .or_insert_with(Over::new)
                .scroll
                .clone()
        })
    });
    let content = content.into_any_element();
    let shown_key = key.clone();
    let shown = keyed(
        SharedString::from(format!("{key}-overscroll")),
        move |_: &mut (), _, cx| {
            let still = cx.reduce_motion();
            let (stretch, next) = OVERS.with_borrow_mut(|overs| {
                let over = overs.entry(shown_key).or_insert_with(Over::new);
                let next = over.advance(Instant::now(), still);
                (over.stretch.value(), next)
            });
            let element = div()
                .relative()
                .top(px(stretch))
                .child(content)
                .into_any_element();
            (element, next)
        },
    );
    let pulled = scroll.clone();
    scrolling
        .track_scroll(&scroll)
        // Heard after the scroll has moved, before it is kept to its ends.
        .on_scroll(move |event: &ScrollEvent, window, cx| {
            let ScrollDelta::Pixels(delta) = event.delta else {
                return;
            };
            let most = f32::from(pulled.max_offset().y);
            if most <= 0.0 || cx.reduce_motion() {
                return;
            }
            let offset = pulled.offset();
            OVERS.with_borrow_mut(|overs| {
                let over = overs.entry(key.clone()).or_insert_with(Over::new);
                let (mut pull, y) =
                    settle(over.pull, f32::from(offset.y), f32::from(delta.y), most);
                // Pulled again as it springs back, it carries on from the stretch showing.
                let showing = over.stretch.value();
                if over.pull == 0.0 && pull * showing > 0.0 {
                    pull += unrubber(showing, REACH);
                }
                over.pull = pull;
                if pull != 0.0 {
                    over.last = Some(Instant::now());
                }
                pulled.set_offset(point(offset.x, px(y)));
            });
            window.refresh();
        })
        .child(shown)
}

#[cfg(test)]
#[path = "overscroll_test.rs"]
mod tests;
