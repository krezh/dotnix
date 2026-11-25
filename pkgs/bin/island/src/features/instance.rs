//! A running feature: its [`Instance`], [`AnyFeature`] to reach it without knowing which it is, and
//! starting one.

use std::{any::Any, cell::RefCell, rc::Rc};

use futures::{
    StreamExt,
    channel::mpsc::{UnboundedReceiver, unbounded},
    stream::{LocalBoxStream, SelectAll},
};
use tracing::debug;

use super::{
    AnyLocal, Battery, Dashboard, Feed, Frontend, IslandInfo, Islands, Local, Outcome, PanelCx,
    Surface, WidgetCx, battery, feature::widgets_of, owner,
};
use crate::{
    bar_widget::BarWidget,
    config::Battery as BatterySettings,
    control::{Reply, Row},
    key::{Key, Modifier},
    panel::{Mark, PanelView},
};

/// Something a feature sent, ready for the frontend to apply.
pub type Arrival = Box<dyn FnOnce(&mut dyn Frontend)>;

/// What every feature sends, as one stream.
pub type Arrivals = SelectAll<LocalBoxStream<'static, Arrival>>;

pub struct Instance<F: Surface> {
    pub(super) feature: F,
    /// Shared with the snapshots islands hold, so taking one copies nothing.
    state: RefCell<Rc<F::State>>,
    live: RefCell<Vec<f32>>,
}

impl<F: Surface> Instance<F> {
    fn new(feature: F) -> Self {
        Self {
            feature,
            state: RefCell::new(Rc::new(F::initial())),
            live: RefCell::default(),
        }
    }

    fn set_state(&self, state: F::State, frontend: &mut dyn Frontend) {
        if **self.state.borrow() == state {
            return;
        }
        let osd = F::osd(&self.state.borrow(), &state);
        self.state.replace(Rc::new(state));
        if let Some(osd) = osd {
            Islands::<F::Local>::new(F::NAME, frontend).update_all(|local| local.osd(osd.clone()));
        }
        frontend.changed(F::NAME);
    }

    fn set_live(&self, live: Vec<f32>, frontend: &mut dyn Frontend) {
        self.live.replace(live);
        frontend.live(F::NAME);
    }

    fn event(&self, event: F::Event, frontend: &mut dyn Frontend) {
        self.feature
            .event(event, &mut Islands::new(F::NAME, frontend));
    }
}

pub(super) fn instance<F: Surface>(instance: &Option<Rc<dyn Any>>) -> Option<Rc<Instance<F>>> {
    instance.clone()?.downcast().ok()
}

pub trait AnyFeature {
    fn name(&self) -> &'static str;
    fn widgets(&self) -> Vec<&'static str>;
    fn live_widgets(&self) -> &'static [&'static str];
    fn present(&self, widget: &str) -> bool;
    /// Kept by island so a folding widget can still draw what it last showed.
    fn snapshot(&self) -> Rc<dyn Any>;
    fn rest_width(&self, widget: &str, spacing: f32) -> f32;
    fn widget(&self, widget: &str, snapshot: &dyn Any, w: &mut WidgetCx<'_>) -> Option<BarWidget>;
    fn glow(&self) -> Option<Vec<f32>>;
    fn panel<'a>(&self, p: &mut PanelCx<'a>) -> Option<PanelView>;
    fn panel_opened(&self) -> Option<Box<dyn Any>>;
    fn dashboard_opened(&self) -> Option<Box<dyn Any>>;
    fn popup<'a>(&self, p: &mut PanelCx<'a>) -> Option<PanelView>;
    fn elapse(&self, local: &mut dyn AnyLocal, hovered: bool);
    fn pointer(&self, local: &mut dyn AnyLocal, over: bool);
    fn lead(&self, local: &dyn AnyLocal) -> Option<Mark>;
    fn dashboard_order(&self) -> u8;
    fn dashboard(&self, p: &mut PanelCx<'_>) -> Dashboard;
    fn batteries(&self) -> Vec<Battery>;
    fn inhibits_idle(&self) -> bool;
    fn command(&self, args: &[&str]) -> Reply;
    fn status(&self) -> Vec<Row>;
    fn key(&self, local: &mut dyn AnyLocal, keystroke: &Key) -> Outcome;
    fn released(&self, local: &mut dyn AnyLocal, modifier: Modifier) -> Outcome;
    fn new_local(&self, island: &IslandInfo<'_>) -> Box<dyn AnyLocal>;
    fn sync_local(&self, local: &mut dyn AnyLocal, island: &IslandInfo<'_>, animate: bool);
}

impl<F: Surface> AnyFeature for Rc<Instance<F>> {
    fn name(&self) -> &'static str {
        F::NAME
    }

    fn widgets(&self) -> Vec<&'static str> {
        widgets_of::<F>().collect()
    }

    fn live_widgets(&self) -> &'static [&'static str] {
        F::LIVE
    }

    fn present(&self, widget: &str) -> bool {
        F::present(widget, &self.state.borrow())
    }

    fn snapshot(&self) -> Rc<dyn Any> {
        self.state.borrow().clone()
    }

    fn rest_width(&self, widget: &str, spacing: f32) -> f32 {
        F::rest_width(widget, spacing)
    }

    fn widget(&self, widget: &str, snapshot: &dyn Any, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        let state = snapshot.downcast_ref::<F::State>()?;
        w.instance = Some(self.clone());
        w.live = self.live.borrow().clone();
        self.feature.widget(widget, state, w)
    }

    fn glow(&self) -> Option<Vec<f32>> {
        F::glow(&self.state.borrow(), &self.live.borrow()).map(<[f32]>::to_vec)
    }

    fn panel<'a>(&self, p: &mut PanelCx<'a>) -> Option<PanelView> {
        F::PANEL?;
        p.instance = Some(self.clone());
        self.feature.panel(&self.state.borrow(), p)
    }

    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        self.feature.panel_opened()
    }

    fn dashboard_opened(&self) -> Option<Box<dyn Any>> {
        self.feature.dashboard_opened()
    }

    fn popup<'a>(&self, p: &mut PanelCx<'a>) -> Option<PanelView> {
        p.instance = Some(self.clone());
        self.feature.popup(&self.state.borrow(), p)
    }

    fn elapse(&self, local: &mut dyn AnyLocal, hovered: bool) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            self.feature.elapse(local, hovered);
        }
    }

    fn pointer(&self, local: &mut dyn AnyLocal, over: bool) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            self.feature.pointer(local, over);
        }
    }

    fn lead(&self, local: &dyn AnyLocal) -> Option<Mark> {
        let local = local.as_any().downcast_ref::<F::Local>()?;
        self.feature.lead(local)
    }

    fn dashboard_order(&self) -> u8 {
        F::DASHBOARD_ORDER
    }

    fn dashboard(&self, p: &mut PanelCx<'_>) -> Dashboard {
        p.instance = Some(self.clone());
        self.feature.dashboard(&self.state.borrow(), p)
    }

    fn batteries(&self) -> Vec<Battery> {
        self.feature.batteries(&self.state.borrow())
    }

    fn inhibits_idle(&self) -> bool {
        F::inhibits_idle(&self.state.borrow())
    }

    fn command(&self, args: &[&str]) -> Reply {
        self.feature.reply(&self.state.borrow(), args)
    }

    fn status(&self) -> Vec<Row> {
        self.feature.status(&self.state.borrow())
    }

    fn key(&self, local: &mut dyn AnyLocal, keystroke: &Key) -> Outcome {
        let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() else {
            return Outcome::Close;
        };
        self.feature.key(&self.state.borrow(), local, keystroke)
    }

    fn released(&self, local: &mut dyn AnyLocal, modifier: Modifier) -> Outcome {
        let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() else {
            return Outcome::Open;
        };
        self.feature.released(&self.state.borrow(), local, modifier)
    }

    fn new_local(&self, island: &IslandInfo<'_>) -> Box<dyn AnyLocal> {
        let mut local = F::Local::default();
        F::sync_local(&self.state.borrow(), &mut local, island, false);
        Box::new(local)
    }

    fn sync_local(&self, local: &mut dyn AnyLocal, island: &IslandInfo<'_>, animate: bool) {
        if let Some(local) = local.as_any_mut().downcast_mut::<F::Local>() {
            F::sync_local(&self.state.borrow(), local, island, animate);
        }
    }
}

#[derive(Default)]
pub struct Features {
    pub(super) all: Vec<Rc<dyn AnyFeature>>,
    /// Alerts of every feature's batteries running low, once asked to.
    alerts: RefCell<Option<battery::Monitor>>,
}

impl Features {
    pub(super) fn new(all: Vec<Rc<dyn AnyFeature>>) -> Self {
        Self {
            all,
            alerts: RefCell::default(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&dyn AnyFeature> {
        self.all
            .iter()
            .find(|feature| feature.name() == name)
            .map(|feature| &**feature)
    }

    /// Every running feature's rows for `island ctl`'s overview, in the registry's order.
    pub fn status(&self) -> Vec<Row> {
        self.all.iter().flat_map(|feature| feature.status()).collect()
    }

    pub fn batteries(&self) -> Vec<Battery> {
        self.all
            .iter()
            .flat_map(|feature| feature.batteries())
            .collect()
    }

    /// Alerts when a battery runs low, as `settings` say, from now on.
    pub fn alert_batteries(&self, settings: BatterySettings) {
        *self.alerts.borrow_mut() = Some(battery::Monitor::new(settings));
        self.changed();
    }

    /// Some feature's state changed.
    pub fn changed(&self) {
        if let Some(alerts) = self.alerts.borrow_mut().as_mut() {
            alerts.update(self);
        }
    }

    pub fn owning(&self, widget: &str) -> Option<&dyn AnyFeature> {
        self.get(owner(widget)?)
    }

    pub fn iter(&self) -> impl Iterator<Item = &dyn AnyFeature> {
        self.all.iter().map(|feature| &**feature)
    }
}

fn arriving<F: Surface, T: 'static>(
    instance: &Rc<Instance<F>>,
    sent: UnboundedReceiver<T>,
    apply: fn(&Instance<F>, T, &mut dyn Frontend),
) -> LocalBoxStream<'static, Arrival> {
    let instance = instance.clone();
    sent.map(move |value| -> Arrival {
        let instance = instance.clone();
        Box::new(move |frontend| apply(&instance, value, frontend))
    })
    .boxed_local()
}

pub(super) fn start_one<F: Surface>(
    live: bool,
    settings: Option<toml::Value>,
    arrivals: &mut Arrivals,
) -> Rc<dyn AnyFeature> {
    let (sender, states) = unbounded();
    let (events, arrived) = unbounded();
    let mut feed = Feed::new(F::NAME, sender);
    feed.events = Some(events);
    feed.settings = settings;
    let levels = live.then(|| {
        let (sender, levels) = unbounded();
        feed.live = Some(sender);
        levels
    });
    let instance = Rc::new(Instance::new(F::start(feed)));
    if let Some(levels) = levels {
        arrivals.push(arriving(&instance, levels, Instance::set_live));
    }
    arrivals.push(arriving(&instance, states, Instance::set_state));
    arrivals.push(arriving(&instance, arrived, Instance::event));
    debug!(feature = F::NAME, "feature started");
    Rc::new(instance)
}

#[cfg(test)]
#[path = "instance_test.rs"]
mod tests;
