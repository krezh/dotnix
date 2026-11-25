//! Something on its way through a series of stops, each reached, on its way, not yet, or where it
//! failed, such as a pull request through the branches it lands in, or a task through its steps,
//! one to a line or across a header.

use crate::panel::SharedString;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StopState {
    Reached,
    /// On its way, with how far it has to go, such as "+63".
    Pending(Option<SharedString>),
    NotYet,
    /// Stopped there, such as a build that failed.
    Failed,
}

pub struct Stop {
    pub label: SharedString,
    pub state: StopState,
}

pub struct Route {
    /// With the detail, left out when empty.
    pub title: SharedString,
    pub detail: SharedString,
    pub done: Option<SharedString>,
    /// One stop to a line, joined down the left, rather than wrapping across.
    pub vertical: bool,
    pub stops: Vec<Stop>,
}
