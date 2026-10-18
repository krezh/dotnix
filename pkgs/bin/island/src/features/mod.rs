//! A feature implements [`Feature`] and is listed once in [`features!`]; nothing else names it.

pub mod ai;
pub mod audio;
pub mod brightness;
pub mod caffeine;
pub mod clipboard;
pub mod clock;
pub mod headset;
pub mod launcher;
pub mod media;
pub mod molt;
pub mod notifications;
pub mod osd;
pub mod session;
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
    AnyElement, AnyEntity, App, AppContext, Context, Div, Entity, EventEmitter, FontWeight,
    Keystroke, SharedString, Stateful, Subscription, WeakEntity, Window,
};
use serde::de::DeserializeOwned;
use tracing::{debug, error};

use crate::{
    bar::{self, Gauge, Look, Slots},
    components::{
        dropdown::Select,
        marquee::{self, TextLayouts},
        slider::OnChange,
        tooltip::Tooltips,
    },
    glyphs::Glyphs,
    island::{Island, Panel, Placement},
    panel::{self, Handler, IconButton, PanelView},
    supervisor::{self, Ended},
    wm::Monitor,
};

pub use osd::Osd;

/// Whether a panel that takes the keyboard stays open after a key.
pub enum Outcome {
    Open,
    Close,
}

/// An icon's key and the theme names to try, in order.
pub type GlyphSpec = (&'static str, &'static [&'static str]);

pub trait Feature: Sized + 'static {
    const NAME: &'static str;
    const GLYPHS: &'static [GlyphSpec] = &[];

    type State: Clone + Default + PartialEq + Send + 'static;

    /// Each island's own part, such as icons animating in and out.
    type Local: Local + Default;

    /// Sent to the islands rather than to the shared state, such as a new notification.
    type Event: Send + 'static;

    fn start(feed: Feed<Self::State, Self::Event>) -> Self;

    fn event(&self, _event: Self::Event, _islands: &mut Islands<'_, Self::Local>) {}

    fn check_settings(_settings: &toml::Value) -> Result<(), String> {
        Err(format!("[features.{}] takes no settings", Self::NAME))
    }

    /// `animate` is false before the island is first laid out, so things appear in place.
    fn sync_local(
        _state: &Self::State,
        _local: &mut Self::Local,
        _island: &IslandInfo<'_>,
        _animate: bool,
    ) {
    }

    fn initial() -> Self::State {
        Self::State::default()
    }

    /// Empty means one widget, named after the feature.
    const WIDGETS: &'static [&'static str] = &[];

    /// False folds the widget away.
    fn present(_widget: &str, _: &Self::State) -> bool {
        true
    }

    fn rest_width(_widget: &str, _spacing: f32) -> f32 {
        0.0
    }

    fn widget(&self, _widget: &str, _: &Self::State, _: &mut WidgetCx<'_>) -> Option<AnyElement> {
        None
    }

    /// Live levels arrive many times a second, so they redraw only these widgets and the glow, and
    /// never re-fit the bar.
    const LIVE: &'static [&'static str] = &[];

    fn glow<'a>(_: &'a Self::State, _live: &'a [f32]) -> Option<&'a [f32]> {
        None
    }

    /// A feature with a panel runs even when the layout leaves its widget out.
    const PANEL: Option<&'static str> = None;

    /// Title and place among the tabs; island's own System tab is 3.
    const TAB: Option<(&'static str, u8)> = None;

    /// For content that moves without new state, such as a play position.
    const PANEL_REFRESH: Option<Duration> = None;

    fn panel_opened(&self) {}

    fn panel<'a>(&self, _: &Self::State, _: &PanelCx<'a>) -> Option<PanelView> {
        None
    }

    /// Takes the bar's place while [`Local::popup`] says so; the pointer leaving does not close it.
    fn popup<'a>(&self, _: &Self::State, _: &PanelCx<'a>) -> Option<PanelView> {
        None
    }

    /// Called when [`Local::due`] comes.
    fn elapse(&self, _local: &mut Self::Local, _hovered: bool) {}

    fn osd(_before: &Self::State, _after: &Self::State) -> Option<Osd> {
        None
    }

    fn system_button(&self, _: &Self::State, _: &PanelCx<'_>) -> Option<IconButton> {
        None
    }

    fn inhibits_idle(_: &Self::State) -> bool {
        false
    }

    /// Empty means just the feature's name.
    const COMMANDS: &'static [&'static str] = &[];

    const USAGE: &'static str = "";

    /// Runs in the calling process, so mistakes reach the caller.
    fn check(_args: &[&str]) -> Result<(), String> {
        Err(format!("{} takes no commands", Self::NAME))
    }

    fn command(&self, _: &Self::State, _args: &[&str]) {}

    const KEYBOARD: bool = false;

    fn key(&self, _: &Self::State, _local: &mut Self::Local, _keystroke: &Keystroke) -> Outcome {
        Outcome::Close
    }
}

pub struct IslandInfo<'a> {
    pub monitor: Option<&'a Monitor>,
    pub panel_open: bool,
}

pub trait Local: 'static {
    fn advance(&mut self, _dt: f32, _now: Instant) {}

    /// While not settled, the island keeps drawing frames.
    fn is_settled(&self) -> bool {
        true
    }

    /// How much wider the widget becomes once it settles, so the island can fit its final size
    /// ahead of the animation.
    fn pending_width(&self, _spacing: f32) -> f32 {
        0.0
    }

    fn panel_closed(&mut self) {}

    fn popup(&self) -> bool {
        false
    }

    fn popup_shown(&mut self, _shown: bool) {}

    fn pointer(&mut self, _over: bool) {}

    fn due(&self) -> Option<Instant> {
        None
    }

    fn osd(&mut self, _osd: Osd) {}

    /// Measures the text it lays out by, before drawing; true when that changed its layout.
    fn measure(&mut self, _window: &Window) -> bool {
        false
    }
}

impl Local for () {}

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

fn answers<F: Feature>(word: &str) -> bool {
    if F::COMMANDS.is_empty() {
        F::NAME == word && !F::USAGE.is_empty()
    } else {
        F::COMMANDS.contains(&word)
    }
}

fn widgets_of<F: Feature>() -> impl Iterator<Item = &'static str> {
    F::WIDGETS
        .iter()
        .copied()
        .chain(F::WIDGETS.is_empty().then_some(F::NAME))
}

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refreshed<T> {
    pub value: T,
    /// A fresh reading asked for is on its way, so the widget can show it is busy.
    pub refreshing: bool,
}

pub fn parse_settings<T: DeserializeOwned>(settings: &toml::Value) -> Result<T, String> {
    settings
        .clone()
        .try_into()
        .map_err(|error: toml::de::Error| error.to_string().trim().to_owned())
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

    pub fn publish(&self, state: S) -> bool {
        self.sender.unbounded_send(state).is_ok()
    }

    pub fn send(&self, event: E) -> bool {
        self.events
            .as_ref()
            .is_some_and(|events| events.unbounded_send(event).is_ok())
    }

    pub fn settings<T: DeserializeOwned + Default>(&self) -> T {
        self.settings
            .as_ref()
            .and_then(|settings| parse_settings(settings).ok())
            .unwrap_or_default()
    }

    pub fn wants_live(&self) -> bool {
        self.live.is_some()
    }

    pub fn publish_live(&self, levels: Vec<f32>) -> bool {
        self.live
            .as_ref()
            .is_some_and(|live| live.unbounded_send(levels).is_ok())
    }

    pub fn supervise(self, mut run: impl FnMut(&Feed<S, E>) -> Ended + Send + 'static) {
        let name = self.name;
        if let Err(error) = supervisor::supervise(name, move || run(&self)) {
            error!(feature = name, "cannot start the worker: {error}");
        }
    }
}

/// An entity, so its updates redraw only what reads it.
pub struct Instance<F: Feature> {
    feature: F,
    state: F::State,
    live: Vec<f32>,
}

pub struct Live;

impl<F: Feature> EventEmitter<Live> for Instance<F> {}

pub trait AnyFeature {
    fn name(&self) -> &'static str;
    fn widgets(&self) -> Vec<&'static str>;
    fn live_widgets(&self) -> &'static [&'static str];
    fn present(&self, widget: &str, cx: &App) -> bool;
    /// Kept by island so a folding widget can still draw what it last showed.
    fn snapshot(&self, cx: &App) -> Rc<dyn Any>;
    fn rest_width(&self, widget: &str, spacing: f32) -> f32;
    fn widget(
        &self,
        widget: &str,
        snapshot: &dyn Any,
        w: &mut WidgetCx<'_>,
        cx: &App,
    ) -> Option<AnyElement>;
    fn glow(&self, cx: &App) -> Option<Vec<f32>>;
    fn panel<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView>;
    fn panel_opened(&self, cx: &App);
    fn popup<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView>;
    fn elapse(&self, local: &mut dyn AnyLocal, hovered: bool, cx: &App);
    fn system_button(&self, p: &mut PanelCx<'_>, cx: &App) -> Option<IconButton>;
    fn inhibits_idle(&self, cx: &App) -> bool;
    fn command(&self, args: &[&str], cx: &App);
    fn key(&self, local: &mut dyn AnyLocal, keystroke: &Keystroke, cx: &App) -> Outcome;
    fn new_local(&self, island: &IslandInfo<'_>, cx: &App) -> Box<dyn AnyLocal>;
    fn sync_local(
        &self,
        local: &mut dyn AnyLocal,
        island: &IslandInfo<'_>,
        animate: bool,
        cx: &App,
    );
    fn observe(&self, cx: &mut Context<Island>) -> Vec<Subscription>;
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

    fn widget(
        &self,
        widget: &str,
        snapshot: &dyn Any,
        w: &mut WidgetCx<'_>,
        cx: &App,
    ) -> Option<AnyElement> {
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

    fn panel<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView> {
        F::PANEL?;
        p.entity = Some(self.clone().into_any());
        let instance = self.read(cx);
        instance.feature.panel(&instance.state, p)
    }

    fn panel_opened(&self, cx: &App) {
        self.read(cx).feature.panel_opened();
    }

    fn popup<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView> {
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

    fn key(&self, local: &mut dyn AnyLocal, keystroke: &Keystroke, cx: &App) -> Outcome {
        let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() else {
            return Outcome::Close;
        };
        let instance = self.read(cx);
        instance.feature.key(&instance.state, local, keystroke)
    }

    fn new_local(&self, island: &IslandInfo<'_>, cx: &App) -> Box<dyn AnyLocal> {
        let mut local = F::Local::default();
        F::sync_local(&self.read(cx).state, &mut local, island, false);
        Box::new(local)
    }

    fn sync_local(
        &self,
        local: &mut dyn AnyLocal,
        island: &IslandInfo<'_>,
        animate: bool,
        cx: &App,
    ) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            F::sync_local(&self.read(cx).state, local, island, animate);
        }
    }

    fn observe(&self, cx: &mut Context<Island>) -> Vec<Subscription> {
        vec![
            cx.observe(self, |island, _, cx| island.feature_changed(F::NAME, cx)),
            cx.subscribe(self, |island, _, _: &Live, cx| {
                island.feature_live(F::NAME, cx)
            }),
        ]
    }
}

#[derive(Default)]
pub struct Features(Vec<Rc<dyn AnyFeature>>);

impl Features {
    pub fn get(&self, name: &str) -> Option<&dyn AnyFeature> {
        self.0
            .iter()
            .find(|feature| feature.name() == name)
            .map(|feature| &**feature)
    }

    pub fn owning(&self, widget: &str) -> Option<&dyn AnyFeature> {
        self.get(owner(widget)?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn AnyFeature> {
        self.0.iter().map(|feature| &**feature)
    }
}

fn start_one<F: Feature>(
    live: bool,
    settings: Option<toml::Value>,
    cx: &mut App,
) -> Rc<dyn AnyFeature> {
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
                        Islands::<F::Local>::new(F::NAME, cx)
                            .update_all(|local| local.osd(osd.clone()));
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandId(usize);

pub struct Islands<'a, L> {
    name: &'static str,
    list: Vec<Entity<Island>>,
    cx: &'a mut App,
    local: PhantomData<fn(&mut L)>,
}

impl<'a, L: Local> Islands<'a, L> {
    fn new(name: &'static str, cx: &'a mut App) -> Self {
        let list = cx
            .try_global::<Placement>()
            .map(|placement| {
                placement
                    .islands
                    .iter()
                    .filter_map(WeakEntity::upgrade)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            name,
            list,
            cx,
            local: PhantomData,
        }
    }

    pub fn find(&self, test: impl Fn(&L) -> bool) -> Option<IslandId> {
        self.list
            .iter()
            .position(|island| {
                island
                    .read(&*self.cx)
                    .local_as::<L>(self.name)
                    .is_some_and(&test)
            })
            .map(IslandId)
    }

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

    pub fn update_all(&mut self, mut action: impl FnMut(&mut L)) {
        for index in 0..self.list.len() {
            self.update(IslandId(index), &mut action);
        }
    }
}

fn instance<F: Feature>(entity: &Option<AnyEntity>) -> Option<WeakEntity<Instance<F>>> {
    entity
        .clone()
        .and_then(|entity| entity.downcast::<Instance<F>>().ok())
        .map(|entity| entity.downgrade())
}

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

    pub fn local<T: Local>(&self) -> Option<&'a T> {
        self.local?.as_any().downcast_ref()
    }

    pub fn live(&self) -> &[f32] {
        &self.live
    }

    pub fn padding(&self) -> f32 {
        self.look.half()
    }

    /// Scrolls when the text does not fit `slot`; the widget then redraws on its own clock.
    pub fn marquee(
        &self,
        text: impl Into<SharedString>,
        size: f32,
        weight: FontWeight,
        slot: f32,
    ) -> Div {
        marquee::marquee(self.layouts, text.into(), size, weight, slot)
    }

    pub fn base(&self, id: impl Into<SharedString>) -> Stateful<Div> {
        bar::widget_base(id, self.look)
    }

    /// For a widget made of several clickable parts, each showing its own hover through
    /// `ui::control` with [`WidgetCx::hover`].
    pub fn group(&self, id: impl Into<SharedString>) -> Stateful<Div> {
        bar::widget_group(id, self.look)
    }

    pub fn hover(&self) -> bool {
        self.look.hover()
    }

    /// As wide as the widest count ("88"), so the widget keeps its width as it changes.
    pub fn count(&self, text: impl Into<SharedString>) -> Div {
        bar::slot(text, self.slots.count())
    }

    pub fn energy(&self, busy: bool) -> impl gpui::IntoElement {
        bar::energy(self.look, busy)
    }

    pub fn gauge(&self, id: &'static str, gauge: Gauge<'_>) -> Stateful<Div> {
        bar::gauge(id, self.look, gauge)
    }

    pub fn glyph(&self, key: &str) -> Option<&'a Path> {
        self.glyphs.get(key)
    }

    pub fn tips(&self) -> &'a Tooltips {
        self.look.tips
    }

    pub fn toggle<E: ?Sized>(&self, panel: Panel) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        bar::on(self.island, move |island, _, cx| island.toggle(panel, cx))
    }

    pub fn call<F: Feature, E: ?Sized>(
        &self,
        action: impl Fn(&F) + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        self.on_event(move |feature: &F, _: &E| action(feature))
    }

    pub fn on_island<F: Feature + Clone, E: ?Sized>(
        &self,
        action: impl Fn(&F, &E, &mut Island, &mut Context<Island>) + 'static,
    ) -> impl Fn(&E, &mut Window, &mut App) + 'static {
        let instance = instance::<F>(&self.entity);
        let island = self.island.clone();
        move |event, _, cx| {
            let Some(feature) = instance
                .as_ref()
                .and_then(WeakEntity::upgrade)
                .map(|instance| instance.read(cx).feature.clone())
            else {
                return;
            };
            let _ = island.update(cx, |island, cx| action(&feature, event, island, cx));
        }
    }

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

    pub fn local<T: Local>(&self) -> Option<&'a T> {
        self.local?.as_any().downcast_ref()
    }

    pub fn on_local<F: Feature>(&self, action: impl Fn(&F, &mut F::Local) + 'static) -> Handler {
        let with = self.on_local_with::<F, ()>(move |feature, local, ()| action(feature, local));
        panel::on(move |island, cx| with((), island, cx))
    }

    pub fn on_local_with<F: Feature, T: 'static>(
        &self,
        action: impl Fn(&F, &mut F::Local, T) + 'static,
    ) -> Rc<dyn Fn(T, &mut Island, &mut Context<Island>)> {
        let instance = instance::<F>(&self.entity);
        Rc::new(move |value, island, cx| {
            let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) else {
                return;
            };
            let feature = &instance.read(cx).feature;
            island.update_local(F::NAME, |local| {
                if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
                    action(feature, local, value);
                }
            });
            island.local_changed(F::NAME, cx);
        })
    }

    pub fn call<F: Feature>(&self, action: impl Fn(&F) + 'static) -> Handler {
        let instance = instance::<F>(&self.entity);
        panel::on(move |_, cx| {
            if let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) {
                action(&instance.read(cx).feature);
            }
        })
    }

    pub fn on_island<F: Feature + Clone>(
        &self,
        action: impl Fn(&F, &mut Island, &mut Context<Island>) + 'static,
    ) -> Handler {
        let instance = instance::<F>(&self.entity);
        panel::on(move |island, cx| {
            let Some(feature) = instance
                .as_ref()
                .and_then(WeakEntity::upgrade)
                .map(|instance| instance.read(cx).feature.clone())
            else {
                return;
            };
            action(&feature, island, cx);
        })
    }

    pub fn slider<F: Feature>(&self, action: impl Fn(&F, f32) + 'static) -> OnChange {
        let instance = instance::<F>(&self.entity);
        Rc::new(move |value, _, cx| {
            if let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) {
                action(&instance.read(cx).feature, value);
            }
        })
    }

    /// For a dropdown: called with the index of the choice picked.
    pub fn pick<F: Feature>(&self, action: impl Fn(&F, usize) + 'static) -> Select {
        let instance = instance::<F>(&self.entity);
        Rc::new(move |index, _, cx| {
            if let Some(instance) = instance.as_ref().and_then(WeakEntity::upgrade) {
                action(&instance.read(cx).feature, index);
            }
        })
    }
}

macro_rules! features {
    ($($feature:ty),* $(,)?) => {
        pub const NAMES: &[&str] = &[$(<$feature as Feature>::NAME),*];

        pub fn widget_names() -> Vec<&'static str> {
            let mut names = Vec::new();
            $(names.extend(widgets_of::<$feature>());)*
            names
        }

        pub fn owner(widget: &str) -> Option<&'static str> {
            $(
                if widgets_of::<$feature>().any(|name| name == widget) {
                    return Some(<$feature as Feature>::NAME);
                }
            )*
            None
        }

        pub fn panel_refresh(feature: &str) -> Option<Duration> {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::PANEL_REFRESH;
                }
            )*
            None
        }

        pub fn check_settings(section: &str, settings: &toml::Value) -> Result<(), String> {
            $(
                if <$feature as Feature>::NAME == section {
                    return <$feature as Feature>::check_settings(settings).map_err(|error| format!("[features.{section}]: {error}"));
                }
            )*
            Err(format!("unknown section [features.{section}]"))
        }

        pub fn glyphs() -> Vec<GlyphSpec> {
            let mut all = Vec::new();
            $(all.extend_from_slice(<$feature as Feature>::GLYPHS);)*
            all
        }

        pub fn panel_named(panel: &str) -> Option<&'static str> {
            $(
                if <$feature as Feature>::PANEL == Some(panel) {
                    return Some(<$feature as Feature>::NAME);
                }
            )*
            None
        }

        pub fn keyboard(feature: &str) -> bool {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::KEYBOARD;
                }
            )*
            false
        }

        pub fn panel_of(feature: &str) -> Option<&'static str> {
            $(
                if <$feature as Feature>::NAME == feature {
                    return <$feature as Feature>::PANEL;
                }
            )*
            None
        }

        /// Feature, title and place among the tabs.
        pub fn tabs() -> Vec<(&'static str, &'static str, u8)> {
            let mut tabs = Vec::new();
            $(
                if let Some((title, order)) = <$feature as Feature>::TAB {
                    tabs.push((<$feature as Feature>::NAME, title, order));
                }
            )*
            tabs
        }

        /// `None` when no feature answers to the first word.
        pub fn check_command(args: &[&str]) -> Option<Result<&'static str, String>> {
            let word = args.first()?;
            $(
                if answers::<$feature>(word) {
                    return Some(<$feature as Feature>::check(args).map(|()| <$feature as Feature>::NAME));
                }
            )*
            None
        }

        pub fn usage() -> String {
            [$(<$feature as Feature>::USAGE),*]
                .into_iter()
                .filter(|usage| !usage.is_empty())
                .map(|usage| format!("\n{usage}"))
                .collect()
        }

        /// Features with a panel or commands start even without a wanted widget, since they can be
        /// reached without one; the others never run.
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
    ai::Ai,
    headset::Headset,
    zowie::Zowie,
    audio::Audio,
    brightness::Brightness,
    media::Media,
    session::Session,
    clipboard::Clipboard,
    launcher::Launcher,
];
