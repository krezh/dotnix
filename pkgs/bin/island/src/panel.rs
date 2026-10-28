//! The panel: a header and the blocks a feature describes it with, laid out from components.
//! Blocks are general; nothing here knows which feature uses them, and each draws through its own
//! component.

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    mem::discriminant,
    path::PathBuf,
    rc::Rc,
};

use gpui::{AnyElement, Context, FontWeight, ScrollHandle, SharedString, div, prelude::*, px, rgb};

use crate::{
    components::{
        actions, badge, bars, button, calendar, card, chart, code, disclose, figure, input, item,
        lines,
        list::ROWS,
        marquee::TextLayouts,
        meter, motion, pill, player,
        popover::Popovers,
        prompt, reveal, route, search, segments, slider, tabs, text,
        tile::{self, Tile},
        timeline::{self, Timeline},
        tooltip::{Tip, Tooltips},
        transcript,
    },
    island::{Island, Mode},
    theme::OVERLAY2,
    ui::{
        self, BLOCK_GAP, CARD_PAD, CONTROL_H, HEADER_GAP, HEADER_H, HEADING_SIZE, INSET,
        SMALL_SIZE, TIGHT_INSET,
    },
};

pub use crate::components::{
    actions::{Action, Actions},
    badge::Mark,
    bars::Bars,
    chart::Series,
    disclose::Disclosure,
    figure::Figure,
    route::{Hop, Path},
    button::Rising,
    calendar::CalendarView,
    card::{Countdown, FrameView},
    code::{Change, Code, CodeLine, Motion as CodeMotion},
    dropdown::{Choice, Dropdown},
    input::{Edit, InputView},
    item::{Item, Lead, Trail},
    lines::Line,
    meter::Meter,
    player::PlayerView,
    prompt::PromptView,
    route::{Route, Stop, StopState},
    search::{Preview, SearchRow, SearchView},
    slider::Slider,
    tabs::TabLabel,
    text::{Style as TextStyle, Text, Tone},
    transcript::{Piece, Transcript, Turn},
};

pub type Handler = Rc<dyn Fn(&mut Island, &mut Context<Island>)>;

pub fn on(handler: impl Fn(&mut Island, &mut Context<Island>) + 'static) -> Handler {
    Rc::new(handler)
}

pub struct Button {
    pub id: SharedString,
    pub label: SharedString,
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
    pub buttons: Vec<Button>,
}

/// Shows its glyph, with its label as the tooltip; without a glyph, its label.
pub struct IconButton {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    /// Parts of the glyph that rise in while the button is active.
    pub rising: Option<Rising>,
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
}

/// Where a block sits in a [`Block::Placed`], and how faded.
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub height: f32,
    pub opacity: f32,
    /// `y` counts up from where the island ends, such as a footer that keeps to the bottom as
    /// the island grows.
    pub from_bottom: bool,
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
    /// Its fill while armed.
    pub color: u32,
    pub armed: bool,
    pub on_click: Handler,
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
    /// Side by side, each scrolling on its own once taller than a panel's room, such as a list
    /// beside what is picked in it.
    Columns(Vec<Block>),
    /// Label and value chips, three to a row, as a tile shows them; a value may have a color.
    Facts(Vec<(SharedString, SharedString, Option<u32>)>),
    /// A large name over fainter lines, as a tile shows a host.
    Hero(SharedString, Vec<SharedString>),
    /// Lanes along one stretch of time from now.
    Timeline(Timeline),
    /// A small card over the whole panel asking for something, wherever it is in the blocks.
    Prompt(PromptView),
    /// A page's own views and their buttons, in a row at its top, for a tab of the panel that has
    /// views of its own.
    Views(Views),
    /// Cards side by side, all this tall, so rows opening inside them never resize the panel.
    Band(f32, Vec<Block>),
    /// Rows divided by rules that start this far in, such as past the rows' leads.
    List(f32, Vec<Block>),
    /// What a row keeps folded away until it opens.
    Disclose(Disclosure, Vec<Block>),
    Chart(ChartView),
    /// Each optionally led by a coloured dot, such as under a chart.
    Legend(Vec<(Option<u32>, SharedString)>),
    /// Label and value chips, this many to a row.
    FactsIn(usize, Vec<(SharedString, SharedString, Option<u32>)>),
    /// Values at a glance; the block beside them sits before the small ones.
    Figures(Vec<Figure>, Option<Box<Block>>),
    Bars(Bars),
    Path(Path),
    /// A thin bar filled part way in a color, such as one app's share under its name.
    Level(SharedString, f32, u32),
    /// A card's views on a row of their own, across its width, such as where its title has no
    /// room for them.
    Segmented(SegmentedView),
}

pub struct SegmentedView {
    pub id: SharedString,
    pub labels: Vec<TabLabel>,
    pub selected: usize,
    pub on_select: tabs::Select,
}

/// A history chart outside a tile; without a height, it takes what its card has spare.
pub struct ChartView {
    pub id: SharedString,
    pub series: Vec<Series>,
    pub height: Option<f32>,
    /// How far it is between its last value and the next, when its values come at their own
    /// pace; it then redraws steadily so it scrolls smoothly between them.
    pub phase: Option<f32>,
}

/// How often a chart scrolling at its own pace redraws: often enough that a slow scroll moves a
/// little every frame rather than stepping.
const CHART_FRAME: std::time::Duration = std::time::Duration::from_micros(16_667);

pub struct PanelView {
    pub header: Header,
    pub blocks: Vec<Block>,
}

pub struct Frame<'a> {
    pub hover: bool,
    pub scroll: &'a ScrollHandle,
    /// One for each column of a [`Block::Columns`].
    pub columns: &'a [ScrollHandle; COLUMNS],
    /// For a [`Block::Placed`] that scrolls; a panel shows one at a time.
    pub placed: &'a ScrollHandle,
    /// True on the first drawing after the panel opened; the columns clear it.
    pub opened: &'a std::cell::Cell<bool>,
    pub island_h: f32,
    pub min_width: f32,
    pub max_width: f32,
    pub max_height: f32,
    pub phase: f32,
    pub layouts: &'a TextLayouts,
    pub tips: &'a Tooltips,
    pub popovers: &'a Popovers,
}

const GRID_GAP: f32 = 10.0;
/// How far past what a scrolling [`Block::Placed`] shows its blocks are still drawn, so ones
/// scrolling in are ready.
const PLACED_MARGIN: f32 = 240.0;
/// The most a column of [`Block::Columns`] shows before it scrolls, leaving the island its header
/// and edges within its tallest.
const COLUMN_H: f32 = 600.0;
/// The most columns side by side, each with its own scroll.
pub const COLUMNS: usize = 4;
/// How near its end a followed panel counts as being read there.
const FOLLOW_SLACK: f32 = 24.0;
const CAPTION_SIZE: f32 = 12.0;
/// A card's title with controls at its end, so each card's content starts at the same height.
const TITLE_H: f32 = 28.0;
/// A card's heading: quieter than a panel's, so what the card holds leads.
const CARD_TITLE_SIZE: f32 = 15.0;
/// The least a chart taking its card's spare height keeps.
const CHART_MIN_H: f32 = 40.0;
const LEVEL_H: f32 = 3.0;
const BUTTON_GAP: f32 = 8.0;
const COMPACT_GAP: f32 = 6.0;

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
    fn follows_end(&self) -> bool {
        match self {
            Block::Transcript(_) => true,
            Block::Card(_, _, blocks)
            | Block::Row(blocks)
            | Block::Columns(blocks)
            | Block::Grid(_, blocks)
            | Block::Frame(_, blocks)
            | Block::Band(_, blocks)
            | Block::List(_, blocks)
            | Block::Disclose(_, blocks)
            | Block::Item(_, blocks) => blocks.iter().any(Block::follows_end),
            _ => false,
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
            Block::Meter(meter) => meter.level.is_some().hash(hasher),
            Block::Tiles(tiles) => tiles.len().hash(hasher),
            Block::Item(item, blocks) => {
                (item.lead.is_some(), item.trail.len()).hash(hasher);
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            Block::Frame(_, blocks) | Block::Row(blocks) | Block::Columns(blocks) => {
                for block in blocks {
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
            Block::Timeline(view) => view.lanes.len().hash(hasher),
            // It floats over the panel, so it never decides its size.
            Block::Prompt(_) => {}
            Block::Views(views) => (views.tabs.len(), views.buttons.len()).hash(hasher),
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
            // Its height is fixed, so nothing inside it decides the panel's size.
            Block::Band(height, blocks) => {
                height.to_bits().hash(hasher);
                blocks.len().hash(hasher);
            }
            Block::List(_, blocks) | Block::Disclose(_, blocks) => {
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            Block::Chart(view) => view.height.map(f32::to_bits).hash(hasher),
            Block::Legend(_) | Block::Bars(_) | Block::Path(_) | Block::Level(..) => {}
            Block::Segmented(view) => view.labels.len().hash(hasher),
            Block::FactsIn(per_row, items) => (per_row, items.len()).hash(hasher),
            Block::Figures(figures, beside) => {
                figures.len().hash(hasher);
                if let Some(beside) = beside {
                    beside.hash_shape(hasher);
                }
            }
        }
    }
}

impl PanelView {
    pub fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        discriminant(&self.header).hash(&mut hasher);
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

    pub fn render(self, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
        let inset = match self.header {
            Header::None => TIGHT_INSET,
            _ => INSET,
        };
        // Each view rises in as its tab is picked.
        let page = match &self.header {
            Header::Views(views) => Some(views.selected),
            _ => None,
        };
        // A conversation keeps its newest text in view while you read at its end; scrolled back
        // to read something earlier, it stays where you are.
        if self.blocks.iter().any(Block::follows_end) {
            let (offset, end) = (frame.scroll.offset().y, frame.scroll.max_offset().y);
            if f32::from(end + offset).abs() < FOLLOW_SLACK {
                frame.scroll.scroll_to_bottom();
            }
        }
        let header = match self.header {
            Header::Tabs => Some(
                div()
                    .h(px(HEADER_H))
                    .min_w(px(crate::bar::tabs_width()))
                    .into_any_element(),
            ),
            Header::Title(title, buttons) => Some(title_header(title, buttons, frame.hover, cx)),
            Header::Views(views) => Some(views_header(views, frame.hover, frame.tips, cx)),
            Header::None => None,
        };
        let island_below = frame.island_h - inset;
        // An input, with whatever follows it, stays at the panel's foot below what scrolls, as a
        // chat's message field does.
        let mut blocks = self.blocks;
        let prompt = blocks
            .iter()
            .rposition(|block| matches!(block, Block::Prompt(_)))
            .map(|at| blocks.remove(at).render("prompt", island_below, frame, cx));
        let foot = blocks
            .iter()
            .rposition(|block| matches!(block, Block::Input(_)))
            .map_or_else(Vec::new, |at| blocks.split_off(at));
        let foot = render_all(foot, "foot", island_below, frame, cx);
        let blocks = render_all(blocks, "block", island_below, frame, cx);
        ui::base()
            .relative()
            .flex()
            .flex_col()
            .min_w(px(frame.min_width))
            .max_w(px(frame.max_width))
            .max_h(px(frame.max_height))
            .p(px(inset))
            .when_some(header, |panel, header| {
                panel
                    .child(header)
                    .child(div().h(px(HEADER_GAP)).flex_none())
            })
            .child(
                div()
                    .id("panel-body")
                    .flex_initial()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .track_scroll(frame.scroll)
                    .child({
                        let column = div().flex().flex_col().gap(px(BLOCK_GAP)).children(blocks);
                        match page {
                            Some(page) => reveal::appear(
                                SharedString::from(format!("panel-page-{page}")),
                                column,
                            ),
                            None => column.into_any_element(),
                        }
                    }),
            )
            .when(!foot.is_empty(), |panel| {
                panel.child(div().h(px(BLOCK_GAP)).flex_none()).child(
                    div()
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(BLOCK_GAP))
                        .children(foot),
                )
            })
            .children(prompt)
            .into_any_element()
    }
}

fn title_header(
    title: SharedString,
    buttons: Vec<Button>,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    let title = div()
        .min_w(px(0.0))
        .truncate()
        .text_size(px(HEADING_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .child(motion::text("panel-title", title));
    header_row(title.into_any_element(), false, None, buttons, hover, cx)
}

fn views_header(
    views: Views,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let tabs = tabs::tabs(
        "panel-views",
        views.tabs,
        views.selected,
        views.on_select,
        hover,
        tips,
        cx,
    );
    let lead = views.lead.map(|lead| header_lead(lead, cx));
    header_row(
        tabs.into_any_element(),
        true,
        lead,
        views.buttons,
        hover,
        cx,
    )
}

/// The views at the start and their buttons at the end, in a page rather than its header, so
/// without the close button.
fn views_row(views: Views, hover: bool, tips: &Tooltips, cx: &mut Context<Island>) -> AnyElement {
    let tabs = tabs::tabs(
        "page-views",
        views.tabs,
        views.selected,
        views.on_select,
        hover,
        tips,
        cx,
    );
    div()
        .min_h(px(HEADER_H))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .child(tabs)
        .child(
            div().flex().items_center().gap(px(6.0)).children(
                views
                    .buttons
                    .into_iter()
                    .map(|button| button_element(button, hover, cx)),
            ),
        )
        .into_any_element()
}

fn header_lead(lead: HeaderLead, cx: &mut Context<Island>) -> AnyElement {
    let mark = div()
        .id("panel-lead-mark")
        .size(px(lead.size))
        .flex_none()
        .child(badge::mark(&lead.mark, lead.size, 1.0))
        .when_some(lead.on_click, |mark, handler| {
            // The mark answers the pointer itself, so it goes without the grey hover behind.
            mark.cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
        });
    div()
        .min_w(px(0.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(mark)
        .child(
            div()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_size(px(SMALL_SIZE))
                        .text_color(rgb(OVERLAY2))
                        .child(lead.overline),
                )
                .child(
                    div()
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(motion::text("panel-lead-title", lead.title)),
                ),
        )
        .into_any_element()
}

/// `centred` puts `leading` in the middle, with equal room either side, as the bar's tabs sit,
/// and `lead` at the start of that room.
fn header_row(
    leading: AnyElement,
    centred: bool,
    lead: Option<AnyElement>,
    buttons: Vec<Button>,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    div()
        .min_h(px(HEADER_H))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .when(centred, |row| {
            row.child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .children(lead),
            )
        })
        .child(leading)
        .child(
            div()
                .flex()
                // Never narrower than its buttons, so a tight panel moves the tabs aside rather
                // than drawing the buttons over them.
                .when(centred, |end| end.flex_1().flex_shrink_0().justify_end())
                .when(!centred, |end| end.flex_none())
                .items_center()
                .gap(px(6.0))
                .children(
                    buttons
                        .into_iter()
                        .map(|button| button_element(button, hover, cx)),
                )
                .child(
                    button::close("panel-close", hover)
                        .on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Collapsed, cx))),
                ),
        )
        .into_any_element()
}

fn button_element(button: Button, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let handler = button.on_click;
    let hover = hover && button.enabled;
    let element = if button.toggle {
        button::toggle(button.id, button.label, button.active, hover)
    } else {
        button::busy(
            button.id,
            button.label,
            button.fits,
            button.active,
            button.busy,
            hover,
        )
    };
    if button.enabled {
        element
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .into_any_element()
    } else {
        element
            .text_color(rgb(OVERLAY2))
            .cursor_default()
            .into_any_element()
    }
}

fn card_title(title: card::Title, at: &str, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
    let heading = div()
        .flex_none()
        .text_size(px(CARD_TITLE_SIZE))
        .font_weight(FontWeight::MEDIUM)
        .child(motion::text(
            SharedString::from(format!("{at}-title")),
            title.text,
        ));
    let detail = (!title.detail.is_empty()).then(|| {
        div()
            .min_w(px(0.0))
            .truncate()
            .text_size(px(CAPTION_SIZE))
            .text_color(rgb(OVERLAY2))
            .child(motion::text(
                SharedString::from(format!("{at}-detail")),
                title.detail,
            ))
    });
    if !title.beside {
        return div()
            .flex()
            .items_baseline()
            .justify_between()
            .gap(px(12.0))
            .child(heading)
            .children(detail)
            .into_any_element();
    }
    // With controls at its end, the detail keeps beside the heading.
    div()
        .h(px(TITLE_H))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(heading)
        .children(detail)
        .child(
            div()
                .flex_1()
                .flex()
                .justify_end()
                .items_center()
                .gap(px(8.0))
                .children(
                    title
                        .end
                        .into_iter()
                        .map(|end| title_end(end, frame, cx)),
                ),
        )
        .into_any_element()
}

fn title_end(end: card::TitleEnd, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
    match end {
        card::TitleEnd::Switch {
            id,
            on,
            color,
            enabled,
            on_click,
        } => button::switch_control(id, on, color, enabled, on_click, cx),
        card::TitleEnd::Pill {
            id,
            text,
            color,
            busy,
        } => pill::pill(id, text, color, busy),
        card::TitleEnd::Tabs {
            id,
            labels,
            selected,
            on_select,
        } => tabs::segmented(&id, labels, selected, on_select, false, frame.hover, cx),
    }
}

impl Block {
    /// `at` names the block's place in the panel, unique and stable across frames, for parts that
    /// keep state, such as morphing text.
    fn render(
        self,
        at: &str,
        island_below: f32,
        frame: &Frame<'_>,
        cx: &mut Context<Island>,
    ) -> AnyElement {
        match self {
            Block::Card(title, size, blocks) => {
                let title = title.map(|title| card_title(title, at, frame, cx));
                let children = render_all(blocks, at, island_below, frame, cx);
                card::surface()
                    .w(px(size.width()))
                    .flex_none()
                    .overflow_hidden()
                    .p(px(CARD_PAD))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .children(title)
                    .children(children)
                    .into_any_element()
            }
            Block::Row(blocks) => {
                let children = render_all(blocks, at, island_below, frame, cx);
                div()
                    .flex()
                    .gap(px(card::GAP))
                    .children(children)
                    .into_any_element()
            }
            Block::Columns(blocks) => {
                let opened = frame.opened.replace(false);
                div()
                    .flex()
                    .items_start()
                    .gap(px(card::GAP))
                    .children(
                        blocks
                            .into_iter()
                            .take(COLUMNS)
                            .enumerate()
                            .map(|(index, block)| {
                                let at = format!("{at}-column-{index}");
                                let scroll = &frame.columns[index];
                                // A column holding a conversation starts at its newest part, and
                                // keeps to it while read there; scrolled back, it stays put.
                                if block.follows_end() {
                                    let (offset, end) = (scroll.offset().y, scroll.max_offset().y);
                                    if opened || f32::from(end + offset).abs() < FOLLOW_SLACK {
                                        scroll.scroll_to_bottom();
                                    }
                                }
                                div()
                                    .id(SharedString::from(at.clone()))
                                    .flex_none()
                                    .max_h(px(COLUMN_H))
                                    .overflow_y_scroll()
                                    .track_scroll(scroll)
                                    .child(block.render(&at, island_below, frame, cx))
                            }),
                    )
                    .into_any_element()
            }
            Block::Grid(columns, blocks) => {
                let columns = columns.max(1);
                let mut cells = render_all(blocks, at, island_below, frame, cx)
                    .into_iter()
                    .peekable();
                let mut rows = Vec::new();
                while cells.peek().is_some() {
                    let row =
                        (0..columns).map(|_| div().flex_1().min_w(px(0.0)).children(cells.next()));
                    rows.push(div().flex().items_start().gap(px(GRID_GAP)).children(row));
                }
                div()
                    .flex()
                    .flex_col()
                    .gap(px(GRID_GAP))
                    .children(rows)
                    .into_any_element()
            }
            Block::Item(item, blocks) => {
                let children = render_all(blocks, at, island_below, frame, cx);
                item::item(item, children, frame.popovers, frame.tips, frame.hover, cx)
            }
            Block::Frame(view, blocks) => {
                let children = render_all(blocks, at, island_below, frame, cx);
                card::frame(view, children, frame.hover, cx)
            }
            Block::Placed(placed) => placed_element(placed, at, island_below, frame, cx),
            Block::Slider(slider) => slider::view(slider, frame.hover, cx),
            Block::Segments(level, hold) => segments::segments(level, hold).into_any_element(),
            Block::Text(content) => text::text(content, SharedString::from(format!("{at}-text"))),
            Block::Search(view) => search::search(view, frame.hover, cx),
            Block::Calendar(view) => calendar::calendar(view, frame.hover, cx),
            Block::Meter(view) => meter::meter(view, at, cx),
            Block::Player(view) => player::player(view, frame.layouts, frame.hover, cx),
            Block::Lines(entries) => lines::lines(entries),
            Block::Routes(entries) => route::routes(entries),
            Block::Board(columns) => tile::board(columns, frame.phase, frame.hover, frame.tips, cx),
            Block::Buttons(row) => buttons(row, frame, cx),
            Block::Code(lines) => code::code(at, lines, frame.hover, cx),
            Block::Input(view) => input::input(view, frame.hover, cx),
            Block::Transcript(turns) => transcript::transcript(turns, frame.hover, cx),
            Block::Facts(items) => tile::colored_facts(items, at).into_any_element(),
            Block::Hero(name, lines) => tile::hero(name, lines).into_any_element(),
            Block::Timeline(view) => timeline::timeline(view, at),
            Block::Prompt(view) => prompt::prompt(view, frame.hover, cx),
            Block::Views(views) => views_row(views, frame.hover, frame.tips, cx),
            Block::Band(height, blocks) => {
                let children = render_all(blocks, at, island_below, frame, cx);
                div()
                    .flex()
                    .h(px(height))
                    .gap(px(card::GAP))
                    .children(children)
                    .into_any_element()
            }
            Block::List(indent, blocks) => {
                lines::divided(render_all(blocks, at, island_below, frame, cx), indent)
            }
            Block::Disclose(view, blocks) => {
                let children = render_all(blocks, at, island_below, frame, cx);
                disclose::disclose(view, children).into_any_element()
            }
            Block::Chart(view) => {
                let own = view.phase.is_some();
                let plot = chart::chart(
                    view.id.clone(),
                    view.series,
                    card::SLOTS,
                    view.phase.unwrap_or(frame.phase),
                    frame.tips,
                )
                .into_any_element();
                // One keeping its own pace asks for the next frame, so the panel is drawn again
                // with its phase moved on.
                let plot = if own {
                    motion::keyed(
                        SharedString::from(format!("{}-pace", view.id)),
                        move |_: &mut (), _, _| (plot, motion::Next::After(CHART_FRAME)),
                    )
                    .into_any_element()
                } else {
                    plot
                };
                div()
                    .relative()
                    .map(|area| match view.height {
                        Some(height) => area.flex_none().h(px(height)),
                        None => area.flex_1().min_h(px(CHART_MIN_H)),
                    })
                    .child(div().absolute().size_full().flex().child(plot))
                    .into_any_element()
            }
            Block::Legend(items) => tile::legend(items).flex_none().into_any_element(),
            Block::FactsIn(per_row, items) => tile::facts_in(items, at, per_row, true)
                .flex_none()
                .into_any_element(),
            Block::Segmented(view) => tabs::segmented(
                &view.id,
                view.labels,
                view.selected,
                view.on_select,
                true,
                frame.hover,
                cx,
            ),
            Block::Level(id, fraction, color) => div()
                .flex()
                .w_full()
                .child(tile::bar(&id, fraction, color, LEVEL_H, None))
                .into_any_element(),
            Block::Figures(figures, beside) => {
                let beside = beside.map(|block| block.render(&format!("{at}-beside"), island_below, frame, cx));
                figure::figures(figures, beside)
            }
            Block::Bars(view) => bars::bars(view),
            Block::Path(view) => route::path(view),
            Block::Actions(tiles) => div()
                .flex()
                .gap(px(card::GAP))
                .children(tiles.into_iter().map(|tile| {
                    let handler = tile.on_click;
                    button::action(
                        tile.id,
                        tile.glyph.as_deref(),
                        tile.label,
                        tile.color,
                        tile.armed,
                        frame.hover,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                }))
                .into_any_element(),
            Block::Tiles(tiles) => div()
                .flex()
                .gap(px(8.0))
                .children(tiles.into_iter().map(|tile| {
                    let handler = tile.on_click;
                    button::pick(
                        tile.id,
                        tile.glyph.as_deref(),
                        tile.label,
                        tile.color,
                        tile.armed,
                        frame.hover,
                        frame.layouts,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                }))
                .into_any_element(),
        }
    }
}

/// Clicks stay with the button, so one inside something clickable, such as a message, does only
/// its own thing.
fn buttons(row: ButtonRow, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
    let gap = if row.compact {
        COMPACT_GAP
    } else {
        BUTTON_GAP - button::BADGE_OVERHANG
    };
    let menu = row
        .menu
        .map(|menu| actions::actions(menu, frame.popovers, frame.tips, frame.hover, cx));
    div()
        .flex()
        .items_center()
        .ml(px(row.indent))
        .gap(px(gap))
        .children(row.buttons.into_iter().map(|button| {
            let handler = button.on_click;
            let click = cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                handler(this, cx);
            });
            let element = match button.glyph {
                Some(glyph) => button::icon(
                    button.id.clone(),
                    Some(&glyph),
                    button.rising.as_ref(),
                    button.active,
                    frame.hover,
                )
                .when(row.compact, |icon| icon.size(px(CONTROL_H)))
                .tip(frame.tips, button.id.clone(), button.label),
                None => button::button(button.id.clone(), button.label, button.active, frame.hover),
            }
            .on_click(click);
            if row.compact {
                element.into_any_element()
            } else {
                let badge = button.badge.map(|text| button::Badge {
                    text,
                    color: crate::theme::ACCENT,
                });
                button::badged(format!("{}-badged", button.id), element, badge, true)
                    .into_any_element()
            }
        }))
        .when_some(menu, |row, menu| row.child(div().flex_1()).child(menu))
        .into_any_element()
}

fn placed_element(
    placed: Placed,
    at: &str,
    island_below: f32,
    frame: &Frame<'_>,
    cx: &mut Context<Island>,
) -> AnyElement {
    // Only what is in view, and a margin round it, is drawn: a long list of placed blocks would
    // otherwise be laid out on every frame, however little of it shows.
    let shown = placed.visible.map(|visible| {
        let from = -f32::from(frame.placed.offset().y) - PLACED_MARGIN;
        (from, from + visible + 2.0 * PLACED_MARGIN)
    });
    let blocks = placed
        .blocks
        .into_iter()
        .enumerate()
        .filter_map(|(index, (place, block))| {
            let top = if place.from_bottom {
                island_below - place.y - place.height
            } else {
                place.y
            };
            if let Some((from, to)) = shown
                && (top + place.height < from || top > to)
            {
                return None;
            }
            Some(
                div()
                    .absolute()
                    .left(px(place.x))
                    .top(px(top))
                    .w_full()
                    .h(px(place.height))
                    .opacity(place.opacity.clamp(0.0, 1.0))
                    .flex()
                    .items_center()
                    .child(block.render(&format!("{at}-{index}"), island_below, frame, cx)),
            )
        });
    let content = div()
        .relative()
        .min_w(px(placed.width))
        .h(px(placed.height))
        .children(blocks);
    match placed.visible {
        Some(visible) => div()
            .id(SharedString::from(format!("{at}-scroll")))
            .min_w(px(placed.width))
            .h(px(visible))
            .overflow_y_scroll()
            .track_scroll(frame.placed)
            .child(content)
            .into_any_element(),
        None => content.into_any_element(),
    }
}

fn render_all(
    blocks: Vec<Block>,
    at: &str,
    island_below: f32,
    frame: &Frame<'_>,
    cx: &mut Context<Island>,
) -> Vec<AnyElement> {
    blocks
        .into_iter()
        .enumerate()
        .map(|(index, block)| block.render(&format!("{at}-{index}"), island_below, frame, cx))
        .collect()
}
