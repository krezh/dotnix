//! A month of days, with today marked and buttons to step between months.

use crate::panel::Handler;

pub struct CalendarView {
    /// Months away from the current one.
    pub offset: i32,
    pub on_previous: Handler,
    pub on_next: Handler,
    pub on_today: Handler,
}
