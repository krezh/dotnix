use std::{
    any::Any,
    cell::Cell,
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App as Gpui, Bounds, Context, Entity, EventEmitter, Global, Keystroke,
    MouseDownEvent, Pixels, Point, Render, ScrollDelta, ScrollEvent, ScrollHandle,
    SpringConfig, StyleRefinement, Subscription, Task, WeakEntity, Window, div, layer_shell::Layer,
    point, prelude::*, px, size,
};
use tracing::debug;
use uuid::Uuid;

use crate::{
    bar,
    components::{
        aura::Aura,
        marquee::{self, TextLayouts},
        measure::{self, Measured},
        popover::Popovers,
        tooltip::Tooltips,
    },
    compositor,
    config::{self, Widget},
    effects::Effects,
    features::{self, AnyLocal, Battery, Features, IslandInfo, Outcome, PanelCx},
    glyphs::Glyphs,
    menu::{self, Menu, MenuNode, Pick},
    panel::{self, Frame},
    panels,
    presence::Presence,
    shape,
    spring::Spring,
    sys::{self, Samples, SysStats},
    ui::{self, HEADER_H, INSET},
    wm::{Monitor, Snapshot},
};

/// Room for panels wider than the bar, such as a list beside text to read.
pub const SURFACE_W: f32 = 1240.0;
pub const SURFACE_H: f32 = 1000.0;

pub(crate) const SHAPE_SPRING: SpringConfig = SpringConfig::new(420.0, 34.0, 1.0);
pub(crate) const CONTENT_IN: SpringConfig = SpringConfig::new(170.0, 26.0, 1.0);
pub(crate) const CONTENT_OUT: SpringConfig = SpringConfig::new(1100.0, 66.0, 1.0);
/// Critically damped and unhurried, so folding widgets glide without overshoot.
const FOLD_SPRING: SpringConfig = SpringConfig::new(150.0, 24.5, 1.0);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fold {
    Now,
    Target,
}
/// How long new content waits for the island to make room before it fades in.
pub(crate) const REVEAL_DELAY: Duration = Duration::from_millis(90);
const MAX_STEP: Duration = Duration::from_millis(50);
pub(crate) const CONTENT_DRIFT: f32 = 8.0;
/// How far the drop shadow reaches past the island's sides and bottom: three sigmas of its blur,
/// plus its drop below.
const SHADOW_SIDE: f32 = 45.0;
const SHADOW_BELOW: f32 = 55.0;
const LINES_PER_NOTCH: f32 = 3.0;
const PIXELS_PER_NOTCH: f32 = 50.0;

/// Keeps the remainder in `accumulated` for the next event.
pub fn notches(accumulated: &mut f32, event: &ScrollEvent) -> i32 {
    *accumulated += match event.delta {
        ScrollDelta::Lines(lines) => lines.y / LINES_PER_NOTCH,
        ScrollDelta::Pixels(pixels) => f32::from(pixels.y) / PIXELS_PER_NOTCH,
    };
    let steps = accumulated.trunc();
    *accumulated -= steps;
    steps as i32
}

const LEAVE_GRACE: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    /// The dashboard's overview: every feature's switches and cards together.
    Dashboard,
    System,
    Feature(&'static str),
    /// Every feature pop-up with something to show, together, such as an OSD below notification
    /// cards.
    Popups,
}

#[derive(Default)]
pub struct Placement {
    pub islands: Vec<WeakEntity<Island>>,
}

impl Global for Placement {}

/// What an island needs covers over the outputs for while something of it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grab {
    None,
    /// Clicks outside the island, leaving the keyboard with the focused window.
    Clicks,
    Keyboard,
}

pub enum IslandEvent {
    Grab(Grab),
}

impl EventEmitter<IslandEvent> for Island {}

impl Panel {
    /// Feature panels' layers follow these.
    const ALL: [Panel; 3] = [Panel::System, Panel::Popups, Panel::Dashboard];

    pub fn name(self) -> &'static str {
        match self {
            Panel::Dashboard => "dashboard",
            Panel::System => "system",
            Panel::Popups => "popups",
            Panel::Feature(name) => features::panel_of(name).unwrap_or(name),
        }
    }

    fn takes_keyboard(self) -> bool {
        matches!(self, Panel::Feature(name) if features::keyboard(name))
    }

    pub fn parse(name: &str) -> Option<Self> {
        [Panel::Dashboard, Panel::System]
            .into_iter()
            .find(|panel| panel.name() == name)
            .or_else(|| features::panel_named(name).map(Panel::Feature))
    }

    /// Island's own panels showing what the system sampler reads: System and the dashboard.
    fn shows_stats(self) -> bool {
        matches!(self, Panel::Dashboard | Panel::System)
    }

    fn layer(self) -> usize {
        let feature = |name| {
            features::NAMES
                .iter()
                .position(|feature| *feature == name)
                .unwrap_or(0)
        };
        let index = match self {
            Panel::Feature(name) => Self::ALL.len() + feature(name),
            _ => Self::ALL.iter().position(|kind| *kind == self).unwrap_or(0),
        };
        index + 1
    }

    fn from_layer(layer: usize) -> Option<Self> {
        let index = layer.checked_sub(1)?;
        match index.checked_sub(Self::ALL.len()) {
            None => Self::ALL.get(index).copied(),
            Some(feature) => features::NAMES
                .get(feature)
                .map(|name| Panel::Feature(name)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Collapsed,
    Open(Panel),
}

#[derive(Clone, Copy)]
struct Place {
    viewport: f32,
    scale: f32,
    x: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Shape {
    w: f32,
    h: f32,
}

pub const COLLAPSED_H: f32 = 38.0;
/// Between the bar and a menu an app draws itself.
const MENU_GAP: f32 = 6.0;
const HOVER_GROW: f32 = 4.0;

const LAYERS: usize = Panel::ALL.len() + features::NAMES.len() + 1;
const COLLAPSED_LAYER: usize = 0;

pub struct Startup {
    pub layout: Rc<config::Bar>,
    pub top: f32,
    pub corner: (f32, f32),
    pub opacity: f32,
    pub shadow: f32,
    pub glow: f32,
    pub workspaces: Option<Snapshot>,
    pub features: Rc<Features>,
    pub samples: Samples,
}

struct AnimStats {
    started: Instant,
    frames: u32,
    max_gap: Duration,
}

pub struct Island {
    mode: Mode,
    closing_panel: bool,
    hovered: bool,
    fullscreen: bool,
    slots: Option<bar::Slots>,
    w: Spring,
    h: Spring,
    layers: [Spring; LAYERS],
    chrome: Spring,
    strip: bar::TabStrip,
    last_frame: Option<Instant>,
    anim: Option<AnimStats>,
    corner: (f32, f32),
    opacity: f32,
    shadow: f32,
    glow: f32,
    aura: Aura,
    input_region: Vec<Bounds<Pixels>>,
    effects: Option<Option<Effects>>,
    samples: Samples,
    pub(crate) stats: SysStats,
    revision: u64,
    /// Held while System or the dashboard is open, so the sampler reads what only they show.
    sys_watch: Option<sys::Watch>,
    panel_watch: Option<Box<dyn Any>>,
    sys_task: Option<Task<()>>,
    panel_due: Cell<Option<Duration>>,
    panel_task: Option<(Instant, Task<()>)>,
    leave_task: Option<Task<()>>,
    due: Option<(Instant, Task<()>)>,
    workspaces: Option<Snapshot>,
    display: Option<Uuid>,
    /// The compositor snapshot changed, so the island's monitor and the features' per-island
    /// parts need another look.
    monitor_stale: bool,
    menu: Option<Menu>,
    scroll: ScrollHandle,
    /// Each column of a panel's side-by-side columns scrolls on its own.
    columns: [ScrollHandle; panel::COLUMNS],
    /// A panel's one scrolling list of placed blocks, which draws only what is in view.
    placed: ScrollHandle,
    /// Set as a panel opens, so a column it follows starts at its end.
    opened: Cell<bool>,
    sizes: [Measured; LAYERS],
    pending: [Cell<Option<u64>>; LAYERS],
    /// The grab last asked of the shell.
    grabbed: Grab,
    frozen: [Option<(u64, Shape)>; LAYERS],
    bar_size: Measured,
    sections: [Measured; 3],
    tips: Tooltips,
    popovers: Popovers,
    layouts: TextLayouts,
    held: Presence<bar::Held>,
    features: Rc<Features>,
    locals: Vec<(&'static str, Box<dyn AnyLocal>)>,
    natural: HashMap<Widget, Measured>,
    drawn: HashMap<String, f32>,
    last_sample: Instant,
    sized: bool,
    top: f32,
    layout: Rc<config::Bar>,
    /// Redraws at `marquee::TICK` while text is scrolling or the island is drifting.
    ticker: Option<Task<()>>,
    /// Moving only slowly, so drawn at the ticker's pace rather than every frame the display has.
    drifting: bool,
    rendered: Instant,
    widgets: HashMap<Widget, (Entity<bar::WidgetView>, Measured)>,
    /// Whether this frame changes anything beyond what widgets redraw themselves, so every widget is
    /// drawn afresh instead of reused.
    live: bool,
    /// Overlay while a panel, popup or menu is open or closing; Top for the collapsed bar.
    layer: Layer,
    _subscriptions: Vec<Subscription>,
}

impl Island {
    pub fn new(startup: Startup, cx: &mut Context<Self>) -> Self {
        let shape_spring = |value| Spring::new(value, SHAPE_SPRING, 0.1);
        let mut layers = [Spring::new(0.0, CONTENT_OUT, 0.002); LAYERS];
        layers[COLLAPSED_LAYER] = Spring::new(1.0, CONTENT_IN, 0.002);

        let this = cx.weak_entity();
        let widgets = [
            &startup.layout.left,
            &startup.layout.middle,
            &startup.layout.right,
        ]
        .into_iter()
        .flatten()
        .map(|kind| {
            let view = cx.new(|_| bar::WidgetView::new(*kind, this.clone()));
            (*kind, (view, measure::measured()))
        })
        .collect();

        let glyphs_subscription = cx.observe_global::<Glyphs>(|this, cx| {
            this.sync_held(cx);
            if matches!(this.mode, Mode::Collapsed | Mode::Open(Panel::System | Panel::Dashboard)) {
                this.retarget();
            }
            this.kick(cx);
        });
        let mut subscriptions = vec![glyphs_subscription];
        subscriptions.extend(
            startup
                .features
                .iter()
                .flat_map(|feature| feature.observe(cx)),
        );
        let nowhere = IslandInfo {
            monitor: None,
            panel_open: false,
        };
        let locals = startup
            .features
            .iter()
            .map(|feature| (feature.name(), feature.new_local(&nowhere, cx)))
            .collect();

        let mut island = Self {
            mode: Mode::Collapsed,
            hovered: false,
            w: shape_spring(0.0),
            h: shape_spring(COLLAPSED_H),
            layers,
            chrome: Spring::new(0.0, CONTENT_OUT, 0.002),
            strip: bar::TabStrip::default(),
            last_frame: None,
            anim: None,
            corner: startup.corner,
            opacity: startup.opacity,
            shadow: startup.shadow,
            glow: startup.glow,
            aura: Aura::default(),
            input_region: Vec::new(),
            effects: None,
            samples: startup.samples,
            stats: SysStats::default(),
            revision: 0,
            sys_watch: None,
            panel_watch: None,
            sys_task: None,
            panel_due: Cell::new(None),
            panel_task: None,
            leave_task: None,
            due: None,
            monitor_stale: startup.workspaces.is_some(),
            workspaces: startup.workspaces,
            display: None,
            menu: None,
            scroll: ScrollHandle::new(),
            columns: std::array::from_fn(|_| ScrollHandle::new()),
            placed: ScrollHandle::new(),
            opened: Cell::new(false),
            sizes: std::array::from_fn(|_| measure::measured()),
            pending: std::array::from_fn(|_| Cell::new(None)),
            grabbed: Grab::None,
            frozen: [None; LAYERS],
            bar_size: measure::measured(),
            sections: std::array::from_fn(|_| measure::measured()),
            tips: Tooltips::default(),
            popovers: Popovers::default(),
            held: Presence::new(FOLD_SPRING),
            drawn: HashMap::new(),
            natural: startup
                .layout
                .widgets()
                .map(|kind| (kind, measure::measured()))
                .collect(),
            features: startup.features.clone(),
            locals,
            layouts: TextLayouts::default(),
            last_sample: Instant::now(),
            sized: false,
            fullscreen: false,
            slots: None,
            top: startup.top,
            layout: startup.layout,
            ticker: None,
            drifting: false,
            rendered: Instant::now(),
            widgets,
            live: true,
            closing_panel: false,
            layer: Layer::Top,
            _subscriptions: subscriptions,
        };
        island.sync_held(cx);
        island
    }

    pub fn input_region(&self) -> &[Bounds<Pixels>] {
        &self.input_region
    }

    pub fn key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        let Mode::Open(Panel::Feature(name)) = self.mode else {
            return;
        };
        let features = self.features.clone();
        let Some(feature) = features.get(name) else {
            return;
        };
        let mut outcome = Outcome::Close;
        self.update_local(name, |local| outcome = feature.key(local, keystroke, cx));
        match outcome {
            Outcome::Open => self.local_changed(name, cx),
            Outcome::Close => self.set_mode(Mode::Collapsed, cx),
        }
    }

    /// `view` turns a feature's panel to that view as it opens; a toggle that closes it ignores it.
    pub fn open_named(
        &mut self,
        name: &str,
        view: Option<&str>,
        toggle: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(kind) = Panel::parse(name) else {
            return false;
        };
        let closing = toggle && self.mode == Mode::Open(kind);
        if let (Some(view), Panel::Feature(feature), false) = (view, kind, closing) {
            self.update_local(feature, |local| local.show(view));
            self.local_changed(feature, cx);
        }
        if toggle {
            self.toggle(kind, cx);
        } else {
            self.set_mode(Mode::Open(kind), cx);
        }
        !closing
    }

    fn keep_ticking(&mut self, cx: &mut Context<Self>) {
        match (
            self.layouts.scrolling() || self.drifting,
            self.ticker.is_some(),
        ) {
            (true, false) => {
                self.ticker = Some(cx.spawn(async move |this, cx| {
                    loop {
                        cx.background_executor().timer(marquee::TICK).await;
                        let tick = this.update(cx, |this, cx| {
                            if this.rendered.elapsed() >= marquee::TICK / 2 {
                                this.redraw_drifting(cx);
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

    fn hidden(&self) -> bool {
        self.mode == Mode::Collapsed && self.fullscreen && self.w.value() < 1.0
    }

    /// Redraws without re-fitting the bar, and not at all while the bar is not showing.
    pub fn feature_live(&mut self, name: &'static str, cx: &mut Context<Self>) {
        if self.fullscreen || self.layers[COLLAPSED_LAYER].value() <= 0.001 {
            return;
        }
        let live = self
            .features
            .get(name)
            .map_or(&[][..], |feature| feature.live_widgets());
        let moving: Vec<Entity<bar::WidgetView>> = self
            .widgets
            .iter()
            .filter(|(kind, (view, _))| live.contains(&kind.0) || view.read(cx).scrolling())
            .map(|(_, (view, _))| view.clone())
            .collect();
        for view in moving {
            view.update(cx, |_, cx| cx.notify());
        }
        self.redraw_drifting(cx);
        cx.notify();
    }

    /// While the island drifts the rest of the bar is reused, so only the widgets of the features
    /// still moving draw afresh.
    fn redraw_drifting(&self, cx: &mut Context<Self>) {
        if !self.drifting {
            return;
        }
        let views: Vec<Entity<bar::WidgetView>> = self
            .locals
            .iter()
            .filter(|(_, local)| !local.is_settled())
            .filter_map(|(name, _)| self.features.get(name))
            .flat_map(|feature| feature.widgets())
            .filter_map(|widget| self.widgets.get(&Widget(widget)))
            .map(|(view, _)| view.clone())
            .collect();
        for view in views {
            view.update(cx, |_, cx| cx.notify());
        }
    }

    pub fn bar_widget(
        &self,
        kind: Widget,
        layouts: &TextLayouts,
        island: &WeakEntity<Island>,
        cx: &Gpui,
    ) -> (Vec<AnyElement>, Option<Duration>) {
        bar::content(kind, &self.bar_parts(layouts, cx), island, cx)
    }

    fn bar_parts<'a>(&'a self, layouts: &'a TextLayouts, cx: &'a Gpui) -> bar::Bar<'a> {
        bar::Bar {
            layout: &self.layout,
            glyphs: cx.global::<Glyphs>(),
            tips: &self.tips,
            layouts,
            held: &self.held,
            features: &self.features,
            locals: &self.locals,
            natural: &self.natural,
            hover: self.hovered,
            slots: self.slots.unwrap_or_default(),
            sections: &self.sections,
            side: self
                .section_widths(Fold::Now)
                .map_or(0.0, |[left, _, right]| left.max(right)),
        }
    }

    /// Drawn afresh on live frames; otherwise the last drawing is reused unless the widget asked to
    /// redraw.
    fn widget_element(&self, kind: Widget) -> AnyElement {
        let Some((view, size)) = self.widgets.get(&kind) else {
            return div().into_any_element();
        };
        let element = match size.get() {
            Some(drawn) if !self.live => view
                .clone()
                .cached(
                    StyleRefinement::default()
                        .flex_none()
                        .w(drawn.width)
                        .h(drawn.height),
                )
                .into_any_element(),
            _ => view.clone().into_any_element(),
        };
        div()
            .flex_none()
            .relative()
            .child(element)
            .child(measure::probe(size))
            .into_any_element()
    }

    pub fn displays(&self) -> usize {
        self.workspaces
            .as_ref()
            .map_or(1, |snapshot| snapshot.monitors.len().max(1))
    }

    /// Follow with [`Island::local_changed`].
    pub fn update_local(&mut self, name: &str, action: impl FnOnce(&mut dyn AnyLocal)) {
        if let Some((_, local)) = self.locals.iter_mut().find(|(owner, _)| *owner == name) {
            action(&mut **local);
        }
    }

    fn local(&self, name: &str) -> Option<&dyn AnyLocal> {
        self.locals
            .iter()
            .find(|(owner, _)| *owner == name)
            .map(|(_, local)| &**local)
    }

    pub fn local_as<L: 'static>(&self, name: &str) -> Option<&L> {
        self.local(name)?.as_any().downcast_ref()
    }

    /// Shows the pop-ups once any has something, and takes them away once none has.
    pub fn local_changed(&mut self, name: &'static str, cx: &mut Context<Self>) {
        let waiting = self.popup_waiting();
        match self.mode {
            Mode::Collapsed if waiting && !self.menu_open() => {
                self.set_mode(Mode::Open(Panel::Popups), cx)
            }
            Mode::Open(Panel::Popups) if !waiting => self.set_mode(Mode::Collapsed, cx),
            Mode::Open(Panel::Popups) => self.retarget(),
            Mode::Open(Panel::Feature(open)) if open == name => self.retarget(),
            _ => {}
        }
        // A view can take or let go of the keys as it changes, such as switching tabs.
        self.sync_grab(cx);
        self.arm_due(cx);
        self.kick(cx);
    }

    fn popup_waiting(&self) -> bool {
        self.locals.iter().any(|(_, local)| local.popup())
    }

    fn arm_due(&mut self, cx: &mut Context<Self>) {
        let next = self
            .locals
            .iter()
            .filter_map(|(_, local)| local.due())
            .min();
        if self.due.as_ref().map(|(at, _)| *at) == next {
            return;
        }
        self.due = next.map(|at| {
            let task = cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(at.saturating_duration_since(Instant::now()))
                    .await;
                let _ = this.update(cx, |this, cx| this.elapse(cx));
            });
            (at, task)
        });
    }

    fn elapse(&mut self, cx: &mut Context<Self>) {
        self.due = None;
        let now = Instant::now();
        let features = self.features.clone();
        let mut elapsed = Vec::new();
        for (name, local) in &mut self.locals {
            if local.due().is_some_and(|at| at <= now)
                && let Some(feature) = features.get(name)
            {
                feature.elapse(&mut **local, self.hovered, cx);
                elapsed.push(*name);
            }
        }
        for name in elapsed {
            self.local_changed(name, cx);
        }
        self.arm_due(cx);
    }

    fn arm_panel_due(&mut self, cx: &mut Context<Self>) {
        let next = (self.mode != Mode::Collapsed)
            .then(|| self.panel_due.get())
            .flatten()
            .map(|after| self.rendered + after);
        if let (Some((armed, _)), Some(at)) = (&self.panel_task, next)
            && at
                .saturating_duration_since(*armed)
                .max(armed.saturating_duration_since(at))
                < Duration::from_millis(50)
        {
            return;
        }
        self.panel_task = next.map(|at| {
            let task = cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(at.saturating_duration_since(Instant::now()))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    this.panel_task = None;
                    cx.notify();
                });
            });
            (at, task)
        });
    }

    pub fn display(&self) -> Option<Uuid> {
        self.display
    }

    pub fn set_workspaces(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.workspaces = Some(snapshot);
        self.monitor_stale = true;
        cx.notify();
    }

    /// `anchor` is across the island's surface; `source` names what opened the menu, for
    /// [`Island::menu_open_for`].
    pub fn show_menu(
        &mut self,
        source: String,
        root: MenuNode,
        pick: Pick,
        anchor: Pixels,
        cx: &mut Context<Self>,
    ) {
        if self.mode != Mode::Collapsed || self.fullscreen {
            return;
        }
        let anchor = f32::from(anchor);
        match &mut self.menu {
            Some(menu) => menu.replace(source, root, pick, anchor),
            None => self.menu = Some(Menu::new(source, root, pick, anchor)),
        }
        debug!("menu opened");
        self.sync_grab(cx);
        self.kick(cx);
    }

    fn menu_open(&self) -> bool {
        self.menu.as_ref().is_some_and(Menu::is_open)
    }

    pub fn menu_open_for(&self, source: &str) -> bool {
        self.menu
            .as_ref()
            .is_some_and(|menu| menu.is_open_for(source))
    }

    /// Also shows any pop-up that waited for the menu.
    pub fn close_menu(&mut self, cx: &mut Context<Self>) {
        if !self.menu_open() {
            return;
        }
        if let Some(menu) = &mut self.menu {
            menu.close();
        }
        debug!("menu closed");
        self.sync_grab(cx);
        if self.popup_waiting() {
            self.set_mode(Mode::Collapsed, cx);
        }
        self.kick(cx);
    }

    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.close_menu(cx);
        self.set_mode(Mode::Collapsed, cx);
    }

    pub fn menu_enter(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.as_mut().filter(|menu| menu.is_open()) {
            menu.enter(index);
            self.kick(cx);
        }
    }

    pub fn menu_back(&mut self, cx: &mut Context<Self>) {
        if let Some(menu) = self.menu.as_mut().filter(|menu| menu.is_open()) {
            menu.back();
            self.kick(cx);
        }
    }

    pub fn menu_pick(&mut self, id: i32, cx: &mut Context<Self>) {
        let Some(pick) = self
            .menu
            .as_ref()
            .filter(|menu| menu.is_open())
            .map(Menu::pick)
        else {
            return;
        };
        self.close_menu(cx);
        pick(id, self, cx);
    }

    fn grab(&self) -> Grab {
        let wants_keys = |panel: Panel| {
            panel.takes_keyboard()
                && match panel {
                    Panel::Feature(name) => self.local(name).is_none_or(|local| local.keyboard()),
                    _ => true,
                }
        };
        if matches!(self.mode, Mode::Open(panel) if wants_keys(panel)) {
            Grab::Keyboard
        } else if self.menu_open() {
            Grab::Clicks
        } else {
            Grab::None
        }
    }

    fn sync_grab(&mut self, cx: &mut Context<Self>) {
        let grab = self.grab();
        if grab != self.grabbed {
            self.grabbed = grab;
            cx.emit(IslandEvent::Grab(grab));
        }
    }

    pub fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        let mode = match mode {
            Mode::Collapsed if self.popup_waiting() => Mode::Open(Panel::Popups),
            mode => mode,
        };
        if self.mode == mode {
            return;
        }
        self.panel_watch = None;
        if let Mode::Open(Panel::Feature(name)) = self.mode {
            self.update_local(name, |local| local.panel_closed());
        }
        // A menu hangs from the bar, so it goes at once when the island morphs into a panel.
        if mode != Mode::Collapsed {
            self.menu = None;
        }
        debug!(from = ?self.mode, to = ?mode, "mode changed");
        self.closing_panel = mode == Mode::Collapsed;
        self.mode = mode;
        self.sync_grab(cx);
        if let Mode::Open(Panel::Feature(name)) = mode
            && let Some(feature) = self.features.get(name)
        {
            self.panel_watch = feature.panel_opened(cx);
        }
        // The overview shows what the dashboard's views show, so it keeps what they watch going.
        if mode == Mode::Open(Panel::Dashboard) {
            let watches: Vec<Box<dyn Any>> = features::dashboard_tabs()
                .into_iter()
                .filter_map(|(name, _, _)| self.features.get(name)?.panel_opened(cx))
                .collect();
            self.panel_watch = Some(Box::new(watches));
        }
        self.scroll.set_offset(point(px(0.0), px(0.0)));
        for column in &self.columns {
            column.set_offset(point(px(0.0), px(0.0)));
        }
        self.opened.set(true);
        self.retarget();

        // System's and the dashboard's graphs slide with the clock; a few px a second needs no more
        // than the ambient tick, not every vblank.
        let refresh = match mode {
            Mode::Open(Panel::System | Panel::Dashboard) => Some(marquee::TICK),
            Mode::Open(Panel::Feature(name)) => features::panel_refresh(name),
            Mode::Open(Panel::Popups) => self
                .locals
                .iter()
                .filter(|(_, local)| local.popup())
                .filter_map(|(name, _)| features::panel_refresh(name))
                .min(),
            _ => None,
        };
        self.sys_watch =
            matches!(mode, Mode::Open(kind) if kind.shows_stats()).then(|| self.samples.watch());
        self.sys_task = refresh.map(|every| {
            cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(every).await;
                    if this.update(cx, |_, cx| cx.notify()).is_err() {
                        break;
                    }
                }
            })
        });
        self.panel_task = None;
        let shown = mode == Mode::Open(Panel::Popups);
        for (_, local) in &mut self.locals {
            local.popup_shown(shown);
        }
        self.arm_due(cx);
        self.leave_task = None;
        self.kick(cx);
    }

    fn panel_shape(&self, kind: Panel) -> Option<Shape> {
        self.frozen[kind.layer()].map(|(_, shape)| shape)
    }

    fn collect_measurements(&mut self) {
        for index in 0..LAYERS {
            if let (Some(size), Some(key)) = (self.sizes[index].take(), self.pending[index].take())
            {
                self.frozen[index] = Some((
                    key,
                    Shape {
                        w: f32::from(size.width).ceil(),
                        h: f32::from(size.height).ceil(),
                    },
                ));
            }
        }
    }

    fn panel_key(&self, panel: &panel::PanelView) -> u64 {
        let mut hasher = DefaultHasher::new();
        panel.key().hash(&mut hasher);
        hasher.finish()
    }

    /// Section widths with every folding widget at its `fold` position. The probes report last
    /// frame's layout, so each folding widget's part as it was drawn then is swapped for its part now
    /// or at rest; the island edge then tracks the content exactly instead of chasing it.
    fn section_widths(&self, fold: Fold) -> Option<[f32; 3]> {
        let mut widths = [0.0; 3];
        for (width, probe) in widths.iter_mut().zip(&self.sections) {
            *width = f32::from(probe.get()?.width);
        }
        let mut add = |kind, extra| {
            if let Some(index) = self.layout.section_of(kind) {
                widths[index] += extra;
            }
        };
        if fold == Fold::Target {
            for (name, local) in &self.locals {
                let widget = self
                    .features
                    .get(name)
                    .and_then(|feature| feature.widgets().first().copied());
                if let Some(widget) = widget {
                    add(Widget(widget), local.pending_width(self.layout.spacing));
                }
            }
        }
        for slot in self.held.iter() {
            let Some(kind) = self
                .layout
                .widgets()
                .find(|kind| bar::presence_key(*kind) == slot.key)
            else {
                continue;
            };
            let natural = self
                .natural
                .get(&kind)
                .and_then(|probe| probe.get())
                .map_or(0.0, |size| f32::from(size.width));
            let span = (natural - bar::rest_width(kind, &self.layout, &self.features)).max(0.0);
            let drawn = self.drawn.get(&slot.key).copied().unwrap_or(0.0);
            let wanted = match fold {
                Fold::Target => slot.presence.target(),
                Fold::Now => slot.presence.value().clamp(0.0, 1.0),
            };
            add(kind, span * (wanted - drawn));
        }
        Some(widths)
    }

    fn folding(&self) -> bool {
        !self.held.is_settled()
    }

    /// Keeps the last state of every feature widget that has something to show, so a widget that
    /// loses its data can still be drawn while it folds away.
    fn sync_held(&mut self, cx: &mut Context<Self>) {
        let items: Vec<(String, bar::Held)> = self
            .layout
            .widgets()
            .filter_map(|kind| {
                let Widget(widget) = kind;
                let feature = self
                    .features
                    .owning(widget)
                    .filter(|feature| feature.present(widget, cx))?;
                Some((bar::presence_key(kind).to_owned(), feature.snapshot(cx)))
            })
            .collect();
        self.held.sync(&items, self.sized);
    }

    pub fn feature_changed(&mut self, name: &'static str, cx: &mut Context<Self>) {
        let widgets = self
            .features
            .get(name)
            .map(|feature| feature.widgets())
            .unwrap_or_default();
        for widget in widgets {
            if let Some((view, _)) = self.widgets.get(&Widget(widget)) {
                view.update(cx, |_, cx| cx.notify());
            }
        }
        let animate = self.sized;
        let monitor = self.island_monitor().cloned();
        let here = IslandInfo {
            monitor: monitor.as_ref(),
            panel_open: self.mode == Mode::Open(Panel::Feature(name)),
        };
        if let (Some(feature), Some((_, local))) = (
            self.features.get(name),
            self.locals.iter_mut().find(|(owner, _)| *owner == name),
        ) {
            feature.sync_local(&mut **local, &here, animate, cx);
        }
        self.sync_held(cx);
        if matches!(
            self.mode,
            Mode::Collapsed | Mode::Open(Panel::System | Panel::Dashboard)
        ) || self.mode == Mode::Open(Panel::Popups)
            || matches!(self.mode, Mode::Open(Panel::Feature(open)) if open == name)
        {
            self.retarget();
        }
        self.kick(cx);
    }

    fn sync_locals(&mut self, monitor: Option<&Monitor>, cx: &Gpui) {
        let animate = self.sized;
        for (name, local) in &mut self.locals {
            if let Some(feature) = self.features.get(name) {
                let here = IslandInfo {
                    monitor,
                    panel_open: self.mode == Mode::Open(Panel::Feature(name)),
                };
                feature.sync_local(&mut **local, &here, animate, cx);
            }
        }
    }

    fn panel_cx<'a>(&'a self, name: &str, cx: &'a Gpui) -> PanelCx<'a> {
        let lend = move |lender: &str| {
            let (_, local) = self.locals.iter().find(|(owner, _)| *owner == lender)?;
            self.features.get(lender)?.lead(&**local, cx)
        };
        PanelCx::new(
            cx.global::<Glyphs>().clone(),
            self.local(name),
            &self.panel_due,
            Box::new(lend),
        )
    }

    pub fn feature_panel(&self, name: &str, cx: &Gpui) -> Option<panel::PanelView> {
        self.features
            .get(name)?
            .panel(&mut self.panel_cx(name, cx), cx)
    }

    pub fn popups(&self, cx: &Gpui) -> Option<panel::PanelView> {
        let blocks: Vec<_> = self
            .locals
            .iter()
            .filter(|(_, local)| local.popup())
            .filter_map(|(name, _)| {
                self.features
                    .get(name)?
                    .popup(&mut self.panel_cx(name, cx), cx)
            })
            .flat_map(|view| view.blocks)
            .collect();
        (!blocks.is_empty()).then_some(panel::PanelView {
            header: panel::Header::None,
            blocks,
            scroll: false,
        })
    }

    /// What every running feature puts on the dashboard's overview, in their places.
    pub fn dashboard(&self, cx: &Gpui) -> (features::Dashboard, Vec<Battery>) {
        let mut features: Vec<_> = self.features.iter().collect();
        features.sort_by_key(|feature| feature.dashboard_order());
        let mut all = features::Dashboard::default();
        let mut batteries = Vec::new();
        for feature in features {
            let part = feature.dashboard(&mut self.panel_cx(feature.name(), cx), cx);
            all.switches.extend(part.switches);
            all.cards.extend(part.cards);
            all.warnings.extend(part.warnings);
            batteries.extend(feature.batteries(cx));
        }
        (all, batteries)
    }

    fn bar_width(&self) -> Option<f32> {
        self.bar_width_at(Fold::Target)
    }

    fn bar_width_at(&self, fold: Fold) -> Option<f32> {
        let [left, middle, right] = self.section_widths(fold)?;
        Some(2.0 * bar::edge(&self.layout) + middle + 2.0 * left.max(right))
    }

    fn collapsed_width(&self, bar: f32, hovered: bool) -> f32 {
        if hovered { bar + 4.0 * HOVER_GROW } else { bar }
    }

    fn collapsed_shape(&self, hovered: bool) -> Shape {
        let w = self.bar_width().unwrap_or(0.0);
        if hovered {
            Shape {
                w: w + 4.0 * HOVER_GROW,
                h: COLLAPSED_H,
            }
        } else {
            Shape { w, h: COLLAPSED_H }
        }
    }

    /// The name of the output it is on, such as `DP-1`.
    pub fn output(&self) -> Option<String> {
        self.island_monitor().map(|monitor| monitor.name.clone())
    }

    fn island_monitor(&self) -> Option<&Monitor> {
        let monitors = &self.workspaces.as_ref()?.monitors;
        self.display
            .and_then(|display| {
                monitors.iter().find(|monitor| {
                    Uuid::new_v5(&Uuid::NAMESPACE_DNS, monitor.name.as_bytes()) == display
                })
            })
            .or_else(|| monitors.iter().find(|monitor| monitor.focused))
    }

    /// Asks the compositor where the window is, else assumes it is centred at the top of its
    /// monitor.
    pub fn to_screen(&self, point: Point<Pixels>) -> Option<(i32, i32)> {
        let monitor = self.island_monitor()?;
        let (left, top) = compositor::backend()
            .and_then(|backend| backend.surface_origin(&monitor.name))
            .unwrap_or((
                monitor.x + (monitor.width - SURFACE_W as i32) / 2,
                monitor.y,
            ));
        Some((
            left + f32::from(point.x).round() as i32,
            top + f32::from(point.y).round() as i32,
        ))
    }

    /// Where a menu an app draws itself should hang from.
    pub fn below_bar(&self, x: Pixels) -> Option<(i32, i32)> {
        self.to_screen(point(x, px(self.top + COLLAPSED_H + MENU_GAP)))
    }

    fn sync_workspaces(&mut self, window: &Window, cx: &mut Context<Self>) {
        let display = window.display(cx).and_then(|display| display.uuid().ok());
        if display == self.display && !self.monitor_stale {
            return;
        }
        self.display = display;
        self.monitor_stale = false;
        let monitor = self.island_monitor().cloned();
        self.fullscreen = monitor.as_ref().is_some_and(|monitor| monitor.fullscreen);
        if self.fullscreen {
            self.close_menu(cx);
        }
        self.sync_locals(monitor.as_ref(), cx);
        if self.mode == Mode::Collapsed {
            self.retarget();
        }
        self.start_frames();
    }

    pub fn toggle(&mut self, kind: Panel, cx: &mut Context<Self>) {
        let mode = if self.mode == Mode::Open(kind) {
            Mode::Collapsed
        } else {
            Mode::Open(kind)
        };
        self.set_mode(mode, cx);
    }

    /// Always redraws: everything clickable draws its hover from this, and a panel drawn while the
    /// pointer was away would otherwise keep showing no hover after it comes back.
    fn set_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.hovered == hovered {
            return;
        }
        self.hovered = hovered;
        let features = self.features.clone();
        for (name, local) in &mut self.locals {
            local.pointer(hovered);
            if let Some(feature) = features.get(name) {
                feature.pointer(&mut **local, hovered, cx);
            }
        }
        self.arm_due(cx);
        cx.notify();
        match self.mode {
            Mode::Collapsed => {
                self.retarget();
                self.kick(cx);
            }
            Mode::Open(Panel::Popups) => {}
            Mode::Open(_) if hovered => self.leave_task = None,
            Mode::Open(_) => {
                self.leave_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(LEAVE_GRACE).await;
                    let _ = this.update(cx, |this, cx| {
                        if !this.hovered {
                            this.set_mode(Mode::Collapsed, cx);
                        }
                    });
                }));
            }
        }
    }

    fn active_layer(&self) -> usize {
        match self.mode {
            Mode::Collapsed => COLLAPSED_LAYER,
            Mode::Open(kind) => kind.layer(),
        }
    }

    fn desired_shape(&self) -> Option<Shape> {
        match self.mode {
            Mode::Collapsed if self.fullscreen => Some(Shape { w: 0.0, h: 0.0 }),
            Mode::Collapsed => self.bar_width().map(|_| self.collapsed_shape(self.hovered)),
            Mode::Open(kind) => self.panel_shape(kind),
        }
    }

    fn apply_shape(&mut self) -> bool {
        let Some(shape) = self.desired_shape() else {
            return false;
        };
        if !self.sized {
            self.w.reset(shape.w);
            self.h.reset(shape.h);
            self.sized = true;
            return true;
        }
        if (self.w.target() - shape.w).abs() < 0.5 && (self.h.target() - shape.h).abs() < 0.5 {
            return false;
        }
        self.w.retarget(shape.w, SHAPE_SPRING);
        self.h.retarget(shape.h, SHAPE_SPRING);
        true
    }

    fn retarget(&mut self) {
        self.apply_shape();

        let active = match self.mode {
            Mode::Collapsed => COLLAPSED_LAYER,
            Mode::Open(kind) => kind.layer(),
        };
        let reveal_at = Instant::now() + REVEAL_DELAY;
        let fade_in = |spring: &mut Spring| {
            if spring.target() == 1.0 {
                return;
            }
            if spring.value() < 0.05 {
                spring.retarget_at(1.0, CONTENT_IN, reveal_at);
            } else {
                spring.retarget(1.0, CONTENT_IN);
            }
        };
        for (index, layer) in self.layers.iter_mut().enumerate() {
            if index == active {
                fade_in(layer);
            } else {
                layer.retarget(0.0, CONTENT_OUT);
            }
        }

        match self.mode {
            Mode::Open(kind) if bar::tab_index(kind).is_some() => {
                self.strip.pick(kind, self.chrome.value() < 0.05);
                fade_in(&mut self.chrome);
            }
            _ => self.chrome.retarget(0.0, CONTENT_OUT),
        }
    }

    pub(crate) fn kick(&mut self, cx: &mut Context<Self>) {
        self.start_frames();
        cx.notify();
    }

    fn start_frames(&mut self) {
        if self.last_frame.is_none() {
            let now = Instant::now();
            self.last_frame = Some(now);
            self.anim = Some(AnimStats {
                started: now,
                frames: 0,
                max_gap: Duration::ZERO,
            });
        }
    }

    fn is_settled(&self) -> bool {
        self.still(|local| local.is_settled())
    }

    /// Unsettled only by features' slow motion, such as a lively mascot breathing.
    fn is_drifting(&self) -> bool {
        self.still(|local| local.is_settled() || local.is_drifting())
    }

    fn still(&self, local_still: impl Fn(&dyn AnyLocal) -> bool) -> bool {
        self.w.is_settled()
            && self.h.is_settled()
            && self.layers.iter().all(Spring::is_settled)
            && self.chrome.is_settled()
            && self.strip.is_settled()
            && self.locals.iter().all(|(_, local)| local_still(&**local))
            && self.held.is_settled()
            && self.menu.as_ref().is_none_or(Menu::is_settled)
    }

    fn step(&mut self, window: &mut Window) {
        let Some(last) = self.last_frame else {
            return;
        };
        let now = Instant::now();
        let gap = now - last;
        let dt = gap.min(MAX_STEP).as_secs_f32();

        self.w.advance(dt, now);
        self.h.advance(dt, now);
        self.chrome.advance(dt, now);
        self.strip.advance(dt, now);
        for (_, local) in &mut self.locals {
            local.advance(dt, now);
        }
        self.held.advance(dt, now);
        for layer in &mut self.layers {
            layer.advance(dt, now);
        }
        if let Some(menu) = &mut self.menu {
            menu.advance(dt, now);
        }
        if self.menu.as_ref().is_some_and(Menu::is_gone) {
            self.menu = None;
        }

        if let Some(anim) = &mut self.anim {
            if anim.frames > 0 {
                anim.max_gap = anim.max_gap.max(gap);
            }
            anim.frames += 1;
        }

        self.drifting = false;
        if self.is_settled() {
            self.last_frame = None;
            if let Some(anim) = self.anim.take() {
                debug!(
                    target: "island::frames",
                    mode = ?self.mode,
                    frames = anim.frames,
                    took = ?anim.started.elapsed(),
                    worst_gap = ?anim.max_gap,
                    "animation settled"
                );
            }
        } else {
            self.last_frame = Some(now);
            // Slow motion goes on at the ticker's pace, which `keep_ticking` starts.
            self.drifting = self.is_drifting();
            if !self.drifting {
                window.request_animation_frame();
            }
        }
    }

    fn refresh_stats(&mut self) {
        if let Some((stats, at, revision)) = self.samples.since(self.revision) {
            self.stats = stats;
            self.last_sample = at;
            self.revision = revision;
        }
    }

    fn layer_element(
        &self,
        index: usize,
        shape: (f32, f32),
        place: Place,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let opacity = self.layers[index].value().clamp(0.0, 1.0);
        let (w, h) = shape;
        let drift = (1.0 - opacity) * CONTENT_DRIFT;
        let mut content_w = None;
        let content = match Panel::from_layer(index) {
            None => {
                let bar = bar::bar(self.bar_parts(&self.layouts, cx), &|kind| {
                    self.widget_element(kind)
                });
                content_w = self.bar_size.get().map(|size| f32::from(size.width));
                div()
                    .relative()
                    .h(px(COLLAPSED_H))
                    .child(bar)
                    .child(measure::probe(&self.bar_size))
                    .into_any_element()
            }
            Some(kind) => {
                let before = self.panel_due.get();
                let panel = panels::panel(self, kind, cx);
                if index != self.active_layer() {
                    self.panel_due.set(before);
                }
                let key = self.panel_key(&panel);
                let frozen = self.frozen[index].filter(|(frozen_key, _)| *frozen_key == key);
                let frame = Frame {
                    hover: self.hovered,
                    scroll: &self.scroll,
                    columns: &self.columns,
                    placed: &self.placed,
                    opened: &self.opened,
                    island_h: h,
                    min_width: frozen.map_or(0.0, |(_, shape)| shape.w),
                    max_width: frozen.map_or(SURFACE_W - 2.0 * SHADOW_SIDE, |(_, shape)| shape.w),
                    max_height: frozen
                        .map_or(SURFACE_H - self.top - SHADOW_BELOW, |(_, shape)| shape.h),
                    phase: self.last_sample.elapsed().as_secs_f32() / sys::INTERVAL.as_secs_f32(),
                    layouts: &self.layouts,
                    tips: &self.tips,
                    popovers: &self.popovers,
                    glass: ui::surface_fill(self.opacity),
                };
                let content = panel.render(&frame, cx);
                match frozen {
                    Some((_, shape)) => {
                        content_w = Some(shape.w);
                        div()
                            .w(px(shape.w))
                            .h(px(shape.h))
                            .child(content)
                            .into_any_element()
                    }
                    None => {
                        self.pending[index].set(Some(key));
                        div()
                            .relative()
                            .child(content)
                            .child(measure::probe(&self.sizes[index]))
                            .into_any_element()
                    }
                }
            }
        };

        let layer = div()
            .absolute()
            .top(px(-drift))
            .left_0()
            .w(px(w))
            .h(px(h))
            .opacity(opacity);
        match content_w {
            Some(content_w) => {
                let screen_left =
                    ((place.viewport - content_w) / 2.0 * place.scale).round() / place.scale;
                let top = if index == COLLAPSED_LAYER {
                    ((h - COLLAPSED_H) / 2.0).max(0.0)
                } else {
                    0.0
                };
                layer
                    .child(
                        div()
                            .absolute()
                            .left(px(screen_left - place.x))
                            .top(px(top))
                            .flex_none()
                            .child(content),
                    )
                    .into_any_element()
            }
            None => layer
                .flex()
                .justify_center()
                .when(index == COLLAPSED_LAYER, |layer| layer.items_center())
                .when(index != COLLAPSED_LAYER, |layer| layer.items_start())
                .child(div().flex_none().child(content))
                .into_any_element(),
        }
    }

    fn chrome_element(&self, w: f32, place: Place, cx: &mut Context<Self>) -> AnyElement {
        let opacity = self.chrome.value().clamp(0.0, 1.0);
        let strip_screen =
            ((place.viewport - self.strip.width()) / 2.0 * place.scale).round() / place.scale;
        let strip_left = strip_screen - place.x - INSET;
        div()
            .absolute()
            .left(px(INSET))
            .top(px(INSET - (1.0 - opacity) * CONTENT_DRIFT))
            .w(px(w - 2.0 * INSET))
            .h(px(HEADER_H))
            .opacity(opacity)
            .child(bar::tabs(
                &self.strip,
                strip_left,
                self.hovered,
                &self.tips,
                cx,
            ))
            .into_any_element()
    }
}

impl Render for Island {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.rendered = Instant::now();
        if self.slots.is_none() {
            self.slots = Some(bar::measure_slots(window));
        }
        self.layouts.begin_frame();
        if self.glow > 0.0 {
            let levels = self.features.iter().find_map(|feature| feature.glow(cx));
            if self
                .aura
                .update(levels.as_deref().unwrap_or_default(), levels.is_some())
            {
                window.request_animation_frame();
            }
        }
        let hovered = window.is_window_hovered();
        if hovered != self.hovered {
            cx.defer_in(window, move |this, _, cx| this.set_hovered(hovered, cx));
        }
        let measured: Vec<&'static str> = self
            .locals
            .iter_mut()
            .filter_map(|(name, local)| local.measure(window).then_some(*name))
            .collect();
        for name in measured {
            self.local_changed(name, cx);
        }
        bar::measure_labels(window);
        if matches!(self.mode, Mode::Open(kind) if kind.shows_stats())
            || self.frozen[Panel::System.layer()].is_none()
            || self.frozen[Panel::Dashboard.layer()].is_none()
        {
            self.refresh_stats();
        }
        self.sync_workspaces(window, cx);
        self.collect_measurements();
        if self.apply_shape() {
            self.start_frames();
        }
        let rest_radius = self.corner.0.min(COLLAPSED_H / 2.0);
        let limit = self
            .bar_width()
            .map(|bar| bar - 2.0 * (rest_radius + menu::FILLET));
        if let Some(menu) = &mut self.menu {
            menu.set_limit(limit.unwrap_or(menu::MAX_W));
            if menu.collect() {
                self.start_frames();
            }
        }
        // Drifting moves only features' own widgets, which `redraw_drifting` redraws; the rest of
        // the bar can be reused.
        self.live = self.last_frame.is_some() && !self.is_drifting();
        self.step(window);
        if self.closing_panel
            && self.w.is_settled()
            && self.h.is_settled()
            && self.layers.iter().all(Spring::is_settled)
            && self.chrome.is_settled()
        {
            self.closing_panel = false;
        }
        let layer = if self.mode != Mode::Collapsed || self.menu.is_some() || self.closing_panel {
            Layer::Overlay
        } else {
            Layer::Top
        };
        if layer != self.layer {
            debug!(?layer, "layer changed");
            window.set_layer(layer);
            self.layer = layer;
        }
        if self.mode == Mode::Collapsed
            && !self.fullscreen
            && self.folding()
            && let Some(width) = self.bar_width_at(Fold::Now)
        {
            self.w.reset(self.collapsed_width(width, self.hovered));
        }

        let w = self.w.value();
        let h = self.h.value();
        let scale = window.scale_factor();
        let x = ((f32::from(window.viewport_size().width) - w) / 2.0 * scale).round() / scale;
        let y = self.top;

        let body = Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        };
        let corner = self.corner;
        self.popovers.set_within(body);
        // Kept within the bar at rest, so the bar growing under the pointer never moves the menu.
        // It is drawn before it shows, so its contents get measured.
        let rest_w = self.bar_width().map_or(w, |rest| rest.min(w));
        let rest_x =
            ((f32::from(window.viewport_size().width) - rest_w) / 2.0 * scale).round() / scale;
        let rest = Bounds {
            origin: point(px(rest_x), px(y)),
            size: size(px(rest_w), px(h)),
        };
        let hung = self
            .menu
            .as_ref()
            .map(|menu| menu.hang(rest, corner.0.min(h / 2.0), scale));
        let hang = hung.filter(|hang| f32::from(hang.bounds.size.height) >= 0.5);
        let floating = self.popovers.reached();
        let region: Vec<Bounds<Pixels>> = std::iter::once(body)
            .chain(hang.map(|hang| hang.bounds))
            .chain(floating.iter().map(|reach| reach.bounds))
            .collect();
        if self.input_region != region {
            window.set_input_region(Some(&region));
            self.input_region = region;
        }
        let awake = self
            .features
            .iter()
            .any(|feature| feature.inhibits_idle(cx));
        if let Some(effects) = self.effects.get_or_insert_with(|| Effects::attach(window)) {
            let mut blur = shape::region(body, corner.0, corner.1, hang);
            for reach in &floating {
                blur.extend(shape::region(reach.bounds, reach.radius, reach.power, None));
            }
            effects.set_blur(blur);
            effects.set_inhibit(awake);
        }

        let place = Place {
            viewport: f32::from(window.viewport_size().width),
            scale: window.scale_factor(),
            x,
        };
        let hidden = self.hidden();
        self.panel_due.set(None);
        let mut layers: Vec<AnyElement> = (0..LAYERS)
            .filter(|index| {
                if *index == COLLAPSED_LAYER && hidden {
                    return false;
                }
                self.layers[*index].value() > 0.001
                    || *index == self.active_layer()
                    || (*index != COLLAPSED_LAYER && self.frozen[*index].is_none())
            })
            .map(|index| self.layer_element(index, (w, h), place, cx))
            .collect();
        // A panel drawn just to be measured needs one more frame to take its new size, even once
        // everything else has settled.
        if self.pending.iter().any(|pending| pending.get().is_some()) {
            window.request_animation_frame();
        }
        if self.chrome.value() > 0.001 {
            layers.push(self.chrome_element(w, place, cx));
        }

        let surface = shape::joined(
            body,
            corner.0,
            corner.1,
            hang,
            ui::surface_fill(self.opacity),
            Some(ui::surface_border()),
        );
        let bar_shown = self.layers[COLLAPSED_LAYER].value().clamp(0.0, 1.0);
        let glow =
            (self.glow > 0.0 && !hidden && bar_shown > 0.001 && self.aura.is_lit()).then(|| {
                div()
                    .absolute()
                    .size_full()
                    .opacity(bar_shown)
                    .child(self.aura.element(self.glow, corner.0))
            });

        let island = div()
            .id("island")
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(w))
            .h(px(h))
            .overflow_hidden()
            .on_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| this.close_menu(cx)))
            .children(glow)
            .children(layers);
        let menu = hung
            .zip(self.menu.as_ref())
            .map(|(hang, menu)| menu.element(&hang, self.hovered, cx));
        if self.menu.as_ref().is_some_and(Menu::is_pending) {
            window.request_animation_frame();
        }
        let shadow = ui::drop_shadow(body, corner.0, hang, self.shadow);

        self.drawn = self
            .held
            .iter()
            .map(|slot| (slot.key.clone(), slot.presence.value().clamp(0.0, 1.0)))
            .collect();
        let (popover, moving) = self.popovers.overlay();
        if moving {
            window.request_animation_frame();
        }
        let tooltip = self.tips.overlay(window);
        self.layouts.resolve(window);
        self.keep_ticking(cx);
        self.arm_panel_due(cx);
        div()
            .size_full()
            .child(shadow)
            .child(surface)
            .child(island)
            .children(menu)
            .children(popover)
            .children(tooltip)
    }
}
