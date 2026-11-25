//! A count stepped down or up by one, such as how many of something to keep: − and + either side
//! of it, each left out of reach at its end.

use crate::panel::{Handler, SharedString};

pub struct Stepper {
    pub value: SharedString,
    /// `None` at the lowest it goes.
    pub on_less: Option<Handler>,
    /// `None` at the highest it goes.
    pub on_more: Option<Handler>,
}
