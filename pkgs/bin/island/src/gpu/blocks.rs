//! Every panel block as a tree of [`Node`]s, following what `panel/` and `components/` draw for GPUI.

use std::{collections::HashMap, rc::Rc};

use chrono::{Datelike, Duration, Local, Months, NaiveDate};

use super::{
    node::{self, BOLD, MEDIUM, Node, Painting, SEMIBOLD, div, glyph, moving, text},
    panel::{Countdown, Easing},
    records,
    renderer::Sprite,
    widget::{color, color_alpha, tinted},
};
use crate::{
    bar::tabs_width,
    components::{
        actions::Actions,
        badge::Mark,
        button::{ACTION_H, ACTION_W, FILL_SPRING, ICON_BUTTON, KNOB_SPRING, POP_DIP, STRIP_GLYPH},
        calendar::CalendarView,
        card::{self, FrameView, Look, RAISED_GAP, RAISED_PAD, Size, Title},
        dropdown::Dropdown,
        input::{CARET, InputView},
        item::{self, HEADLINE_H, Item, LEAD_GAP, Lead, MESSAGE_GAP, OVERLINE_H, Tint, Trail},
        lines::Line,
        list::ROWS,
        meter::{Meter, Shape},
        player::PlayerView,
        prompt::PromptView,
        route::{Route, Stop, StopState},
        search::{Preview, RowIcon, SELECTION_SPRING, SearchView},
        segments,
        slider::{self as slider_fill, HIT_H, Slider, SliderLabel, TRACK_H},
        tabs::{PILL_SPRING, TabLabel},
        text::{BODY_LINE, BODY_SIZE, Style as TextStyle, Text, Tone},
        tile::{self, Drive, Entry, EntryLead, Part, Tile, Volume},
    },
    panel::PLACED_MARGIN,
    panel::{
        ActionTile, Block, Button, ButtonRow, Handler, Header, HeaderLead, IconButton, PanelView,
        Placed, SharedString, Split, Views, Warning,
    },
    stage::CONTENT_IN,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, CARD_ALPHA, CARD_BORDER_ALPHA, CONTROL_ALPHA, CRITICAL, GREEN,
        HOVER_ALPHA, NOTIFICATION_ALPHA, ON_ACCENT, OVERLAY1, OVERLAY2, PEACH, POPOVER_OPACITY,
        RED, SELECTED_ALPHA, SUBTEXT1, SURFACE, SURFACE_BORDER_ALPHA, SURFACE1, TEXT, TINT,
        TRACK_ALPHA, YELLOW, with_alpha,
    },
    ui::{
        BLOCK_GAP, CARD_PAD, CONTROL_H, CONTROL_RADIUS, HEADER_GAP, HEADER_H, HEADING_SIZE, INSET,
        PANEL_RADIUS, SMALL_SIZE, STRIP_PAD, TEXT_SIZE, TIGHT_INSET, TITLE_SIZE,
    },
};

pub(super) const ROW_RADIUS: f32 = 8.0;
pub(super) const BADGE_RADIUS: f32 = 6.0;
const PILL: f32 = 1.0e4;
const CAPTION: f32 = 12.0;
const FACT_LEADING: f32 = 1.3;
const HERO_LEADING: f32 = 1.4;
const GRID_GAP: f32 = 10.0;
const SWITCH_GAP: f32 = 8.0;
const SWITCHES_PER_ROW: usize = 4;
const COMPACT_GAP: f32 = 6.0;
const BUTTON_GAP: f32 = 8.0;
const COLUMN_H: f32 = 600.0;
const LEAD: f32 = 40.0;
const AMOUNT_W: f32 = 40.0;
const STRIP_ICON_W: f32 = 32.0;
const STRIP_ICON_H: f32 = 28.0;
const ICON_GLYPH: f32 = 18.0;
const LARGE_H: f32 = 42.0;
const LARGE_GLYPH: f32 = 20.0;
const LARGE_PAD: f32 = 12.0;
const DIM_ALPHA: u8 = 0x8c;
const URGENT_ALPHA: u8 = 0xb3;
const PILL_ALPHA: u8 = 0xe6;
const STRIP_TAB_H: f32 = 28.0;
const STRIP_TAB_GAP: f32 = 2.0;
const STRIP_TAB_PAD: f32 = 14.0;
const COUNT_GAP: f32 = 7.0;
const COUNT_SIZE: f32 = 12.0;
const STRIP_ALPHA: u8 = 0x0d;
const STRIP_BORDER_ALPHA: u8 = 0x10;
const STRIP_DIVIDER_ALPHA: u8 = 0x1a;
const TAB_W: f32 = 96.0;
const GLYPH_TAB_W: f32 = 48.0;
const TAB_GAP: f32 = 4.0;
const PAD: f32 = 14.0;
const PART_GAP: f32 = 9.0;
const CHART_H: f32 = 40.0;
const TALL_CHART_H: f32 = 56.0;
const LABEL_W: f32 = 70.0;
const VALUE_W: f32 = 64.0;
const CORE_COLUMNS: usize = 16;
const CORE_GAP: f32 = 3.0;
const ENTRY_BLEED: f32 = 8.0;
const ENTRY_GLYPH: f32 = 16.0;
const CHOSEN_ALPHA: u8 = 0x26;
const OPEN_SIZE: f32 = 24.0;
const TITLE_GLYPH: f32 = 16.0;
const NOTE_PAD: f32 = 14.0;
/// A note held to a width keeps room for a second line.
const HELD_NOTE_LINE: f32 = 20.0;
const PASSING_GAP: f32 = 96.0;

pub(super) fn surface_fill(active: bool) -> [f32; 4] {
    color_alpha(if active {
        with_alpha(ACCENT, ACCENT_TINT_ALPHA)
    } else {
        with_alpha(SURFACE1, CONTROL_ALPHA)
    })
}

fn track() -> [f32; 4] {
    tinted(TINT, TRACK_ALPHA / 2)
}

fn hairline() -> [f32; 4] {
    tinted(SURFACE1, 0x59)
}

/// A clickable surface, centring what it holds.
pub(super) fn control(rest: Option<[f32; 4]>, radius: f32) -> Node {
    let node = div().flex().items_center().justify_center().hover(radius);
    match rest {
        Some(fill) => node.bg(fill, radius),
        None => node,
    }
}

/// A popover's own surface, nearly solid over what is under it.
pub(super) fn card_fill(radius: f32) -> Node {
    div()
        .bg(
            color(SURFACE).map(|channel| channel * POPOVER_OPACITY),
            radius,
        )
        .outline(tinted(SURFACE1, SURFACE_BORDER_ALPHA), radius)
}

fn card_surface() -> Node {
    div()
        .bg(tinted(SURFACE1, CARD_ALPHA), PANEL_RADIUS)
        .outline(tinted(SURFACE1, CARD_BORDER_ALPHA), PANEL_RADIUS)
}

pub fn mark(mark: &Mark, size: f32, fill: f32, key: &str) -> Node {
    let picture = (size * fill).round();
    match mark {
        Mark::Glyph(path, tint) => glyph(path.clone(), (size * 0.56).round(), *tint),
        Mark::Image(path) => node::picture(crate::picture::Picture::File(path.clone()), picture),
        Mark::Picture(source, true) => node::picture(source.clone(), picture)
            .cover()
            .rounded(CONTROL_RADIUS),
        Mark::Picture(source, false) => node::picture(source.clone(), picture),
        Mark::Letter(name, tint) => div()
            .size(picture)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .bg(color(*tint), CONTROL_RADIUS)
            .text_size((picture * 0.45).round())
            .weight(SEMIBOLD)
            .color(ON_ACCENT)
            .child(text(
                name.chars()
                    .next()
                    .map(|first| first.to_uppercase().to_string())
                    .unwrap_or_default(),
            )),
        Mark::Choice { picked, many, .. } => choice(*picked, *many, (picture * 0.9).round()),
        Mark::Drawn(draw) => node::drawing(format!("{key}-mark"), draw(size)),
        Mark::Ring {
            fraction,
            color: tint,
            value,
            ..
        } => ring(size, *fraction, *tint)
            .flex()
            .col()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_size(18.0)
                    .weight(SEMIBOLD)
                    .leading(1.15)
                    .child(moving(value.clone())),
            ),
        Mark::Marked(inner, path, tint) => {
            let disc = (size * 0.5).round();
            let hang = -(disc * 0.28).round();
            div()
                .size(size)
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(self::mark(inner, size, fill, key))
                .child(
                    div()
                        .absolute()
                        .right(hang)
                        .bottom(hang)
                        .size(disc + 4.0)
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(color(crate::theme::SURFACE), PILL)
                        .child(glyph(path.clone(), (disc * 0.66).round(), *tint)),
                )
        }
        Mark::Pulsing(inner) => self::mark(inner, size, fill, key).pulsing(),
    }
}

fn choice(picked: bool, many: bool, side: f32) -> Node {
    let radius = if many { BADGE_RADIUS } else { PILL };
    let lit = f32::from(u8::from(picked));
    div()
        .size(side)
        .flex_none()
        .child(
            div()
                .absolute()
                .inset(0.0)
                .outline(tinted(OVERLAY1, 0xb3), radius)
                .eases(Easing::Strength, 1.0 - lit, CONTENT_IN),
        )
        .child(
            div()
                .absolute()
                .inset(0.0)
                .flex()
                .items_center()
                .justify_center()
                .bg(color(ACCENT), radius)
                .child(if many {
                    text("✓").color(ON_ACCENT).text_size(side * 0.7)
                } else {
                    div().size(side * 0.4).bg(color(ON_ACCENT), PILL)
                })
                .eases(Easing::Strength, lit, CONTENT_IN),
        )
}

/// Filled from the top, clockwise, with the middle left for what it holds.
pub fn ring(side: f32, fraction: f32, tint: u32) -> Node {
    const STROKE: f32 = 7.0;
    let fraction = fraction.clamp(0.0, 1.0);
    let whole = [0.0, 0.0, side, side];
    div()
        .size(side)
        .flex_none()
        .child(
            node::sprites(vec![Sprite::arc(whole, STROKE, 0.0, 1.0, track())])
                .absolute()
                .left(0.0)
                .top(0.0),
        )
        .child(
            node::sprites(vec![Sprite::arc(whole, STROKE, 0.0, fraction, color(tint))])
                .absolute()
                .left(0.0)
                .top(0.0)
                .eases(Easing::Arc, fraction, CONTENT_IN),
        )
}

/// A list opened from an anchored chip, with what closes it.
pub struct Popover {
    pub anchor: SharedString,
    pub list: Node,
    pub on_toggle: Handler,
    /// Under the anchor with their right edges together; otherwise over it, centred.
    pub drops: bool,
    pub open: bool,
    /// What the chip it grows from says, and whether that is drawn as a strip.
    pub label: Option<(SharedString, bool)>,
}

/// A chip's words, as they lie on the surface growing from it.
pub fn chip_label(words: SharedString, strip: bool, size: (f32, f32)) -> Node {
    div()
        .w(size.0)
        .h(size.1)
        .flex()
        .items_center()
        .justify_center()
        .text_size(if strip { SMALL_SIZE } else { 12.0 })
        .weight(SEMIBOLD)
        .color(SUBTEXT1)
        .child(text(words).nowrap())
}

/// How a chip looks at rest: its fill, border and corners.
pub fn chip_look(strip: bool) -> ([f32; 4], [f32; 4], f32) {
    if strip {
        (
            tinted(TINT, STRIP_ALPHA),
            tinted(TINT, STRIP_BORDER_ALPHA),
            ROW_RADIUS + STRIP_PAD,
        )
    } else {
        (surface_fill(false), [0.0; 4], BADGE_RADIUS)
    }
}

pub struct Blocks<'a, 'b> {
    pub painting: &'a mut Painting<'b>,
    pub popovers: Vec<Popover>,
    /// A card asking for something, over all of the panel.
    pub prompt: Option<PromptView>,
    /// How wide the island is now.
    pub island_w: f32,
    /// A view is sliding out for another, so the panel changes every frame.
    pub sliding: bool,
    /// What was chosen on each dropdown when it last stood open, so it leaves as it stood.
    pub chosen: HashMap<SharedString, Option<usize>>,
}

impl Blocks<'_, '_> {
    /// `max` is the most room it may take, across and down.
    pub fn panel(&mut self, mut view: PanelView, min_w: f32, max: (f32, f32)) -> Node {
        let inset = match view.header {
            Header::None => TIGHT_INSET,
            _ => INSET,
        };
        let passing = match &mut view.header {
            Header::Views(views) => views.passing.take(),
            _ => None,
        };
        let header = match std::mem::replace(&mut view.header, Header::None) {
            Header::Tabs => Some(div().h(HEADER_H).min_w(tabs_width().ceil())),
            Header::Title(title, buttons) => Some(self.title_header(title, buttons)),
            Header::Views(views) => Some(self.views_header(views)),
            Header::None => None,
        };
        let mut blocks = view.blocks;
        if let Some(at) = blocks
            .iter()
            .rposition(|block| matches!(block, Block::Prompt(_)))
            && let Block::Prompt(prompt) = blocks.remove(at)
        {
            self.prompt = Some(prompt);
        }
        let foot_at = blocks
            .iter()
            .rposition(|block| matches!(block, Block::Input(_)));
        let foot = foot_at.map_or_else(Vec::new, |at| blocks.split_off(at));
        let foot = self.all(foot, "foot");
        let views = matches!(blocks.first(), Some(Block::Views(_)))
            .then(|| self.block(blocks.remove(0), "page-views"));
        let column = div()
            .col()
            .gap(BLOCK_GAP)
            .children(self.all(blocks, "block"));
        let content = match views {
            Some(views) => div().col().gap(BLOCK_GAP).child(views).child(column),
            None => column,
        };
        let content = match passing {
            Some(passing) => {
                self.sliding = true;
                let side = if passing.rightwards { 1.0 } else { -1.0 };
                let room = self.island_w - 2.0 * inset;
                let width = passing.width.get().unwrap_or(room);
                passing.width.set(Some(width));
                let travel = width.max(room) + PASSING_GAP;
                let leaving = self.all(passing.blocks, "passing");
                div()
                    .child(
                        div()
                            .left(((1.0 - passing.at) * side * travel).round())
                            .child(content),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(0.0)
                            .left_share(0.5)
                            .ml((-width / 2.0 - passing.at * side * travel).round())
                            .w(width)
                            .col()
                            .gap(BLOCK_GAP)
                            .children(leaving),
                    )
            }
            None => content,
        };
        let body = if view.scroll {
            div()
                .flex_1()
                .min_h(0.0)
                .scrolls("panel-body")
                .soft_edges()
                .child(content)
        } else {
            div().flex_none().child(content)
        };
        div()
            .col()
            .min_w(min_w)
            .max_w(max.0)
            .when(view.scroll, |panel| panel.max_h(max.1))
            .p(inset)
            .when(header.is_some(), |panel| {
                panel
                    .children(header)
                    .child(div().h(HEADER_GAP).flex_none())
            })
            .child(body)
            .when(!foot.is_empty(), |panel| {
                panel
                    .child(div().h(BLOCK_GAP).flex_none())
                    .child(div().flex_none().col().gap(BLOCK_GAP).children(foot))
            })
    }

    fn all(&mut self, blocks: Vec<Block>, at: &str) -> Vec<Node> {
        blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| self.block(block, &format!("{at}-{index}")))
            .collect()
    }

    pub fn block(&mut self, block: Block, at: &str) -> Node {
        match block {
            Block::Card(title, size, blocks) => self.card(title, size, blocks, at),
            Block::Row(blocks) => div().flex().gap(card::GAP).children(self.all(blocks, at)),
            Block::Columns(Split::Balanced, blocks) => self.balanced(blocks, at),
            Block::Columns(Split::Scrolling, blocks) => {
                let columns: Vec<Node> = blocks
                    .into_iter()
                    .enumerate()
                    .map(|(index, block)| {
                        let key = format!("{at}-column-{index}");
                        let inner = self.block(block, &key);
                        div().flex_none().max_h(COLUMN_H).scrolls(key).child(inner)
                    })
                    .collect();
                div().flex().items_start().gap(card::GAP).children(columns)
            }
            Block::Grid(columns, blocks) => {
                let columns = columns.max(1);
                let cells = self.all(blocks, at);
                let mut cells = cells.into_iter().peekable();
                let mut rows = Vec::new();
                while cells.peek().is_some() {
                    let row: Vec<Node> = (0..columns)
                        .map(|_| div().flex_1().min_w(0.0).children(cells.next()))
                        .collect();
                    rows.push(div().flex().items_start().gap(GRID_GAP).children(row));
                }
                div().col().gap(GRID_GAP).children(rows)
            }
            Block::Item(item, blocks) => {
                let under = self.all(blocks, at);
                self.item(item, under, at)
            }
            Block::Frame(view, blocks) => {
                let held = self.all(blocks, at);
                frame(view, held)
            }
            Block::Placed(placed) => self.placed(placed, at),
            Block::Slider(view) => slider(view),
            Block::Segments(level, hold) => segments_row(level, hold),
            Block::Text(content) => text_block(&content),
            Block::Calendar(view) => calendar(view),
            Block::Meter(view) => meter(view, at),
            Block::Meters(columns, meters) => meters_row(columns, meters, at),
            Block::Player(view) => player(view),
            Block::Lines(entries) => lines(entries),
            Block::Routes(entries) => routes(entries),
            Block::Board(columns) => board(columns),
            Block::Buttons(row) => self.buttons(row),
            Block::Facts(items) => facts(items),
            Block::Hero(name, lines) => hero(&name, &lines),
            Block::Views(views) => self.views_row(views),
            Block::Actions(tiles) => div()
                .flex()
                .gap(card::GAP)
                .children(tiles.into_iter().map(action)),
            Block::Switches(tiles) => {
                let mut tiles = tiles.into_iter().peekable();
                let mut rows = Vec::new();
                while tiles.peek().is_some() {
                    let row: Vec<Node> = (0..SWITCHES_PER_ROW)
                        .map(|_| {
                            let cell = div().flex_1().min_w(0.0).col();
                            match tiles.next() {
                                Some(tile) => cell.child(action(tile)),
                                None => cell,
                            }
                        })
                        .collect();
                    rows.push(div().flex().gap(SWITCH_GAP).children(row));
                }
                div().col().gap(SWITCH_GAP).children(rows)
            }
            Block::Warnings(warnings) => div()
                .wrap()
                .gap(8.0)
                .children(warnings.into_iter().map(warning)),
            Block::Chart(view) => div()
                .h(view.height)
                .child(node::chart(view.series, view.phase).size_full()),
            Block::Tiles(tiles) => div().flex().gap(8.0).children(tiles.into_iter().map(pick)),
            Block::Search(view) => search(view),
            Block::Input(view) => input(view),
            Block::Code(view) => records::code(view),
            Block::Transcript(view) => records::transcript(view),
            Block::Timeline(view) => records::timeline(view),
            Block::Ledger(view) => records::ledger(view, at),
            Block::Prompt(_) => div(),
        }
    }

    fn card(&mut self, title: Option<Title>, size: Size, blocks: Vec<Block>, at: &str) -> Node {
        let children = self.all(blocks, at);
        card_surface()
            .w(size.width())
            .flex_none()
            .clip()
            .p(CARD_PAD)
            .col()
            .gap(12.0)
            .children(title.map(card_title))
            .children(children)
    }

    /// The taller column as short as it can be, each keeping the cards' order.
    fn balanced(&mut self, blocks: Vec<Block>, at: &str) -> Node {
        let cards: Vec<Node> = blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| self.block(block, &format!("{at}-{index}")))
            .collect();
        let heights: Vec<f32> = cards
            .iter()
            .map(|card| self.painting.measure(card, None).1 + card::GAP)
            .collect();
        let columns = card::balance_heights(&heights);
        let mut sides: [Vec<Node>; card::BALANCED] = Default::default();
        for (card, column) in cards.into_iter().zip(columns) {
            sides[column].push(card);
        }
        let [left, right] = sides;
        div()
            .flex()
            .items_start()
            .gap(card::GAP)
            .child(div().col().gap(card::GAP).children(left))
            .child(div().col().gap(card::GAP).children(right))
    }

    fn item(&mut self, item: Item, under: Vec<Node>, at: &str) -> Node {
        let top = item.lead.as_ref().is_some_and(|lead| lead.top);
        let lead = item.lead.map(|lead| lead_node(lead, at));
        let trail: Vec<Node> = item
            .trail
            .into_iter()
            .map(|trail| match trail {
                Trail::Dropdown(picker) => self.dropdown(picker),
                other => trail_node(other),
            })
            .collect();
        let tints = |overline: bool| -> Vec<Node> {
            item.tinted
                .iter()
                .filter_map(|tint| match (tint, overline) {
                    (Tint::Overline(words, tint), true) | (Tint::Title(words, tint), false) => {
                        Some(
                            div()
                                .flex_none()
                                .nowrap()
                                .when(!overline, |tint| tint.weight(SEMIBOLD))
                                .color(*tint)
                                .child(moving(words.clone())),
                        )
                    }
                    _ => None,
                })
                .collect()
        };
        let column = div().flex_1().min_w(0.0).col();
        let column = match &item.overline {
            Some(overline) => column
                .gap(MESSAGE_GAP)
                .child(
                    div()
                        .h(OVERLINE_H)
                        .line(OVERLINE_H)
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(8.0)
                        .text_size(SMALL_SIZE)
                        .color(SUBTEXT1)
                        .child(
                            div()
                                .min_w(0.0)
                                .flex()
                                .child(div().min_w(0.0).truncate().child(text(overline.clone())))
                                .children(tints(true)),
                        )
                        .children(trail),
                )
                .child(
                    div()
                        .min_h(HEADLINE_H)
                        .line(HEADLINE_H)
                        .text_size(TEXT_SIZE)
                        .weight(SEMIBOLD)
                        .when_some(item.title_color, Node::color)
                        .when(!item.wrap, |title| title.h(HEADLINE_H).truncate())
                        .child(moving(item.title.clone())),
                ),
            None => {
                let title = div()
                    .flex_1()
                    .min_w(0.0)
                    .flex()
                    .gap(6.0)
                    .child(
                        div()
                            .min_w(0.0)
                            .when(!item.wrap, Node::truncate)
                            .weight(SEMIBOLD)
                            .when_some(item.title_color, Node::color)
                            .child(moving(item.title.clone())),
                    )
                    .children(tints(false))
                    .when(!item.detail.is_empty(), |line| {
                        line.child(
                            div()
                                .min_w(0.0)
                                .truncate()
                                .color(OVERLAY2)
                                .child(moving(item.detail.clone())),
                        )
                    });
                column.gap(4.0).child(
                    div()
                        .flex()
                        .items_center()
                        .gap(8.0)
                        .text_size(SMALL_SIZE)
                        .child(title)
                        .children(trail),
                )
            }
        };
        div()
            .flex()
            .when(top, Node::items_start)
            .when(!top, Node::items_center)
            .gap(LEAD_GAP)
            .children(lead)
            .child(column.children(under))
            .named(&item.id)
    }

    fn placed(&mut self, placed: Placed, at: &str) -> Node {
        let scroll = SharedString::from(format!("{at}-scroll"));
        let shown = placed.visible.map(|visible| {
            let reach = (placed.height - visible).max(0.0);
            let offset = self.painting.offsets.get(&scroll).copied().unwrap_or(0.0);
            let from = offset.min(reach) - PLACED_MARGIN;
            (from, from + visible + 2.0 * PLACED_MARGIN)
        });
        let blocks: Vec<Node> = placed
            .blocks
            .into_iter()
            .enumerate()
            .filter_map(|(index, (place, block))| {
                let top = if place.from_bottom {
                    placed.height - place.y - place.height
                } else {
                    place.y
                };
                if let Some((from, to)) = shown
                    && (top + place.height < from || top > to)
                {
                    return None;
                }
                let inner = self.block(block, &format!("{at}-{index}"));
                let content = match place.drawn {
                    Some(drawn) => div()
                        .absolute()
                        .left(0.0)
                        .right(0.0)
                        .bottom(0.0)
                        .h(drawn)
                        .flex()
                        .child(inner),
                    None => div().w_full().child(inner),
                };
                Some(
                    div()
                        .absolute()
                        .left(place.inset + place.offset)
                        .right(place.inset - place.offset)
                        .top(top)
                        .h(place.height)
                        .opacity(place.opacity.clamp(0.0, 1.0))
                        .when(place.drawn.is_some(), Node::clip)
                        .flex()
                        .items_center()
                        .child(content),
                )
            })
            .collect();
        let content = div()
            .flex_none()
            .min_w(placed.width)
            .h(placed.height)
            .children(blocks);
        match placed.visible {
            Some(visible) => div()
                .h(visible)
                .min_w(placed.width)
                .scrolls(scroll)
                .soft_edges()
                .child(content),
            None => content,
        }
    }

    fn title_header(&mut self, title: SharedString, buttons: Vec<Button>) -> Node {
        let leading = div()
            .min_w(0.0)
            .truncate()
            .text_size(HEADING_SIZE)
            .weight(SEMIBOLD)
            .child(moving(title));
        header_row(leading, false, None, buttons)
    }

    fn views_header(&mut self, views: Views) -> Node {
        let tabs = tabs(views.tabs, views.selected, views.on_select);
        let lead = views.lead.map(header_lead);
        header_row(tabs, true, lead, views.buttons)
    }

    fn views_row(&mut self, views: Views) -> Node {
        let tabs = self.strip(views.tabs, views.selected, views.on_select);
        let pick = views.pick.map(|pick| self.dropdown(pick));
        let groups = views.icons.len();
        let mut strip = Vec::new();
        for (index, group) in views.icons.into_iter().enumerate() {
            for button in group {
                strip.push(strip_icon(button));
            }
            if index + 1 < groups {
                strip.push(
                    div()
                        .flex_none()
                        .w(1.0)
                        .h(18.0)
                        .mx(3.0)
                        .bg(tinted(TINT, STRIP_DIVIDER_ALPHA), 0.0),
                );
            }
        }
        let icons = (!strip.is_empty()).then(|| {
            strip_surface(div().flex_none().p(STRIP_PAD))
                .child(div().flex().items_center().gap(2.0).children(strip))
        });
        div()
            .min_h(HEADER_H)
            .flex()
            .items_center()
            .justify_between()
            .gap(16.0)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(12.0)
                    .child(tabs)
                    .children(pick),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(6.0)
                    .children(views.buttons.into_iter().map(button_node))
                    .children(icons),
            )
    }

    fn dropdown(&mut self, dropdown: Dropdown) -> Node {
        let label = dropdown
            .selected
            .and_then(|index| dropdown.choices.get(index))
            .map(|choice| choice.label.clone())
            .unwrap_or_default();
        let toggle = dropdown.on_toggle.clone();
        let strip = dropdown.height.is_some();
        let chip = match dropdown.height {
            Some(height) => strip_surface(control(None, ROW_RADIUS + STRIP_PAD))
                .flex_none()
                .h(height)
                .px(14.0)
                .text_size(SMALL_SIZE)
                .weight(SEMIBOLD)
                .color(SUBTEXT1)
                .child(text(label.clone()).nowrap()),
            None => control(Some(surface_fill(false)), BADGE_RADIUS)
                .flex_none()
                .h(24.0)
                .px(8.0)
                .gap(4.0)
                .text_size(12.0)
                .weight(SEMIBOLD)
                .color(SUBTEXT1)
                .child(text(label.clone()).nowrap()),
        };
        let chip = chip.anchor(dropdown.id.clone()).on_press(toggle.clone());
        {
            let selected = if dropdown.open {
                self.chosen.insert(dropdown.id.clone(), dropdown.selected);
                dropdown.selected
            } else {
                self.chosen
                    .get(&dropdown.id)
                    .copied()
                    .unwrap_or(dropdown.selected)
            };
            let select = dropdown.on_select.clone();
            let rows: Vec<Node> = dropdown
                .choices
                .into_iter()
                .enumerate()
                .map(|(index, choice)| {
                    let current = selected == Some(index);
                    let select = select.clone();
                    let close = toggle.clone();
                    let (shown, label, detail) = if current {
                        (ACCENT, ACCENT, ACCENT)
                    } else {
                        (SUBTEXT1, TEXT, OVERLAY1)
                    };
                    let row = control(None, CONTROL_RADIUS)
                        .h(30.0)
                        .px(8.0)
                        .justify_start()
                        .gap(8.0)
                        .text_size(SMALL_SIZE)
                        .weight(SEMIBOLD)
                        .color(label)
                        .nowrap()
                        .when(current, |row| {
                            row.bg(tinted(TINT, SELECTED_ALPHA), ROW_RADIUS)
                        })
                        .children(choice.glyph.map(|path| glyph(path, 16.0, shown)))
                        .child(text(choice.label));
                    let row = match choice.detail {
                        Some(value) => row
                            .child(div().flex_1().min_w(8.0))
                            .child(div().color(detail).child(text(value))),
                        None => row,
                    };
                    row.on_press(if current {
                        close
                    } else {
                        Rc::new(move |ui| select(index, ui))
                    })
                })
                .collect();
            self.popovers.push(Popover {
                anchor: dropdown.id,
                open: dropdown.open,
                label: Some((label, strip)),
                list: div().p(4.0).col().gap(2.0).children(rows),
                on_toggle: toggle,
                drops: false,
            });
        }
        chip
    }

    fn actions(&mut self, view: Actions) -> Node {
        let tint = if view.open { ACCENT } else { SUBTEXT1 };
        let button = control(Some(surface_fill(view.open)), CONTROL_RADIUS)
            .flex_none()
            .size(CONTROL_H)
            .color(tint)
            .children(view.glyph.map(|path| glyph(path, ICON_GLYPH, tint)))
            .anchor(view.id.clone())
            .tip(view.label)
            .on_press(view.on_toggle.clone());
        {
            let rows: Vec<Node> = view
                .actions
                .into_iter()
                .map(|action| {
                    let tint = if action.danger { RED } else { TEXT };
                    control(None, CONTROL_RADIUS)
                        .h(30.0)
                        .px(8.0)
                        .justify_between()
                        .gap(12.0)
                        .text_size(SMALL_SIZE)
                        .weight(SEMIBOLD)
                        .color(tint)
                        .nowrap()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(8.0)
                                .children(action.glyph.map(|path| {
                                    glyph(path, 16.0, if action.danger { RED } else { SUBTEXT1 })
                                }))
                                .child(text(action.label)),
                        )
                        .children(action.switch.map(switch))
                        .on_click(action.on_click)
                })
                .collect();
            self.popovers.push(Popover {
                anchor: view.id,
                open: view.open,
                label: None,
                list: div().p(4.0).col().gap(2.0).children(rows),
                on_toggle: view.on_toggle,
                drops: true,
            });
        }
        button
    }

    fn buttons(&mut self, mut row: ButtonRow) -> Node {
        let menu = row.menu.take().map(|menu| self.actions(menu));
        buttons(row).when_some(menu, |row, menu| row.child(div().flex_1()).child(menu))
    }

    fn strip(
        &mut self,
        labels: Vec<TabLabel>,
        selected: usize,
        on_select: crate::components::tabs::Select,
    ) -> Node {
        let mut pill_at = 0.0;
        let mut tabs = Vec::new();
        let mut pill_w = 0.0;
        let mut x = 0.0;
        for (index, tab) in labels.into_iter().enumerate() {
            let label = self
                .painting
                .painter
                .line(&tab.label, SMALL_SIZE, SEMIBOLD, self.painting.scale)
                .width
                .ceil();
            let count = tab.count.as_ref().map(|count| {
                self.painting
                    .painter
                    .line(count, COUNT_SIZE, SEMIBOLD, self.painting.scale)
                    .width
                    .ceil()
            });
            let width = label + count.map_or(0.0, |count| COUNT_GAP + count) + 2.0 * STRIP_TAB_PAD;
            let picked = index == selected;
            if picked {
                pill_at = x;
                pill_w = width;
            }
            x += width + STRIP_TAB_GAP;
            let select = on_select.clone();
            let mut node = control(None, ROW_RADIUS)
                .flex_none()
                .w(width)
                .h(STRIP_TAB_H)
                .gap(COUNT_GAP)
                .text_size(SMALL_SIZE)
                .weight(SEMIBOLD)
                .color(if picked { TEXT } else { SUBTEXT1 })
                .when(picked, Node::unlit)
                .child(text(tab.label.clone()).nowrap());
            if let Some(count) = tab.count {
                node = node.child(
                    div()
                        .text_size(COUNT_SIZE)
                        .color(if picked { ACCENT } else { OVERLAY1 })
                        .child(moving(count).nowrap()),
                );
            }
            tabs.push(node.on_click(Rc::new(move |ui| select(index, ui))));
        }
        let pill = div()
            .absolute()
            .top(0.0)
            .left(pill_at)
            .w(pill_w)
            .h(STRIP_TAB_H)
            .bg(tinted(SURFACE1, PILL_ALPHA), ROW_RADIUS)
            .eases(Easing::Size, pill_w, PILL_SPRING)
            .eases(Easing::Shift { x: 1.0, y: 0.0 }, pill_at, PILL_SPRING);
        strip_surface(div().flex_none().p(STRIP_PAD)).child(
            div()
                .w((x - STRIP_TAB_GAP).max(0.0))
                .h(STRIP_TAB_H)
                .flex()
                .gap(STRIP_TAB_GAP)
                .child(pill)
                .children(tabs),
        )
    }
}

fn strip_surface(node: Node) -> Node {
    let radius = ROW_RADIUS + STRIP_PAD;
    node.bg(tinted(TINT, STRIP_ALPHA), radius)
        .outline(tinted(TINT, STRIP_BORDER_ALPHA), radius)
}

fn card_title(title: Title) -> Node {
    let open = title.open.map(|handler| {
        control(None, CONTROL_RADIUS)
            .flex_none()
            .size(OPEN_SIZE)
            .text_size(TITLE_SIZE)
            .color(SUBTEXT1)
            .lit(TEXT)
            .child(text("›"))
            .on_click(handler)
    });
    let centred = title.glyph.is_some() || open.is_some();
    div()
        .flex()
        .when(centred, Node::items_center)
        .when(!centred, |line| line.items_baseline().justify_between())
        .gap(if centred { 8.0 } else { 12.0 })
        .children(
            title
                .glyph
                .map(|(path, tint)| glyph(path, TITLE_GLYPH, tint)),
        )
        .child(
            div()
                .when(centred, |words| words.flex_1().min_w(0.0).truncate())
                .text_size(TITLE_SIZE)
                .weight(SEMIBOLD)
                .child(moving(title.text)),
        )
        .when(!title.detail.is_empty(), |line| {
            line.child(
                div()
                    .min_w(0.0)
                    .truncate()
                    .text_size(CAPTION)
                    .color(OVERLAY2)
                    .child(moving(title.detail)),
            )
        })
        .children(open)
}

fn header_lead(lead: HeaderLead) -> Node {
    let holder = div()
        .size(lead.size)
        .flex_none()
        .child(mark(&lead.mark, lead.size, 1.0, "header"));
    let holder = match lead.on_click {
        Some(handler) => holder.on_click(handler),
        None => holder,
    };
    div()
        .min_w(0.0)
        .flex()
        .items_center()
        .gap(10.0)
        .child(holder)
        .child(
            div()
                .min_w(0.0)
                .col()
                .child(
                    div()
                        .truncate()
                        .text_size(SMALL_SIZE)
                        .color(OVERLAY2)
                        .child(text(lead.overline)),
                )
                .child(div().truncate().weight(SEMIBOLD).child(moving(lead.title))),
        )
}

fn close_button() -> Node {
    control(None, CONTROL_RADIUS)
        .size(HEADER_H)
        .color(SUBTEXT1)
        .lit(TEXT)
        .child(text("✕"))
        .on_click(crate::panel::on(|ui| ui.collapse()))
}

fn header_row(leading: Node, centred: bool, lead: Option<Node>, buttons: Vec<Button>) -> Node {
    let end = div()
        .flex()
        .when(centred, |end| end.flex_1().no_shrink().justify_end())
        .when(!centred, Node::flex_none)
        .items_center()
        .gap(6.0)
        .children(buttons.into_iter().map(button_node))
        .child(close_button());
    div()
        .min_h(HEADER_H)
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(16.0)
        .when(centred, |row| {
            row.child(
                div()
                    .flex_1()
                    .min_w(0.0)
                    .flex()
                    .items_center()
                    .children(lead),
            )
        })
        .child(leading)
        .child(end)
}

fn button_node(button: Button) -> Node {
    let enabled = button.enabled;
    let has_glyph = button.glyph.is_some();
    let lit = button.active && !button.toggle;
    let has_label = !has_glyph;
    let default_fg = if has_label { TEXT } else { SUBTEXT1 };
    let (rest, tint) = match (button.bare, lit) {
        (true, true) => (None, ACCENT),
        (true, false) => (None, default_fg),
        (false, true) => (Some(surface_fill(true)), ACCENT),
        (false, false) => (Some(surface_fill(false)), default_fg),
    };
    let side = if button.bare { HEADER_H } else { ICON_BUTTON };
    let node = control(rest, CONTROL_RADIUS)
        .flex_none()
        .when(!has_label, |node| node.size(side))
        .when(has_label, |node| {
            node.h(CONTROL_H)
                .px(12.0)
                .gap(8.0)
                .text_size(SMALL_SIZE)
                .weight(SEMIBOLD)
        })
        .color(tint)
        .when(button.busy, |node| node.beams(true))
        .when(button.toggle, |node| node.child(switch(button.active)))
        .children(button.glyph.map(|path| glyph(path, ICON_GLYPH, tint)))
        .when(has_label, |node| {
            node.child(node::steady(button.label.clone(), button.fits.clone()))
        })
        .when(has_glyph && !button.label.is_empty(), |node| {
            node.tip(button.label.clone())
        });
    if enabled {
        node.on_click(button.on_click)
    } else {
        node.unlit().opacity(0.5).color(OVERLAY2)
    }
}

fn switch(on: bool) -> Node {
    const TRACK_W: f32 = 32.0;
    const TRACK: f32 = 18.0;
    const KNOB_INSET: f32 = 2.0;
    let knob = TRACK - 2.0 * KNOB_INSET;
    let lit = f32::from(u8::from(on));
    let span = TRACK_W - knob - 2.0 * KNOB_INSET;
    div()
        .w(TRACK_W)
        .h(TRACK)
        .flex_none()
        .bg(tinted(TINT, TRACK_ALPHA), PILL)
        .child(div().absolute().inset(0.0).bg(color(ACCENT), PILL).eases(
            Easing::Strength,
            lit,
            KNOB_SPRING,
        ))
        .child(
            div()
                .absolute()
                .top(KNOB_INSET)
                .left(if on {
                    TRACK_W - knob - KNOB_INSET
                } else {
                    KNOB_INSET
                })
                .size(knob)
                .bg(color(TEXT), PILL)
                .eases(Easing::Shift { x: span, y: 0.0 }, lit, KNOB_SPRING),
        )
}

fn strip_icon(button: IconButton) -> Node {
    let (on, tint) = match button.color {
        Some(own) => (color(own), ON_ACCENT),
        None => (surface_fill(true), ACCENT),
    };
    let lit = f32::from(u8::from(button.active));
    let shown = button.glyph.map(|path| {
        let layer = |tint: u32, strength: f32| {
            glyph(path.clone(), STRIP_GLYPH, tint)
                .absolute()
                .left(0.0)
                .top(0.0)
                .eases(Easing::Strength, strength, FILL_SPRING)
                .eases(Easing::Pop { dip: POP_DIP }, lit, FILL_SPRING)
        };
        div()
            .size(STRIP_GLYPH)
            .child(layer(SUBTEXT1, 1.0 - lit))
            .child(layer(tint, lit))
    });
    control(None, ROW_RADIUS)
        .flex_none()
        .w(STRIP_ICON_W)
        .h(STRIP_ICON_H)
        .child(div().absolute().inset(0.0).bg(on, ROW_RADIUS).eases(
            Easing::Strength,
            lit,
            FILL_SPRING,
        ))
        .children(shown)
        .tip(button.label)
        .on_click(button.on_click)
}

fn tabs(
    labels: Vec<TabLabel>,
    selected: usize,
    on_select: crate::components::tabs::Select,
) -> Node {
    let glyphs = !labels.is_empty() && labels.iter().all(|tab| tab.glyph.is_some());
    let tab_w = if glyphs { GLYPH_TAB_W } else { TAB_W };
    let width = labels.len() as f32 * (tab_w + TAB_GAP) - TAB_GAP;
    let pill_at = selected as f32 * (tab_w + TAB_GAP);
    let pill = div()
        .absolute()
        .top(0.0)
        .left(pill_at)
        .w(tab_w)
        .h(HEADER_H)
        .bg(tinted(TINT, SELECTED_ALPHA), ROW_RADIUS)
        .eases(Easing::Shift { x: 1.0, y: 0.0 }, pill_at, PILL_SPRING);
    div()
        .flex_none()
        .w(width)
        .h(HEADER_H)
        .flex()
        .gap(TAB_GAP)
        .child(pill)
        .children(labels.into_iter().enumerate().map(|(index, tab)| {
            let picked = index == selected;
            let select = on_select.clone();
            let tint = if picked { ACCENT } else { TEXT };
            let node = control(None, ROW_RADIUS)
                .w(tab_w)
                .h(HEADER_H)
                .text_size(TEXT_SIZE)
                .weight(SEMIBOLD)
                .color(tint)
                .when(picked, Node::unlit);
            match tab.glyph {
                Some(path) => node.child(glyph(path, 18.0, tint)).tip(tab.label),
                None => node.child(text(tab.label).nowrap()),
            }
            .on_click(Rc::new(move |ui| select(index, ui)))
        }))
}

fn lead_node(lead: Lead, at: &str) -> Node {
    let (size, fill) = lead.size.map_or((LEAD, 0.8), |size| (size, 1.0));
    let own_hover = matches!(lead.mark, Some(Mark::Drawn(_)));
    let inner = lead
        .mark
        .as_ref()
        .map(|shown| mark(shown, size, fill, at).opacity(if lead.faded { 0.4 } else { 1.0 }));
    let mut node = div()
        .size(size)
        .flex_none()
        .flex()
        .items_center()
        .justify_center();
    if let Some(tint) = lead.tint {
        node = node.bg(tinted(tint, ACCENT_TINT_ALPHA), CONTROL_RADIUS);
    }
    if let Some((tint, strength)) = lead.glow.filter(|(_, strength)| *strength > 0.0) {
        let alpha = (strength.clamp(0.0, 1.0) * 96.0) as u8;
        node = node.child(
            div()
                .absolute()
                .inset(0.0)
                .bg(tinted(tint, alpha), CONTROL_RADIUS),
        );
    }
    let node = node.children(inner);
    match lead.on_click {
        Some(handler) if own_hover => node.on_click(handler),
        Some(handler) => node.hover(CONTROL_RADIUS).on_click(handler),
        None => node,
    }
}

fn trail_node(trail: Trail) -> Node {
    match trail {
        Trail::Text(words, tint) => div()
            .flex_none()
            .min_w(AMOUNT_W)
            .flex()
            .justify_end()
            .weight(SEMIBOLD)
            .color(tint)
            .child(moving(words).nowrap()),
        Trail::Faint(words) => div()
            .flex_none()
            .color(SUBTEXT1)
            .child(moving(words).nowrap()),
        Trail::Status(words, tint) => div()
            .flex_none()
            .flex()
            .items_center()
            .gap(6.0)
            .weight(SEMIBOLD)
            .color(tint)
            .child(div().size(8.0).bg(color(tint), PILL))
            .child(moving(words).nowrap()),
        Trail::Dropdown(_) => div(),
        Trail::Close { group, on_click } => control(None, CONTROL_RADIUS)
            .flex_none()
            .size(OVERLINE_H)
            .color(SUBTEXT1)
            .lit(TEXT)
            .shown_in(group)
            .child(text("✕"))
            .on_click(on_click),
        Trail::Button(trail) => {
            let tint = trail.tint.unwrap_or(SUBTEXT1);
            let rest = trail.tint.map(|own| tinted(own, ACCENT_TINT_ALPHA));
            control(rest, ROW_RADIUS)
                .flex_none()
                .h(24.0)
                .px(8.0)
                .gap(4.0)
                .text_size(SMALL_SIZE)
                .weight(SEMIBOLD)
                .color(tint)
                .when(trail.tint.is_none(), |button| button.lit(TEXT))
                .children(trail.glyph.map(|path| {
                    let turn = if trail.turned {
                        std::f32::consts::PI
                    } else {
                        0.0
                    };
                    glyph(path, 15.0, tint).eases(Easing::Turn, turn, item::TURN_SPRING)
                }))
                .children(trail.label.map(|label| moving(label).nowrap()))
                .on_click(trail.on_click)
        }
    }
}

fn frame(view: FrameView, children: Vec<Node>) -> Node {
    let motion = view.motion;
    let FrameView {
        id,
        look,
        on_click,
        on_dismiss,
        countdown,
        selected,
        shown_height,
        ..
    } = view;
    let chosen = tinted(TINT, SELECTED_ALPHA);
    let lit = f32::from(u8::from(selected));
    let node = match look {
        Look::Inset => div()
            .p(8.0)
            .col()
            .gap(6.0)
            .bg(tinted(SURFACE1, CARD_ALPHA), CONTROL_RADIUS)
            .child(
                div()
                    .absolute()
                    .inset(0.0)
                    .bg(chosen, CONTROL_RADIUS)
                    .eases(Easing::Strength, lit, CONTENT_IN),
            )
            .when(on_click.is_some() && !selected, |node| {
                node.child(div().absolute().inset(0.0).hover(CONTROL_RADIUS))
            })
            .children(children),
        Look::Raised { urgent } => {
            let border = if urgent {
                tinted(CRITICAL, URGENT_ALPHA)
            } else {
                tinted(SURFACE1, CARD_BORDER_ALPHA)
            };
            let calm = tinted(SURFACE1, NOTIFICATION_ALPHA);
            let shape = || {
                div()
                    .absolute()
                    .left(0.0)
                    .top(0.0)
                    .w_full()
                    .when_some(shown_height, Node::h)
                    .when(shown_height.is_none(), Node::h_full)
            };
            let hover_at = shape;
            let tint = shape()
                .bg(chosen, PANEL_RADIUS)
                .eases(Easing::Strength, lit, CONTENT_IN);
            let shape = shape().bg(calm, PANEL_RADIUS).outline(border, PANEL_RADIUS);
            let timer = countdown.map(|timer| {
                node::countdown(Countdown {
                    key: format!("{id}-{}", timer.round).into(),
                    rect: [0.0; 4],
                    left: timer.left,
                    running: timer.running.unwrap_or_default(),
                    color: timer.color,
                    opacity: 1.0,
                })
            });
            let content = div()
                .p(RAISED_PAD)
                .col()
                .gap(RAISED_GAP)
                .children(children)
                .when(shown_height.is_some(), |content| {
                    content.absolute().left(0.0).top(0.0).w_full()
                });
            div()
                .size_full()
                .when(shown_height.is_some(), Node::clip)
                .child(shape)
                .child(tint)
                .when(on_click.is_some() && !selected, |node| {
                    node.child(hover_at().hover(PANEL_RADIUS))
                })
                .children(timer)
                .child(content)
        }
    };
    let node = node.named(&id).anchor(id);
    let node = match on_click {
        Some(handler) => node.on_click(handler),
        None => node,
    };
    let node = match on_dismiss {
        Some(handler) => node.on_right(handler),
        None => node,
    };
    match motion {
        card::Motion::Still => node,
        card::Motion::Appear => node.rises(),
        card::Motion::Leave => node.leaves(),
    }
}

fn slider(view: Slider) -> Node {
    let value = view.value.clamp(0.0, 1.0);
    let (height, radius) = match view.label {
        Some(_) => (LARGE_H, CONTROL_RADIUS),
        None => (TRACK_H, PILL),
    };
    let fill_tint = if view.signal.is_some() {
        tinted(view.color, DIM_ALPHA)
    } else {
        color(view.color)
    };
    let label_row = |inside: &SliderLabel, tint: u32| {
        div()
            .absolute()
            .top(0.0)
            .left(0.0)
            .right(0.0)
            .h(height)
            .flex()
            .items_center()
            .gap(10.0)
            .px(LARGE_PAD)
            .text_size(SMALL_SIZE)
            .weight(SEMIBOLD)
            .color(tint)
            .nowrap()
            .children(
                inside
                    .glyph
                    .clone()
                    .map(|path| glyph(path, LARGE_GLYPH, tint)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(0.0)
                    .truncate()
                    .child(moving(inside.label.clone())),
            )
            .child(div().flex_none().child(moving(inside.amount.clone())))
    };
    let light = view.label.as_ref().map(|inside| label_row(inside, TEXT));
    let dark = view
        .label
        .as_ref()
        .filter(|_| value > 0.0)
        .map(|inside| label_row(inside, ON_ACCENT).w_share(1.0 / value));
    let signal = view
        .signal
        .map(|signal| signal.clamp(0.0, 1.0))
        .filter(|signal| *signal > 0.0)
        .map(|signal| {
            div()
                .absolute()
                .top(0.0)
                .left(0.0)
                .h_full()
                .w_share(signal * value)
                .bg(color(view.color), radius)
        });
    let track = div()
        .w_full()
        .h(height)
        .bg(tinted(TINT, TRACK_ALPHA), radius)
        .children(light)
        .child(
            div()
                .absolute()
                .top(0.0)
                .left(0.0)
                .h_full()
                .springy_with(value, slider_fill::FILL_SPRING)
                .clip()
                .bg(fill_tint, radius)
                .children(dark),
        )
        .children(signal);
    let node = div()
        .w_full()
        .when(view.label.is_none(), |hit| hit.py((HIT_H - TRACK_H) / 2.0))
        .child(track);
    match view.on_change {
        Some(on_change) => node.slides(on_change),
        None => node,
    }
}

fn segments_row(level: f32, hold: f32) -> Node {
    const COUNT: usize = 32;
    let half = 0.5 / COUNT as f32;
    div()
        .flex()
        .gap(2.0)
        .h(4.0)
        .children((0..COUNT).map(|index| {
            let at = (index as f32 + 0.5) / COUNT as f32;
            let lit = at <= level || (hold > level && (at - hold).abs() < half);
            let fill = if lit {
                color(segments::color(at))
            } else {
                tinted(TINT, TRACK_ALPHA)
            };
            div().flex_1().h_full().bg(fill, 1.0)
        }))
}

pub(super) fn text_block(content: &Text) -> Node {
    let node = match content.tone {
        Tone::Heading => div()
            .text_size(SMALL_SIZE)
            .weight(SEMIBOLD)
            .color(SUBTEXT1)
            .child(text(content.text.clone())),
        Tone::Caption(tint) => div()
            .text_size(CAPTION)
            .weight(SEMIBOLD)
            .color(tint)
            .child(moving(content.text.clone())),
        Tone::Note => div()
            .w_full()
            .py(NOTE_PAD)
            .when(content.min_width.is_some(), |note| {
                note.pb(NOTE_PAD + HELD_NOTE_LINE)
            })
            .flex()
            .justify_center()
            .text_size(SMALL_SIZE)
            .color(SUBTEXT1)
            .child(moving(content.text.clone())),
        Tone::Body | Tone::Reading => {
            let reading = matches!(content.tone, Tone::Reading);
            let (size, line) = if reading {
                (TEXT_SIZE, 22.0)
            } else {
                (BODY_SIZE, BODY_LINE)
            };
            let words = if content.spans.is_empty() && content.links.is_empty() {
                match content.lines {
                    Some(most) => node::clamped(content.text.clone(), most),
                    None => text(content.text.clone()),
                }
            } else {
                node::styled(
                    content.text.clone(),
                    content.spans.clone(),
                    content.links.clone(),
                    content.lines,
                )
            };
            div()
                .line(line)
                .text_size(size)
                .when(reading, |prose| prose.weight(400))
                .when_some(content.lines, |body, most| {
                    body.max_h(line * most as f32).clip()
                })
                .child(words)
        }
    };
    node.when_some(content.min_width, Node::min_w)
        .when_some(content.max_width, |node, width| {
            node.max_w(width).truncate()
        })
}

fn calendar(view: CalendarView) -> Node {
    const DAY_H: f32 = 40.0;
    const TODAY: f32 = 36.0;
    const DAY_GAP: f32 = 4.0;
    let offset = view.offset;
    let today = Local::now().date_naive();
    let first_of_today = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let first = if offset >= 0 {
        first_of_today.checked_add_months(Months::new(offset.unsigned_abs()))
    } else {
        first_of_today.checked_sub_months(Months::new(offset.unsigned_abs()))
    }
    .unwrap_or(first_of_today);
    let start = first - Duration::days(first.weekday().num_days_from_monday().into());
    let nav = |word: &'static str, handler: Handler| {
        control(None, CONTROL_RADIUS)
            .size(HEADER_H)
            .text_size(HEADING_SIZE)
            .child(text(word))
            .on_click(handler)
    };
    let back = (offset != 0).then(|| {
        control(None, CONTROL_RADIUS)
            .h(HEADER_H)
            .px(12.0)
            .text_size(SMALL_SIZE)
            .weight(SEMIBOLD)
            .color(ACCENT)
            .child(text("Today"))
            .on_click(view.on_today.clone())
    });
    let weekdays = div().flex().children(
        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
            .iter()
            .enumerate()
            .map(|(index, day)| {
                div()
                    .flex_1()
                    .flex()
                    .justify_center()
                    .text_size(SMALL_SIZE)
                    .weight(SEMIBOLD)
                    .color(if index >= 5 { PEACH } else { TEXT })
                    .child(text(*day))
            }),
    );
    let weeks = (0..6).map(|week| {
        div().flex().gap(DAY_GAP).children((0..7).map(|weekday| {
            let date = start + Duration::days(week * 7 + weekday);
            let in_month = date.month() == first.month();
            let is_today = date == today;
            div()
                .flex_1()
                .min_w(DAY_H)
                .h(DAY_H)
                .flex()
                .items_center()
                .justify_center()
                .color(match (is_today, in_month) {
                    (true, _) => ON_ACCENT,
                    (false, true) => TEXT,
                    (false, false) => OVERLAY2,
                })
                .child(
                    div()
                        .size(TODAY)
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(is_today, |cell| {
                            cell.bg(color(ACCENT), CONTROL_RADIUS).weight(BOLD)
                        })
                        .child(text(date.day().to_string())),
                )
        }))
    });
    div()
        .col()
        .gap(DAY_GAP)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb(6.0)
                .child(
                    div()
                        .pl(8.0)
                        .text_size(HEADING_SIZE)
                        .weight(SEMIBOLD)
                        .child(text(first.format("%B %Y").to_string())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(2.0)
                        .children(back)
                        .child(nav("‹", view.on_previous.clone()))
                        .child(nav("›", view.on_next.clone())),
                ),
        )
        .child(div().pb(4.0).child(weekdays))
        .children(weeks)
}

fn meter(view: Meter, at: &str) -> Node {
    match view.shape {
        Shape::Bar => bar_meter(view),
        Shape::Ring => ring_meter(view),
        Shape::History { .. } => history_meter(view, at),
    }
}

fn meters_row(columns: usize, meters: Vec<Meter>, at: &str) -> Node {
    if meters
        .iter()
        .all(|meter| matches!(meter.shape, Shape::Ring))
    {
        return div()
            .wrap()
            .gap_y(10.0)
            .children(meters.into_iter().map(ring_meter));
    }
    let columns = columns.max(1);
    let mut cells = meters.into_iter().enumerate().peekable();
    let mut rows = Vec::new();
    while cells.peek().is_some() {
        let row: Vec<Node> = (0..columns)
            .map(|_| match cells.next() {
                Some((index, next)) => div()
                    .flex_1()
                    .min_w(0.0)
                    .child(meter(next, &format!("{at}-{index}"))),
                None => div().flex_1(),
            })
            .collect();
        rows.push(div().flex().gap(18.0).children(row));
    }
    div().col().gap(12.0).children(rows)
}

fn bar_meter(view: Meter) -> Node {
    const WIDTH: f32 = 360.0;
    const GLYPH: f32 = 22.0;
    const TILE: f32 = 40.0;
    let muted = view.level.is_some_and(|(_, muted)| muted);
    let tint = if muted { OVERLAY2 } else { view.color };
    let tile = div()
        .size(TILE)
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .bg(tinted(tint, ACCENT_TINT_ALPHA), CONTROL_RADIUS)
        .children(view.glyph.clone().map(|path| glyph(path, GLYPH, tint)));
    let row = div()
        .w(WIDTH)
        .px(4.0)
        .flex()
        .items_center()
        .gap(12.0)
        .child(tile);
    let Some((value, muted)) = view.level else {
        return row.child(
            div()
                .flex_1()
                .min_w(0.0)
                .col()
                .gap(2.0)
                .when(!view.caption.is_empty(), |words| {
                    words.child(
                        div()
                            .text_size(SMALL_SIZE)
                            .color(SUBTEXT1)
                            .child(moving(view.caption.clone())),
                    )
                })
                .child(
                    div()
                        .min_w(0.0)
                        .truncate()
                        .weight(SEMIBOLD)
                        .color(view.color)
                        .child(moving(view.label.clone())),
                ),
        );
    };
    let percent = (value * 100.0).round() as i32;
    let heading = div()
        .flex()
        .items_baseline()
        .gap(8.0)
        .min_w(0.0)
        .when(!view.caption.is_empty(), |line| {
            line.child(
                div()
                    .flex_none()
                    .text_size(SMALL_SIZE)
                    .color(SUBTEXT1)
                    .child(moving(view.caption.clone()).nowrap()),
            )
        })
        .child(
            div()
                .min_w(0.0)
                .truncate()
                .weight(SEMIBOLD)
                .child(moving(view.label.clone())),
        );
    let amount = div()
        .flex_none()
        .weight(SEMIBOLD)
        .color(if muted { OVERLAY2 } else { TEXT })
        .child(
            moving(if muted {
                "Muted".to_owned()
            } else {
                format!("{percent}%")
            })
            .nowrap(),
        );
    row.child(
        div()
            .flex_1()
            .min_w(0.0)
            .col()
            .gap(2.0)
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .justify_between()
                    .gap(12.0)
                    .child(heading)
                    .child(amount),
            )
            .child(slider(Slider {
                id: view.id.clone(),
                value: if muted { 0.0 } else { value },
                color: tint,
                signal: None,
                on_change: None,
                label: None,
            })),
    )
}

fn ring_meter(view: Meter) -> Node {
    const RING: f32 = 54.0;
    const RING_GLYPH: f32 = 22.0;
    const RING_W: f32 = 84.0;
    let level = view.level.map_or(0.0, |(level, _)| level.clamp(0.0, 1.0));
    let dial = ring(RING, level, view.color)
        .flex()
        .items_center()
        .justify_center()
        .children(
            view.glyph
                .clone()
                .map(|path| glyph(path, RING_GLYPH, SUBTEXT1)),
        );
    div()
        .w(RING_W)
        .col()
        .items_center()
        .gap(4.0)
        .child(dial)
        .child(
            div()
                .pt(2.0)
                .text_size(13.0)
                .weight(SEMIBOLD)
                .child(moving(format!("{}%", (level * 100.0).round() as i32))),
        )
        .child(
            div()
                .max_w(RING_W)
                .truncate()
                .text_size(11.0)
                .color(OVERLAY2)
                .child(moving(view.label)),
        )
}

fn history_meter(view: Meter, _at: &str) -> Node {
    const NAME: f32 = 11.5;
    const GLYPH: f32 = 14.0;
    const VALUE: f32 = 20.0;
    const UNIT: f32 = 12.0;
    const HEIGHT: f32 = 24.0;
    let Shape::History {
        amount,
        unit,
        history,
    } = view.shape
    else {
        return div();
    };
    let history = history.map(|series| {
        div()
            .h(HEIGHT)
            .mt(4.0)
            .child(node::chart(vec![series], None).size_full())
    });
    let spaced = !unit.starts_with(['%', '°']);
    div()
        .col()
        .child(
            div()
                .flex()
                .items_center()
                .gap(5.0)
                .text_size(NAME)
                .weight(SEMIBOLD)
                .color(view.color)
                .children(view.glyph.map(|path| glyph(path, GLYPH, view.color)))
                .child(div().min_w(0.0).truncate().child(moving(view.label))),
        )
        .child(
            div()
                .flex()
                .items_baseline()
                .when(spaced, |figure| figure.gap(3.0))
                .leading(1.2)
                .child(
                    div()
                        .text_size(VALUE)
                        .weight(SEMIBOLD)
                        .child(moving(amount).nowrap()),
                )
                .child(
                    div()
                        .text_size(UNIT)
                        .color(SUBTEXT1)
                        .child(moving(unit).nowrap()),
                ),
        )
        .children(history)
}

fn player(view: PlayerView) -> Node {
    const ART: f32 = 112.0;
    const ART_GAP: f32 = 16.0;
    let text_w = card::Size::Large.width() - 2.0 * CARD_PAD - ART - ART_GAP;
    let art = div()
        .size(ART)
        .flex_none()
        .clip()
        .bg(tinted(SURFACE1, CONTROL_ALPHA), PANEL_RADIUS)
        .children(view.art.clone().map(|path| {
            node::picture(crate::picture::Picture::File(path), ART)
                .cover()
                .rounded(PANEL_RADIUS)
        }));
    let round = |path: Option<std::path::PathBuf>, size: f32, primary: bool, action: Handler| {
        let (rest, tint) = if primary {
            (Some(color(ACCENT)), ON_ACCENT)
        } else {
            (None, TEXT)
        };
        control(rest, PILL)
            .size(size)
            .flex_none()
            .children(path.map(|path| glyph(path, size * 0.45, tint)))
            .on_click(action)
    };
    let clock = |seconds: f64| {
        let seconds = seconds.max(0.0) as u64;
        format!("{}:{:02}", seconds / 60, seconds % 60)
    };
    let progress = view.length.map(|length| {
        let fraction = (view.position / length.max(1.0)) as f32;
        div()
            .col()
            .gap(4.0)
            .child(slider(Slider {
                id: "player-progress".into(),
                value: fraction,
                color: ACCENT,
                signal: None,
                on_change: view.on_seek.clone(),
                label: None,
            }))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(SMALL_SIZE)
                    .color(SUBTEXT1)
                    .child(moving(clock(view.position)))
                    .child(moving(clock(length))),
            )
    });
    let line = |words: SharedString, size: f32, weight: u16| {
        node::marquee(words, text_w)
            .min_w(0.0)
            .h((size * 1.4).ceil())
            .text_size(size)
            .weight(weight)
    };
    div()
        .w(ART + ART_GAP + text_w)
        .flex()
        .gap(ART_GAP)
        .child(art)
        .child(
            div()
                .flex_1()
                .min_w(0.0)
                .col()
                .justify_between()
                .child(
                    div()
                        .col()
                        .gap(2.0)
                        .child(
                            div()
                                .text_size(SMALL_SIZE)
                                .color(SUBTEXT1)
                                .child(text(view.source)),
                        )
                        .child(line(view.title, TITLE_SIZE, SEMIBOLD))
                        .child(line(view.artist, TEXT_SIZE, MEDIUM))
                        .when(!view.album.is_empty(), |words| {
                            words.child(line(view.album, SMALL_SIZE, MEDIUM))
                        }),
                )
                .children(progress)
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .items_center()
                        .gap(12.0)
                        .child(round(view.previous, 36.0, false, view.on_previous))
                        .child(round(view.play, 44.0, true, view.on_play))
                        .child(round(view.next, 36.0, false, view.on_next)),
                ),
        )
}

fn lines(entries: Vec<Line>) -> Node {
    div()
        .col()
        .children(entries.into_iter().enumerate().map(|(index, line)| {
            div()
                .col()
                .gap(2.0)
                .py(8.0)
                .when(index > 0, |row| row.rule(tinted(SURFACE1, CONTROL_ALPHA)))
                .child(
                    div()
                        .min_w(0.0)
                        .truncate()
                        .text_size(SMALL_SIZE)
                        .weight(SEMIBOLD)
                        .child(text(line.summary)),
                )
                .child(
                    div()
                        .min_w(0.0)
                        .truncate()
                        .text_size(CAPTION)
                        .color(SUBTEXT1)
                        .child(text(line.detail)),
                )
        }))
}

fn stop(stop: Stop) -> Node {
    const DOT: f32 = 7.0;
    let (tint, words) = match stop.state {
        StopState::Reached => (GREEN, TEXT),
        StopState::Pending(_) => (YELLOW, TEXT),
        StopState::NotYet => (OVERLAY2, OVERLAY2),
    };
    let dot = div().size(DOT).flex_none();
    let dot = match stop.state {
        StopState::NotYet => dot.outline(color(tint), PILL),
        _ => dot.bg(color(tint), PILL),
    };
    let mut node = div()
        .flex()
        .flex_none()
        .items_center()
        .gap(6.0)
        .child(dot)
        .child(div().color(words).child(text(stop.label).nowrap()));
    if let StopState::Pending(Some(left)) = stop.state {
        node = node.child(div().color(SUBTEXT1).child(text(left).nowrap()));
    }
    node
}

fn routes(entries: Vec<Route>) -> Node {
    const LINK: f32 = 14.0;
    const STEP_H: f32 = 22.0;
    const DOT: f32 = 7.0;
    let link = tinted(SURFACE1, CONTROL_ALPHA);
    div()
        .col()
        .children(entries.into_iter().enumerate().map(|(index, route)| {
            let stops = if route.vertical {
                div()
                    .col()
                    .text_size(CAPTION)
                    .children(route.stops.into_iter().enumerate().map(|(position, next)| {
                        div()
                            .h(STEP_H)
                            .flex()
                            .items_center()
                            .when(position > 0, |row| {
                                row.child(
                                    div()
                                        .absolute()
                                        .left(DOT / 2.0 - 0.5)
                                        .top(-(STEP_H - DOT) / 2.0)
                                        .w(1.0)
                                        .h(STEP_H - DOT)
                                        .bg(link, 0.0),
                                )
                            })
                            .child(self::stop(next).min_w(0.0))
                            .rises()
                    }))
            } else {
                let mut row = div().wrap().items_center().gap_y(4.0).text_size(CAPTION);
                for (position, next) in route.stops.into_iter().enumerate() {
                    if position > 0 {
                        row = row.child(div().w(LINK).mx(6.0).h(1.0).flex_none().bg(link, 0.0));
                    }
                    row = row.child(self::stop(next));
                }
                row
            };
            let titled = !route.title.is_empty();
            div()
                .col()
                .gap(4.0)
                .py(8.0)
                .when(index > 0, |row| row.rule(link))
                .when(titled, |row| {
                    row.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(12.0)
                            .child(
                                div()
                                    .min_w(0.0)
                                    .truncate()
                                    .text_size(SMALL_SIZE)
                                    .weight(SEMIBOLD)
                                    .child(text(route.title.clone())),
                            )
                            .children(route.done.clone().map(|done| {
                                div()
                                    .flex_none()
                                    .text_size(CAPTION)
                                    .weight(SEMIBOLD)
                                    .color(GREEN)
                                    .child(text(done).nowrap())
                            })),
                    )
                    .child(
                        div()
                            .min_w(0.0)
                            .truncate()
                            .text_size(CAPTION)
                            .color(SUBTEXT1)
                            .child(text(route.detail.clone())),
                    )
                })
                .child(stops)
        }))
}

fn buttons(row: ButtonRow) -> Node {
    let gap = if row.compact {
        COMPACT_GAP
    } else {
        BUTTON_GAP - crate::components::button::BADGE_OVERHANG
    };
    let compact = row.compact;
    div()
        .flex()
        .items_center()
        .when(row.end, Node::justify_end)
        .ml(row.indent)
        .gap(gap)
        .children(row.buttons.into_iter().map(move |button| {
            let side = if compact { CONTROL_H } else { ICON_BUTTON };
            let tint = button.color.unwrap_or(if button.glyph.is_some() {
                SUBTEXT1
            } else {
                TEXT
            });
            let lit = button.active;
            let rest = match (lit, button.color) {
                (true, Some(own)) => color(own),
                (true, None) => surface_fill(true),
                (false, _) => surface_fill(false),
            };
            let shown = if lit && button.color.is_some() {
                ON_ACCENT
            } else if lit {
                ACCENT
            } else {
                tint
            };
            let has_glyph = button.glyph.is_some();
            let badge = button.badge.clone();
            let node = control(Some(rest), CONTROL_RADIUS)
                .flex_none()
                .when(has_glyph, |node| node.size(side))
                .when(!has_glyph, |node| {
                    node.h(CONTROL_H)
                        .px(12.0)
                        .gap(8.0)
                        .text_size(SMALL_SIZE)
                        .weight(SEMIBOLD)
                })
                .color(shown)
                .children(button.glyph.map(|path| glyph(path, ICON_GLYPH, shown)))
                .when(!has_glyph, |node| {
                    node.child(text(button.label.clone()).nowrap())
                })
                .when(has_glyph, |node| node.tip(button.label.clone()))
                .on_click(button.on_click);
            if compact { node } else { badged(node, badge) }
        }))
}

/// With room kept past its corner, so a badge coming and going moves nothing.
fn badged(target: Node, badge: Option<SharedString>) -> Node {
    const OVERHANG: f32 = crate::components::button::BADGE_OVERHANG;
    const SIDE: f32 = 17.0;
    const RING: f32 = 2.0;
    div()
        .flex_none()
        .pt(OVERHANG)
        .pr(OVERHANG)
        .child(target)
        .when_some(badge, |holder, words| {
            holder.child(
                div()
                    .absolute()
                    .top(0.0)
                    .right(0.0)
                    .flex()
                    .min_w(SIDE)
                    .h(SIDE)
                    .p(RING)
                    .bg(color(crate::theme::BASE), BADGE_RADIUS)
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .px(4.0)
                            .text_size(10.5)
                            .weight(BOLD)
                            .color(ON_ACCENT)
                            .bg(color(ACCENT), BADGE_RADIUS - RING)
                            .child(moving(words)),
                    ),
            )
        })
}

fn action(tile: ActionTile) -> Node {
    const GLYPH: f32 = 26.0;
    const SWITCH_H: f32 = 58.0;
    const MARK: f32 = 36.0;
    const SWITCH_GLYPH: f32 = 20.0;
    let ActionTile {
        glyph: path,
        label,
        state,
        color: tint,
        armed,
        on_click,
        ..
    } = tile;
    if let Some(state) = state {
        let state_color = if armed { tint } else { OVERLAY2 };
        let glyph_color = if armed { ON_ACCENT } else { TEXT };
        let lit = f32::from(u8::from(armed));
        let mark = div()
            .flex_none()
            .size(MARK)
            .flex()
            .items_center()
            .justify_center()
            .bg(surface_fill(false), CONTROL_RADIUS)
            .child(
                div()
                    .absolute()
                    .inset(0.0)
                    .bg(color(tint), CONTROL_RADIUS)
                    .eases(Easing::Strength, lit, KNOB_SPRING),
            )
            .children(path.map(|path| glyph(path, SWITCH_GLYPH, glyph_color)));
        return control(Some(tinted(SURFACE1, CARD_ALPHA)), CONTROL_RADIUS)
            .h(SWITCH_H)
            .min_w(0.0)
            .justify_start()
            .px(10.0)
            .gap(11.0)
            .child(mark)
            .child(
                div()
                    .min_w(0.0)
                    .col()
                    .gap(2.0)
                    .child(
                        div()
                            .truncate()
                            .text_size(SMALL_SIZE)
                            .weight(SEMIBOLD)
                            .child(text(label)),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(11.5)
                            .color(state_color)
                            .child(moving(state)),
                    ),
            )
            .on_click(on_click);
    }
    let (rest, words, shown) = if armed {
        (color(tint), ON_ACCENT, ON_ACCENT)
    } else {
        (surface_fill(false), TEXT, tint)
    };
    control(Some(rest), CONTROL_RADIUS)
        .w(ACTION_W)
        .h(ACTION_H)
        .flex_none()
        .col()
        .items_center()
        .justify_center()
        .gap(10.0)
        .text_size(SMALL_SIZE)
        .weight(SEMIBOLD)
        .color(words)
        .children(path.map(|path| glyph(path, GLYPH, shown)))
        .child(moving(label))
        .on_click(on_click)
}

fn pick(tile: ActionTile) -> Node {
    const PICK_H: f32 = 72.0;
    const PICK_GLYPH: f32 = 22.0;
    const PICK_PAD: f32 = 8.0;
    let (rest, words) = if tile.armed {
        (tinted(tile.color, ACCENT_TINT_ALPHA), tile.color)
    } else {
        (surface_fill(false), TEXT)
    };
    control(Some(rest), CONTROL_RADIUS)
        .flex_1()
        .min_w(0.0)
        .h(PICK_H)
        .px(PICK_PAD)
        .col()
        .items_center()
        .justify_center()
        .gap(8.0)
        .text_size(SMALL_SIZE)
        .weight(SEMIBOLD)
        .color(words)
        .children(tile.glyph.map(|path| glyph(path, PICK_GLYPH, words)))
        .child(
            node::marquee_on_hover(tile.label)
                .w_full()
                .max_w(ACTION_W)
                .h((SMALL_SIZE * 1.4).ceil()),
        )
        .on_click(tile.on_click)
}

fn warning(warning: Warning) -> Node {
    let tint = warning.color;
    let chip = div()
        .flex()
        .items_center()
        .justify_center()
        .flex_none()
        .h(30.0)
        .pl(6.0)
        .pr(12.0)
        .gap(7.0)
        .text_size(12.0)
        .weight(SEMIBOLD)
        .bg(tinted(tint, ACCENT_TINT_ALPHA / 2), CONTROL_RADIUS)
        .child(
            div()
                .size(20.0)
                .flex()
                .items_center()
                .justify_center()
                .bg(color(tint), BADGE_RADIUS)
                .children(warning.glyph.map(|path| glyph(path, 14.0, ON_ACCENT))),
        )
        .child(moving(warning.label).nowrap())
        .child(div().color(tint).child(moving(warning.value).nowrap()));
    match warning.on_click {
        Some(handler) => chip.hover(CONTROL_RADIUS).on_click(handler),
        None => chip,
    }
}

fn facts(items: Vec<(SharedString, SharedString, Option<u32>)>) -> Node {
    let mut rows = Vec::new();
    let mut items = items.into_iter().peekable();
    while items.peek().is_some() {
        let row: Vec<_> = items.by_ref().take(3).collect();
        rows.push(
            div()
                .flex()
                .gap(6.0)
                .children(row.into_iter().map(|(label, value, tint)| {
                    div()
                        .flex_1()
                        .min_w(0.0)
                        .px(8.0)
                        .py(4.0)
                        .bg(tinted(TINT, 0x0d), 9.0)
                        .col()
                        .leading(FACT_LEADING)
                        .child(
                            div()
                                .text_size(10.5)
                                .leading(FACT_LEADING)
                                .color(OVERLAY2)
                                .child(text(label)),
                        )
                        .child(
                            div()
                                .text_size(SMALL_SIZE)
                                .leading(FACT_LEADING)
                                .weight(SEMIBOLD)
                                .when_some(tint, Node::color)
                                .truncate()
                                .child(moving(value)),
                        )
                })),
        );
    }
    div().col().gap(6.0).children(rows)
}

fn hero(name: &str, lines: &[SharedString]) -> Node {
    div()
        .col()
        .gap(4.0)
        .child(
            div()
                .text_size(20.0)
                .weight(BOLD)
                .child(text(name.to_owned())),
        )
        .child(
            div()
                .col()
                .text_size(CAPTION)
                .leading(HERO_LEADING)
                .color(SUBTEXT1)
                .children(
                    lines
                        .iter()
                        .map(|line| div().truncate().child(text(line.clone()))),
                ),
        )
}

fn tile_meters(rows: Vec<tile::Meter>) -> Node {
    div().col().gap(7.0).children(rows.into_iter().map(|row| {
        div()
            .flex()
            .items_center()
            .gap(8.0)
            .text_size(CAPTION)
            .child(
                div()
                    .w(LABEL_W)
                    .flex_none()
                    .truncate()
                    .color(SUBTEXT1)
                    .child(text(row.label)),
            )
            .child(bar(row.fraction, row.color, row.mark))
            .child(
                div()
                    .min_w(VALUE_W)
                    .flex_none()
                    .flex()
                    .justify_end()
                    .nowrap()
                    .weight(SEMIBOLD)
                    .child(moving(row.value)),
            )
    }))
}

fn bar(fraction: f32, tint: u32, mark: Option<f32>) -> Node {
    let node = div()
        .flex_1()
        .min_w(0.0)
        .h(5.0)
        .clip()
        .bg(track(), PILL)
        .child(
            div()
                .h_full()
                .springy(fraction.clamp(0.0, 1.0))
                .bg(color(tint), PILL),
        );
    match mark {
        Some(mark) => div().flex_1().min_w(0.0).h(5.0).child(node.w_full()).child(
            div()
                .absolute()
                .top(-3.0)
                .left_share(mark.clamp(0.0, 1.0))
                .eases(Easing::Along, mark.clamp(0.0, 1.0), CONTENT_IN)
                .ml(-1.0)
                .w(2.0)
                .h(11.0)
                .bg(color(TEXT), PILL),
        ),
        None => node,
    }
}

fn cores(loads: Vec<f32>, tint: u32) -> Node {
    let inner = card::Size::Small.width() - 2.0 * PAD;
    let cell = (inner - CORE_GAP * (CORE_COLUMNS - 1) as f32) / CORE_COLUMNS as f32;
    div()
        .wrap()
        .gap(CORE_GAP)
        .children(loads.into_iter().map(move |load| {
            div()
                .w(cell)
                .h(8.0)
                .bg(color(tint), 2.5)
                .opacity(0.15 + load.clamp(0.0, 1.0) * 0.85)
        }))
}

fn legend(items: Vec<(Option<u32>, SharedString)>) -> Node {
    div()
        .flex()
        .gap(12.0)
        .text_size(CAPTION)
        .weight(SEMIBOLD)
        .color(SUBTEXT1)
        .children(items.into_iter().map(|(dot, words)| {
            div()
                .flex()
                .items_center()
                .gap(5.0)
                .nowrap()
                .when(dot.is_some(), |item| item.color(TEXT))
                .children(dot.map(|tint| div().size(7.0).flex_none().bg(color(tint), PILL)))
                .child(text(words))
        }))
}

fn volumes(key: String, items: Vec<Volume>) -> Node {
    let rows = div()
        .col()
        .children(items.into_iter().enumerate().map(|(index, volume)| {
            div()
                .flex_none()
                .when(index > 0, |row| row.rule(hairline()))
                .col()
                .gap(4.0)
                .py(6.0)
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(10.0)
                        .text_size(CAPTION)
                        .leading(FACT_LEADING)
                        .child(
                            div()
                                .min_w(0.0)
                                .truncate()
                                .color(SUBTEXT1)
                                .child(moving(volume.mount)),
                        )
                        .child(
                            div()
                                .flex_none()
                                .nowrap()
                                .weight(SEMIBOLD)
                                .child(moving(volume.usage)),
                        ),
                )
                .child(div().flex().child(bar(volume.fraction, volume.color, None)))
        }));
    div().flex_1().min_h(0.0).scrolls(key).child(rows)
}

fn drives(items: Vec<Drive>) -> Node {
    div()
        .flex_none()
        .col()
        .gap(10.0)
        .pt(12.0)
        .rule(hairline())
        .child(
            div()
                .text_size(11.0)
                .weight(SEMIBOLD)
                .color(OVERLAY2)
                .child(text("DRIVES")),
        )
        .children(items.into_iter().map(|drive| {
            let total: f32 = drive
                .segments
                .iter()
                .map(|segment| segment.weight)
                .sum::<f32>()
                .max(f32::EPSILON);
            div()
                .col()
                .gap(6.0)
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(10.0)
                        .text_size(CAPTION)
                        .color(SUBTEXT1)
                        .child(div().min_w(0.0).truncate().child(text(drive.name)))
                        .child(
                            div()
                                .flex_none()
                                .weight(SEMIBOLD)
                                .color(TEXT)
                                .child(text(drive.size).nowrap()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap(2.0)
                        .h(8.0)
                        .clip()
                        .bg(track(), PILL)
                        .children(drive.segments.into_iter().map(move |segment| {
                            let piece = div().h_full().w(0.0).grow(segment.weight / total);
                            match segment.color {
                                Some(tint) => piece.bg(color(tint), 0.0),
                                None => piece,
                            }
                        })),
                )
        }))
}

fn entry_node(entry: Entry) -> Node {
    let clickable = entry.on_click.is_some();
    let lead = entry.lead.map(|lead| match lead {
        EntryLead::Glyph(path, tint) => div()
            .size(ENTRY_GLYPH)
            .flex_none()
            .children(path.map(|path| glyph(path, ENTRY_GLYPH, tint))),
        EntryLead::Dot(tint) => div().size(7.0).flex_none().bg(color(tint), PILL),
    });
    let content = div()
        .flex()
        .items_center()
        .gap(10.0)
        .px(ENTRY_BLEED)
        .py(5.0)
        .children(lead)
        .child(
            div()
                .flex_1()
                .min_w(0.0)
                .col()
                .leading(FACT_LEADING)
                .child(
                    div()
                        .truncate()
                        .text_size(SMALL_SIZE)
                        .leading(FACT_LEADING)
                        .weight(SEMIBOLD)
                        .when_some(entry.title_color, Node::color)
                        .child(text(entry.title)),
                )
                .when(!entry.detail.is_empty(), |words| {
                    words.child(
                        div()
                            .truncate()
                            .text_size(11.0)
                            .leading(FACT_LEADING)
                            .color(OVERLAY2)
                            .child(moving(entry.detail)),
                    )
                }),
        )
        .when(!entry.value.is_empty(), |line| {
            line.child(
                div()
                    .flex_none()
                    .nowrap()
                    .text_size(CAPTION)
                    .weight(SEMIBOLD)
                    .color(entry.value_color)
                    .child(moving(entry.value)),
            )
        });
    let row = div().flex_none().child(content);
    let row = match entry.chosen {
        Some(tint) => row.bg(tinted(tint, CHOSEN_ALPHA), CONTROL_RADIUS),
        None => row,
    };
    let row = if clickable {
        row.hover(CONTROL_RADIUS)
    } else {
        row
    };
    match entry.on_click {
        Some(handler) => row.on_click(handler),
        None => row,
    }
}

fn part(part: Part, id: &str, index: usize, span: usize) -> Node {
    match part {
        Part::Chart {
            series,
            grow,
            phase,
        } => div()
            .when(grow, |area| area.flex_1().min_h(CHART_H))
            .when(!grow, |area| {
                area.flex_none()
                    .h(if span > 1 { TALL_CHART_H } else { CHART_H })
            })
            .child(node::chart(series, phase).absolute().inset(0.0)),
        Part::Cores { loads, color } => cores(loads, color),
        Part::Meters {
            rows,
            scroll: false,
        } => tile_meters(rows).flex_none(),
        Part::Meters { rows, scroll: true } => div()
            .flex_1()
            .min_h(0.0)
            .scrolls(format!("{id}-meters-{index}"))
            .child(tile_meters(rows)),
        Part::Legend(items) => legend(items).flex_none(),
        Part::Facts(items) => facts(
            items
                .into_iter()
                .map(|(label, value)| (label, value, None))
                .collect(),
        )
        .flex_none(),
        Part::Hero { name, lines } => hero(&name, &lines).flex_none(),
        Part::Volumes(items) => volumes(format!("{id}-volumes"), items),
        Part::Drives(items) => drives(items),
        Part::Entries { entries, scroll } => {
            let list = div()
                .mx(-ENTRY_BLEED)
                .col()
                .gap(2.0)
                .children(entries.into_iter().map(entry_node));
            if scroll {
                div()
                    .flex_1()
                    .min_h(0.0)
                    .scrolls(format!("{id}-entries-{index}"))
                    .child(list)
            } else {
                list.flex_none()
            }
        }
        Part::Spacer => div().flex_1(),
    }
}

fn tile_node(tile: Tile) -> Node {
    let Tile {
        id,
        span,
        title,
        on_click,
        parts,
    } = tile;
    let mut parts = parts.into_iter().enumerate();
    let top: Vec<Node> = parts
        .by_ref()
        .take_while(|(_, piece)| !matches!(piece, Part::Spacer))
        .map(|(index, piece)| part(piece, &id, index, span))
        .collect();
    let bottom: Vec<Node> = parts
        .map(|(index, piece)| part(piece, &id, index, span))
        .collect();
    let node = card_surface()
        .w(card::Size::Small.width())
        .h(tile::height(span))
        .flex_none()
        .clip()
        .p(PAD)
        .col()
        .gap(PART_GAP)
        .children(title.map(|(words, value)| {
            div()
                .flex_none()
                .flex()
                .items_baseline()
                .justify_between()
                .gap(12.0)
                .child(div().text_size(15.0).weight(SEMIBOLD).child(text(words)))
                .children(value.map(|value| {
                    div()
                        .text_size(15.0)
                        .weight(SEMIBOLD)
                        .nowrap()
                        .color(value.color)
                        .child(moving(value.text))
                }))
        }))
        .children(top)
        .when(!bottom.is_empty(), |tile| {
            tile.child(
                div()
                    .flex_1()
                    .min_h(0.0)
                    .col()
                    .justify_end()
                    .gap(PART_GAP)
                    .children(bottom),
            )
        });
    match on_click {
        Some(handler) => node.hover(PANEL_RADIUS).on_click(handler),
        None => node,
    }
}

fn board(columns: Vec<Vec<Tile>>) -> Node {
    div()
        .flex()
        .justify_center()
        .items_start()
        .gap(card::GAP)
        .children(columns.into_iter().map(|column| {
            div()
                .col()
                .gap(card::GAP)
                .children(column.into_iter().map(tile_node))
        }))
}

pub(super) fn input(view: InputView) -> Node {
    const LINE: f32 = 18.0;
    const PAD_X: f32 = 12.0;
    const PAD_Y: f32 = 13.0;
    let empty = view.text.is_empty();
    let typed = if view.secret {
        "•".repeat(view.text.chars().count())
    } else {
        view.text.to_string()
    };
    let (shown, caret) = if empty {
        (format!("{CARET}{}", view.placeholder), 0..CARET.len())
    } else {
        let at = typed.len();
        (format!("{typed}{CARET}"), at..at + CARET.len())
    };
    let accent = TextStyle {
        link: true,
        ..TextStyle::default()
    };
    let words = div()
        .when(empty, |words| words.color(OVERLAY2))
        .child(node::styled(shown, vec![(caret, accent)], Vec::new(), None));
    let ready = view.on_send.is_some();
    let send = control(Some(surface_fill(ready)), CONTROL_RADIUS)
        .size(ICON_BUTTON)
        .flex_none()
        .color(if ready { ACCENT } else { SUBTEXT1 })
        .children(
            view.send_glyph
                .clone()
                .map(|path| glyph(path, ICON_GLYPH, if ready { ACCENT } else { SUBTEXT1 })),
        )
        .when(view.send_glyph.is_none(), |send| send.child(text("↑")));
    let send = match view.on_send {
        Some(handler) => send.on_click(handler),
        None => send,
    };
    let inset = (LINE + 2.0 * PAD_Y - ICON_BUTTON) / 2.0;
    div()
        .min_w(0.0)
        .beams(view.busy)
        .bg(tinted(SURFACE1, CONTROL_ALPHA), CONTROL_RADIUS)
        .child(
            div()
                .flex()
                .items_end()
                .gap(8.0)
                .pr(inset)
                .child(
                    div()
                        .flex_1()
                        .min_w(0.0)
                        .px(PAD_X)
                        .py(PAD_Y)
                        .max_h(LINE * view.max_lines.max(1) as f32 + 2.0 * PAD_Y)
                        .clip()
                        .col()
                        .justify_end()
                        .text_size(SMALL_SIZE)
                        .line(LINE)
                        .child(words),
                )
                .child(div().flex_none().pb(inset).child(send)),
        )
}

fn search(view: SearchView) -> Node {
    const WIDTH: f32 = 560.0;
    const LIST_W: f32 = 400.0;
    const PANE_W: f32 = 460.0;
    const PANE_GAP: f32 = 10.0;
    const FIELD_H: f32 = 48.0;
    const CARET_H: f32 = 22.0;
    const ROW_H: f32 = 48.0;
    const ROW_GAP: f32 = 2.0;
    const ICON: f32 = 30.0;
    const GLYPH: f32 = 22.0;
    const PREVIEW_W: f32 = 56.0;
    const PREVIEW_H: f32 = 40.0;
    let field = card_surface()
        .flex_none()
        .h(FIELD_H)
        .px(16.0)
        .flex()
        .items_center()
        .gap(1.0)
        .text_size(HEADING_SIZE);
    let field = if view.query.is_empty() {
        field.child(node::caret(CARET_H)).child(
            div()
                .pl(4.0)
                .color(OVERLAY2)
                .child(text(view.placeholder.clone()).nowrap()),
        )
    } else {
        field
            .child(div().min_w(0.0).truncate().child(text(view.query.clone())))
            .child(node::caret(CARET_H))
    };
    let selected = view.rows.iter().position(|row| row.selected);
    let empty = view.rows.is_empty();
    let slot_w = if view.pictures { PREVIEW_W } else { ICON };
    let rows: Vec<Node> = view
        .rows
        .into_iter()
        .map(|row| {
            let slot = div()
                .w(slot_w)
                .h(PREVIEW_H)
                .flex()
                .items_center()
                .justify_center()
                .flex_none();
            let slot = match row.icon {
                RowIcon::Image(path) => {
                    slot.child(node::picture(crate::picture::Picture::File(path), ICON))
                }
                RowIcon::Glyph(path) => slot.child(glyph(path, GLYPH, SUBTEXT1)),
                RowIcon::Picture(path) => slot.child(
                    div()
                        .size_full()
                        .p(3.0)
                        .bg(tinted(TINT, HOVER_ALPHA), 6.0)
                        .child(node::picture(crate::picture::Picture::File(path), 0.0).size_full()),
                ),
                RowIcon::None => slot,
            };
            let node = div()
                .flex()
                .items_center()
                .flex_none()
                .h(ROW_H)
                .px(10.0)
                .justify_start()
                .gap(12.0)
                .child(slot)
                .child(
                    div()
                        .flex_1()
                        .min_w(0.0)
                        .col()
                        .child(
                            div()
                                .text_size(TITLE_SIZE)
                                .weight(SEMIBOLD)
                                .truncate()
                                .child(text(row.name)),
                        )
                        .when(!row.detail.is_empty(), |words| {
                            words.child(
                                div()
                                    .text_size(SMALL_SIZE)
                                    .color(SUBTEXT1)
                                    .truncate()
                                    .child(text(row.detail)),
                            )
                        }),
                )
                .on_click(row.on_click)
                .on_hover(row.on_hover);
            if row.selected {
                node
            } else {
                node.hover(ROW_RADIUS)
            }
        })
        .collect();
    let width = if view.preview.is_some() {
        LIST_W
    } else {
        WIDTH
    };
    let list_h = ROWS as f32 * ROW_H + (ROWS - 1) as f32 * ROW_GAP;
    let pill = selected.map(|index| {
        div()
            .absolute()
            .left(0.0)
            .right(0.0)
            .top(index as f32 * (ROW_H + ROW_GAP))
            .h(ROW_H)
            .bg(tinted(ACCENT, HOVER_ALPHA), ROW_RADIUS)
            .eases(
                Easing::Shift { x: 0.0, y: 1.0 },
                index as f32 * (ROW_H + ROW_GAP),
                SELECTION_SPRING,
            )
    });
    let nothing = empty.then(|| {
        div()
            .absolute()
            .inset(0.0)
            .flex()
            .items_center()
            .justify_center()
            .color(SUBTEXT1)
            .child(text(if view.query.is_empty() {
                view.empty.clone()
            } else {
                format!("Nothing matches “{}”", view.query).into()
            }))
    });
    let list = div()
        .flex_none()
        .w(width)
        .h(list_h)
        .on_rows(view.on_scroll)
        .children(pill)
        .child(div().col().gap(ROW_GAP).children(rows))
        .children(nothing);
    let body = match view.preview {
        Some((preview, detail)) => {
            let content = match preview {
                Preview::Text(words) => div().text_size(SMALL_SIZE).child(text(words)),
                Preview::Picture(path) => {
                    node::picture(crate::picture::Picture::File(path), 0.0).size_full()
                }
                Preview::Empty => div(),
            };
            div().flex().gap(PANE_GAP).child(list).child(
                card_surface()
                    .w(PANE_W)
                    .h(list_h)
                    .flex_none()
                    .p(CARD_PAD)
                    .col()
                    .gap(10.0)
                    .child(div().flex_1().min_h(0.0).clip().child(content))
                    .child(
                        div()
                            .flex_none()
                            .truncate()
                            .text_size(SMALL_SIZE)
                            .color(SUBTEXT1)
                            .child(text(detail)),
                    ),
            )
        }
        None => list,
    };
    div().col().gap(10.0).child(field).child(body)
}
