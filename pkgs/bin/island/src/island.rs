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
    MouseDownEvent, Pixels, Point, Render, ScrollDelta, ScrollEvent, ScrollHandle, StyleRefinement,
    Subscription, Task, WeakEntity, Window, div, point, prelude::*, px, size,
};
use tracing::debug;

use crate::{
    bar, components,
    components::measure::Measure,
    components::{
        aura::Aura,
        icon,
        marquee::{self, TextLayouts},
        measure::{self, Measured},
        popover::Popovers,
        tooltip::Tooltips,
    },
    compositor,
    config::{self, Widget},
    effects::Effects,
    features::{self, AnyLocal, Battery, Features, Outcome},
    glyphs::Glyphs,
    key::Key,
    menu::{self, Menu, MenuNode, Pick},
    panel::{self, Frame},
    panels, shape,
    sys::{self, Samples, SysStats},
    ui::{self, HEADER_H, INSET},
    wm::Snapshot,
};

pub use crate::stage::{
    COLLAPSED_H, COLLAPSED_LAYER, CONTENT_DRIFT, CONTENT_IN, CONTENT_OUT, Grab, LAYERS, Mode,
    Panel, SHAPE_SPRING, SURFACE_H, SURFACE_W, Shape, Stage, collapsed_width,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fold {
    Now,
    Target,
}
const MAX_STEP: Duration = Duration::from_millis(50);
/// How far the drop shadow reaches past the island's sides and bottom: three sigmas of its blur,
/// plus its drop below.
const SHADOW_SIDE: f32 = 45.0;
const SHADOW_BELOW: f32 = 55.0;
const LINES_PER_NOTCH: f32 = 3.0;
const PIXELS_PER_NOTCH: f32 = 50.0;

/// How far `event` scrolled, in notches and parts of one.
pub fn scrolled(event: &ScrollEvent) -> f32 {
    match event.delta {
        ScrollDelta::Lines(lines) => lines.y / LINES_PER_NOTCH,
        ScrollDelta::Pixels(pixels) => f32::from(pixels.y) / PIXELS_PER_NOTCH,
    }
}

const LEAVE_GRACE: Duration = Duration::from_millis(500);

#[derive(Default)]
pub struct Placement {
    pub islands: Vec<WeakEntity<Island>>,
}

impl Global for Placement {}

pub enum IslandEvent {
    Grab(Grab),
}

impl EventEmitter<IslandEvent> for Island {}

#[derive(Clone, Copy)]
struct Place {
    viewport: f32,
    scale: f32,
    x: f32,
}

/// Between the bar and a menu an app draws itself.
const MENU_GAP: f32 = 6.0;

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
    stage: Stage,
    slots: Option<bar::Slots>,
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
    menu: Option<Menu>,
    /// Each panel's own, so one leaving stays where it was scrolled to as the next opens.
    scroll: [ScrollHandle; LAYERS],
    /// Each column of a panel's side-by-side columns scrolls on its own.
    columns: [[ScrollHandle; panel::COLUMNS]; LAYERS],
    /// A panel's one scrolling list of placed blocks, which draws only what is in view.
    placed: [ScrollHandle; LAYERS],
    /// Set as a panel opens, so a column it follows starts at its end.
    opened: Cell<bool>,
    sizes: [Measured; LAYERS],
    pending: [Cell<Option<u64>>; LAYERS],
    /// The grab last asked of the shell.
    grabbed: Grab,
    bar_size: Measured,
    sections: [Measured; 3],
    tips: Tooltips,
    popovers: Popovers,
    layouts: TextLayouts,
    natural: HashMap<Widget, Measured>,
    drawn: HashMap<String, f32>,
    last_sample: Instant,
    top: f32,
    /// Redraws while text is scrolling or features alone are moving, at the pace it holds.
    ticker: Option<(Duration, Task<()>)>,
    /// Moving only by features' own motion, so drawn at this pace rather than every frame the
    /// display has.
    pace: Option<Duration>,
    rendered: Instant,
    widgets: HashMap<Widget, (Entity<bar::WidgetView>, Measured)>,
    /// Whether this frame changes anything beyond what widgets redraw themselves, so every widget is
    /// drawn afresh instead of reused.
    live: bool,
    _subscriptions: Vec<Subscription>,
}

impl Island {
    pub fn new(startup: Startup, cx: &mut Context<Self>) -> Self {
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
            this.stage.sync_held();
            if matches!(
                this.stage.mode,
                Mode::Collapsed | Mode::Open(Panel::System | Panel::Dashboard)
            ) {
                this.retarget();
            }
            this.kick(cx);
        });
        let natural = startup
            .layout
            .widgets()
            .map(|kind| (kind, measure::measured()))
            .collect();
        Self {
            stage: Stage::new(startup.layout, startup.features, startup.workspaces),
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
            menu: None,
            scroll: std::array::from_fn(|_| ScrollHandle::new()),
            columns: std::array::from_fn(|_| std::array::from_fn(|_| ScrollHandle::new())),
            placed: std::array::from_fn(|_| ScrollHandle::new()),
            opened: Cell::new(false),
            sizes: std::array::from_fn(|_| measure::measured()),
            pending: std::array::from_fn(|_| Cell::new(None)),
            grabbed: Grab::None,
            bar_size: measure::measured(),
            sections: std::array::from_fn(|_| measure::measured()),
            tips: Tooltips::drawn_by(cx.entity_id()),
            popovers: Popovers::default(),
            drawn: HashMap::new(),
            natural,
            layouts: TextLayouts::default(),
            last_sample: Instant::now(),
            slots: None,
            top: startup.top,
            ticker: None,
            pace: None,
            rendered: Instant::now(),
            widgets,
            live: true,
            _subscriptions: vec![glyphs_subscription],
        }
    }

    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    /// Follow with [`Island::local_changed`].
    pub fn update_local(&mut self, name: &str, action: impl FnOnce(&mut dyn AnyLocal)) {
        self.stage.update_local(name, action);
    }

    pub fn feature_panel(&self, name: &str, cx: &Gpui) -> Option<panel::PanelView> {
        self.stage
            .feature_panel(name, cx.global::<Glyphs>(), &self.panel_due)
    }

    pub fn popups(&self, cx: &Gpui) -> Option<panel::PanelView> {
        self.stage.popups(cx.global::<Glyphs>(), &self.panel_due)
    }

    pub fn dashboard(&self, cx: &Gpui) -> (features::Dashboard, Vec<Battery>) {
        self.stage.dashboard(cx.global::<Glyphs>(), &self.panel_due)
    }

    pub fn input_region(&self) -> &[Bounds<Pixels>] {
        &self.input_region
    }

    pub fn key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        let Mode::Open(Panel::Feature(name)) = self.stage.mode else {
            return;
        };
        let features = self.stage.features.clone();
        let Some(feature) = features.get(name) else {
            return;
        };
        let key = Key::from(keystroke);
        let mut outcome = Outcome::Close;
        self.stage
            .update_local(name, |local| outcome = feature.key(local, &key));
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
        let closing = toggle && self.stage.mode == Mode::Open(kind);
        if let (Some(view), Panel::Feature(feature), false) = (view, kind, closing) {
            self.stage.update_local(feature, |local| local.show(view));
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
        let every = self
            .pace
            .into_iter()
            .chain(self.layouts.scrolling().then_some(marquee::TICK))
            .min();
        if self.ticker.as_ref().map(|(every, _)| *every) == every {
            return;
        }
        self.ticker = every.map(|every| {
            let task = cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor().timer(every).await;
                    let tick = this.update(cx, |this, cx| {
                        if this.rendered.elapsed() >= every / 2 {
                            this.redraw_drifting(cx);
                            cx.notify();
                        }
                    });
                    if tick.is_err() {
                        break;
                    }
                }
            });
            (every, task)
        });
    }

    /// Redraws without re-fitting the bar, and not at all while the bar is not showing.
    pub fn feature_live(&mut self, name: &'static str, cx: &mut Context<Self>) {
        if self.stage.fullscreen || self.stage.morph.layers[COLLAPSED_LAYER].value() <= 0.001 {
            return;
        }
        let live = self
            .stage
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
        if self.pace.is_none() {
            return;
        }
        let views: Vec<Entity<bar::WidgetView>> = self
            .stage
            .locals
            .iter()
            .filter(|(_, local)| !local.is_settled())
            .filter_map(|(name, _)| self.stage.features.get(name))
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
        bar::content(kind, &self.bar_parts(layouts, cx), island)
    }

    fn bar_parts<'a>(&'a self, layouts: &'a TextLayouts, cx: &'a Gpui) -> bar::Bar<'a> {
        bar::Bar {
            layout: &self.stage.layout,
            glyphs: cx.global::<Glyphs>(),
            tips: &self.tips,
            layouts,
            held: &self.stage.held,
            features: &self.stage.features,
            locals: &self.stage.locals,
            natural: &self.natural,
            hover: self.stage.hovered,
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

    /// Shows the pop-ups once any has something, and takes them away once none has.
    pub fn local_changed(&mut self, name: &'static str, cx: &mut Context<Self>) {
        let waiting = self.stage.popup_waiting();
        match self.stage.mode {
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

    fn arm_due(&mut self, cx: &mut Context<Self>) {
        let next = self.stage.next_due();
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
        for name in self.stage.elapse(Instant::now()) {
            self.local_changed(name, cx);
        }
        self.arm_due(cx);
    }

    fn arm_panel_due(&mut self, cx: &mut Context<Self>) {
        let next = (self.stage.mode != Mode::Collapsed)
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

    pub fn set_workspaces(&mut self, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.stage.workspaces = Some(snapshot);
        self.stage.monitor_stale = true;
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
        if self.stage.mode != Mode::Collapsed || self.stage.fullscreen {
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
        if self.stage.popup_waiting() {
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

    fn sync_grab(&mut self, cx: &mut Context<Self>) {
        let grab = self.stage.grab(self.menu_open());
        if grab != self.grabbed {
            self.grabbed = grab;
            cx.emit(IslandEvent::Grab(grab));
        }
    }

    pub fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        let mode = self.stage.resolved(mode);
        if self.stage.mode == mode {
            return;
        }
        self.panel_watch = None;
        // What it was over may be gone, and nothing leaving tells it so.
        self.tips.clear();
        if let Some(previous) = self.stage.closing.take()
            && mode != Mode::Open(Panel::Feature(previous))
        {
            self.stage
                .update_local(previous, |local| local.panel_closed());
        }
        // Told it has closed once it is out of sight, so it does not change on its way out.
        if let Mode::Open(Panel::Feature(name)) = self.stage.mode {
            self.stage.closing = Some(name);
        }
        if let (Mode::Open(from), Mode::Open(to)) = (self.stage.mode, mode)
            && let (Some(from_tab), Some(to_tab)) = (bar::tab_index(from), bar::tab_index(to))
            && !cx.reduce_motion()
        {
            self.stage
                .morph
                .slide(from.layer(), to.layer(), to_tab > from_tab);
        }
        // A menu hangs from the bar, so it goes at once when the island morphs into a panel.
        if mode != Mode::Collapsed {
            self.menu = None;
        }
        debug!(from = ?self.stage.mode, to = ?mode, "mode changed");
        self.stage.mode = mode;
        self.sync_grab(cx);
        if let Mode::Open(Panel::Feature(name)) = mode
            && let Some(feature) = self.stage.features.get(name)
        {
            self.panel_watch = feature.panel_opened();
        }
        // The overview shows what the dashboard's views show, so it keeps what they watch going.
        if mode == Mode::Open(Panel::Dashboard) {
            let watches: Vec<Box<dyn Any>> = features::dashboard_tabs()
                .into_iter()
                .filter_map(|(name, _, _)| self.stage.features.get(name)?.panel_opened())
                .collect();
            self.panel_watch = Some(Box::new(watches));
        }
        if let Mode::Open(kind) = mode {
            let layer = kind.layer();
            self.scroll[layer].set_offset(point(px(0.0), px(0.0)));
            self.placed[layer].set_offset(point(px(0.0), px(0.0)));
            for column in &self.columns[layer] {
                column.set_offset(point(px(0.0), px(0.0)));
            }
        }
        self.opened.set(true);
        self.retarget();

        // System's and the dashboard's graphs slide with the clock; a few px a second needs no more
        // than the ambient tick, not every vblank.
        let refresh = match mode {
            Mode::Open(Panel::System | Panel::Dashboard) => Some(marquee::TICK),
            Mode::Open(Panel::Feature(name)) => features::panel_refresh(name),
            Mode::Open(Panel::Popups) => self
                .stage
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
        for (_, local) in &mut self.stage.locals {
            local.popup_shown(shown);
        }
        self.arm_due(cx);
        self.leave_task = None;
        self.kick(cx);
    }

    fn collect_measurements(&mut self) {
        for index in 0..LAYERS {
            if let (Some(size), Some(key)) = (self.sizes[index].take(), self.pending[index].take())
            {
                self.stage.morph.frozen[index] = Some((
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
            if let Some(index) = self.stage.layout.section_of(kind) {
                widths[index] += extra;
            }
        };
        if fold == Fold::Target {
            for (name, local) in &self.stage.locals {
                let widget = self
                    .stage
                    .features
                    .get(name)
                    .and_then(|feature| feature.widgets().first().copied());
                if let Some(widget) = widget {
                    add(
                        Widget(widget),
                        local.pending_width(self.stage.layout.spacing),
                    );
                }
            }
        }
        for slot in self.stage.held.iter() {
            let Some(kind) = self
                .stage
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
            let span = (natural - bar::rest_width(kind, &self.stage.layout, &self.stage.features))
                .max(0.0);
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
        !self.stage.held.is_settled()
    }

    pub fn feature_changed(&mut self, name: &'static str, cx: &mut Context<Self>) {
        icon::release(cx);
        let widgets = self
            .stage
            .features
            .get(name)
            .map(|feature| feature.widgets())
            .unwrap_or_default();
        for widget in widgets {
            if let Some((view, _)) = self.widgets.get(&Widget(widget)) {
                view.update(cx, |_, cx| cx.notify());
            }
        }
        self.stage.sync_local(name);
        self.stage.sync_held();
        if matches!(
            self.stage.mode,
            Mode::Collapsed | Mode::Open(Panel::System | Panel::Dashboard)
        ) || self.stage.mode == Mode::Open(Panel::Popups)
            || matches!(self.stage.mode, Mode::Open(Panel::Feature(open)) if open == name)
        {
            self.retarget();
        }
        self.kick(cx);
    }

    fn bar_width(&self) -> Option<f32> {
        self.bar_width_at(Fold::Target)
    }

    fn bar_width_at(&self, fold: Fold) -> Option<f32> {
        let [left, middle, right] = self.section_widths(fold)?;
        Some(2.0 * bar::edge(&self.stage.layout) + middle + 2.0 * left.max(right))
    }

    /// Asks the compositor where the window is, else assumes it is centred at the top of its
    /// monitor.
    pub fn to_screen(&self, point: Point<Pixels>) -> Option<(i32, i32)> {
        let monitor = self.stage.monitor()?;
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
        if display == self.stage.display && !self.stage.monitor_stale {
            return;
        }
        self.stage.display = display;
        self.stage.monitor_stale = false;
        let monitor = self.stage.monitor().cloned();
        let was_fullscreen = self.stage.fullscreen;
        self.stage.fullscreen = monitor.as_ref().is_some_and(|monitor| monitor.fullscreen);
        if self.stage.fullscreen {
            self.close_menu(cx);
            if !was_fullscreen && self.stage.mode == Mode::Collapsed {
                self.stage.morph.layers[COLLAPSED_LAYER].retarget(0.0, CONTENT_OUT);
            }
        }
        self.stage.sync_locals(monitor.as_ref());
        if self.stage.mode == Mode::Collapsed {
            self.retarget();
        }
        self.start_frames();
    }

    pub fn toggle(&mut self, kind: Panel, cx: &mut Context<Self>) {
        let mode = if self.stage.mode == Mode::Open(kind) {
            Mode::Collapsed
        } else {
            Mode::Open(kind)
        };
        self.set_mode(mode, cx);
    }

    /// Always redraws: everything clickable draws its hover from this, and a panel drawn while the
    /// pointer was away would otherwise keep showing no hover after it comes back.
    fn set_hovered(&mut self, hovered: bool, cx: &mut Context<Self>) {
        if self.stage.hovered == hovered {
            return;
        }
        self.stage.set_pointer(hovered);
        self.arm_due(cx);
        cx.notify();
        match self.stage.mode {
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
                        if !this.stage.hovered {
                            this.set_mode(Mode::Collapsed, cx);
                        }
                    });
                }));
            }
        }
    }

    fn apply_shape(&mut self) -> bool {
        let bar = self.bar_width();
        let shape = self.stage.desired_shape(bar);
        self.stage.morph.fit(shape)
    }

    fn retarget(&mut self) {
        self.apply_shape();
        let active = self.stage.active_layer();
        self.stage.morph.fade(active);
        match self.stage.mode {
            Mode::Open(kind) if bar::tab_index(kind).is_some() => {
                self.strip
                    .pick(kind, self.stage.morph.chrome.value() < 0.05);
                self.stage.morph.show_chrome();
            }
            _ => self.stage.morph.hide_chrome(),
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

    fn is_brisk(&self) -> bool {
        self.still(|local| local.is_settled() || local.is_drifting() || local.is_brisk())
    }

    fn feature_pace(&self) -> Option<Duration> {
        if self.is_drifting() {
            Some(marquee::TICK)
        } else {
            self.is_brisk().then_some(marquee::BRISK)
        }
    }

    fn still(&self, local_still: impl Fn(&dyn AnyLocal) -> bool) -> bool {
        self.stage.still(local_still)
            && self.strip.is_settled()
            && self.menu.as_ref().is_none_or(Menu::is_settled)
    }

    fn step(&mut self, window: &mut Window) {
        let Some(last) = self.last_frame else {
            return;
        };
        let now = Instant::now();
        let gap = now - last;
        let dt = gap.min(MAX_STEP).as_secs_f32();

        self.stage.advance(dt, now);
        self.strip.advance(dt, now);
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

        self.pace = None;
        if self.is_settled() {
            self.stage.finish_closing();
            self.last_frame = None;
            if let Some(anim) = self.anim.take() {
                debug!(
                    target: "island::frames",
                    mode = ?self.stage.mode,
                    frames = anim.frames,
                    took = ?anim.started.elapsed(),
                    worst_gap = ?anim.max_gap,
                    "animation settled"
                );
            }
        } else {
            self.last_frame = Some(now);
            // Features' own motion goes on at the ticker's pace, which `keep_ticking` starts.
            self.pace = self.feature_pace();
            if self.pace.is_none() {
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
        let faded = self.stage.morph.layers[index].value().clamp(0.0, 1.0);
        // Whole while it slides, unless the island is closing round it.
        let opacity =
            if self.stage.morph.slides[index].is_settled() || self.stage.mode == Mode::Collapsed {
                faded
            } else {
                1.0
            };
        let (w, h) = shape;
        let drift = (1.0 - opacity) * CONTENT_DRIFT;
        let across =
            (self.stage.morph.slides[index].value() * self.stage.morph.slide_width).round();
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
                if Some(index) != self.stage.active_layer() {
                    self.panel_due.set(before);
                }
                let key = self.panel_key(&panel);
                let frozen =
                    self.stage.morph.frozen[index].filter(|(frozen_key, _)| *frozen_key == key);
                let leaving = Cell::new(false);
                let frame = Frame {
                    hover: self.stage.hovered,
                    scroll: &self.scroll[index],
                    columns: &self.columns[index],
                    placed: &self.placed[index],
                    opened: if Some(index) == self.stage.active_layer() {
                        &self.opened
                    } else {
                        &leaving
                    },
                    island_h: h,
                    island_w: w,
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
                self.popovers
                    .enter_panel(faded, Some(index) == self.stage.active_layer());
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
            .left(px(across))
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
        let opacity = self.stage.morph.chrome.value().clamp(0.0, 1.0);
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
                self.stage.hovered,
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
            let levels = self
                .stage
                .features
                .iter()
                .find_map(|feature| feature.glow());
            if self
                .aura
                .update(levels.as_deref().unwrap_or_default(), levels.is_some())
            {
                window.request_animation_frame();
            }
        }
        let hovered = window.is_window_hovered();
        if hovered != self.stage.hovered {
            cx.defer_in(window, move |this, _, cx| this.set_hovered(hovered, cx));
        }
        let measured: Vec<&'static str> = self
            .stage
            .locals
            .iter_mut()
            .filter_map(|(name, local)| {
                let lines = |text: &str, size: f32, width: f32| {
                    components::text::wrapped_lines(text, size, width, window)
                };
                local.measure(&Measure::new(&lines)).then_some(*name)
            })
            .collect();
        for name in measured {
            self.local_changed(name, cx);
        }
        bar::measure_labels(&mut |title, size| {
            marquee::shape(window, title, size, gpui::FontWeight::SEMIBOLD).width
        });
        if matches!(self.stage.mode, Mode::Open(kind) if kind.shows_stats())
            || self.stage.morph.frozen[Panel::System.layer()].is_none()
            || self.stage.morph.frozen[Panel::Dashboard.layer()].is_none()
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
        // Features' own motion moves only their widgets, which `redraw_drifting` redraws; the rest
        // of the bar can be reused.
        self.live = self.last_frame.is_some() && self.feature_pace().is_none();
        self.step(window);
        if self.stage.mode == Mode::Collapsed
            && !self.stage.fullscreen
            && self.folding()
            && let Some(width) = self.bar_width_at(Fold::Now)
        {
            self.stage
                .morph
                .w
                .reset(collapsed_width(width, self.stage.hovered));
        }

        let w = self.stage.morph.w.value();
        let h = self.stage.morph.h.value();
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
            .chain(floating)
            .collect();
        if self.input_region != region {
            window.set_input_region(Some(&region));
            self.input_region = region;
        }
        let awake = self
            .stage
            .features
            .iter()
            .any(|feature| feature.inhibits_idle());
        if let Some(effects) = self.effects.get_or_insert_with(|| Effects::attach(window)) {
            effects.set_blur(shape::region(body, corner.0, corner.1, hang));
            effects.set_inhibit(awake);
        }

        let place = Place {
            viewport: f32::from(window.viewport_size().width),
            scale: window.scale_factor(),
            x,
        };
        let hidden = self.stage.hidden();
        self.panel_due.set(None);
        let mut layers: Vec<AnyElement> = (0..LAYERS)
            .filter(|index| {
                if *index == COLLAPSED_LAYER && hidden {
                    return false;
                }
                self.stage.morph.layers[*index].value() > 0.001
                    || !self.stage.morph.slides[*index].is_settled()
                    || Some(*index) == self.stage.active_layer()
                    || (*index != COLLAPSED_LAYER && self.stage.morph.frozen[*index].is_none())
            })
            .map(|index| self.layer_element(index, (w, h), place, cx))
            .collect();
        // A panel drawn just to be measured needs one more frame to take its new size, even once
        // everything else has settled.
        if self.pending.iter().any(|pending| pending.get().is_some()) {
            window.request_animation_frame();
        }
        if self.stage.morph.chrome.value() > 0.001 {
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
        let bar_shown = self.stage.morph.layers[COLLAPSED_LAYER]
            .value()
            .clamp(0.0, 1.0);
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
            .map(|(hang, menu)| menu.element(&hang, self.stage.hovered, cx));
        if self.menu.as_ref().is_some_and(Menu::is_pending) {
            window.request_animation_frame();
        }
        let shadow = ui::drop_shadow(body, corner.0, hang, self.shadow);

        self.drawn = self
            .stage
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
