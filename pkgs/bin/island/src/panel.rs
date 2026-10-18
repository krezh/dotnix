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
        button, calendar, card, item, lines,
        list::ROWS,
        marquee::TextLayouts,
        meter, motion, player,
        popover::Popovers,
        route, search, segments, slider, tabs, text,
        tile::{self, Tile},
        tooltip::{Tip, Tooltips},
    },
    island::{Island, Mode},
    theme::OVERLAY2,
    ui::{
        self, BLOCK_GAP, CARD_PAD, CONTROL_H, HEADER_GAP, HEADER_H, HEADING_SIZE, INSET,
        TIGHT_INSET, TITLE_SIZE,
    },
};

pub use crate::components::{
    badge::Mark,
    button::Rising,
    calendar::CalendarView,
    card::{Countdown, FrameView},
    dropdown::{Choice, Dropdown},
    item::{Item, Lead, Trail},
    lines::Line,
    meter::Meter,
    player::PlayerView,
    route::{Route, Stop, StopState},
    search::{Preview, SearchRow, SearchView},
    slider::Slider,
    text::{Style as TextStyle, Text, Tone},
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

pub struct Views {
    pub tabs: Vec<SharedString>,
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
}

pub struct PanelView {
    pub header: Header,
    pub blocks: Vec<Block>,
}

pub struct Frame<'a> {
    pub hover: bool,
    pub scroll: &'a ScrollHandle,
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
const CAPTION_SIZE: f32 = 12.0;
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
            Block::Frame(_, blocks) | Block::Row(blocks) => {
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
            Block::Player(view) => (view.art.is_some(), view.length.is_some()).hash(hasher),
            Block::Card(title, size, blocks) => {
                (title.is_some(), size).hash(hasher);
                for block in blocks {
                    block.hash_shape(hasher);
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
        let header = match self.header {
            Header::Tabs => Some(
                div()
                    .h(px(HEADER_H))
                    .min_w(px(crate::bar::tabs_width()))
                    .into_any_element(),
            ),
            Header::Title(title, buttons) => Some(title_header(title, buttons, frame.hover, cx)),
            Header::Views(views) => Some(views_header(views, frame.hover, cx)),
            Header::None => None,
        };
        let island_below = frame.island_h - inset;
        let blocks = render_all(self.blocks, "block", island_below, frame, cx);
        ui::base()
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
                    .child(div().flex().flex_col().gap(px(BLOCK_GAP)).children(blocks)),
            )
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
    header_row(title.into_any_element(), buttons, hover, cx)
}

fn views_header(views: Views, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let tabs = tabs::tabs(
        "panel-views",
        views.tabs,
        views.selected,
        views.on_select,
        hover,
        cx,
    );
    header_row(tabs.into_any_element(), views.buttons, hover, cx)
}

fn header_row(
    leading: AnyElement,
    buttons: Vec<Button>,
    hover: bool,
    cx: &mut Context<Island>,
) -> AnyElement {
    div()
        .h(px(HEADER_H))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .child(leading)
        .child(
            div()
                .flex()
                .flex_none()
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

fn card_title(title: card::Title, at: &str) -> impl IntoElement {
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .gap(px(12.0))
        .child(
            div()
                .text_size(px(TITLE_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .child(motion::text(
                    SharedString::from(format!("{at}-title")),
                    title.text,
                )),
        )
        .when(!title.detail.is_empty(), |line| {
            line.child(
                div()
                    .min_w(px(0.0))
                    .truncate()
                    .text_size(px(CAPTION_SIZE))
                    .text_color(rgb(OVERLAY2))
                    .child(motion::text(
                        SharedString::from(format!("{at}-detail")),
                        title.detail,
                    )),
            )
        })
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
                let children = render_all(blocks, at, island_below, frame, cx);
                card::surface()
                    .w(px(size.width()))
                    .flex_none()
                    .overflow_hidden()
                    .p(px(CARD_PAD))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .when_some(title, |group, title| group.child(card_title(title, at)))
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
                item::item(item, children, frame.popovers, frame.hover, cx)
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
    div()
        .flex()
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
                button::badged(format!("{}-badged", button.id), element, button.badge)
                    .into_any_element()
            }
        }))
        .into_any_element()
}

fn placed_element(
    placed: Placed,
    at: &str,
    island_below: f32,
    frame: &Frame<'_>,
    cx: &mut Context<Island>,
) -> AnyElement {
    let blocks = placed
        .blocks
        .into_iter()
        .enumerate()
        .map(|(index, (place, block))| {
            let top = if place.from_bottom {
                island_below - place.y - place.height
            } else {
                place.y
            };
            div()
                .absolute()
                .left(px(place.x))
                .top(px(top))
                .w_full()
                .h(px(place.height))
                .opacity(place.opacity.clamp(0.0, 1.0))
                .flex()
                .items_center()
                .child(block.render(&format!("{at}-{index}"), island_below, frame, cx))
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
