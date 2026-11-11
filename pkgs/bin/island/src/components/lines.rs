//! A list of two-line entries divided by rules, such as commits: a summary over a fainter detail,
//! each with a menu of what to do with it at its end.

use super::actions::Actions;
use crate::panel::SharedString;

pub struct Line {
    pub summary: SharedString,
    pub detail: SharedString,
    /// Things to do with this entry, such as locking an input at this commit.
    pub menu: Option<Actions>,
}
