//! A row in a panel: a lead, a title line ending in amounts, statuses or a dropdown, and
//! whatever it holds under the title, such as a slider. With a line over the title it reads as a
//! message: a faint line naming where it is from, then the title, larger, then what it says.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{
    components::{
        badge::Mark, chip::Chip, dropdown::Dropdown, search::Filters, stepper::Stepper,
    },
    panel::Handler,
    spring::SpringConfig,
};

pub const TURN_SPRING: SpringConfig = SpringConfig::new(300.0, 34.6, 1.0);

/// Between the lead and the text.
pub const LEAD_GAP: f32 = 12.0;
/// The line over the title, and the title under it.
pub const OVERLINE_H: f32 = 16.0;
pub const HEADLINE_H: f32 = 18.0;
/// Between the lines of a message.
pub const MESSAGE_GAP: f32 = 2.0;

/// What sits left of the title.
#[derive(Default)]
pub struct Lead {
    pub mark: Option<Mark>,
    /// A square of this color behind the mark.
    pub tint: Option<u32>,
    /// A color showing through at a strength from 0 to 1, such as a level heard right now.
    pub glow: Option<(u32, f32)>,
    /// Drawn faint, such as something muted.
    pub faded: bool,
    /// The square's side, with the mark filling it; otherwise a standard square the mark sits in.
    pub size: Option<f32>,
    pub on_click: Option<Handler>,
    /// Stands at the top, beside the first lines, rather than centred on the row, such as a large
    /// lead beside a tall card.
    pub top: bool,
}

/// What sits at the end of the first line, in order.
pub enum Trail {
    Text(SharedString, u32),
    /// Quiet, such as how long ago.
    Faint(SharedString),
    /// In a tinted pill, such as how old something is once that matters.
    Chip(Chip),
    /// A dot before the text, both in the color, such as "Recording".
    Status(SharedString, u32),
    Dropdown(Dropdown),
    /// A ✕ that shows while the pointer is over `group`, such as the frame the item is in.
    Close {
        group: SharedString,
        on_click: Handler,
    },
    /// A quiet button, such as one showing more.
    Button(TrailButton),
    /// A tick box, such as whether the row is in a plan; `None` when it cannot change.
    Tick(bool, Option<Handler>),
    Stepper(Stepper),
    /// A short field this wide, such as a duration, its text typed where focus sends it.
    Field(Field),
    /// A strip of tabs, each as wide as its words and count, such as which rows a list shows.
    Strip(Filters),
}

pub struct Field {
    pub text: SharedString,
    pub placeholder: SharedString,
    pub width: f32,
    pub focused: bool,
    pub on_focus: Option<Handler>,
}

pub struct TrailButton {
    pub label: Option<SharedString>,
    /// Before the label.
    pub glyph: Option<PathBuf>,
    /// The glyph turned upside down, turning as it changes, such as a chevron for more or less.
    pub turned: bool,
    /// Tinted with its own color, such as one putting something right.
    pub tint: Option<u32>,
    pub on_click: Handler,
}

/// A row: an optional lead, a title line, and whatever the item holds under the title.
#[derive(Default)]
pub struct Item {
    pub id: SharedString,
    pub lead: Option<Lead>,
    /// A faint line over the title, such as where a message is from; the trail moves onto it.
    pub overline: Option<SharedString>,
    pub title: SharedString,
    pub title_color: Option<u32>,
    /// A longer title goes onto more lines rather than ending in an ellipsis.
    pub wrap: bool,
    /// Fainter, after the title.
    pub detail: SharedString,
    /// Words in a color of their own, after the line they belong to.
    pub tinted: Vec<Tint>,
    pub trail: Vec<Trail>,
    /// The title larger than a row's, from `ui`'s sizes, such as a card's name or a figure.
    pub title_size: Option<f32>,
    /// The title in the monospaced face, such as a commit.
    pub title_mono: bool,
    /// Quiet words before the title, naming what it is, such as "Locked".
    pub label: Option<SharedString>,
    /// The trail beside what is under the title too, centred on it all, such as a control beside
    /// a setting's name and hint.
    pub trail_beside: bool,
    /// After the label, before the title, such as tabs choosing which rows under it show.
    pub before: Vec<Trail>,
}

/// Words that stand out in their color, such as how urgent a message is or where it is from.
pub enum Tint {
    /// After the line over the title.
    Overline(SharedString, u32),
    /// After the title, before its detail.
    Title(SharedString, u32),
    /// After the title in a tinted pill, such as what kind of thing it names.
    Chip(SharedString, u32),
}
