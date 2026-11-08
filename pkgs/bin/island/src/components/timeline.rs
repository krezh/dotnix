//! Spans and pins along one stretch of time, packed onto lanes over dashed gridlines and an axis.

use crate::panel::SharedString;

pub struct Span {
    pub label: SharedString,
    pub tooltip: SharedString,
    pub from: f32,
    pub to: f32,
    pub color: u32,
    pub start_tick: bool,
    pub end_dot: bool,
}

pub struct Pin {
    pub at: f32,
    pub label: SharedString,
    pub tooltip: SharedString,
    pub color: u32,
    pub width: f32,
}

pub struct Side {
    pub title: SharedString,
    pub detail: SharedString,
    pub striped_key: Option<SharedString>,
    pub dot_key: Option<SharedString>,
}

pub struct Timeline {
    pub side: Option<Side>,
    pub lanes: Vec<Vec<Span>>,
    pub pins: Vec<Pin>,
    pub axis: Vec<SharedString>,
}
