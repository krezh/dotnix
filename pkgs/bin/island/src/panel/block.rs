use gpui::{AnyElement, Context, SharedString, div, point, prelude::*, px};

use super::{
    frame::Frame,
    model::{Block, ButtonRow, COLUMNS, IconButton, Placed, Split},
    view::{FOLLOW_SLACK, card_title, views_row},
};
use crate::{
    components::{
        actions, button, calendar, card, chart, code, edges, input, item, ledger, lines, meter,
        overscroll::overscroll, player, prompt, route, search, segments, slider, text, tile,
        timeline, tooltip::Tip, transcript,
    },
    island::Island,
    panel::IslandUi,
    ui::CARD_PAD,
};

const COMPACT_GAP: f32 = 6.0;

const BUTTON_GAP: f32 = 8.0;

const SWITCH_GAP: f32 = 8.0;

const SWITCHES_PER_ROW: usize = 4;

/// The most a column of [`Block::Columns`] shows before it scrolls, leaving the island its header
/// and edges within its tallest.
const COLUMN_H: f32 = 600.0;

/// How far past what a scrolling [`Block::Placed`] shows its blocks are still drawn, so ones
/// scrolling in are ready.
pub const PLACED_MARGIN: f32 = 240.0;

const GRID_GAP: f32 = 10.0;

impl Block {
    /// `at` names the block's place in the panel, unique and stable across frames, for parts that
    /// keep state, such as morphing text.
    pub(super) fn render(
        self,
        at: &str,
        island_below: f32,
        frame: &Frame<'_>,
        cx: &mut Context<Island>,
    ) -> AnyElement {
        match self {
            Block::Card(title, size, blocks) => {
                let children = render_all(blocks, at, island_below, frame, cx);
                let title = title.map(|title| card_title(title, at, frame.hover, cx));
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
            Block::Columns(Split::Balanced, blocks) => {
                let cards = blocks
                    .into_iter()
                    .enumerate()
                    .map(|(index, block)| {
                        let key = block.balanced_key(index);
                        let at = format!("{at}-{index}");
                        (key, block.render(&at, island_below, frame, cx))
                    })
                    .collect();
                card::balanced(cards)
            }
            Block::Columns(Split::Scrolling, blocks) => {
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
                                let content = block.render(&at, island_below, frame, cx);
                                edges::edges(
                                    overscroll(
                                        at.clone(),
                                        div()
                                            .id(SharedString::from(at.clone()))
                                            .flex_none()
                                            .max_h(px(COLUMN_H))
                                            .overflow_y_scroll(),
                                        Some(scroll),
                                        content,
                                    ),
                                    scroll,
                                    frame.glass,
                                )
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
            Block::Search(view) => search::search(view, frame.layouts, frame.hover, cx),
            Block::Calendar(view) => calendar::calendar(view, frame.hover, cx),
            Block::Meter(view) => meter::meter(view, frame.phase, frame.tips, cx),
            Block::Player(view) => player::player(view, frame.layouts, frame.hover, cx),
            Block::Lines(entries) => lines::lines(entries),
            Block::Routes(entries) => route::routes(entries),
            Block::Board(columns) => tile::board(columns, frame.phase, frame.hover, frame.tips, cx),
            Block::Buttons(row) => buttons(row, frame, cx),
            Block::Code(lines) => code::code(at, lines, frame.hover, cx),
            Block::Input(view) => input::input(view, frame.hover, cx),
            Block::Transcript(turns) => {
                transcript::transcript(turns, frame.layouts, frame.hover, cx)
            }
            Block::Facts(items) => tile::colored_facts(items, at).into_any_element(),
            Block::Hero(name, lines) => tile::hero(name, lines).into_any_element(),
            Block::Timeline(view) => timeline::timeline(view, at, frame.tips),
            Block::Ledger(view) => {
                ledger::ledger(view, at, frame.placed, frame.glass, frame.hover, cx)
            }
            Block::Prompt(view) => prompt::prompt(view, frame.hover, cx),
            Block::Views(views) => views_row(views, frame, cx),
            Block::Actions(tiles) => div()
                .flex()
                .gap(px(card::GAP))
                .children(tiles.into_iter().map(|tile| {
                    let handler = tile.on_click;
                    button::action(
                        tile.id,
                        tile.glyph.as_deref(),
                        tile.label,
                        tile.state,
                        tile.color,
                        tile.armed,
                        frame.hover,
                    )
                    .on_click(
                        cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))),
                    )
                }))
                .into_any_element(),
            Block::Switches(tiles) => {
                let mut tiles = tiles.into_iter().peekable();
                let mut rows = Vec::new();
                while tiles.peek().is_some() {
                    let row = (0..SWITCHES_PER_ROW).map(|_| {
                        let cell = div().flex_1().min_w(px(0.0)).flex().flex_col();
                        match tiles.next() {
                            Some(tile) => {
                                let handler = tile.on_click;
                                cell.child(
                                    button::action(
                                        tile.id,
                                        tile.glyph.as_deref(),
                                        tile.label,
                                        tile.state,
                                        tile.color,
                                        tile.armed,
                                        frame.hover,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| handler(&mut IslandUi::new(this, cx)),
                                    )),
                                )
                            }
                            None => cell,
                        }
                    });
                    rows.push(div().flex().gap(px(SWITCH_GAP)).children(row));
                }
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SWITCH_GAP))
                    .children(rows)
                    .into_any_element()
            }
            Block::Warnings(warnings) => div()
                .flex()
                .flex_wrap()
                .gap(px(8.0))
                .children(warnings.into_iter().map(|warning| {
                    let clickable = frame.hover && warning.on_click.is_some();
                    let chip = button::tinted_chip(
                        warning.id,
                        warning.glyph.as_deref(),
                        warning.label,
                        warning.value,
                        warning.color,
                        clickable,
                    );
                    match warning.on_click {
                        Some(handler) => chip
                            .on_click(cx.listener(move |this, _, _, cx| {
                                handler(&mut IslandUi::new(this, cx))
                            }))
                            .into_any_element(),
                        None => chip.cursor_default().into_any_element(),
                    }
                }))
                .into_any_element(),
            Block::Meters(columns, meters) => {
                meter::meters(columns, meters, frame.phase, frame.tips, cx)
            }
            Block::Chart(view) => div()
                .h(px(view.height))
                .child(chart::chart(
                    view.id,
                    view.series,
                    card::SLOTS,
                    view.phase.map_or(frame.phase, chart::Phase::now),
                    frame.tips,
                ))
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
                    .on_click(
                        cx.listener(move |this, _, _, cx| handler(&mut IslandUi::new(this, cx))),
                    )
                }))
                .into_any_element(),
        }
    }
}

/// Clicks stay with the button, so one inside something clickable, such as a message, does only
/// its own thing.
pub(super) fn buttons(row: ButtonRow, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
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
        .when(row.end, |row| row.justify_end())
        .ml(px(row.indent))
        .gap(px(gap))
        .children(row.buttons.into_iter().map(|button| {
            let badge = button.badge.clone();
            let id = button.id.clone();
            let element = icon_button(button, row.compact, frame, cx);
            if row.compact {
                element.into_any_element()
            } else {
                let badge = badge.map(|text| button::Badge {
                    text,
                    color: crate::theme::ACCENT,
                });
                button::badged(format!("{id}-badged"), element, badge, true).into_any_element()
            }
        }))
        .when_some(menu, |row, menu| row.child(div().flex_1()).child(menu))
        .into_any_element()
}

/// Its glyph named by its label as the tooltip, or its label without one; the click stays with it.
fn icon_button(
    button: IconButton,
    compact: bool,
    frame: &Frame<'_>,
    cx: &mut Context<Island>,
) -> gpui::Stateful<gpui::Div> {
    let handler = button.on_click;
    let click = cx.listener(move |this, _, _, cx| {
        cx.stop_propagation();
        handler(&mut IslandUi::new(this, cx));
    });
    let has_glyph = button.glyph.is_some();
    let tip_id = button.id.clone();
    let tip_label = button.label.clone();
    let mut spec = button::Button::new(button.id)
        .glyph(button.glyph.as_deref())
        .color(button.color)
        .compact(compact)
        .active(button.active);
    if !has_glyph {
        spec = spec.label(button.label);
    }
    button::button(spec, frame.hover)
        .when(has_glyph && !tip_label.is_empty(), |el| {
            el.tip(frame.tips, tip_id, tip_label)
        })
        .on_click(click)
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
    if let Some(rewind) = placed.rewind {
        let offset = frame.placed.offset();
        if -f32::from(offset.y) > rewind {
            frame
                .placed
                .set_offset(point(offset.x, px(-rewind.max(0.0))));
        }
    }
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
            let element = block.render(&format!("{at}-{index}"), island_below, frame, cx);
            let content = match place.drawn {
                Some(drawn) => div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(drawn))
                    .flex()
                    .child(element),
                None => div().w_full().child(element),
            };
            Some(
                div()
                    .absolute()
                    .left(px(place.inset + place.offset))
                    .right(px(place.inset - place.offset))
                    .top(px(top))
                    .h(px(place.height))
                    .opacity(place.opacity.clamp(0.0, 1.0))
                    .when(place.drawn.is_some(), |slot| slot.overflow_hidden())
                    .flex()
                    .items_center()
                    .child(content),
            )
        });
    let content = div()
        .relative()
        .min_w(px(placed.width))
        .h(px(placed.height))
        .children(blocks);
    match placed.visible {
        Some(visible) => {
            let key = SharedString::from(format!("{at}-scroll"));
            edges::edges_within(
                overscroll(
                    key.clone(),
                    div()
                        .id(key)
                        .min_w(px(placed.width))
                        .h(px(visible))
                        .overflow_y_scroll(),
                    Some(frame.placed),
                    content,
                ),
                frame.placed,
                frame.glass,
                placed.still - visible,
            )
        }
        None => content.into_any_element(),
    }
}

pub(super) fn render_all(
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
