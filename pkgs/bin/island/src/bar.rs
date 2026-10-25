use std::{
    any::Any,
    collections::HashMap,
    path::Path,
    rc::Rc,
    sync::LazyLock,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, Context, Div, FontWeight, Render, SharedString, Stateful, Task, WeakEntity,
    Window, div, prelude::*, px, rgb, rgba,
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
    island::{Island, Mode, Panel},
    presence::Presence,
    theme::{ACCENT, TEXT, TINT, TRACK_ALPHA, with_alpha},
    ui::{self, HEADER_H, Radius, SMALL_SIZE},
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
pub const TAB_STEP: f32 = TAB_W + TAB_GAP;
const CLOSE_GAP: f32 = 16.0;

/// Island's own tab; each feature's tab glyph comes with the feature's glyphs.
pub const GLYPHS: &[features::GlyphSpec] = &[(
    "tab-system",
    &["utilities-system-monitor-symbolic", "computer-symbolic"],
)];

fn tab_list() -> &'static [(Panel, &'static str)] {
    static TABS: LazyLock<Vec<(Panel, &'static str)>> = LazyLock::new(|| {
        let mut tabs = vec![(Panel::System, "System", 4)];
        tabs.extend(
            features::tabs()
                .into_iter()
                .map(|(name, title, order)| (Panel::Feature(name), title, order)),
        );
        tabs.sort_by_key(|(_, _, order)| *order);
        tabs.into_iter()
            .map(|(panel, title, _)| (panel, title))
            .collect()
    });
    &TABS
}

pub fn first_tab() -> Panel {
    tab_list()
        .first()
        .map_or(Panel::System, |(panel, _)| *panel)
}

pub fn strip_width() -> f32 {
    tab_list().len() as f32 * TAB_STEP - TAB_GAP
}

/// The strip with room for the close button on both sides.
pub fn tabs_width() -> f32 {
    strip_width() + 2.0 * (CLOSE_GAP + HEADER_H)
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
    pub(crate) tips: &'a Tooltips,
}

impl<'a> Look<'a> {
    fn new(hover: bool, spacing: f32, tips: &'a Tooltips) -> Self {
        Self {
            hover,
            half: half(spacing),
            inset: spacing / 2.0,
            tips,
        }
    }

    pub(crate) fn half(self) -> f32 {
        self.half
    }

    pub(crate) fn hover(self) -> bool {
        self.hover
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
        .when(look.hover, |widget| widget.hover(|style| style))
        .child(ui::inset_control(Radius::Control, look.inset, look.hover))
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
    let look = Look::new(bar.hover, bar.layout.spacing, bar.tips);
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
            && at.saturating_duration_since(*armed).max(armed.saturating_duration_since(at))
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
    let sections: Vec<Vec<AnyElement>> = [&bar.layout.left, &bar.layout.middle, &bar.layout.right]
        .into_iter()
        .map(|widgets| {
            widgets
                .iter()
                .filter_map(|kind| place(*kind, &bar, element))
                .collect()
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

pub fn tabs(
    selected: Panel,
    pill_x: f32,
    strip_left: f32,
    hover: bool,
    tips: &Tooltips,
    cx: &mut Context<Island>,
) -> AnyElement {
    let glyphs = cx.global::<Glyphs>().clone();
    ui::base()
        .relative()
        .size_full()
        .child(
            div()
                .absolute()
                .top_0()
                .left(px(strip_left))
                .flex()
                .gap(px(TAB_GAP))
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left(px(pill_x))
                        .w(px(TAB_W))
                        .h(px(HEADER_H))
                        .child(ui::shape(Radius::Row, Some(ui::selected_fill().into()))),
                )
                .children(tab_list().iter().map(|&(kind, label)| {
                    let selected = selected == kind;
                    let id = SharedString::from(format!("tab-{}", kind.name()));
                    let color = if selected { ACCENT } else { TEXT };
                    let tab = button::fill(id.clone(), Radius::Row, None, !selected && hover)
                        .w(px(TAB_W))
                        .h(px(HEADER_H))
                        .text_size(px(ui::TEXT_SIZE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(color));
                    match glyphs.get(&id) {
                        Some(path) => tab.child(icon::glyph(path, TAB_GLYPH, color)),
                        None => tab.child(label),
                    }
                    .tip(tips, id, label)
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.set_mode(Mode::Open(kind), cx)),
                    )
                })),
        )
        .child(
            div().absolute().top_0().right_0().child(
                button::close("tabs-close", hover)
                    .on_click(cx.listener(|this, _, _, cx| this.set_mode(Mode::Collapsed, cx))),
            ),
        )
        .into_any_element()
}

pub fn tab_index(kind: Panel) -> Option<usize> {
    tab_list().iter().position(|(tab, _)| *tab == kind)
}
