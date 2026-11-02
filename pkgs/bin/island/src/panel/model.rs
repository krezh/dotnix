//! The panel: a header and the blocks a feature describes it with, laid out from components.
//! Blocks are general; nothing here knows which feature uses them, and each draws through its own
//! component.

use std::{
    cell::Cell,
    hash::{DefaultHasher, Hash, Hasher},
    mem::discriminant,
    path::PathBuf,
    rc::Rc,
};

use gpui_shared_string::SharedString;

use super::{
    Actions, CalendarView, Code, Dropdown, FrameView, InputView, Item, Line, Mark, Meter,
    MeterShape, Piece, PlayerView, PromptView, Route, SearchView, Series, Slider, TabLabel, Text,
    Tone, Transcript, Turn, Ui,
};
use crate::components::{
    card, chart::Phase, ledger::Ledger, list::ROWS, tabs, tile::Tile, timeline::Timeline,
};

pub type Handler = Rc<dyn Fn(&mut dyn Ui)>;

pub fn on(handler: impl Fn(&mut dyn Ui) + 'static) -> Handler {
    Rc::new(handler)
}

pub struct Button {
    pub id: SharedString,
    pub label: SharedString,
    pub glyph: Option<PathBuf>,
    pub bare: bool,
    pub active: bool,
    pub toggle: bool,
    /// Every label the button can show, when it changes label, so it keeps one width.
    pub fits: Vec<SharedString>,
    pub busy: bool,
    /// A button that sometimes has nothing to do stays in place, drawn as inactive, so the header
    /// keeps its size.
    pub enabled: bool,
    pub on_click: Handler,
}

pub enum Header {
    Tabs,
    Title(SharedString, Vec<Button>),
    Views(Views),
    None,
}

/// What sits at the start of a header with tabs: a mark, with a faint line over a title beside it,
/// such as who a panel is about and what it is doing.
pub struct HeaderLead {
    pub mark: Mark,
    pub size: f32,
    pub overline: SharedString,
    pub title: SharedString,
    pub on_click: Option<Handler>,
}

pub struct Views {
    /// At the start of the row, before the centred tabs; the header grows to fit it.
    pub lead: Option<HeaderLead>,
    pub tabs: Vec<TabLabel>,
    pub selected: usize,
    pub on_select: tabs::Select,
    /// Beside the tabs in a page's row, such as which of a view's items to show.
    pub pick: Option<Dropdown>,
    pub buttons: Vec<Button>,
    /// After the buttons in a page's row, in a strip of their own, each group parted from the next
    /// by a line and each button named by its tooltip.
    pub icons: Vec<Vec<IconButton>>,
    /// The view being left, while the picked one slides in over it.
    pub passing: Option<Passing>,
}

/// One view moving out as another comes in, as their tabs sit side by side.
pub struct Passing {
    pub blocks: Vec<Block>,
    /// How far across, 0 to 1.
    pub at: f32,
    /// The view coming in has its tab to the right of the one leaving.
    pub rightwards: bool,
    /// How wide the leaving view was as it started leaving, kept across frames: the panel is
    /// already changing size, and it has to clear what it passes.
    pub width: Rc<Cell<Option<f32>>>,
}

/// Shows its glyph, with its label as the tooltip; without a glyph, its label.
pub struct IconButton {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    /// Its glyph's color, filling the button while active, as the session's actions are drawn.
    pub color: Option<u32>,
    pub label: SharedString,
    pub active: bool,
    pub badge: Option<SharedString>,
    pub on_click: Handler,
}

pub struct ButtonRow {
    pub buttons: Vec<IconButton>,
    /// Control-sized and closer together, such as a message's actions.
    pub compact: bool,
    /// From the left, such as to line up with an item's text past its lead.
    pub indent: f32,
    /// A menu of further actions at the row's far end.
    pub menu: Option<Actions>,
    /// At the row's far end, such as actions closing what is shown.
    pub end: bool,
}

/// Lines of history over a shared ceiling, as wide as their place and this tall.
pub struct ChartView {
    pub id: SharedString,
    pub series: Vec<Series>,
    pub height: f32,
    /// The clock of a history sampled on its own; otherwise the system sampler sets the pace.
    pub phase: Option<Phase>,
}

/// Where a block sits in a [`Block::Placed`], and how faded.
pub struct Place {
    /// From both sides.
    pub inset: f32,
    /// Moved this far right, such as a card sliding out.
    pub offset: f32,
    pub y: f32,
    pub height: f32,
    pub opacity: f32,
    /// `y` counts up from where the island ends, such as a footer that keeps to the bottom as
    /// the island grows.
    pub from_bottom: bool,
    /// Drawn this tall, its bottom at the place's, and cut to the place, such as the edge of a
    /// card showing below another.
    pub drawn: Option<f32>,
}

/// Blocks each at its own place, such as cards sliding and fading as a list changes.
pub struct Placed {
    pub width: f32,
    pub height: f32,
    /// Shows this much and scrolls the rest.
    pub visible: Option<f32>,
    /// Back to front.
    pub blocks: Vec<(Place, Block)>,
}

pub struct ActionTile {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    pub label: SharedString,
    /// How what it switches is now, such as how many are connected; it then shows as a switch,
    /// armed while on.
    pub state: Option<SharedString>,
    /// Its fill while armed.
    pub color: u32,
    pub armed: bool,
    pub on_click: Handler,
}

/// Something needing a look, such as a battery running low, with the value that raised it.
pub struct Warning {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    pub label: SharedString,
    pub value: SharedString,
    pub color: u32,
    pub on_click: Option<Handler>,
}

/// How [`Block::Columns`] shares its blocks between columns.
pub enum Split {
    /// Each block a column of its own, scrolling on its own once taller than a panel's room, such
    /// as a list beside what is picked in it.
    Scrolling,
    /// Cards shared between two columns of about the same height, each named by its title.
    Balanced,
}

pub enum Block {
    Card(Option<card::Title>, card::Size, Vec<Block>),
    Row(Vec<Block>),
    /// This many equal columns, filled a row at a time; cells line up at the top.
    Grid(usize, Vec<Block>),
    /// The blocks go under its title.
    Item(Item, Vec<Block>),
    /// Blocks on a surface of their own, such as one of many in a grid, or a message.
    Frame(FrameView, Vec<Block>),
    Placed(Placed),
    Slider(Slider),
    /// A level and its recent peak, both 0 to 1.
    Segments(f32, f32),
    Text(Text),
    Search(SearchView),
    Calendar(CalendarView),
    Meter(Meter),
    Player(PlayerView),
    /// Two-line entries, such as commits, newest first.
    Lines(Vec<Line>),
    Routes(Vec<Route>),
    /// Columns of tiles.
    Board(Vec<Vec<Tile>>),
    Buttons(ButtonRow),
    Actions(Vec<ActionTile>),
    /// Tiles sharing the width, the armed one chosen, such as devices to switch to.
    Tiles(Vec<ActionTile>),
    Code(Code),
    Input(InputView),
    Transcript(Transcript),
    /// Side by side, as the split says.
    Columns(Split, Vec<Block>),
    /// Label and value chips, three to a row, as a tile shows them; a value may have a color.
    Facts(Vec<(SharedString, SharedString, Option<u32>)>),
    /// A large name over fainter lines, as a tile shows a host.
    Hero(SharedString, Vec<SharedString>),
    /// Lanes along one stretch of time from now.
    Timeline(Timeline),
    Ledger(Ledger),
    /// A small card over the whole panel asking for something, wherever it is in the blocks.
    Prompt(PromptView),
    /// A page's own views and their buttons, in a row at its top, for a tab of the panel that has
    /// views of its own.
    Views(Views),
    /// Action tiles that are switches, with their states, four to a row.
    Switches(Vec<ActionTile>),
    /// In a row that wraps.
    Warnings(Vec<Warning>),
    /// This many to a row; rings take as many as fit.
    Meters(usize, Vec<Meter>),
    Chart(ChartView),
}

pub struct PanelView {
    pub header: Header,
    pub blocks: Vec<Block>,
    /// Shows what fits and scrolls the rest, such as a conversation; otherwise the panel is as
    /// tall as what it shows.
    pub scroll: bool,
}

/// The most columns side by side, each with its own scroll.
pub const COLUMNS: usize = 4;

impl Block {
    pub fn note(text: impl Into<SharedString>) -> Self {
        Block::Text(Text::new(Tone::Note, text))
    }

    pub fn heading(text: impl Into<SharedString>) -> Self {
        Block::Text(Text::new(Tone::Heading, text))
    }

    pub fn caption(text: impl Into<SharedString>, color: u32) -> Self {
        Block::Text(Text::new(Tone::Caption(color), text))
    }

    /// Whether it, or anything in it, is a conversation the panel should follow to its end.
    pub fn follows_end(&self) -> bool {
        match self {
            Block::Transcript(_) => true,
            Block::Card(_, _, blocks)
            | Block::Row(blocks)
            | Block::Columns(_, blocks)
            | Block::Grid(_, blocks)
            | Block::Frame(_, blocks)
            | Block::Item(_, blocks) => blocks.iter().any(Block::follows_end),
            _ => false,
        }
    }

    /// Names a card in balanced columns across frames: its title, or else its place.
    pub(super) fn balanced_key(&self, index: usize) -> SharedString {
        match self {
            Block::Card(Some(title), _, _) => title.text.clone(),
            _ => SharedString::from(format!("card-{index}")),
        }
    }

    fn hash_shape(&self, hasher: &mut DefaultHasher) {
        discriminant(self).hash(hasher);
        match self {
            Block::Calendar(_) | Block::Slider(_) | Block::Segments(..) => {}
            Block::Text(text) => text.lines.hash(hasher),
            // What it shows decides its size, not where its blocks are in their motion.
            Block::Placed(placed) => placed
                .visible
                .unwrap_or(placed.height)
                .to_bits()
                .hash(hasher),
            Block::Meter(meter) => (meter.level.is_some(), discriminant(&meter.shape)).hash(hasher),
            Block::Tiles(tiles) => tiles.len().hash(hasher),
            Block::Item(item, blocks) => {
                (item.lead.is_some(), item.trail.len()).hash(hasher);
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            Block::Frame(_, blocks)
            | Block::Row(blocks)
            | Block::Columns(Split::Scrolling, blocks) => {
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            // Which column each card goes in decides the height as much as the cards do.
            Block::Columns(Split::Balanced, blocks) => {
                let keys: Vec<SharedString> = blocks
                    .iter()
                    .enumerate()
                    .map(|(index, block)| block.balanced_key(index))
                    .collect();
                card::balance(&keys).hash(hasher);
                for (key, block) in keys.iter().zip(blocks) {
                    key.hash(hasher);
                    block.hash_shape(hasher);
                }
            }
            Block::Grid(columns, blocks) => {
                columns.hash(hasher);
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            Block::Lines(lines) => lines.len().hash(hasher),
            Block::Routes(routes) => {
                for route in routes {
                    route.done.is_some().hash(hasher);
                    for stop in &route.stops {
                        (&stop.label, &stop.state).hash(hasher);
                    }
                }
            }
            Block::Board(columns) => {
                for column in columns {
                    column
                        .iter()
                        .map(|tile| tile.span)
                        .collect::<Vec<_>>()
                        .hash(hasher);
                }
            }
            Block::Buttons(row) => (row.buttons.len(), row.compact).hash(hasher),
            Block::Actions(tiles) => tiles.len().hash(hasher),
            Block::Search(view) => (ROWS, view.preview.is_some()).hash(hasher),
            Block::Code(code) => {
                (code.heading.is_some(), code.wrap, code.lines.len()).hash(hasher);
                if code.wrap {
                    for line in &code.lines {
                        line.text.hash(hasher);
                    }
                }
            }
            Block::Facts(items) => items.len().hash(hasher),
            Block::Hero(_, lines) => lines.len().hash(hasher),
            Block::Timeline(view) => {
                (view.side.is_some(), view.lanes.len(), !view.pins.is_empty()).hash(hasher)
            }
            Block::Ledger(ledger) => {
                let overflows = ledger.overflows();
                (ledger.columns.is_some(), overflows).hash(hasher);
                if !overflows {
                    ledger.entries.len().hash(hasher);
                    for entry in &ledger.entries {
                        (entry.bars.len(), entry.folded.is_some()).hash(hasher);
                    }
                }
            }
            // It floats over the panel, so it never decides its size.
            Block::Prompt(_) => {}
            Block::Views(views) => (
                views.tabs.len(),
                views.buttons.len(),
                views.icons.iter().map(Vec::len).collect::<Vec<_>>(),
                views.pick.is_some(),
            )
                .hash(hasher),
            // Text that wraps decides the height, so the panel grows as it is typed or arrives.
            Block::Input(view) => view.text.hash(hasher),
            Block::Transcript(transcript) => {
                for turn in &transcript.turns {
                    match turn {
                        Turn::Mine(text) => text.text.hash(hasher),
                        Turn::Theirs {
                            pieces,
                            streaming,
                            actions,
                            ..
                        } => {
                            (streaming, actions.len()).hash(hasher);
                            for piece in pieces {
                                match piece {
                                    Piece::Text(text) => text.text.hash(hasher),
                                    Piece::Code(code) => code.lines.len().hash(hasher),
                                }
                            }
                        }
                        Turn::Waiting { .. } => 0u8.hash(hasher),
                    }
                }
            }
            Block::Player(view) => (view.art.is_some(), view.length.is_some()).hash(hasher),
            Block::Card(title, size, blocks) => {
                (title.is_some(), size).hash(hasher);
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            Block::Switches(tiles) => tiles.len().hash(hasher),
            // They wrap, so what they say decides how many lines they take.
            Block::Warnings(warnings) => {
                for warning in warnings {
                    (&warning.label, &warning.value).hash(hasher);
                }
            }
            Block::Meters(columns, meters) => {
                (columns, meters.len()).hash(hasher);
                for meter in meters {
                    discriminant(&meter.shape).hash(hasher);
                    if let MeterShape::History { history, .. } = &meter.shape {
                        history.is_some().hash(hasher);
                    }
                }
            }
            Block::Chart(view) => view.height.to_bits().hash(hasher),
        }
    }
}

impl PanelView {
    pub fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        discriminant(&self.header).hash(&mut hasher);
        self.scroll.hash(&mut hasher);
        match &self.header {
            Header::Title(title, buttons) => {
                title.hash(&mut hasher);
                buttons.len().hash(&mut hasher);
            }
            Header::Views(views) => {
                views.tabs.len().hash(&mut hasher);
                views.buttons.len().hash(&mut hasher);
            }
            Header::Tabs | Header::None => {}
        }
        for block in &self.blocks {
            block.hash_shape(&mut hasher);
        }
        hasher.finish()
    }
}
