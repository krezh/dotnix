use std::{
    cell::Cell,
    hash::{DefaultHasher, Hash, Hasher},
    mem::discriminant,
    path::PathBuf,
    rc::Rc,
    time::Duration as StdDuration,
};

use chrono::{Datelike, Duration, Local, Months, NaiveDate};
use gpui::{
    Animation, AnimationExt, AnyElement, Context, Div, FontWeight, ScrollHandle, ScrollWheelEvent, SharedString,
    SpringAnimation, SpringConfig, div, prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        button,
        card,
        icon,
        marquee::{TextLayouts, marquee},
        motion,
        slider::{OnChange, slider},
        tabs,
        tile::{self, Tile},
        tooltip::{Tip, Tooltips},
    },
    island::{Island, Mode},
    launcher,
    theme::{
        ACCENT, ACCENT_TINT_ALPHA, CONTROL_ALPHA, GREEN, ON_ACCENT, OVERLAY2, PEACH, SUBTEXT1, SURFACE1, TEXT, YELLOW,
        with_alpha,
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
    /// Every label the button can show, when it changes label, so it keeps one width.
    pub fits: Vec<SharedString>,
    /// What the button started is still going, such as a check; the energy border runs round it.
    pub busy: bool,
    /// It can be clicked. A button that sometimes has nothing to do stays in place, drawn as
    /// inactive, so the header keeps its size.
    pub enabled: bool,
    pub on_click: Handler,
}

pub enum Header {
    Tabs,
    Title(SharedString, Vec<Button>),
    /// Tabs between the panel's views, in the title's place.
    Views(Views),
    None,
}

/// A panel's views as tabs, with the buttons beside them.
pub struct Views {
    pub tabs: Vec<SharedString>,
    pub selected: usize,
    pub on_select: tabs::Select,
    pub buttons: Vec<Button>,
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
    /// The level and whether it is muted; `None` shows only what the meter is of, such as a device
    /// just switched to.
    pub level: Option<(f32, bool)>,
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
    pub name: SharedString,
    pub detail: SharedString,
    pub icon: Option<PathBuf>,
    /// The icon is a picture to preview, drawn wider than an app icon.
    pub preview: bool,
    pub selected: bool,
    pub on_click: Handler,
    /// The pointer came over the row, which selects it.
    pub on_hover: Handler,
}

/// Moves a list's selection by this many rows.
pub type Step = Rc<dyn Fn(i32, &mut Island, &mut Context<Island>)>;

/// What a list's preview pane shows of the selected row.
pub enum Preview {
    /// Text, wrapped and cut off at the bottom.
    Text(SharedString),
    /// A picture, fitted.
    Picture(PathBuf),
    Empty,
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

/// A large tile with a glyph over its label; see [`button::action`].
pub struct ActionTile {
    pub id: SharedString,
    pub glyph: Option<PathBuf>,
    pub label: SharedString,
    /// Its fill while armed.
    pub color: u32,
    pub armed: bool,
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

/// Where something stands at one step of a path, such as a pull request at one branch.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum StepState {
    Reached,
    /// On its way, with how far it has to go, such as "+63".
    Pending(Option<SharedString>),
    NotYet,
}

pub struct PathStep {
    pub label: SharedString,
    pub state: StepState,
}

/// Something moving through a fixed path of steps, such as a pull request through the branches
/// it lands in, with a mark when it has reached what matters.
pub struct PathRow {
    pub title: SharedString,
    pub detail: SharedString,
    /// Shown as a mark beside the title, such as "Ready".
    pub done: Option<SharedString>,
    pub steps: Vec<PathStep>,
}

/// A search field over a fixed number of rows, for the launcher and other panels that take the
/// keyboard.
pub struct LauncherView {
    pub query: SharedString,
    /// Shown in the empty search field.
    pub placeholder: SharedString,
    /// Shown when there are no rows and nothing is typed.
    pub empty: SharedString,
    pub rows: Vec<AppRow>,
    /// The scroll wheel over the list.
    pub on_step: Step,
    /// A pane beside the list showing the selected row, with a line about it.
    pub preview: Option<(Preview, SharedString)>,
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
    /// Things moving through their steps; see [`PathRow`].
    Paths(Vec<PathRow>),
    /// Columns of fixed-size tiles; see `components::tile`.
    Board(Vec<Vec<Tile>>),
    /// A row of square icon buttons.
    Icons(Vec<IconButton>),
    /// A row of large action tiles.
    Actions(Vec<ActionTile>),
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
            Block::Calendar(_) | Block::Note(_) => {}
            Block::Meter(meter) => meter.level.is_some().hash(hasher),
            Block::Levels(levels) => levels.len().hash(hasher),
            Block::Commits(rows) => rows.len().hash(hasher),
            Block::Paths(rows) => {
                for row in rows {
                    row.done.is_some().hash(hasher);
                    for step in &row.steps {
                        (&step.label, &step.state).hash(hasher);
                    }
                }
            }
            Block::Board(columns) => {
                for column in columns {
                    column.iter().map(|tile| tile.span).collect::<Vec<_>>().hash(hasher);
                }
            }
            Block::Icons(buttons) => buttons.len().hash(hasher),
            Block::Actions(tiles) => tiles.len().hash(hasher),
            Block::Launcher(view) => (launcher::ROWS, view.preview.is_some()).hash(hasher),
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
            Header::Tabs => Some(div().h(px(HEADER_H)).min_w(px(crate::bar::tabs_width())).into_any_element()),
            Header::Title(title, buttons) => Some(title_header(title, buttons, frame.hover, cx)),
            Header::Views(views) => Some(views_header(views, frame.hover, cx)),
            Header::None => None,
        };
        let island_below = frame.island_h - inset;
        let blocks: Vec<AnyElement> = self
            .blocks
            .into_iter()
            .enumerate()
            .map(|(index, block)| block.render(&format!("block-{index}"), island_below, frame, cx))
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
    let title = div()
        .min_w(px(0.0))
        .truncate()
        .text_size(px(HEADING_SIZE))
        .font_weight(FontWeight::SEMIBOLD)
        .child(motion::text("panel-title", title));
    header_row(title.into_any_element(), buttons, hover, cx)
}

fn views_header(views: Views, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let tabs = tabs::tabs("panel-views", views.tabs, views.selected, views.on_select, hover, cx);
    header_row(tabs.into_any_element(), views.buttons, hover, cx)
}

/// A header: `leading`, such as the title, then the buttons and the close button.
fn header_row(leading: AnyElement, buttons: Vec<Button>, hover: bool, cx: &mut Context<Island>) -> AnyElement {
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
    let hover = hover && button.enabled;
    let element = if button.toggle {
        button::toggle(button.id, button.label, button.active, hover)
    } else {
        button::busy(button.id, button.label, button.fits, button.active, button.busy, hover)
    };
    if button.enabled {
        element.on_click(cx.listener(move |this, _, _, cx| handler(this, cx))).into_any_element()
    } else {
        element.text_color(rgb(OVERLAY2)).cursor_default().into_any_element()
    }
}

impl Block<'_> {
    /// Draws the block; `at` names its place in the panel, unique and the same from frame to frame,
    /// for the parts of it that keep state, such as morphing text.
    fn render(self, at: &str, island_below: f32, frame: &Frame<'_>, cx: &mut Context<Island>) -> AnyElement {
        match self {
            Block::Card(title, size, blocks) => {
                let children: Vec<AnyElement> = blocks
                    .into_iter()
                    .enumerate()
                    .map(|(index, block)| block.render(&format!("{at}-{index}"), island_below, frame, cx))
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
                        group.child(
                            div()
                                .text_size(px(TITLE_SIZE))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(motion::text(SharedString::from(format!("{at}-title")), title)),
                        )
                    })
                    .children(children)
                    .into_any_element()
            }
            Block::Row(blocks) => {
                let children: Vec<AnyElement> = blocks
                    .into_iter()
                    .enumerate()
                    .map(|(index, block)| block.render(&format!("{at}-{index}"), island_below, frame, cx))
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
            Block::Meter(meter) => meter_element(meter, at, cx),
            Block::Launcher(view) => launcher_element(view, frame.hover, cx),
            Block::Note(text) => div()
                .py(px(14.0))
                .flex()
                .justify_center()
                .text_size(px(SMALL_SIZE))
                .text_color(rgb(SUBTEXT1))
                .child(motion::text(SharedString::from(format!("{at}-note")), text))
                .into_any_element(),
            Block::Commits(rows) => commits(rows),
            Block::Paths(rows) => paths(rows),
            Block::Board(columns) => tile::board(columns, frame.phase),
            Block::Icons(buttons) => div()
                .flex()
                .gap(px(8.0 - button::BADGE_OVERHANG))
                .children(buttons.into_iter().map(|button| {
                    let handler = button.on_click;
                    let key = button.id.clone();
                    button::badged(
                        format!("{}-badged", button.id),
                        button::icon(button.id, button.glyph.as_deref(), button.active, frame.hover)
                            .tip(frame.tips, key, button.label)
                            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx))),
                        button.badge,
                    )
                }))
                .into_any_element(),
            Block::Actions(tiles) => div()
                .flex()
                .gap(px(card::GAP))
                .children(tiles.into_iter().map(|tile| {
                    let handler = tile.on_click;
                    button::action(tile.id, tile.glyph.as_deref(), tile.label, tile.color, tile.armed, frame.hover)
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
const STEP_DOT: f32 = 7.0;
const STEP_LINK: f32 = 14.0;

/// One step of a path: a dot and its label, coloured by where things stand there.
fn step_element(step: PathStep) -> Div {
    let (mark, text) = match step.state {
        StepState::Reached => (GREEN, TEXT),
        StepState::Pending(_) => (YELLOW, TEXT),
        StepState::NotYet => (OVERLAY2, OVERLAY2),
    };
    let dot = div().size(px(STEP_DOT)).flex_none().rounded_full();
    let dot = match step.state {
        StepState::NotYet => dot.border_1().border_color(rgb(mark)),
        _ => dot.bg(rgb(mark)),
    };
    let mut element = div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .child(dot)
        .child(div().text_color(rgb(text)).child(step.label));
    if let StepState::Pending(Some(left)) = step.state {
        element = element.child(div().text_color(rgb(SUBTEXT1)).child(left));
    }
    element
}

fn paths(rows: Vec<PathRow>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .children(rows.into_iter().enumerate().map(|(index, row)| {
            let mut steps = div().flex().flex_wrap().items_center().gap_y(px(4.0)).text_size(px(CAPTION_SIZE));
            for (position, step) in row.steps.into_iter().enumerate() {
                if position > 0 {
                    steps = steps.child(
                        div()
                            .w(px(STEP_LINK))
                            .mx(px(6.0))
                            .h(px(1.0))
                            .flex_none()
                            .bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))),
                    );
                }
                steps = steps.child(step_element(step));
            }
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .py(px(8.0))
                .when(index > 0, |row| row.border_t_1().border_color(rgba(with_alpha(SURFACE1, CONTROL_ALPHA))))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(12.0))
                        .child(
                            div()
                                .min_w(px(0.0))
                                .truncate()
                                .text_size(px(SMALL_SIZE))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(row.title),
                        )
                        .when_some(row.done, |line, done| {
                            line.child(
                                div()
                                    .flex_none()
                                    .text_size(px(CAPTION_SIZE))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(GREEN))
                                    .child(done),
                            )
                        }),
                )
                .child(
                    div()
                        .min_w(px(0.0))
                        .truncate()
                        .text_size(px(CAPTION_SIZE))
                        .text_color(rgb(SUBTEXT1))
                        .child(row.detail),
                )
                .child(steps)
        }))
        .into_any_element()
}

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
    let id = |part: &str| SharedString::from(format!("{}-{part}", level.id));
    let amount = if level.muted { "Muted".to_owned() } else { format!("{percent}%") };
    let (label, detail, amount) =
        (motion::text(id("label"), level.label), motion::text(id("detail"), level.detail.clone()), motion::text(id("amount"), amount));
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
                                .child(div().min_w(px(0.0)).truncate().font_weight(FontWeight::SEMIBOLD).child(label))
                                .when(!level.detail.is_empty(), |line| {
                                    line.child(div().min_w(px(0.0)).truncate().child(detail))
                                }),
                        )
                        .child(
                            div()
                                .flex_none()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(glyph_color))
                                .child(amount),
                        ),
                )
                .child(slider),
        )
        .into_any_element()
}

const LAUNCHER_W: f32 = 560.0;
/// The list's width beside a preview pane, and the pane's.
const LIST_W: f32 = 400.0;
const PANE_W: f32 = 460.0;
const PANE_GAP: f32 = 10.0;
const SEARCH_H: f32 = 48.0;
const CARET_H: f32 = 22.0;
const APP_ROW_H: f32 = 48.0;
const APP_ROW_GAP: f32 = 2.0;
const APP_ICON: f32 = 30.0;
const PREVIEW_W: f32 = 56.0;
const PREVIEW_H: f32 = 40.0;
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

thread_local! {
    /// Pixels scrolled over a list that do not yet add up to a whole row.
    static SCROLLED: Cell<f32> = const { Cell::new(0.0) };
}

/// Whole rows the scroll wheel moved, down being positive; touchpads add up across events.
fn rows_scrolled(event: &ScrollWheelEvent) -> i32 {
    let total = SCROLLED.get() - f32::from(event.delta.pixel_delta(px(APP_ROW_H)).y);
    let rows = (total / APP_ROW_H).trunc();
    SCROLLED.set(total - rows * APP_ROW_H);
    rows as i32
}

fn preview_pane(preview: Preview, detail: SharedString, height: f32) -> AnyElement {
    let content = match preview {
        Preview::Text(text) => div()
            .text_size(px(SMALL_SIZE))
            .child(text)
            .into_any_element(),
        Preview::Picture(path) => gpui::img(path)
            .size_full()
            .object_fit(gpui::ObjectFit::Contain)
            .into_any_element(),
        Preview::Empty => div().into_any_element(),
    };
    card::surface()
        .w(px(PANE_W))
        .h(px(height))
        .flex_none()
        .p(px(CARD_PAD))
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(div().flex_1().min_h(px(0.0)).overflow_hidden().child(content))
        .child(
            div()
                .flex_none()
                .truncate()
                .text_size(px(SMALL_SIZE))
                .text_color(rgb(SUBTEXT1))
                .child(detail),
        )
        .into_any_element()
}

/// The launcher is always the same size: a search field over `launcher::ROWS` row slots, beside a
/// preview pane when it has one. Typing only changes what fills the slots, so the island never
/// reshapes mid-search, and the selection glides between rows on a spring instead of jumping.
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
                field.child(caret()).child(div().pl(px(4.0)).text_color(rgb(OVERLAY2)).child(view.placeholder.clone()))
            } else {
                field
                    .child(div().min_w(px(0.0)).truncate().child(view.query.clone()))
                    .child(caret())
            }
        });

    let selected = view.rows.iter().position(|row| row.selected);
    let empty = view.rows.is_empty();
    let rows = view.rows.into_iter().map(|row| {
        let handler = row.on_click;
        let entered = row.on_hover;
        let slot = if row.preview {
            div().w(px(PREVIEW_W)).h(px(PREVIEW_H)).flex().items_center().justify_center().when_some(
                row.icon.as_deref(),
                |slot, path| slot.child(gpui::img(path.to_path_buf()).size_full().object_fit(gpui::ObjectFit::Contain)),
            )
        } else {
            div()
                .size(px(APP_ICON))
                .when_some(row.icon.as_deref(), |slot, path| slot.child(icon::image(path, APP_ICON)))
        };
        button::fill(row.id, Radius::Row, None, hover && !row.selected)
            .flex_none()
            .h(px(APP_ROW_H))
            .px(px(10.0))
            .justify_start()
            .gap(px(12.0))
            .child(slot.flex_none())
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
            .on_click(cx.listener(move |this, _, _, cx| handler(this, cx)))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    entered(this, cx);
                }
            }))
    });

    let on_step = view.on_step;
    let width = if view.preview.is_some() { LIST_W } else { LAUNCHER_W };
    let list_h = launcher::ROWS as f32 * APP_ROW_H + (launcher::ROWS - 1) as f32 * APP_ROW_GAP;
    let list = div()
        .id("launcher-list")
        .relative()
        .flex_none()
        .w(px(width))
        .h(px(list_h))
        .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
            let step = rows_scrolled(event);
            if step != 0 {
                on_step(step, this, cx);
            }
        }))
        .when_some(selected, |list, index| {
            let top = px(index as f32 * (APP_ROW_H + APP_ROW_GAP));
            list.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .h(px(APP_ROW_H))
                    .child(ui::shape(Radius::Row, Some(ui::selected_fill().into())))
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
                        view.empty.clone()
                    } else {
                        SharedString::from(format!("Nothing matches “{}”", view.query))
                    }),
            )
        });

    let body = match view.preview {
        Some((preview, detail)) => div()
            .flex()
            .gap(px(PANE_GAP))
            .child(list)
            .child(preview_pane(preview, detail, list_h))
            .into_any_element(),
        None => list.into_any_element(),
    };
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(search)
        .child(body)
        .into_any_element()
}

const METER_W: f32 = 360.0;
const METER_GLYPH: f32 = 22.0;
const METER_TILE: f32 = 40.0;

/// A level beside its glyph, with what it is of; without a level, only what it is of, the name in
/// its colour, for a device just switched to.
fn meter_element(meter: Meter, at: &str, cx: &mut Context<Island>) -> AnyElement {
    let muted = meter.level.is_some_and(|(_, muted)| muted);
    let tint = if muted { OVERLAY2 } else { meter.color };
    let tile = div()
        .relative()
        .size(px(METER_TILE))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(ui::shape(Radius::Control, Some(rgba(with_alpha(tint, ACCENT_TINT_ALPHA)).into())))
        .when_some(meter.glyph.as_deref(), |slot, path| slot.child(icon::glyph(path, METER_GLYPH, tint)));
    let id = |part: &str| SharedString::from(format!("{at}-{part}"));
    let caption = motion::text(id("caption"), meter.caption.clone());
    let label = motion::text(id("label"), meter.label.clone());
    let row = div().w(px(METER_W)).px(px(4.0)).flex().items_center().gap(px(12.0)).child(tile);
    let Some((value, muted)) = meter.level else {
        return row
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .when(!meter.caption.is_empty(), |text| {
                        text.child(div().text_size(px(SMALL_SIZE)).text_color(rgb(SUBTEXT1)).child(caption))
                    })
                    .child(
                        div()
                            .min_w(px(0.0))
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(meter.color))
                            .child(label),
                    ),
            )
            .into_any_element();
    };
    let percent = (value * 100.0).round() as i32;
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
                    .child(caption),
            )
        })
        .child(div().min_w(px(0.0)).truncate().font_weight(FontWeight::SEMIBOLD).child(label));
    let amount = div()
        .flex_none()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(if muted { OVERLAY2 } else { TEXT }))
        .child(motion::text(id("amount"), if muted { "Muted".to_owned() } else { format!("{percent}%") }));
    row.child(
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(div().flex().items_baseline().justify_between().gap(px(12.0)).child(heading).child(amount))
            .child(slider(id("slider"), if muted { 0.0 } else { value }, tint, None, cx)),
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
        let (rest, tint) = if primary { (Some(rgb(ACCENT).into()), ON_ACCENT) } else { (None, TEXT) };
        button::fill(id, Radius::Pill, rest, hover)
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
                    .child(motion::text("media-position", clock_text(view.position)))
                    .child(motion::text("media-length", clock_text(length))),
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
                            cell.child(ui::shape(Radius::Control, Some(rgb(ACCENT).into())))
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
