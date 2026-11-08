//! A search field over a list of results, with an optional preview of the selected one beside it,
//! such as apps to launch or clipboard history to paste.

use std::{cell::Cell, path::PathBuf, rc::Rc};

use crate::panel::SharedString;

use crate::{panel::Handler, panel::Ui, spring::SpringConfig};

pub const SELECTION_SPRING: SpringConfig = SpringConfig::new(560.0, 47.3, 1.0);

pub struct SearchRow {
    pub name: SharedString,
    pub detail: SharedString,
    pub icon: RowIcon,
    pub selected: bool,
    pub on_click: Handler,
    /// Hovering a row selects it.
    pub on_hover: Handler,
}

pub enum RowIcon {
    /// Full color, such as an app's icon.
    Image(PathBuf),
    /// Symbolic, tinted like the text beside it.
    Glyph(PathBuf),
    /// A picture to preview, drawn wider than an icon.
    Picture(PathBuf),
    None,
}

/// Moves the rows in view by a number of rows, leaving the selection where it is.
pub type Scroll = Rc<dyn Fn(i32, &mut dyn Ui)>;

pub enum Preview {
    Text(SharedString),
    Picture(PathBuf),
    Empty,
}

pub struct SearchView {
    pub query: SharedString,
    pub placeholder: SharedString,
    /// Shown when there are no rows and nothing is typed.
    pub empty: SharedString,
    pub rows: Vec<SearchRow>,
    /// Rows may show pictures, so every row's icon slot is made wide enough for one and names
    /// line up however many are in view.
    pub pictures: bool,
    pub on_scroll: Scroll,
    pub preview: Option<(Preview, SharedString)>,
}

thread_local! {
    /// Pixels scrolled over a list that do not yet add up to a whole row.
    static SCROLLED: Cell<f32> = const { Cell::new(0.0) };
}
