//! Picks one of several values, showing the current one while closed, such as which device an app
//! plays on. Opening grows the chip itself into the list, down and equally to both sides, as the
//! island grows into a panel: one shape, whose size, corners and color move from the chip's to the
//! list's, the choices fading in where they stay as it grows. Closing folds it back into the chip. The list keeps its order, the current choice marked in its place.

use std::{path::PathBuf, rc::Rc};

use crate::panel::SharedString;

use crate::{panel::Handler, panel::Ui};

#[derive(Clone)]
pub struct Choice {
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    /// At the row's end in the open list, such as how many it holds; the chip shows only the label.
    pub detail: Option<SharedString>,
}

pub type Select = Rc<dyn Fn(usize, &mut dyn Ui)>;

pub struct Dropdown {
    pub id: SharedString,
    pub choices: Vec<Choice>,
    pub selected: Option<usize>,
    pub open: bool,
    /// Opens or closes it; also called to close it when a click lands elsewhere.
    pub on_toggle: Handler,
    pub on_select: Select,
    /// As tall as this and drawn as a strip, such as beside a strip of tabs; otherwise a chip.
    pub height: Option<f32>,
}
