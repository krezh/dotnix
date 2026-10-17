use std::{cell::Cell, rc::Rc, time::Instant};

use gpui::{AnyElement, ScrollWheelEvent, SharedString, SpringConfig, div, prelude::*, px, rgb};

use super::{Feature, Feed, IslandInfo, Local, WidgetCx};
use crate::{
    compositor,
    components::tooltip::Tip,
    island,
    spring::Spring,
    theme::{ACCENT, CRITICAL, OVERLAY1, SUBTEXT1},
    ui::{self, Radius},
    wm::Monitor,
};

const DOT: f32 = 8.0;
const ACTIVE_W: f32 = 22.0;
const GAP: f32 = 6.0;
const MOTION: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DotState {
    Active,
    Occupied,
    Empty,
    Urgent,
}

struct Dot {
    id: i64,
    state: DotState,
    width: Spring,
    presence: Spring,
}

#[derive(Default)]
pub struct Dots {
    dots: Vec<Dot>,
    /// Shared with the scroll handler.
    scrolled: Rc<Cell<f32>>,
}

impl Dots {
    fn update(&mut self, monitor: Option<&Monitor>) {
        let workspaces = monitor.map_or(&[][..], |monitor| &monitor.workspaces[..]);
        let active = monitor.and_then(|monitor| monitor.active);

        let order: Vec<i64> = self.dots.iter().map(|dot| dot.id).collect();
        let mut old = std::mem::take(&mut self.dots);
        let mut next: Vec<Dot> = Vec::with_capacity(workspaces.len() + old.len());
        for workspace in workspaces {
            let state = if Some(workspace.id) == active {
                DotState::Active
            } else if workspace.urgent {
                DotState::Urgent
            } else if workspace.occupied {
                DotState::Occupied
            } else {
                DotState::Empty
            };
            let width = if state == DotState::Active { ACTIVE_W } else { DOT };
            let dot = match old.iter().position(|dot| dot.id == workspace.id) {
                Some(index) => {
                    let mut dot = old.remove(index);
                    dot.state = state;
                    dot.width.retarget(width, MOTION);
                    dot.presence.retarget(1.0, MOTION);
                    dot
                }
                None => {
                    let mut presence = Spring::new(0.0, MOTION, 0.002);
                    presence.retarget(1.0, MOTION);
                    Dot {
                        id: workspace.id,
                        state,
                        width: Spring::new(width, MOTION, 0.05),
                        presence,
                    }
                }
            };
            next.push(dot);
        }

        for mut gone in old {
            gone.presence.retarget(0.0, MOTION);
            let at = order
                .iter()
                .position(|id| *id == gone.id)
                .and_then(|index| {
                    order[..index]
                        .iter()
                        .rev()
                        .find_map(|before| next.iter().position(|dot| dot.id == *before))
                })
                .map_or(0, |index| index + 1);
            next.insert(at, gone);
        }
        self.dots = next;
    }
}

impl Local for Dots {
    fn advance(&mut self, dt: f32, now: Instant) {
        for dot in &mut self.dots {
            dot.width.advance(dt, now);
            dot.presence.advance(dt, now);
        }
        self.dots.retain(|dot| !(dot.presence.target() == 0.0 && dot.presence.is_settled()));
    }

    fn is_settled(&self) -> bool {
        self.dots.iter().all(|dot| dot.width.is_settled() && dot.presence.is_settled())
    }

    fn pending_width(&self, _spacing: f32) -> f32 {
        self.dots
            .iter()
            .map(|dot| {
                (dot.width.target() + GAP) * dot.presence.target()
                    - (dot.width.value() + GAP) * dot.presence.value().clamp(0.0, 1.0)
            })
            .sum()
    }
}

fn focus(id: i64) {
    if let Some(backend) = compositor::backend() {
        std::thread::spawn(move || backend.focus_workspace(id));
    }
}

fn scroll(scrolled: &Cell<f32>, event: &ScrollWheelEvent) {
    let mut left = scrolled.get();
    let steps = island::notches(&mut left, event);
    scrolled.set(left);
    if steps != 0
        && let Some(backend) = compositor::backend()
    {
        std::thread::spawn(move || backend.focus_relative(-steps));
    }
}

pub struct Workspaces;

impl Feature for Workspaces {
    const NAME: &'static str = "workspaces";

    type State = ();
    type Local = Dots;
    type Event = ();

    fn start(_: Feed<()>) -> Self {
        Self
    }

    fn sync_local(_: &(), dots: &mut Dots, island: &IslandInfo<'_>, _animate: bool) {
        dots.update(island.monitor);
    }

    fn widget(&self, _: &str, _: &(), w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        let dots = w.local::<Dots>()?;
        if dots.dots.is_empty() {
            return None;
        }
        let scrolled = dots.scrolled.clone();
        let hover = w.hover();
        Some(
            w.group("bar-workspaces")
                .pl(px((w.padding() - GAP / 2.0).max(0.0)))
                .pr(px((w.padding() - GAP / 2.0).max(0.0)))
                .justify_start()
                .on_scroll_wheel(move |event, _, _| scroll(&scrolled, event))
                .children(dots.dots.iter().map(|dot| {
                    let presence = dot.presence.value().clamp(0.0, 1.0);
                    let width = dot.width.value();
                    let color = match dot.state {
                        DotState::Active => rgb(ACCENT),
                        DotState::Urgent => rgb(CRITICAL),
                        DotState::Occupied => rgb(SUBTEXT1),
                        DotState::Empty => rgb(OVERLAY1),
                    };
                    let id = dot.id;
                    div()
                        .id(SharedString::from(format!("workspace-{id}")))
                        .relative()
                        .h_full()
                        .w(px((width + GAP) * presence))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .when(hover, |cell| cell.hover(|style| style))
                        .opacity(presence)
                        .tip(w.tips(), format!("workspace-{id}"), format!("Workspace {id}"))
                        .on_click(move |_, _, _| focus(id))
                        .child(
                            div()
                                .absolute()
                                .size_full()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .relative()
                                        .w_full()
                                        .h(px(DOT + GAP))
                                        .child(ui::control(Radius::Pill, None, hover)),
                                ),
                        )
                        .child(div().w(px(width * presence)).h(px(DOT)).rounded_full().bg(color))
                }))
                .into_any_element(),
        )
    }
}
