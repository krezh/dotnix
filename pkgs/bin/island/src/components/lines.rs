//! A list of two-line entries divided by rules, such as commits: a summary over a fainter detail.

use crate::panel::SharedString;

pub struct Line {
    pub summary: SharedString,
    pub detail: SharedString,
}
