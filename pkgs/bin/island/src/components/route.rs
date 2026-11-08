//! Something on its way through a series of stops, each reached, on its way, or not yet, such as a
//! pull request through the branches it lands in, or a task through its steps, one to a line.

use crate::panel::SharedString;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StopState {
    Reached,
    /// On its way, with how far it has to go, such as "+63".
    Pending(Option<SharedString>),
    NotYet,
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
