//! A search field over a list of results, with an optional preview of the selected one beside it,
//! such as apps to launch or clipboard history to paste. The results may be filtered by a strip of
//! tabs under the field and parted into labelled groups.

use std::{path::PathBuf, rc::Rc};

use crate::panel::SharedString;

use crate::{
    components::{
        chip::Chip,
        item::Trail,
        table::Table,
        tabs::{Select, TabLabel},
    },
    panel::{Code, Handler, IconButton, Ui},
    spring::SpringConfig,
};

pub const SELECTION_SPRING: SpringConfig = SpringConfig::new(560.0, 47.3, 1.0);

/// A group label takes half a row's place, so two take one row's.
pub const SECTION_SHARE: f32 = 0.5;

pub struct SearchRow {
    pub name: SharedString,
    pub detail: SharedString,
    pub icon: RowIcon,
    /// The name in the monospaced face, such as a command.
    pub mono: bool,
    /// Over the row, naming the group it starts.
    pub section: Option<SharedString>,
    /// A small glyph after the words, in its color, such as a pin.
    pub mark: Option<(PathBuf, u32)>,
    /// At the row's end, such as how old what it names is.
    pub trail: Vec<Trail>,
    pub selected: bool,
    pub on_click: Handler,
    /// Hovering a row selects it.
    pub on_hover: Handler,
}

pub enum RowIcon {
    /// Full color, such as an app's icon.
    Image(PathBuf),
    /// Symbolic, in its color on a square washed in it, such as the kind of an entry.
    Tinted(PathBuf, u32),
    /// A square of the color itself.
    Swatch(u32),
    /// A picture to preview; drawn wider than an icon when the view has `pictures`.
    Picture(PathBuf),
    None,
}

/// Moves the rows in view by a number of rows, leaving the selection where it is.
pub type Scroll = Rc<dyn Fn(i32, &mut dyn Ui)>;

pub struct Filters {
    pub tabs: Vec<TabLabel>,
    pub selected: usize,
    pub on_select: Select,
}

pub struct FileRow {
    pub icon: RowIcon,
    pub name: SharedString,
    /// Fainter, at the far end, such as the folder it is in.
    pub place: SharedString,
}

pub enum Preview {
    Text(SharedString),
    Picture(PathBuf),
    /// Numbered lines filling the pane.
    Code(Code),
    /// An address under the site it leads to.
    Link {
        glyph: Option<PathBuf>,
        color: u32,
        site: SharedString,
        address: SharedString,
    },
    /// A color filling the pane over its values, named.
    Swatch {
        color: u32,
        values: Vec<(SharedString, SharedString)>,
    },
    Files(Vec<FileRow>),
    Empty,
}

pub struct SearchPreview {
    /// Naming what the selected result is.
    pub tag: Option<Chip>,
    pub detail: SharedString,
    /// At the far end of the heading, such as a pin.
    pub mark: Option<(PathBuf, u32)>,
    pub content: Preview,
    /// Under the content, their glyphs beside their labels.
    pub buttons: Vec<IconButton>,
    /// At the far end of the buttons, such as forgetting it.
    pub end: Vec<IconButton>,
}

pub struct SearchView {
    /// Before what is typed, such as a magnifying glass.
    pub glyph: Option<PathBuf>,
    pub query: SharedString,
    pub placeholder: SharedString,
    /// Shown when there are no rows and nothing is typed.
    pub empty: SharedString,
    pub filters: Option<Filters>,
    pub rows: Vec<SearchRow>,
    /// Rows may show pictures, so every row's icon slot is made wide enough for one and names
    /// line up however many are in view.
    pub pictures: bool,
    pub on_scroll: Scroll,
    pub preview: Option<SearchPreview>,
    /// As wide as what holds it, such as a card beside what is picked in it.
    pub fill: bool,
    /// A smaller field and rows, as a list inside a card rather than a panel of its own.
    pub compact: bool,
    /// The list this tall rather than eight rows, such as filling the card it is in; it then
    /// holds every row and the wheel scrolls it, rather than its owner.
    pub list_height: Option<f32>,
    /// Words at the end of the filters' row, such as how many results there are.
    pub status: Option<SharedString>,
    /// The list this wide, with what is beside it taking the rest.
    pub list_width: Option<f32>,
    /// Beside the list in place of a preview, such as a card about what is picked.
    pub beside: Vec<crate::panel::Block>,
    /// Typing goes to its field, so the caret shows there.
    pub focused: bool,
    /// Its field clicked, such as to take the typing from another field.
    pub on_focus: Option<Handler>,
    /// The results as rows of cells under headings in place of `rows`, such as files with their
    /// sizes.
    pub table: Option<Table>,
}
