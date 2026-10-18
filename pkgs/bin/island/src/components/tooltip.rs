//! Hover tooltips, drawn over everything after a short delay.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, Div, MouseMoveEvent, Pixels, Point, SharedString, Stateful, Window, anchored,
    point, prelude::*, px,
};

use crate::{
    components::popover,
    ui::{self, Radius, SMALL_SIZE},
};

const DELAY: Duration = Duration::from_millis(500);
const CURSOR_GAP: f32 = 18.0;
const EDGE_MARGIN: f32 = 4.0;

struct Hover {
    key: SharedString,
    at: Point<Pixels>,
    since: Instant,
}

#[derive(Default)]
struct State {
    texts: HashMap<SharedString, SharedString>,
    hover: Option<Hover>,
}

/// Texts outlive the frame, so widgets whose drawing is reused keep theirs.
#[derive(Clone, Default)]
pub struct Tooltips(Rc<RefCell<State>>);

impl Tooltips {
    pub fn position(&self, key: &SharedString) -> Option<Point<Pixels>> {
        self.0
            .borrow()
            .hover
            .as_ref()
            .filter(|hover| &hover.key == key)
            .map(|hover| hover.at)
    }

    pub fn update(&self, key: &SharedString, text: String, window: &mut Window) {
        let mut state = self.0.borrow_mut();
        if state.texts.get(key).is_none_or(|previous| previous.as_ref() != text) {
            state.texts.insert(key.clone(), text.into());
            window.refresh();
        }
    }

    pub fn overlay(&self, window: &mut Window) -> Option<AnyElement> {
        let mut state = self.0.borrow_mut();
        if !window.is_window_hovered() {
            state.hover = None;
            return None;
        }
        let hover = state.hover.as_ref()?;
        let text = state.texts.get(&hover.key)?.clone();
        if text.is_empty() {
            return None;
        }
        if hover.since.elapsed() < DELAY {
            window.request_animation_frame();
            return None;
        }
        Some(
            anchored()
                .position(hover.at + point(px(0.0), px(CURSOR_GAP)))
                .snap_to_window_with_margin(px(EDGE_MARGIN))
                .child(
                    ui::base().child(
                        popover::surface(Radius::Tooltip)
                            .text_size(px(SMALL_SIZE))
                            .whitespace_nowrap()
                            .px(px(12.0))
                            .py(px(7.0))
                            .child(text),
                    ),
                )
                .into_any_element(),
        )
    }

    pub fn track(&self, element: Stateful<Div>, key: SharedString) -> Stateful<Div> {
        let (hover_state, move_state) = (self.0.clone(), self.0.clone());
        let move_key = key.clone();
        element.on_hover(move |hovered, window, _| {
            let mut state = hover_state.borrow_mut();
            if *hovered {
                state.hover = Some(Hover {
                    key: key.clone(),
                    at: window.mouse_position(),
                    since: Instant::now(),
                });
            } else if state.hover.as_ref().is_some_and(|hover| hover.key == key) {
                state.hover = None;
            }
            window.refresh();
        })
        .on_mouse_move(move |event: &MouseMoveEvent, window, _| {
            if let Some(hover) = move_state
                .borrow_mut()
                .hover
                .as_mut()
                .filter(|hover| hover.key == move_key)
            {
                hover.at = event.position;
                window.refresh();
            }
        })
    }
}

pub trait Tip: Sized {
    fn tip(
        self,
        tips: &Tooltips,
        key: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self;
}

impl Tip for Stateful<Div> {
    fn tip(
        self,
        tips: &Tooltips,
        key: impl Into<SharedString>,
        text: impl Into<SharedString>,
    ) -> Self {
        let key = key.into();
        tips.0.borrow_mut().texts.insert(key.clone(), text.into());
        tips.track(self, key)
    }
}
