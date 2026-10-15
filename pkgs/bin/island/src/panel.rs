use std::{
    hash::{DefaultHasher, Hash, Hasher},
    mem::discriminant,
    path::PathBuf,
    rc::Rc,
    time::Duration as StdDuration,
};

use chrono::{Datelike, Duration, Local, Months, NaiveDate};
use gpui::{
    Animation, AnimationExt, AnyElement, Context, FontWeight, ScrollHandle, SharedString, SpringAnimation,
    SpringConfig, div, prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        button,
        card,
        icon,
        marquee::{TextLayouts, marquee},
        slider::{OnChange, slider},
        tile::{self, Tile},
        tooltip::{Tip, Tooltips},
    },
    island::{Island, Mode},
    launcher,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, CONTROL_ALPHA, LAVENDER, ON_ACCENT, OVERLAY2, PEACH, SUBTEXT1, SURFACE1, TEXT, with_alpha,
    },
    ui::{
        self, BLOCK_GAP, CARD_PAD, Radius, HEADER_GAP, HEADER_H, HEADING_SIZE, INSET, SMALL_SIZE, TEXT_SIZE, TIGHT_INSET,
        TITLE_SIZE,
    },
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
    pub on_click: Handler,
}

pub enum Header {
    Tabs,
    Title(SharedString, Vec<Button>),
    None,
}

pub struct Entry {
    pub id: SharedString,
    pub label: SharedString,
    pub enabled: bool,
    pub separator: bool,
    pub check: Option<bool>,
    pub submenu: bool,
    pub on_click: Option<Handler>,
}

pub struct Level {
    pub id: SharedString,
    pub label: SharedString,
    pub detail: SharedString,
    pub glyph: Option<PathBuf>,
    pub value: f32,
    pub muted: bool,
    pub color: u32,
    pub on_change: Option<OnChange>,
    pub on_mute: Option<Handler>,
}

pub struct Meter {
    pub glyph: Option<PathBuf>,
    pub caption: SharedString,
    pub label: SharedString,
    pub switched: bool,
    pub value: f32,
    pub muted: bool,
    pub color: u32,
}

pub struct MediaView {
    pub art: Option<PathBuf>,
    pub title: SharedString,
    pub artist: SharedString,
    pub album: SharedString,
    pub source: SharedString,
    pub position: f64,
    pub length: Option<f64>,
    pub play: Option<PathBuf>,
    pub previous: Option<PathBuf>,
    pub next: Option<PathBuf>,
    pub on_play: Handler,
    pub on_previous: Handler,
    pub on_next: Handler,
    /// Seeks to a fraction of the track; `None` when the track has no length.
    pub on_seek: Option<OnChange>,
}

pub struct AppRow {
    pub id: SharedString,
    pub position: usize,
    pub name: SharedString,
    pub detail: SharedString,
    pub icon: Option<PathBuf>,
    pub selected: bool,
}

pub struct IconButton {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    /// Shown as the tooltip.
    pub label: SharedString,
    pub active: bool,
    pub badge: Option<SharedString>,
    pub on_click: Handler,
}

/// A month grid with buttons to page through months and back to today.
pub struct CalendarView {
    /// Months away from the current one.
    pub offset: i32,
    pub on_previous: Handler,
    pub on_next: Handler,
    pub on_today: Handler,
}

pub struct CommitRow {
    pub summary: SharedString,
    pub detail: SharedString,
}

pub struct LauncherView {
    pub query: SharedString,
    pub rows: Vec<AppRow>,
}

pub enum Block<'a> {
    Card(Option<SharedString>, card::Size, Vec<Block<'a>>),
    Row(Vec<Block<'a>>),
    Launcher(LauncherView),
    Calendar(CalendarView),
    Menu(Vec<Entry>),
    Levels(Vec<Level>),
    Meter(Meter),
    Choices(SharedString, Vec<Entry>),
    Media(MediaView),
    Note(SharedString),
    /// Commits, newest first.
    Commits(Vec<CommitRow>),
    /// Columns of fixed-size tiles; see `components::tile`.
    Board(Vec<Vec<Tile>>),
    /// A row of square icon buttons.
    Icons(Vec<IconButton>),
    /// A block its feature draws itself, for content no shared block covers such as notification
    /// cards. The key stands in for its size: the panel is measured again when it changes.
    Custom(u64, Draw<'a>),
}

/// Draws a [`Block::Custom`], given how far the island reaches below the top of the block.
pub type Draw<'a> = Box<dyn FnOnce(&Frame<'_>, f32, &mut Context<Island>) -> AnyElement + 'a>;

pub struct PanelView<'a> {
    pub header: Header,
    pub blocks: Vec<Block<'a>>,
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
}

const DAY_H: f32 = 40.0;
const TODAY: f32 = 36.0;
const DAY_GAP: f32 = 4.0;
const ENTRY_H: f32 = 28.0;

impl Block<'_> {
    fn hash_shape(&self, hasher: &mut DefaultHasher) {
        discriminant(self).hash(hasher);
        match self {
            Block::Calendar(_) | Block::Note(_) | Block::Meter(_) => {}
            Block::Levels(levels) => levels.len().hash(hasher),
            Block::Commits(rows) => rows.len().hash(hasher),
            Block::Board(columns) => {
                for column in columns {
                    column.iter().map(|tile| tile.span).collect::<Vec<_>>().hash(hasher);
                }
            }
            Block::Icons(buttons) => buttons.len().hash(hasher),
            Block::Launcher(_) => launcher::ROWS.hash(hasher),
            Block::Media(view) => (view.art.is_some(), view.length.is_some()).hash(hasher),
            Block::Menu(entries) | Block::Choices(_, entries) => {
                for entry in entries {
                    entry.label.hash(hasher);
                    (entry.separator, entry.check.is_some(), entry.submenu).hash(hasher);
                }
            }
            Block::Custom(shape, _) => shape.hash(hasher),
            Block::Card(title, size, blocks) => {
                (title.is_some(), size).hash(hasher);
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
            Block::Row(blocks) => {
                for block in blocks {
                    block.hash_shape(hasher);
                }
            }
        }
    }
}

impl PanelView<'_> {
    pub fn key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        discriminant(&self.header).hash(&mut hasher);
        if let Header::Title(title, buttons) = &self.header {
            title.hash(&mut hasher);
            buttons.len().hash(&mut hasher);
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
            Header::Tabs => Some(div().h(px(HEADER_H)).min_w(px(crate::bar::tabs_width())).into_any_element()),
            Header::Title(title, buttons) => Some(title_header(title, buttons, frame.hover, cx)),
            Header::None => None,
        };
        let island_below = frame.island_h - inset;
        let blocks: Vec<AnyElement> = self
            .blocks
            .into_iter()
            .map(|block| block.render(island_below, frame, cx))
            .collect();
        ui::base()
            .flex()
            .flex_col()
            .min_w(px(frame.min_width))
            .max_w(px(frame.max_width))
            .max_h(px(frame.max_height))
            .p(px(inset))
            .when_some(header, |panel, header| panel.child(header).child(div().h(px(HEADER_GAP)).flex_none()))
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

fn title_header(title: SharedString, buttons: Vec<Button>, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    div()
        .h(px(HEADER_H))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .text_size(px(HEADING_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .children(buttons.into_iter().map(|button| button_element(button, hover, cx)))
                .child(
                    button::close("panel-close", hover)
                        .on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Collapsed, cx))),
                ),
        )
        .into_any_element()
}

fn button_element(button: Button, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let handler = button.on_click;
    if button.toggle {
        button::toggle(button.id, button.label, button.active, hover)
    } else {
        button::button(button.id, button.label, button.active, hover)
    }
        .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
        .into_any_element()
}

impl Block<'_> {
    fn render(self, island_below: f32, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
        match self {
            Block::Card(title, size, blocks) => {
                let children: Vec<AnyElement> = blocks
                    .into_iter()
                    .map(|block| block.render(island_below, frame, cx))
                    .collect();
                card::surface()
                    .w(px(size.width()))
                    .flex_none()
                    .overflow_hidden()
                    .p(px(CARD_PAD))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .when_some(title, |group, title| {
                        group.child(div().text_size(px(TITLE_SIZE)).font_weight(FontWeight::SEMIBOLD).child(title))
                    })
                    .children(children)
                    .into_any_element()
            }
            Block::Row(blocks) => {
                let children: Vec<AnyElement> = blocks
                    .into_iter()
                    .map(|block| block.render(island_below, frame, cx))
                    .collect();
                div().flex().gap(px(card::GAP)).children(children).into_any_element()
            }
            Block::Calendar(view) => calendar(view, frame.hover, cx),
            Block::Menu(entries) => {
                let checks = entries.iter().any(|entry| entry.check.is_some());
                div()
                    .flex()
                    .flex_col()
                    .children(entries.into_iter().map(|entry| entry_element(entry, checks, frame.hover, cx)))
                    .into_any_element()
            }
            Block::Levels(levels) => div()
                .flex()
                .flex_col()
                .gap(px(14.0))
                .children(levels.into_iter().map(|level| level_element(level, frame.hover, cx)))
                .into_any_element(),
            Block::Choices(title, entries) => {
                let checks = entries.iter().any(|entry| entry.check.is_some());
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .pb(px(6.0))
                            .text_size(px(SMALL_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .children(entries.into_iter().map(|entry| entry_element(entry, checks, frame.hover, cx)))
                    .into_any_element()
            }
            Block::Media(view) => media_element(view, frame, cx),
            Block::Meter(meter) => meter_element(meter, cx),
            Block::Launcher(view) => launcher_element(view, frame.hover, cx),
            Block::Note(text) => div()
                .py(px(14.0))
                .flex()
                .justify_center()
                .text_size(px(SMALL_SIZE))
                .text_color(rgb(SUBTEXT1))
                .child(text)
                .into_any_element(),
            Block::Commits(rows) => commits(rows),
            Block::Board(columns) => tile::board(columns, frame.phase),
            Block::Icons(buttons) => div()
                .flex()
                .gap(px(8.0))
                .children(buttons.into_iter().map(|button| {
                    let handler = button.on_click;
                    let key = button.id.clone();
                    button::icon(button.id, button.glyph.as_deref(), button.active, button.badge, frame.hover)
                        .tip(frame.tips, key, button.label)
                        .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
                }))
                .into_any_element(),
            Block::Custom(_, draw) => draw(frame, island_below, cx),
        }
    }
}

fn commits(rows: Vec<CommitRow>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .py(px(8.0))
                .when(index > 0, |row| row.border_t_1().border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))))
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(SMALL_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(row.summary),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(CAPTION_SIZE))
                        .text_color(rgb(SUBTEXT1))
                        .child(row.detail),
                )
        }))
        .into_any_element()
}

const CAPTION_SIZE: f32 = 12.0;

fn level_element(level: Level, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let percent = (level.value * 100.0).round() as i32;
    let glyph_color = if level.muted { OVERLAY2 } else { level.color };
    let mute = button::ghost(SharedString::from(format!("{}-mute", level.id)), hover && level.on_mute.is_some())
        .size(px(HEADER_H))
        .flex_none()
        .when_some(level.glyph.as_deref(), |mute, path| mute.child(icon::glyph(path, 18.0, glyph_color)))
        .when_some(level.on_mute, |mute, handler| {
            mute.on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
        });
    let slider = slider(level.id.clone(), if level.muted { 0.0 } else { level.value }, level.color, level.on_change, cx);
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(mute)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .gap(px(12.0))
                        .text_size(px(SMALL_SIZE))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .flex()
                                .gap(px(6.0))
                                .child(div().min_w(px(0.0)).truncate().font_weight(FontWeight::SEMIBOLD).child(level.label))
                                .when(!level.detail.is_empty(), |label| {
                                    label.child(div().min_w(px(0.0)).truncate().child(level.detail))
                                }),
                        )
                        .child(
                            div()
                                .flex_none()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(glyph_color))
                                .child(if level.muted { "Muted".to_owned() } else { format!("{percent}%") }),
                        ),
                )
                .child(slider),
        )
        .into_any_element()
}

const LAUNCHER_W: f32 = 560.0;
const SEARCH_H: f32 = 48.0;
const CARET_H: f32 = 22.0;
const APP_ROW_H: f32 = 48.0;
const APP_ROW_GAP: f32 = 2.0;
const APP_ICON: f32 = 30.0;
const SELECTION_SPRING: SpringConfig = SpringConfig::new(560.0, 44.0, 1.0);

/// A text caret that breathes rather than blinks: fully lit most of the cycle, easing out and back.
fn caret() -> impl IntoElement {
    div()
        .flex_none()
        .w(px(2.0))
        .h(px(CARET_H))
        .rounded_full()
        .bg(rgb(ACCENT))
        .with_animation(
            "launcher-caret",
            Animation::new(StdDuration::from_millis(1100)).repeat(),
            |caret, t| caret.opacity(((t * std::f32::consts::TAU).cos() * 1.6).clamp(-1.0, 1.0) * 0.5 + 0.5),
        )
}

/// The launcher is always the same size: a search field over `launcher::ROWS` row slots. Typing
/// only changes what fills the slots, so the island never reshapes mid-search, and the selection
/// glides between rows on a spring instead of jumping.
fn launcher_element(view: LauncherView, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let search = card::surface()
        .flex_none()
        .h(px(SEARCH_H))
        .px(px(16.0))
        .flex()
        .items_center()
        .gap(px(1.0))
        .text_size(px(HEADING_SIZE))
        .map(|field| {
            if view.query.is_empty() {
                field.child(caret()).child(div().pl(px(4.0)).text_color(rgb(OVERLAY2)).child("Search apps"))
            } else {
                field
                    .child(div().min_w(px(0.0)).truncate().child(view.query.clone()))
                    .child(caret())
            }
        });

    let selected = view.rows.iter().position(|row| row.selected);
    let empty = view.rows.is_empty();
    let rows = view.rows.into_iter().map(|row| {
        let position = row.position;
        let over = (hover && !row.selected).then(|| ui::hover_fill().into());
        button::fill(row.id, Radius::Row, None, over)
            .flex_none()
            .h(px(APP_ROW_H))
            .px(px(10.0))
            .justify_start()
            .gap(px(12.0))
            .child(
                div()
                    .size(px(APP_ICON))
                    .flex_none()
                    .when_some(row.icon.as_deref(), |slot, path| slot.child(icon::image(path, APP_ICON))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(div().text_size(px(TITLE_SIZE)).font_weight(FontWeight::SEMIBOLD).truncate().child(row.name))
                    .when(!row.detail.is_empty(), |text| {
                        text.child(div().text_size(px(SMALL_SIZE)).text_color(rgb(SUBTEXT1)).truncate().child(row.detail))
                    }),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.launch_app(position, cx)))
    });

    let list_h = launcher::ROWS as f32 * APP_ROW_H + (launcher::ROWS - 1) as f32 * APP_ROW_GAP;
    let list = div()
        .relative()
        .flex_none()
        .h(px(list_h))
        .when_some(selected, |list, index| {
            let top = px(index as f32 * (APP_ROW_H + APP_ROW_GAP));
            list.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .h(px(APP_ROW_H))
                    .child(ui::shape(Radius::Row, Some(ui::selected_fill().into()), None))
                    .with_spring("launcher-selection", SpringAnimation::new(SELECTION_SPRING).to(top), |pill, top| {
                        pill.top(top)
                    }),
            )
        })
        .child(div().relative().flex().flex_col().gap(px(APP_ROW_GAP)).children(rows))
        .when(empty, |list| {
            list.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(SUBTEXT1))
                    .child(if view.query.is_empty() {
                        "No apps found".into()
                    } else {
                        SharedString::from(format!("No apps match “{}”", view.query))
                    }),
            )
        });

    div()
        .w(px(LAUNCHER_W))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(search)
        .child(list)
        .into_any_element()
}

const METER_W: f32 = 360.0;
const METER_GLYPH: f32 = 22.0;
const METER_TILE: f32 = 40.0;

fn meter_element(meter: Meter, cx: &mut Context<Island>) -> AnyElement {
    let tint = if meter.muted { OVERLAY2 } else { meter.color };
    let percent = (meter.value * 100.0).round() as i32;
    let tile = div()
        .relative()
        .size(px(METER_TILE))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(ui::shape(Radius::Control, Some(rgba(with_alpha(tint, ACCENT_TINT_ALPHA)).into()), None))
        .when_some(meter.glyph.as_deref(), |slot, path| slot.child(icon::glyph(path, METER_GLYPH, tint)));
    let heading = div()
        .flex()
        .items_baseline()
        .gap(px(8.0))
        .min_w(px(0.0))
        .when(!meter.caption.is_empty(), |line| {
            line.child(
                div()
                    .flex_none()
                    .text_size(px(SMALL_SIZE))
                    .text_color(rgb(SUBTEXT1))
                    .child(meter.caption.clone()),
            )
        })
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .font_weight(FontWeight::SEMIBOLD)
                .when(meter.switched, |label| label.text_color(rgb(meter.color)))
                .child(meter.label.clone()),
        );
    let value = div()
        .flex_none()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if meter.muted { OVERLAY2 } else { TEXT }))
        .child(if meter.muted { "Muted".to_owned() } else { format!("{percent}%") });
    div()
        .w(px(METER_W))
        .px(px(4.0))
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(tile)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div().flex().items_baseline().justify_between().gap(px(12.0)).child(heading).child(value))
                .child(slider("osd-meter", if meter.muted { 0.0 } else { meter.value }, tint, None, cx)),
        )
        .into_any_element()
}

fn clock_text(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn media_element(view: MediaView, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
    const ART: f32 = 112.0;
    const ART_GAP: f32 = 16.0;
    let hover = frame.hover;
    let text_w = card::Size::Large.width() - 2.0 * CARD_PAD - ART - ART_GAP;
    let line = |text: SharedString, size: f32, weight: FontWeight| marquee(frame.layouts, text, size, weight, text_w);
    let art = div()
        .size(px(ART))
        .flex_none()
        .rounded(px(ui::PANEL_RADIUS))
        .overflow_hidden()
        .bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)))
        .when_some(view.art.clone(), |art, path| {
            art.child(gpui::img(path).size_full().object_fit(gpui::ObjectFit::Cover))
        });
    let control = |id: &'static str, glyph: Option<PathBuf>, size: f32, primary: bool, action: Handler, cx: &mut Context<Island>| {
        let (rest, over, tint) = if primary {
            (Some(rgb(ACCENT).into()), hover.then(|| rgb(LAVENDER).into()), ON_ACCENT)
        } else {
            (None, hover.then(|| ui::hover_fill().into()), TEXT)
        };
        button::fill(id, Radius::Pill, rest, over)
            .size(px(size))
            .flex_none()
            .when_some(glyph, |control, path| control.child(icon::glyph(&path, size * 0.45, tint)))
            .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
    };
    let progress = view.length.map(|length| {
        let fraction = (view.position / length.max(1.0)) as f32;
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(slider("media-progress", fraction, ACCENT, view.on_seek.clone(), cx))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(px(SMALL_SIZE))
                    .text_color(rgb(SUBTEXT1))
                    .child(clock_text(view.position))
                    .child(clock_text(length)),
            )
    });
    div()
        .flex()
        .gap(px(ART_GAP))
        .child(art)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(div().text_size(px(SMALL_SIZE)).text_color(rgb(SUBTEXT1)).child(view.source))
                        .child(line(view.title, TITLE_SIZE, FontWeight::SEMIBOLD))
                        .child(line(view.artist, TEXT_SIZE, FontWeight::MEDIUM))
                        .when(!view.album.is_empty(), |text| text.child(line(view.album, SMALL_SIZE, FontWeight::MEDIUM))),
                )
                .children(progress)
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .items_center()
                        .gap(px(12.0))
                        .child(control("media-previous", view.previous, 36.0, false, view.on_previous, cx))
                        .child(control("media-play", view.play, 44.0, true, view.on_play, cx))
                        .child(control("media-next", view.next, 36.0, false, view.on_next, cx)),
                ),
        )
        .into_any_element()
}

fn entry_element(entry: Entry, checks: bool, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    if entry.separator {
        return div()
            .py(px(4.0))
            .child(div().h(px(1.0)).bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))))
            .into_any_element();
    }
    let enabled = entry.enabled && entry.on_click.is_some();
    let mark = entry.check.map(|on| if on { "✓" } else { "" });
    button::ghost(entry.id, hover && enabled)
        .h(px(ENTRY_H))
        .px(px(8.0))
        .justify_start()
        .gap(px(8.0))
        .text_size(px(SMALL_SIZE))
        .text_color(rgb(if enabled { TEXT } else { OVERLAY2 }))
        .when(!enabled, |row| row.cursor_default())
        .when(checks, |row| {
            row.child(
                div()
                    .w(px(14.0))
                    .flex_none()
                    .text_color(rgb(ACCENT))
                    .child(mark.unwrap_or_default()),
            )
        })
        .child(div().flex_1().min_w(px(0.0)).truncate().child(entry.label))
        .when(entry.submenu, |row| row.child(div().text_color(rgb(SUBTEXT1)).child("›")))
        .when_some(entry.on_click.filter(|_| entry.enabled), |row, handler| {
            row.on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
        })
        .into_any_element()
}

fn calendar(view: CalendarView, hover: bool, cx: &mut Context<Island>) -> AnyElement {
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
    let weekend = |weekday: i64| weekday >= 5;

    let nav = |id: &'static str, glyph: &'static str, handler: Handler, cx: &mut Context<Island>| {
        button::ghost(id, hover)
            .size(px(HEADER_H))
            .text_size(px(HEADING_SIZE))
            .child(glyph)
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
    };
    let on_today = view.on_today;
    let back = (offset != 0).then(|| {
        button::ghost("calendar-today", hover)
            .h(px(HEADER_H))
            .px(px(12.0))
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(ACCENT))
            .child("Today")
            .on_click(cx.listener(move |this, _, _, cx| on_today(this, cx)))
    });

    let weekdays = div().flex().children(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().enumerate().map(
        |(index, day)| {
            div()
                .flex_1()
                .flex()
                .justify_center()
                .text_size(px(SMALL_SIZE))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if weekend(index as i64) { PEACH } else { TEXT }))
                .child(*day)
        },
    ));

    let weeks = (0..6).map(|week| {
        div().flex().gap(px(DAY_GAP)).children((0..7).map(|weekday| {
            let date = start + Duration::days(week * 7 + weekday);
            let in_month = date.month() == first.month();
            let is_today = date == today;
            div()
                .flex_1()
                .min_w(px(DAY_H))
                .h(px(DAY_H))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(match (is_today, in_month) {
                    (true, _) => ON_ACCENT,
                    (false, true) => TEXT,
                    (false, false) => OVERLAY2,
                }))
                .child(
                    div()
                        .relative()
                        .size(px(TODAY))
                        .flex()
                        .items_center()
                        .justify_center()
                        .when(is_today, |cell| {
                            cell.child(ui::shape(Radius::Control, Some(rgb(ACCENT).into()), None))
                                .font_weight(FontWeight::BOLD)
                        })
                        .child(date.day().to_string()),
                )
        }))
    });

    div()
        .flex()
        .flex_col()
        .gap(px(DAY_GAP))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb(px(6.0))
                .child(
                    div()
                        .pl(px(8.0))
                        .text_size(px(HEADING_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(first.format("%B %Y").to_string()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.0))
                        .children(back)
                        .child(nav("calendar-prev", "‹", view.on_previous.clone(), cx))
                        .child(nav("calendar-next", "›", view.on_next.clone(), cx)),
                ),
        )
        .child(div().pb(px(4.0)).child(weekdays))
        .children(weeks)
        .into_any_element()
}
