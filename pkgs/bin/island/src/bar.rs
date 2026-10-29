use std::{
    any::Any,
    collections::HashMap,
    path::Path,
    rc::Rc,
    sync::{LazyLock, OnceLock},
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Context, Div, FontWeight, Render, SharedString, Stateful, Task, WeakEntity,
    Rgba, Window, div, prelude::*, px, rgb, rgba,
};

use crate::{
    components::{
        button, energy, icon,
        marquee::{self, TextLayouts},
        measure::{self, Measured},
        motion,
        tooltip::{Tip, Tooltips},
    },
    config::{self, Widget},
    features::{self, AnyLocal, Features, WidgetCx},
    glyphs::Glyphs,
    island::{Island, Mode, Panel, SHAPE_SPRING},
    presence::Presence,
    spring::Spring,
    theme::{ACCENT, ON_ACCENT, OVERLAY2, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, HEADER_H, Radius, SMALL_SIZE, STRIP_PAD},
};

const EDGE: f32 = 14.0;
const WIDGET_H: f32 = 28.0;
const WIDGET_PAD: f32 = 8.0;
pub const GLYPH: f32 = 15.0;
pub const ICON_GAP: f32 = 6.0;
const METER_W: f32 = 4.0;
const METER_H: f32 = 16.0;
/// Each tab shows its glyph, `tab-<panel>`, and names itself in a tooltip.
const TAB_W: f32 = 48.0;
const TAB_GLYPH: f32 = 18.0;
const TAB_GAP: f32 = 4.0;
const CLOSE_GAP: f32 = 16.0;
/// The Dashboard tab's place among the tabs.
const DASHBOARD_ORDER: u8 = 4;
/// Between a dashboard view's glyph and the name it shows while picked.
const LABEL_GAP: f32 = 8.0;
const LABEL_SIZE: f32 = 13.0;
/// The glyph's place from a tab's start, where it stays as the tab widens to show its name.
const GLYPH_X: f32 = (TAB_W - TAB_GLYPH) / 2.0;
/// Room for the line between the Dashboard tab and its views, while they show.
const DIVIDER_GAP: f32 = 7.0;
const DIVIDER_H: f32 = 16.0;
/// The picked tab's tint, outside the dashboard's views.
const PICKED_ALPHA: u8 = 0x2e;
const VIEWS_ALPHA: u8 = 0x1a;
const VIEWS_BORDER_ALPHA: u8 = 0x38;
const DIVIDER_ALPHA: u8 = 0x4d;

/// Island's own tabs; each feature's tab glyph comes with the feature's glyphs.
pub const GLYPHS: &[features::GlyphSpec] = &[
    ("tab-system", &["cpu"]),
    ("tab-dashboard", &["layout-grid"]),
    ("dashboard-warning", &["triangle-alert"]),
    ("dashboard-battery", &["battery-full"]),
];

struct Tab {
    panel: Panel,
    title: &'static str,
    /// One of the dashboard's views, which show only while one of them is open.
    view: bool,
}

/// The dashboard's views follow one another where its tab goes, the overview first.
fn tab_list() -> &'static [Tab] {
    static TABS: LazyLock<Vec<Tab>> = LazyLock::new(|| {
        let mut views = vec![(Panel::Dashboard, "Dashboard", 0), (Panel::System, "System", 2)];
        views.extend(
            features::dashboard_tabs()
                .into_iter()
                .map(|(name, title, order)| (Panel::Feature(name), title, order)),
        );
        views.sort_by_key(|(_, _, order)| *order);
        let mut tabs: Vec<(u8, u8, Tab)> = features::tabs()
            .into_iter()
            .map(|(name, title, order)| {
                let panel = Panel::Feature(name);
                (order, 0, Tab { panel, title, view: false })
            })
            .collect();
        tabs.extend(views.into_iter().map(|(panel, title, order)| {
            (DASHBOARD_ORDER, order, Tab { panel, title, view: true })
        }));
        tabs.sort_by_key(|(order, within, _)| (*order, *within));
        tabs.into_iter().map(|(_, _, tab)| tab).collect()
    });
    &TABS
}

static LABELS: OnceLock<Vec<f32>> = OnceLock::new();

/// Measures the names the dashboard's views show while picked, once, as text is first drawn.
pub fn measure_labels(window: &Window) {
    LABELS.get_or_init(|| {
        tab_list()
            .iter()
            .map(|tab| {
                if tab.view {
                    marquee::shape(window, tab.title, LABEL_SIZE, FontWeight::SEMIBOLD).width
                } else {
                    0.0
                }
            })
            .collect()
    });
}

fn label_w(index: usize) -> f32 {
    LABELS
        .get()
        .and_then(|labels| labels.get(index))
        .map_or(0.0, |width| LABEL_GAP + width)
}

pub fn tab_index(kind: Panel) -> Option<usize> {
    tab_list().iter().position(|tab| tab.panel == kind)
}

/// The strip with room for the close button on both sides, at its widest, with every view showing
/// and the longest name.
pub fn tabs_width() -> f32 {
    let tabs = tab_list();
    let widest = (0..tabs.len()).map(label_w).fold(0.0, f32::max);
    tabs.len() as f32 * (TAB_W + TAB_GAP) - TAB_GAP
        + widest
        + DIVIDER_GAP
        + 2.0 * STRIP_PAD
        + 2.0 * (CLOSE_GAP + HEADER_H)
}

/// The tabs' motion: the pill sliding to the picked one, the dashboard's views fanning out of its
/// tab while one of them is open, and the picked view widening to show its name.
pub struct TabStrip {
    pill_x: Spring,
    pill_w: Spring,
    fan: Spring,
    labels: Vec<Spring>,
    picked: Panel,
    /// The dashboard's view last open, where its tab goes back to.
    last_view: Panel,
}

impl Default for TabStrip {
    fn default() -> Self {
        let spring = |value| Spring::new(value, SHAPE_SPRING, 0.1);
        let shown = |value| Spring::new(value, SHAPE_SPRING, 0.002);
        Self {
            pill_x: spring(0.0),
            pill_w: spring(TAB_W),
            fan: shown(0.0),
            labels: tab_list().iter().map(|_| shown(0.0)).collect(),
            picked: Panel::Dashboard,
            last_view: Panel::Dashboard,
        }
    }
}

impl TabStrip {
    /// Where each tab starts and how wide it is, and the strip's width, with the springs where
    /// they are now or, with `settled`, where they are going.
    fn layout(&self, settled: bool) -> (Vec<(f32, f32)>, f32) {
        let at = |spring: &Spring| if settled { spring.target() } else { spring.value() };
        let fan = at(&self.fan).clamp(0.0, 1.0);
        let mut x = 0.0;
        let mut places = Vec::new();
        for (index, tab) in tab_list().iter().enumerate() {
            let label = at(&self.labels[index]).clamp(0.0, 1.0) * label_w(index);
            let shown = if tab.view && tab.panel != Panel::Dashboard {
                fan
            } else {
                1.0
            };
            let width = shown * (TAB_W + label);
            places.push((x, width));
            x += width + shown * TAB_GAP;
            if tab.panel == Panel::Dashboard {
                x += fan * DIVIDER_GAP;
            }
        }
        (places, (x - TAB_GAP).max(0.0))
    }

    pub fn width(&self) -> f32 {
        self.layout(false).1
    }

    /// `at_once` puts everything in place without moving, such as while the tabs are hidden.
    pub fn pick(&mut self, panel: Panel, at_once: bool) {
        let Some(index) = tab_index(panel) else {
            return;
        };
        let view = tab_list()[index].view;
        self.picked = panel;
        if view {
            self.last_view = panel;
        }
        let fan = if view { 1.0 } else { 0.0 };
        self.fan.retarget(fan, SHAPE_SPRING);
        for (at, label) in self.labels.iter_mut().enumerate() {
            let shown = if view && at == index { 1.0 } else { 0.0 };
            label.retarget(shown, SHAPE_SPRING);
        }
        let (places, _) = self.layout(true);
        let (x, w) = places[index];
        self.pill_x.retarget(x, SHAPE_SPRING);
        self.pill_w.retarget(w, SHAPE_SPRING);
        if at_once {
            for spring in [&mut self.pill_x, &mut self.pill_w, &mut self.fan]
                .into_iter()
                .chain(self.labels.iter_mut())
            {
                spring.reset(spring.target());
            }
        }
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        for spring in [&mut self.pill_x, &mut self.pill_w, &mut self.fan]
            .into_iter()
            .chain(self.labels.iter_mut())
        {
            spring.advance(dt, now);
        }
    }

    pub fn is_settled(&self) -> bool {
        [&self.pill_x, &self.pill_w, &self.fan]
            .into_iter()
            .chain(self.labels.iter())
            .all(Spring::is_settled)
    }
}

pub struct Bar<'a> {
    pub layout: &'a config::Bar,
    pub glyphs: &'a Glyphs,
    pub tips: &'a Tooltips,
    pub layouts: &'a TextLayouts,
    pub held: &'a Presence<Held>,
    pub features: &'a Features,
    pub locals: &'a [(&'static str, Box<dyn AnyLocal>)],
    pub natural: &'a HashMap<Widget, Measured>,
    pub hover: bool,
    pub slots: Slots,
    pub sections: &'a [Measured; 3],
    pub side: f32,
}

/// Kept so a feature widget can still draw while it folds away.
pub type Held = Rc<dyn Any>;

#[derive(Clone, Copy)]
pub(crate) struct Look<'a> {
    hover: bool,
    half: f32,
    inset: f32,
    /// Part of a group of widgets, which shows the hover for all of them.
    grouped: bool,
    pub(crate) tips: &'a Tooltips,
}

impl<'a> Look<'a> {
    fn new(hover: bool, spacing: f32, tips: &'a Tooltips) -> Self {
        Self {
            hover,
            half: half(spacing),
            inset: spacing / 2.0,
            grouped: false,
            tips,
        }
    }

    pub(crate) fn half(self) -> f32 {
        self.half
    }

    pub(crate) fn hover(self) -> bool {
        self.hover
    }

    pub(crate) fn grouped(self) -> bool {
        self.grouped
    }
}

pub(crate) fn on<E: ?Sized>(
    island: &WeakEntity<Island>,
    handler: impl Fn(&mut Island, &E, &mut Context<Island>) + 'static,
) -> impl Fn(&E, &mut Window, &mut App) + 'static {
    let island = island.clone();
    move |event, _, cx| {
        let _ = island.update(cx, |island, cx| handler(island, event, cx));
    }
}

pub fn half(spacing: f32) -> f32 {
    WIDGET_PAD + spacing / 2.0
}

pub(crate) fn widget_base(id: impl Into<SharedString>, look: Look) -> Stateful<Div> {
    widget_group(id, look)
        .cursor_pointer()
        .when(!look.grouped, |widget| {
            widget
                .when(look.hover, |widget| widget.hover(|style| style))
                .child(ui::inset_control(Radius::Control, look.inset, look.hover))
        })
}

/// Widgets side by side on one surface, which shows the hover for all of them.
fn widget_set(id: usize, widgets: Vec<AnyElement>, look: Look) -> AnyElement {
    div()
        .id(SharedString::from(format!("bar-group-{id}")))
        .relative()
        .flex_none()
        .flex()
        .items_center()
        .when(look.hover, |set| set.hover(|style| style))
        .child(ui::inset_control(Radius::Control, look.inset, look.hover))
        .children(widgets)
        .into_any_element()
}

/// For a widget made of several clickable parts that each show the hover tint themselves.
pub(crate) fn widget_group(id: impl Into<SharedString>, look: Look) -> Stateful<Div> {
    div()
        .id(id.into())
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .h(px(WIDGET_H))
        .flex_none()
        .px(px(look.half))
}

#[derive(Clone, Copy, Default)]
pub struct Slots {
    count: f32,
}

impl Slots {
    pub fn count(self) -> f32 {
        self.count
    }
}

pub fn measure_slots(window: &Window) -> Slots {
    let widest = |texts: Vec<String>, size: f32, weight: FontWeight| {
        texts
            .into_iter()
            .map(|text| marquee::shape(window, &text, size, weight).width)
            .fold(0.0, f32::max)
    };
    Slots {
        count: widest(
            vec!["88".to_owned(), "!".to_owned()],
            SMALL_SIZE,
            FontWeight::SEMIBOLD,
        ),
    }
}

pub(crate) fn slot(text: impl Into<SharedString>, width: f32) -> Div {
    div()
        .min_w(px(width))
        .flex_none()
        .whitespace_nowrap()
        .child(motion::text("count", text))
}

pub struct Gauge<'a> {
    pub glyph: Option<&'a Path>,
    pub glyph_color: u32,
    pub busy: bool,
    pub level: Option<f32>,
    pub level_color: u32,
}

pub(crate) fn gauge(id: impl Into<SharedString>, look: Look, gauge: Gauge<'_>) -> Stateful<Div> {
    widget_base(id, look)
        .gap(px(ICON_GAP))
        .when_some(gauge.glyph, |widget, path| {
            widget.child(icon::glyph(path, GLYPH, gauge.glyph_color))
        })
        .child(meter(gauge.level, gauge.level_color))
        .child(energy(look, gauge.busy))
}

pub(crate) fn energy(look: Look, busy: bool) -> impl IntoElement {
    energy::border("energy", Radius::Control, look.inset, busy)
}

fn meter(level: Option<f32>, color: u32) -> Div {
    let level = level.unwrap_or(0.0).clamp(0.0, 1.0);
    div()
        .flex_none()
        .w(px(METER_W))
        .h(px(METER_H))
        .rounded_full()
        .bg(rgba(with_alpha(TINT, TRACK_ALPHA)))
        .flex()
        .flex_col()
        .justify_end()
        .when(level > 0.0, |track| {
            track.child(
                div()
                    .w_full()
                    .h(px((METER_H * level).max(METER_W)))
                    .rounded_full()
                    .bg(rgb(color)),
            )
        })
}

pub fn presence_key(Widget(name): Widget) -> &'static str {
    name
}

/// Widgets that rest at 0 disappear; others keep a core (media keeps its icon) and fold only their
/// detail.
pub fn rest_width(Widget(name): Widget, layout: &config::Bar, features: &Features) -> f32 {
    features
        .owning(name)
        .map_or(0.0, |feature| feature.rest_width(name, layout.spacing))
}

/// Fully shown, a widget keeps its own width: clipping it to the width measured last frame would
/// cut it off for a frame whenever it grows, such as tray icons sliding in.
fn collapse(content: AnyElement, presence: f32, rest: f32, natural: &Measured) -> AnyElement {
    let width = natural
        .get()
        .map(|size| f32::from(size.width))
        .filter(|_| presence < 1.0);
    let inner = div()
        .flex_none()
        .relative()
        .child(content)
        .child(measure::probe(natural));
    div()
        .flex_none()
        .flex()
        .overflow_hidden()
        .when_some(width, |clip, width| {
            clip.w(px(rest + (width - rest).max(0.0) * presence))
        })
        .when(width.is_none() && presence < 1.0, |clip| clip.w(px(rest)))
        .when(rest <= 0.0, |clip| clip.opacity(smoothstep(presence)))
        .child(inner)
        .into_any_element()
}

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A feature widget draws the state it holds while it folds away.
pub fn content(
    widget: Widget,
    bar: &Bar<'_>,
    island: &WeakEntity<Island>,
    cx: &App,
) -> (Vec<AnyElement>, Option<Duration>) {
    let mut look = Look::new(bar.hover, bar.layout.spacing, bar.tips);
    look.grouped = bar.layout.group.contains(&widget);
    let Widget(name) = widget;
    let Some(feature) = bar.features.owning(name) else {
        return (Vec::new(), None);
    };
    let state = bar
        .held
        .iter()
        .find(|slot| slot.key == name)
        .map_or_else(|| feature.snapshot(cx), |slot| slot.item.clone());
    let local = bar
        .locals
        .iter()
        .find(|(owner, _)| *owner == feature.name())
        .map(|(_, local)| &**local);
    let mut w = WidgetCx::new(look, bar.slots, bar.glyphs, bar.layouts, island, local);
    let elements = feature
        .widget(name, &*state, &mut w, cx)
        .into_iter()
        .collect();
    (elements, w.next_redraw())
}

/// Gone widgets are left out entirely, unless they rest at a visible core.
fn place(
    widget: Widget,
    bar: &Bar<'_>,
    element: &dyn Fn(Widget) -> AnyElement,
) -> Option<AnyElement> {
    let key = presence_key(widget);
    let rest = rest_width(widget, bar.layout, bar.features);
    let natural = bar.natural.get(&widget)?;
    let presence = match bar.held.iter().find(|slot| slot.key == key) {
        Some(slot) => slot.presence.value().clamp(0.0, 1.0),
        None if rest > 0.0 => 0.0,
        None => return None,
    };
    Some(collapse(element(widget), presence, rest, natural))
}

/// Its own view, so GPUI can replay its last drawing while the island is at rest, and scrolling
/// text redraws just this widget.
pub struct WidgetView {
    kind: Widget,
    island: WeakEntity<Island>,
    layouts: TextLayouts,
    ticker: Option<Task<()>>,
    due: Option<(Instant, Task<()>)>,
    rendered: Instant,
}

impl WidgetView {
    pub fn new(kind: Widget, island: WeakEntity<Island>) -> Self {
        Self {
            kind,
            island,
            layouts: TextLayouts::default(),
            ticker: None,
            due: None,
            rendered: Instant::now(),
        }
    }

    pub fn scrolling(&self) -> bool {
        self.ticker.is_some()
    }

    fn keep_ticking(&mut self, cx: &mut Context<Self>) {
        match (self.layouts.scrolling(), self.ticker.is_some()) {
            (true, false) => {
                self.ticker = Some(cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor().timer(marquee::TICK).await;
                        let tick = this.update(cx, |this, cx| {
                            if this.rendered.elapsed() >= marquee::TICK / 2 {
                                cx.notify();
                            }
                        });
                        if tick.is_err() {
                            break;
                        }
                    }
                }));
            }
            (false, true) => self.ticker = None,
            _ => {}
        }
    }
    fn arm_due(&mut self, after: Option<Duration>, cx: &mut Context<Self>) {
        let next = after.map(|after| self.rendered + after);
        if let (Some((armed, _)), Some(at)) = (&self.due, next)
            && at
                .saturating_duration_since(*armed)
                .max(armed.saturating_duration_since(at))
                < Duration::from_millis(50)
        {
            return;
        }
        self.due = next.map(|at| {
            let task = cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(at.saturating_duration_since(Instant::now()))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    this.due = None;
                    if let Some(island) = this.island.upgrade() {
                        let _ = island.update(cx, |island, cx| island.kick(cx));
                    }
                    cx.notify();
                });
            });
            (at, task)
        });
    }
}

impl Render for WidgetView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = Instant::now();
        self.layouts.begin_frame();
        let (content, redraw) = match self.island.upgrade() {
            Some(island) => island
                .read(cx)
                .bar_widget(self.kind, &self.layouts, &self.island, cx),
            None => (Vec::new(), None),
        };
        if self.layouts.resolve(window) {
            window.request_animation_frame();
        }
        self.keep_ticking(cx);
        self.arm_due(redraw, cx);
        div().flex_none().flex().items_center().children(content)
    }
}

pub fn edge(layout: &config::Bar) -> f32 {
    (EDGE - half(layout.spacing)).max(0.0)
}

pub fn bar(bar: Bar<'_>, element: &dyn Fn(Widget) -> AnyElement) -> AnyElement {
    let look = Look::new(bar.hover, bar.layout.spacing, bar.tips);
    let mut sets = 0;
    let sections: Vec<Vec<AnyElement>> = [&bar.layout.left, &bar.layout.middle, &bar.layout.right]
        .into_iter()
        .map(|widgets| {
            let mut placed = Vec::new();
            let mut set = Vec::new();
            for kind in widgets {
                let shown = place(*kind, &bar, element);
                if bar.layout.group.contains(kind) {
                    set.extend(shown);
                    continue;
                }
                if !set.is_empty() {
                    placed.push(widget_set(sets, std::mem::take(&mut set), look));
                    sets += 1;
                }
                placed.extend(shown);
            }
            if !set.is_empty() {
                placed.push(widget_set(sets, set, look));
                sets += 1;
            }
            placed
        })
        .collect();
    ui::base()
        .h_full()
        .px(px(edge(bar.layout)))
        .flex()
        .items_center()
        .children(sections.into_iter().zip(bar.sections).enumerate().map(
            |(index, (section, probe))| {
                let content = div()
                    .relative()
                    .flex()
                    .items_center()
                    .children(section)
                    .child(measure::probe(probe));
                let outer = div().flex_none().flex().items_center().child(content);
                match index {
                    1 => outer,
                    0 => outer.min_w(px(bar.side)).justify_start(),
                    _ => outer.min_w(px(bar.side)).justify_end(),
                }
            },
        ))
        .into_any_element()
}

/// Partway from one color to another.
fn mix(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let at = |a: f32, b: f32| a + (b - a) * t;
    Rgba {
        r: at(from.r, to.r),
        g: at(from.g, to.g),
        b: at(from.b, to.b),
        a: at(from.a, to.a),
    }
}

pub fn tabs(
    strip: &TabStrip,
    strip_left: f32,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let glyphs = cx.global::<Glyphs>().clone();
    let (places, width) = strip.layout(false);
    let fan = strip.fan.value().clamp(0.0, 1.0);
    let in_views = tab_index(strip.picked).is_some_and(|index| tab_list()[index].view);
    // The dashboard's views sit on a tinted surface of their own while they show, the Dashboard
    // tab parted from the rest by a line.
    let views = tab_list()
        .iter()
        .zip(&places)
        .filter(|(tab, _)| tab.view)
        .map(|(_, place)| *place)
        .collect::<Vec<_>>();
    let group = match (views.first(), views.last()) {
        (Some(first), Some(last)) if fan > 0.01 => Some(
            div()
                .absolute()
                .top_0()
                .left(px(first.0))
                .w(px(last.0 + last.1 - first.0))
                .h(px(HEADER_H))
                .opacity(fan)
                .child(ui::outlined(
                    Radius::Row,
                    rgba(with_alpha(ACCENT, VIEWS_ALPHA)).into(),
                    rgba(with_alpha(ACCENT, VIEWS_BORDER_ALPHA)).into(),
                )),
        ),
        _ => None,
    };
    let divider = views.first().filter(|_| fan > 0.01).map(|root| {
        div()
            .absolute()
            .top(px((HEADER_H - DIVIDER_H) / 2.0))
            .left(px(root.0 + root.1 + (TAB_GAP + fan * DIVIDER_GAP) / 2.0))
            .w(px(1.0))
            .h(px(DIVIDER_H))
            .opacity(fan)
            .bg(rgba(with_alpha(ACCENT, DIVIDER_ALPHA)))
    });
    // A tint on a tab of its own, filling in as it becomes one of the dashboard's views.
    let picked = mix(rgba(with_alpha(ACCENT, PICKED_ALPHA)), rgb(ACCENT), fan);
    let pill = div()
        .absolute()
        .top_0()
        .left(px(strip.pill_x.value()))
        .w(px(strip.pill_w.value().max(0.0)))
        .h(px(HEADER_H))
        .child(ui::shape(Radius::Row, Some(picked.into())));
    let tabs = tab_list().iter().zip(places).enumerate().map(|(index, (tab, (x, w)))| {
        let selected = strip.picked == tab.panel;
        let kind = tab.panel;
        // The Dashboard tab goes back to the view last open, and from a view to the overview.
        let target = if kind == Panel::Dashboard && !in_views {
            strip.last_view
        } else {
            kind
        };
        let id = SharedString::from(format!("tab-{}", kind.name()));
        let color = match (selected, tab.view) {
            (true, true) => ON_ACCENT,
            (true, false) => ACCENT,
            (false, _) => OVERLAY2,
        };
        let label = strip.labels[index].value().clamp(0.0, 1.0);
        let shown = if tab.view && kind != Panel::Dashboard { fan } else { 1.0 };
        let content = match glyphs.get(&id) {
            Some(path) => div()
                .flex_none()
                .child(icon::glyph(path, TAB_GLYPH, color))
                .into_any_element(),
            None => div().flex_none().child(tab.title).into_any_element(),
        };
        button::fill(id.clone(), Radius::Row, None, !selected && hover)
            .absolute()
            .top_0()
            .left(px(x))
            .w(px(w.max(0.0)))
            .h(px(HEADER_H))
            .overflow_hidden()
            .justify_start()
            .pl(px(GLYPH_X))
            .opacity(shown)
            .text_size(px(LABEL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(color))
            .whitespace_nowrap()
            .child(content)
            .when(label > 0.01, |button| {
                button.child(
                    div()
                        .flex_none()
                        .pl(px(LABEL_GAP))
                        .opacity(label)
                        .child(tab.title),
                )
            })
            .tip(tips, id, tab.title)
            .on_click(cx.listener(move |this, _, _, cx| this.set_mode(Mode::Open(target), cx)))
    });
    let surface = ui::strip();
    ui::base()
        .relative()
        .size_full()
        .child(
            div()
                .absolute()
                .top(px(-STRIP_PAD))
                .left(px(strip_left - STRIP_PAD))
                .w(px(width + 2.0 * STRIP_PAD))
                .h(px(HEADER_H + 2.0 * STRIP_PAD))
                .child(surface)
                .child(
                    div()
                        .absolute()
                        .top(px(STRIP_PAD))
                        .left(px(STRIP_PAD))
                        .w(px(width))
                        .h(px(HEADER_H))
                        .children(group)
                        .children(divider)
                        .child(pill)
                        .children(tabs),
                ),
        )
        .child(
            div().absolute().top_0().right_0().child(
                button::close("tabs-close", hover)
                    .on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Collapsed, cx))),
            ),
        )
        .into_any_element()
}
