//! A running feature: its [`Instance`] entity, [`AnyFeature`] to reach it without knowing which it
//! is, and starting one.

use std::{any::Any, rc::Rc};

use futures::{StreamExt, channel::mpsc::unbounded};
use gpui::{
    AnyElement, AnyEntity, App, AppContext, Context, Entity, EventEmitter, Keystroke, Subscription,
    WeakEntity,
};
use tracing::debug;

use super::{
    AnyLocal, Battery, Dashboard, Feature, Feed, IslandInfo, Islands, Local, Outcome, PanelCx,
    WidgetCx, feature::widgets_of, owner,
};
use crate::{
    control::{Reply, Row},
    island::Island,
    panel::{Mark, PanelView},
};

/// An entity, so its updates redraw only what reads it.
pub struct Instance<F: Feature> {
    pub(super) feature: F,
    state: F::State,
    live: Vec<f32>,
}

pub struct Live;

impl<F: Feature> EventEmitter<Live> for Instance<F> {}

pub(super) fn instance<F: Feature>(entity: &Option<AnyEntity>) -> Option<WeakEntity<Instance<F>>> {
    entity
        .clone()
        .and_then(|entity| entity.downcast::<Instance<F>>().ok())
        .map(|entity| entity.downgrade())
}

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
    fn panel_opened(&self, cx: &App) -> Option<Box<dyn Any>>;
    fn popup<'a>(&self, p: &mut PanelCx<'a>, cx: &App) -> Option<PanelView>;
    fn elapse(&self, local: &mut dyn AnyLocal, hovered: bool, cx: &App);
    fn pointer(&self, local: &mut dyn AnyLocal, over: bool, cx: &App);
    fn lead(&self, local: &dyn AnyLocal, cx: &App) -> Option<Mark>;
    fn dashboard_order(&self) -> u8;
    fn dashboard(&self, p: &mut PanelCx<'_>, cx: &App) -> Dashboard;
    fn batteries(&self, cx: &App) -> Vec<Battery>;
    fn inhibits_idle(&self, cx: &App) -> bool;
    fn command(&self, args: &[&str], cx: &App) -> Reply;
    fn status(&self, cx: &App) -> Vec<Row>;
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

    fn panel_opened(&self, cx: &App) -> Option<Box<dyn Any>> {
        self.read(cx).feature.panel_opened()
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

    fn pointer(&self, local: &mut dyn AnyLocal, over: bool, cx: &App) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            self.read(cx).feature.pointer(local, over);
        }
    }

    fn lead(&self, local: &dyn AnyLocal, cx: &App) -> Option<Mark> {
        let local = local.as_any().downcast_ref::<F::Local>()?;
        self.read(cx).feature.lead(local)
    }

    fn dashboard_order(&self) -> u8 {
        F::DASHBOARD_ORDER
    }

    fn dashboard(&self, p: &mut PanelCx<'_>, cx: &App) -> Dashboard {
        p.entity = Some(self.clone().into_any());
        let instance = self.read(cx);
        instance.feature.dashboard(&instance.state, p)
    }

    fn batteries(&self, cx: &App) -> Vec<Battery> {
        let instance = self.read(cx);
        instance.feature.batteries(&instance.state)
    }

    fn inhibits_idle(&self, cx: &App) -> bool {
        F::inhibits_idle(&self.read(cx).state)
    }

    fn command(&self, args: &[&str], cx: &App) -> Reply {
        let instance = self.read(cx);
        instance.feature.reply(&instance.state, args)
    }

    fn status(&self, cx: &App) -> Vec<Row> {
        let instance = self.read(cx);
        instance.feature.status(&instance.state)
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
pub struct Features(pub(super) Vec<Rc<dyn AnyFeature>>);

impl Features {
    pub fn get(&self, name: &str) -> Option<&dyn AnyFeature> {
        self.0
            .iter()
            .find(|feature| feature.name() == name)
            .map(|feature| &**feature)
    }

    /// Every running feature's rows for `island ctl`'s overview, in the registry's order.
    pub fn status(&self, cx: &App) -> Vec<Row> {
        self.0
            .iter()
            .flat_map(|feature| feature.status(cx))
            .collect()
    }

    pub fn owning(&self, widget: &str) -> Option<&dyn AnyFeature> {
        self.get(owner(widget)?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn AnyFeature> {
        self.0.iter().map(|feature| &**feature)
    }
}

pub(super) fn start_one<F: Feature>(
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
