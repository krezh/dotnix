//! Features: self-contained parts of island, each in one file with its supervised worker, its state,
//! its bar widget and its panel. A new feature implements [`Feature`] and is listed once in
//! [`features!`]; nothing else in island names it.

pub mod audio;
pub mod caffeine;
pub mod clock;
pub mod headset;
pub mod media;
pub mod molt;
pub mod notifications;
pub mod tray;
pub mod workspaces;
pub mod zowie;

use std::{
    any::Any,
    marker::PhantomData,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

use futures::{
    StreamExt,
    channel::mpsc::{UnboundedSender, unbounded},
};
use gpui::{
    AnyElement, AnyEntity, App, AppContext, Context, Div, Entity, EventEmitter, FontWeight, SharedString, Stateful,
    Subscription, WeakEntity, Window,
};
use serde::de::DeserializeOwned;
use tracing::{debug, error};

use crate::{
    bar::{self, Gauge, Look, Slots},
    components::{
        marquee::{self, TextLayouts},
        slider::OnChange,
        tooltip::Tooltips,
    },
    glyphs::Glyphs,
    island::{Island, Panel, Placement},
    panel::{self, Handler, IconButton, PanelView},
    panels::Osd,
    supervisor::{self, Ended},
    wm::Monitor,
};

/// An icon a feature draws: its key and the theme names to try, in order.
pub type GlyphSpec = (&'static str, &'static [&'static str]);

pub trait Feature: Sized + 'static {
    /// The name the layout config and the Nix module use, e.g. `headset`.
    const NAME: &'static str;
    /// Icons this feature draws, looked up once at startup and read with [`WidgetCx::glyph`].
    const GLYPHS: &'static [GlyphSpec] = &[];

    /// What the worker produces; island keeps the latest one.
    type State: Clone + Default + PartialEq + Send + 'static;

    /// What each island keeps for itself to draw this feature's widgets with, such as icons
    /// animating in and out; `()` for none. Read it while drawing with [`WidgetCx::local`].
    type Local: Local + Default;

    /// What the worker sends to the islands rather than to their shared state, such as a new
    /// notification that appears on one of them; `()` for none. Sent with [`Feed::send`] and
    /// handled by [`Feature::event`].
    type Event: Send + 'static;

    /// Starts the worker, normally with [`Feed::supervise`], and returns the handle clicks talk to.
    fn start(feed: Feed<Self::State, Self::Event>) -> Self;

    /// Handles an `event` from the worker: changes the feature's own state, or places it on some of
    /// the `islands`.
    fn event(&self, _event: Self::Event, _islands: &mut Islands<'_, Self::Local>) {}

    /// Checks the feature's section of the config file, `[NAME]`, when the config loads; read it
    /// with [`Feed::settings`]. Without this the feature takes no section.
    fn check_settings(_settings: &toml::Value) -> Result<(), String> {
        Err(format!("[{}] takes no settings", Self::NAME))
    }

    /// Brings an island's `local` up to date with a new `state`, or with where the island is;
    /// `animate` is false before the island is first laid out, so things appear in place instead
    /// of animating in.
    fn sync_local(_state: &Self::State, _local: &mut Self::Local, _island: &IslandInfo<'_>, _animate: bool) {}

    /// The state before the worker first publishes.
    fn initial() -> Self::State {
        Self::State::default()
    }

    /// The bar widgets this feature draws, by their layout names; empty means one, named after the
    /// feature.
    const WIDGETS: &'static [&'static str] = &[];

    /// Whether `widget` has anything to show; false folds it away.
    fn present(_widget: &str, _: &Self::State) -> bool {
        true
    }

    /// The width `widget` folds down to instead of disappearing.
    fn rest_width(_widget: &str, _spacing: f32) -> f32 {
        0.0
    }

    /// `widget` drawn for `state`, or `None` to draw nothing.
    fn widget(&self, _widget: &str, _: &Self::State, _: &mut WidgetCx<'_>) -> Option<AnyElement> {
        None
    }

    /// The widgets that draw the live levels from [`Feed::publish_live`]. Live levels arrive many
    /// times a second, so they redraw only these widgets and the glow, and never re-fit the bar.
    const LIVE: &'static [&'static str] = &[];

    /// The levels the glow behind the island follows, if this feature drives it.
    fn glow<'a>(_: &'a Self::State, _live: &'a [f32]) -> Option<&'a [f32]> {
        None
    }

    /// The feature's panel name, if it has one: opened as `Panel::Feature(NAME)` or with
    /// `island ctl open PANEL`. A feature with a panel runs even when the layout leaves its widget out.
    const PANEL: Option<&'static str> = None;

    /// The panel's tab title and its place among the tabs (island's own System tab is 3).
    const TAB: Option<(&'static str, u8)> = None;

    /// How often the open panel or pop-up redraws on its own, for content that moves without new
    /// state such as a play position.
    const PANEL_REFRESH: Option<Duration> = None;

    /// The panel for `state`; only asked for when [`Feature::PANEL`] is set. It reads the island's
    /// own part with [`PanelCx::local`] and changes it with [`PanelCx::on_local`].
    fn panel<'a>(&self, _: &Self::State, _: &PanelCx<'a>) -> Option<PanelView<'a>> {
        None
    }

    /// The pop-up an island shows on its own while its [`Local::popup`] says so, such as
    /// notification cards. It takes the bar's place until it has nothing left, and the pointer
    /// leaving does not close it.
    fn popup<'a>(&self, _: &Self::State, _: &PanelCx<'a>) -> Option<PanelView<'a>> {
        None
    }

    /// The time an island's [`Local::due`] named has come; `hovered` says whether the pointer is
    /// over that island.
    fn elapse(&self, _local: &mut Self::Local, _hovered: bool) {}

    /// The popup to show when the state changes from `before` to `after`, such as a volume change.
    fn osd(_before: &Self::State, _after: &Self::State) -> Option<Osd> {
        None
    }

    /// A quick button for the System panel's top row.
    fn system_button(&self, _: &Self::State, _: &PanelCx<'_>) -> Option<IconButton> {
        None
    }

    /// Whether island should keep the screen awake for `state`.
    fn inhibits_idle(_: &Self::State) -> bool {
        false
    }

    /// The `island ctl` words this feature answers to; empty means just its name.
    const COMMANDS: &'static [&'static str] = &[];

    /// Help lines for its commands; empty when the feature takes none.
    const USAGE: &'static str = "";

    /// Checks `island ctl WORD ARGS…` (`args` starts with the word) in the calling process, so
    /// mistakes reach the caller.
    fn check(_args: &[&str]) -> Result<(), String> {
        Err(format!("{} takes no commands", Self::NAME))
    }

    /// Runs a command [`Feature::check`] accepted.
    fn command(&self, _: &Self::State, _args: &[&str]) {}
}

/// Where an island is, for its own part of a feature.
pub struct IslandInfo<'a> {
    /// The monitor the island is on, from the compositor.
    pub monitor: Option<&'a Monitor>,
    /// Whether the feature's panel is open on the island.
    pub panel_open: bool,
}

/// An island's own part of a feature, animated with the island.
pub trait Local: 'static {
    fn advance(&mut self, _dt: f32, _now: Instant) {}

    /// Whether it has stopped moving; while not, the island keeps drawing frames.
    fn is_settled(&self) -> bool {
        true
    }

    /// How much wider the widget becomes once it settles (narrower when negative), so the island
    /// can fit its final size ahead of the animation.
    fn pending_width(&self, _spacing: f32) -> f32 {
        0.0
    }

    /// The feature's panel closed on this island, such as to forget where it was paged to.
    fn panel_closed(&mut self) {}

    /// Whether it has something to show over the bar on its own, the feature's
    /// [`Feature::popup`].
    fn popup(&self) -> bool {
        false
    }

    /// The island started or stopped showing the feature's pop-up.
    fn popup_shown(&mut self, _shown: bool) {}

    /// When it next needs [`Feature::elapse`], such as a card timing out.
    fn due(&self) -> Option<Instant> {
        None
    }

    /// Measures the text it lays out by, before drawing; true when that changed its layout.
    fn measure(&mut self, _window: &Window) -> bool {
        false
    }
}

impl Local for () {}

/// A [`Local`] with its type erased, as island holds it.
pub trait AnyLocal: Local {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Local> AnyLocal for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Whether `word` names one of `F`'s commands.
fn answers<F: Feature>(word: &str) -> bool {
    if F::COMMANDS.is_empty() {
        F::NAME == word && !F::USAGE.is_empty()
    } else {
        F::COMMANDS.contains(&word)
    }
}

/// `F`'s widgets by their layout names.
fn widgets_of<F: Feature>() -> impl Iterator<Item = &'static str> {
    F::WIDGETS.iter().copied().chain(F::WIDGETS.is_empty().then_some(F::NAME))
}

/// Where a feature's workers publish its state, its live levels and its events.
pub struct Feed<S, E = ()> {
    name: &'static str,
    sender: UnboundedSender<S>,
    live: Option<UnboundedSender<Vec<f32>>>,
    events: Option<UnboundedSender<E>>,
    settings: Option<toml::Value>,
}

impl<S, E> Clone for Feed<S, E> {
    fn clone(&self) -> Self {
        Self {
            name: self.name,
            sender: self.sender.clone(),
            live: self.live.clone(),
            events: self.events.clone(),
            settings: self.settings.clone(),
        }
    }
}

/// `settings`, a section of the config file, as `T`.
pub fn parse_settings<T: DeserializeOwned>(settings: &toml::Value) -> Result<T, String> {
    settings.clone().try_into().map_err(|error: toml::de::Error| error.to_string().trim().to_owned())
}

impl<S: Send + 'static, E: Send + 'static> Feed<S, E> {
    pub(crate) fn new(name: &'static str, sender: UnboundedSender<S>) -> Self {
        Self {
            name,
            sender,
            live: None,
            events: None,
            settings: None,
        }
    }

    /// Sends a new state to island; false once island no longer listens.
    pub fn publish(&self, state: S) -> bool {
        self.sender.unbounded_send(state).is_ok()
    }

    /// Sends an event to [`Feature::event`]; false once island no longer listens.
    pub fn send(&self, event: E) -> bool {
        self.events.as_ref().is_some_and(|events| events.unbounded_send(event).is_ok())
    }

    /// The feature's section of the config file, `[NAME]`, or the defaults without one. It was
    /// checked by [`Feature::check_settings`] when the config loaded.
    pub fn settings<T: DeserializeOwned + Default>(&self) -> T {
        self.settings
            .as_ref()
            .and_then(|settings| parse_settings(settings).ok())
            .unwrap_or_default()
    }

    /// Whether anything draws live levels, so a worker producing them is worth running.
    pub fn wants_live(&self) -> bool {
        self.live.is_some()
    }

    /// Sends live levels to the [`Feature::LIVE`] widgets and the glow; false once nothing listens.
    pub fn publish_live(&self, levels: Vec<f32>) -> bool {
        self.live.as_ref().is_some_and(|live| live.unbounded_send(levels).is_ok())
    }

    /// Runs `run` under the supervisor, named after the feature, so it restarts if it fails.
    pub fn supervise(self, mut run: impl FnMut(&Feed<S, E>) -> Ended + Send + 'static) {
        let name = self.name;
        if let Err(error) = supervisor::supervise(name, move || run(&self)) {
            error!(feature = name, "cannot start the worker: {error}");
        }
    }
}

/// A running feature and its latest state, as a GPUI entity so its updates redraw only what reads it.
pub struct Instance<F: Feature> {
    feature: F,
    state: F::State,
    live: Vec<f32>,
}

impl<F: Feature> EventEmitter<Osd> for Instance<F> {}

/// New live levels arrived.
pub struct Live;

impl<F: Feature> EventEmitter<Live> for Instance<F> {}

/// A running feature with its type erased, as island holds it.
pub trait AnyFeature {
    fn name(&self) -> &'static str;
    /// Its widgets by their layout names.
    fn widgets(&self) -> Vec<&'static str>;
    /// The widgets that draw live levels.
    fn live_widgets(&self) -> &'static [&'static str];
    fn present(&self, widget: &str, cx: &App) -> bool;
    /// The current state, kept by island so a folding widget can still draw what it last showed.
    fn snapshot(&self, cx: &App) -> Rc<dyn Any>;
    fn rest_width(&self, widget: &str, spacing: f32) -> f32;
    fn widget(&self, widget: &str, snapshot: &dyn Any, w: &mut WidgetCx<'_>, cx: &App) -> Option<AnyElement>;
    fn glow(&self, cx: &App) -> Option<Vec<f32>>;
    fn panel<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView<'a>>;
    fn popup<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView<'a>>;
    fn elapse(&self, local: &mut dyn AnyLocal, hovered: bool, cx: &App);
    fn system_button(&self, p: &mut PanelCx<'_>, cx: &App) -> Option<IconButton>;
    fn inhibits_idle(&self, cx: &App) -> bool;
    fn command(&self, args: &[&str], cx: &App);
    /// A new island's own part of this feature, brought up to date.
    fn new_local(&self, island: &IslandInfo<'_>, cx: &App) -> Box<dyn AnyLocal>;
    fn sync_local(&self, local: &mut dyn AnyLocal, island: &IslandInfo<'_>, animate: bool, cx: &App);
    /// Calls [`Island::feature_changed`] whenever this feature's state changes, and
    /// [`Island::feature_live`] whenever its live levels do.
    fn observe(&self, cx: &mut Context<Island>) -> Vec<Subscription>;
    /// Hands every popup this feature asks for to `show`.
    fn on_osd(&self, show: Box<dyn Fn(Osd, &mut App)>, cx: &mut App) -> Subscription;
}

impl<F: Feature> AnyFeature for Entity<Instance<F>> {
    fn name(&self) -> &'static str {
        F::NAME
    }

    fn widgets(&self) -> Vec<&'static str> {
        widgets_of::<F>().collect()
    }

    fn live_widgets(&self) -> &'static [&'static str] {
        F::LIVE
    }

    fn present(&self, widget: &str, cx: &App) -> bool {
        F::present(widget, &self.read(cx).state)
    }

    fn snapshot(&self, cx: &App) -> Rc<dyn Any> {
        Rc::new(self.read(cx).state.clone())
    }

    fn rest_width(&self, widget: &str, spacing: f32) -> f32 {
        F::rest_width(widget, spacing)
    }

    fn widget(&self, widget: &str, snapshot: &dyn Any, w: &mut WidgetCx<'_>, cx: &App) -> Option<AnyElement> {
        let state = snapshot.downcast_ref::<F::State>()?;
        let instance = self.read(cx);
        w.entity = Some(self.clone().into_any());
        w.live = instance.live.clone();
        instance.feature.widget(widget, state, w)
    }

    fn glow(&self, cx: &App) -> Option<Vec<f32>> {
        let instance = self.read(cx);
        F::glow(&instance.state, &instance.live).map(<[f32]>::to_vec)
    }

    fn panel<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView<'a>> {
        F::PANEL?;
        p.entity = Some(self.clone().into_any());
        let instance = self.read(cx);
        instance.feature.panel(&instance.state, p)
    }

    fn popup<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView<'a>> {
        p.entity = Some(self.clone().into_any());
        let instance = self.read(cx);
        instance.feature.popup(&instance.state, p)
    }

    fn elapse(&self, local: &mut dyn AnyLocal, hovered: bool, cx: &App) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            self.read(cx).feature.elapse(local, hovered);
        }
    }

    fn system_button(&self, p: &mut PanelCx<'_>, cx: &App) -> Option<IconButton> {
        p.entity = Some(self.clone().into_any());
        let instance = self.read(cx);
        instance.feature.system_button(&instance.state, p)
    }

    fn inhibits_idle(&self, cx: &App) -> bool {
        F::inhibits_idle(&self.read(cx).state)
    }

    fn command(&self, args: &[&str], cx: &App) {
        let instance = self.read(cx);
        instance.feature.command(&instance.state, args);
    }

    fn new_local(&self, island: &IslandInfo<'_>, cx: &App) -> Box<dyn AnyLocal> {
        let mut local = F::Local::default();
        F::sync_local(&self.read(cx).state, &mut local, island, false);
        Box::new(local)
    }

    fn sync_local(&self, local: &mut dyn AnyLocal, island: &IslandInfo<'_>, animate: bool, cx: &App) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            F::sync_local(&self.read(cx).state, local, island, animate);
        }
    }

    fn observe(&self, cx: &mut Context<Island>) -> Vec<Subscription> {
        vec![
            cx.observe(self, |island, _, cx| island.feature_changed(F::NAME, cx)),
            cx.subscribe(self, |island, _, _: &Live, cx| island.feature_live(F::NAME, cx)),
        ]
    }

    fn on_osd(&self, show: Box<dyn Fn(Osd, &mut App)>, cx: &mut App) -> Subscription {
        cx.subscribe(self, move |_, osd: &Osd, cx| show(osd.clone(), cx))
    }
}

/// The running features, in registry order.
#[derive(Default)]
pub struct Features(Vec<Rc<dyn AnyFeature>>);

impl Features {
    pub fn get(&self, name: &str) -> Option<&dyn AnyFeature> {
        self.0.iter().find(|feature| feature.name() == name).map(|feature| &**feature)
    }

    /// The running feature that draws `widget`.
    pub fn owning(&self, widget: &str) -> Option<&dyn AnyFeature> {
        self.get(owner(widget)?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn AnyFeature> {
        self.0.iter().map(|feature| &**feature)
    }
}

/// Starts `F` with its section of the config file; `live` says whether anything draws its live
/// levels.
fn start_one<F: Feature>(live: bool, settings: Option<toml::Value>, cx: &mut App) -> Rc<dyn AnyFeature> {
    let (sender, mut states) = unbounded();
    let (events, mut arrived) = unbounded();
    let mut feed = Feed::new(F::NAME, sender);
    feed.events = Some(events);
    feed.settings = settings;
    let mut levels = live.then(|| {
        let (sender, levels) = unbounded();
        feed.live = Some(sender);
        levels
    });
    let feature = F::start(feed);
    let entity = cx.new(|_| Instance {
        feature,
        state: F::initial(),
        live: Vec::new(),
    });
    if let Some(mut levels) = levels.take() {
        let instance = entity.downgrade();
        cx.spawn(async move |cx| {
            while let Some(live) = levels.next().await {
                let alive = instance.update(cx, |instance, cx| {
                    instance.live = live;
                    cx.emit(Live);
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }
    let instance = entity.downgrade();
    cx.spawn(async move |cx| {
        while let Some(state) = states.next().await {
            let alive = instance.update(cx, |instance, cx| {
                if instance.state != state {
                    let osd = F::osd(&instance.state, &state);
                    instance.state = state;
                    cx.notify();
                    if let Some(osd) = osd {
                        cx.emit(osd);
                    }
                }
            });
            if alive.is_err() {
                break;
            }
        }
    })
    .detach();
    let instance = entity.downgrade();
    cx.spawn(async move |cx| {
        while let Some(event) = arrived.next().await {
            let alive = instance.update(cx, |instance, cx| {
                let mut islands = Islands::new(F::NAME, cx);
                instance.feature.event(event, &mut islands);
            });
            if alive.is_err() {
                break;
            }
        }
    })
    .detach();
    debug!(feature = F::NAME, "feature started");
    Rc::new(entity)
}

/// One of the open islands, as [`Islands`] names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandId(usize);

/// The open islands, for a feature placing an event on some of them through their own part of it.
pub struct Islands<'a, L> {
    name: &'static str,
    list: Vec<Entity<Island>>,
    focused: Option<usize>,
    cx: &'a mut App,
    local: PhantomData<fn(&mut L)>,
}

impl<'a, L: Local> Islands<'a, L> {
    fn new(name: &'static str, cx: &'a mut App) -> Self {
        let (list, focused) = match cx.try_global::<Placement>() {
            Some(placement) => {
                let list: Vec<Entity<Island>> = placement.islands.iter().filter_map(WeakEntity::upgrade).collect();
                let focused = placement
                    .focused
                    .as_ref()
                    .and_then(|focused| list.iter().position(|island| island.entity_id() == focused.entity_id()));
                (list, focused)
            }
            None => (Vec::new(), None),
        };
        Self {
            name,
            focused: focused.or((!list.is_empty()).then_some(0)),
            list,
            cx,
            local: PhantomData,
        }
    }

    /// The island on the focused monitor, where something new appears.
    pub fn focused(&self) -> Option<IslandId> {
        self.focused.map(IslandId)
    }

    /// The first island whose own part passes `test`.
    pub fn find(&self, test: impl Fn(&L) -> bool) -> Option<IslandId> {
        self.list
            .iter()
            .position(|island| island.read(&*self.cx).local_as::<L>(self.name).is_some_and(&test))
            .map(IslandId)
    }

    /// Changes `island`'s own part with `action`; the island then shows or hides the feature's
    /// pop-up as its part asks.
    pub fn update(&mut self, island: IslandId, action: impl FnOnce(&mut L)) {
        let Some(entity) = self.list.get(island.0).cloned() else {
            return;
        };
        let name = self.name;
        entity.update(self.cx, |island, _| {
            island.update_local(name, |local| {
                if let Some(local) = local.as_any_mut().downcast_mut::<L>() {
                    action(local);
                }
            });
        });
        let island = entity.downgrade();
        self.cx.defer(move |cx| {
            let _ = island.update(cx, |island, cx| island.local_changed(name, cx));
        });
    }

    /// Changes every island's own part with `action`.
    pub fn update_all(&mut self, mut action: impl FnMut(&mut L)) {
        for index in 0..self.list.len() {
            self.update(IslandId(index), &mut action);
        }
    }
}

/// The running feature `F` behind a type-erased entity, for click handlers.
fn instance<F: Feature>(entity: &Option<AnyEntity>) -> Option<WeakEntity<Instance<F>>> {
    entity
        .clone()
        .and_then(|entity| entity.downcast::<Instance<F>>().ok())
        .map(|entity| entity.downgrade())
}

/// What a feature's widget draws with: the bar's building blocks, its icons and tooltips, and click
/// handlers that reach island or the feature's own handle.
pub struct WidgetCx<'a> {
    look: Look<'a>,
    slots: Slots,
    glyphs: &'a Glyphs,
    layouts: &'a TextLayouts,
    island: &'a WeakEntity<Island>,
    local: Option<&'a dyn AnyLocal>,
    entity: Option<AnyEntity>,
    live: Vec<f32>,
}

impl<'a> WidgetCx<'a> {
    pub(crate) fn new(
        look: Look<'a>,
        slots: Slots,
        glyphs: &'a Glyphs,
        layouts: &'a TextLayouts,
        island: &'a WeakEntity<Island>,
        local: Option<&'a dyn AnyLocal>,
    ) -> Self {
        Self {
            look,
            slots,
            glyphs,
            layouts,
            island,
            local,
            entity: None,
            live: Vec::new(),
        }
    }

    /// This island's own part of the feature, its [`Feature::Local`].
    pub fn local<T: Local>(&self) -> Option<&'a T> {
        self.local?.as_any().downcast_ref()
    }

    /// The latest live levels, for [`Feature::LIVE`] widgets.
    pub fn live(&self) -> &[f32] {
        &self.live
    }

    /// The widget's inner padding on each side.
    pub fn padding(&self) -> f32 {
        self.look.half()
    }

    /// One line of text in a `slot`-wide space that scrolls when it does not fit; the widget then
    /// redraws on its own clock.
    pub fn marquee(&self, text: impl Into<SharedString>, size: f32, weight: FontWeight, slot: f32) -> Div {
        marquee::marquee(self.layouts, text.into(), size, weight, slot)
    }

    /// The common widget shell: height, padding, hover highlight and pointer.
    pub fn base(&self, id: impl Into<SharedString>) -> Stateful<Div> {
        bar::widget_base(id, self.look)
    }

    /// A small count as wide as the widest one ("88"), so the widget keeps its width as it changes.
    pub fn count(&self, text: impl Into<SharedString>) -> Div {
        bar::slot(text, self.slots.count())
    }

    /// A glyph beside a vertical level meter.
    pub fn gauge(&self, id: &'static str, gauge: Gauge<'_>) -> Stateful<Div> {
        bar::gauge(id, self.look, gauge)
    }

    pub fn glyph(&self, key: &str) -> Option<&'a Path> {
        self.glyphs.get(key)
    }

    pub fn tips(&self) -> &'a Tooltips {
        self.look.tips
    }

    /// A handler that opens `panel`, or closes it when it is already open.
    pub fn toggle<E: ?Sized>(&self, panel: Panel) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        bar::on(self.island, move |island, _, cx| island.toggle(panel, cx))
    }

    /// A handler that calls `action` with this feature's handle, from [`Feature::start`].
    pub fn call<F: Feature, E: ?Sized>(&self, action: impl Fn(&F) + 'static) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        self.on_event(move |feature: &F, _: &E| action(feature))
    }

    /// A handler that calls `action` with this feature's handle, the event and the island the
    /// widget is on, for actions that open something on that island or need where it is.
    pub fn on_island<F: Feature + Clone, E: ?Sized>(
        &self,
        action: impl Fn(&F, &E, &mut Island, &mut Context<Island>) + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let instance = instance::<F>(&self.entity);
        let island = self.island.clone();
        move |event, _, cx| {
            let Some(feature) = instance.as_ref().and_then(WeakEntity::upgrade).map(|instance| instance.read(cx).feature.clone())
            else {
                return;
            };
            let _ = island.update(cx, |island, cx| action(&feature, event, island, cx));
        }
    }

    /// Like [`WidgetCx::call`], for handlers that need the event, such as scrolling.
    pub fn on_event<F: Feature, E: ?Sized>(
        &self,
        action: impl Fn(&F, &E) + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let instance = instance::<F>(&self.entity);
        move |event, _, cx| {
            if let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) {
                action(&instance.read(cx).feature, event);
            }
        }
    }
}

/// What a feature's panel and System-panel button are built with: its icons, the island's own part
/// of the feature, and handlers that reach the feature's own handle. Handlers that act on island use
/// [`panel::on`].
pub struct PanelCx<'a> {
    glyphs: Glyphs,
    local: Option<&'a dyn AnyLocal>,
    entity: Option<AnyEntity>,
}

impl<'a> PanelCx<'a> {
    pub(crate) fn new(glyphs: Glyphs, local: Option<&'a dyn AnyLocal>) -> Self {
        Self {
            glyphs,
            local,
            entity: None,
        }
    }

    pub fn glyph(&self, key: &str) -> Option<PathBuf> {
        self.glyphs.get(key).map(Path::to_path_buf)
    }

    /// This island's own part of the feature, its [`Feature::Local`].
    pub fn local<T: Local>(&self) -> Option<&'a T> {
        self.local?.as_any().downcast_ref()
    }

    /// A handler that calls `action` with this feature's handle and this island's own part of it,
    /// then redraws the island.
    pub fn on_local<F: Feature>(&self, action: impl Fn(&F, &mut F::Local) + 'static) -> Handler {
        let instance = instance::<F>(&self.entity);
        panel::on(move |island, cx| {
            let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) else {
                return;
            };
            let feature = &instance.read(cx).feature;
            island.update_local(F::NAME, |local| {
                if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
                    action(feature, local);
                }
            });
            island.local_changed(F::NAME, cx);
        })
    }

    /// A handler that calls `action` with this feature's handle, from [`Feature::start`].
    pub fn call<F: Feature>(&self, action: impl Fn(&F) + 'static) -> Handler {
        let instance = instance::<F>(&self.entity);
        panel::on(move |_, cx| {
            if let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) {
                action(&instance.read(cx).feature);
            }
        })
    }

    /// A slider handler that calls `action` with this feature's handle and the new value.
    pub fn slider<F: Feature>(&self, action: impl Fn(&F, f32) + 'static) -> OnChange {
        let instance = instance::<F>(&self.entity);
        Rc::new(move |value, _, cx| {
            if let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) {
                action(&instance.read(cx).feature, value);
            }
        })
    }
}

macro_rules! features {
    ($($feature:ty),* $(,)?) => {
        /// Every feature's name, in registry order.
        pub const NAMES: &[&str] = &[$(<$feature as Feature>::NAME),*];

        /// Every feature widget's layout name.
        pub fn widget_names() -> Vec<&'static str> {
            let mut names = Vec::new();
            $(names.extend(widgets_of::<$feature>());)*
            names
        }

        /// The feature that draws `widget`.
        pub fn owner(widget: &str) -> Option<&'static str> {
            $(
                if widgets_of::<$feature>().any(|name| name == widget) {
                    return Some(<$feature as Feature>::NAME);
                }
            )*
            None
        }

        /// How often the open panel of the feature called `feature` redraws on its own.
        pub fn panel_refresh(feature: &str) -> Option<Duration> {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::PANEL_REFRESH;
                }
            )*
            None
        }

        /// Checks the config file's `[section]`, which belongs to the feature of that name.
        pub fn check_settings(section: &str, settings: &toml::Value) -> Result<(), String> {
            $(
                if <$feature as Feature>::NAME == section {
                    return <$feature as Feature>::check_settings(settings).map_err(|error| format!("[{section}]: {error}"));
                }
            )*
            Err(format!("unknown section [{section}]"))
        }

        /// Every feature's icons.
        pub fn glyphs() -> Vec<GlyphSpec> {
            let mut all = Vec::new();
            $(all.extend_from_slice(<$feature as Feature>::GLYPHS);)*
            all
        }

        /// The feature whose panel is called `panel`.
        pub fn panel_named(panel: &str) -> Option<&'static str> {
            $(
                if <$feature as Feature>::PANEL == Some(panel) {
                    return Some(<$feature as Feature>::NAME);
                }
            )*
            None
        }

        /// The panel name of the feature called `feature`.
        pub fn panel_of(feature: &str) -> Option<&'static str> {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::PANEL;
                }
            )*
            None
        }

        /// Every feature tab: its feature, title and place among the tabs.
        pub fn tabs() -> Vec<(&'static str, &'static str, u8)> {
            let mut tabs = Vec::new();
            $(
                if let Some((title, order)) = <$feature as Feature>::TAB {
                    tabs.push((<$feature as Feature>::NAME, title, order));
                }
            )*
            tabs
        }

        /// Checks `island ctl WORD ARGS…` (`args` starts with the word); `None` when no feature
        /// answers to the word.
        pub fn check_command(args: &[&str]) -> Option<Result<&'static str, String>> {
            let word = args.first()?;
            $(
                if answers::<$feature>(word) {
                    return Some(<$feature as Feature>::check(args).map(|()| <$feature as Feature>::NAME));
                }
            )*
            None
        }

        /// The help lines of every feature that takes commands.
        pub fn usage() -> String {
            [$(<$feature as Feature>::USAGE),*]
                .into_iter()
                .filter(|usage| !usage.is_empty())
                .map(|usage| format!("\n{usage}"))
                .collect()
        }

        /// Starts the features whose widgets `wanted` names, plus those with a panel or commands,
        /// which can be reached without their widget; the others never run. Live levels flow when a
        /// wanted widget draws them, or when `glow` is on for a feature that has live widgets. Each
        /// gets its section of the config file from `settings`.
        pub fn start(wanted: impl Fn(&'static str) -> bool, glow: bool, settings: &toml::Table, cx: &mut App) -> Features {
            let mut started = Vec::new();
            $(
                if <$feature as Feature>::PANEL.is_some()
                    || !<$feature as Feature>::USAGE.is_empty()
                    || widgets_of::<$feature>().any(&wanted)
                {
                    let live = <$feature as Feature>::LIVE.iter().any(|widget| wanted(widget))
                        || (glow && !<$feature as Feature>::LIVE.is_empty());
                    let section = settings.get(<$feature as Feature>::NAME).cloned();
                    started.push(start_one::<$feature>(live, section, cx));
                }
            )*
            Features(started)
        }
    };
}

features![
    clock::Clock,
    notifications::Notifications,
    workspaces::Workspaces,
    tray::Tray,
    caffeine::Caffeine,
    molt::Molt,
    headset::Headset,
    zowie::Zowie,
    audio::Audio,
    media::Media,
];
