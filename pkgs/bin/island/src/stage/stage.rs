//! One island's state, whatever draws it: what is open, what the pointer is over, the features'
//! parts of it, its monitor, and what each panel shows.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    time::Duration,
    time::Instant,
};

use uuid::Uuid;

use super::{COLLAPSED_H, COLLAPSED_LAYER, FOLD_SPRING, Grab, Mode, Morph, Panel, Shape};
use crate::{
    config,
    features::{self, AnyLocal, Battery, Features, IslandInfo, PanelCx},
    glyphs::Glyphs,
    panel::PanelView,
    presence::Presence,
    wm::{Monitor, Snapshot},
};

const HOVER_GROW: f32 = 4.0;

/// Kept so a feature widget can still draw while it folds away.
pub type Held = Rc<dyn Any>;

pub struct Stage {
    pub mode: Mode,
    /// A feature panel on its way out, told it has closed once it is out of sight.
    pub closing: Option<&'static str>,
    /// Whose marks the panel being shown borrowed.
    pub lent: RefCell<HashSet<String>>,
    pub hovered: bool,
    pub fullscreen: bool,
    pub morph: Morph,
    pub features: Rc<Features>,
    pub locals: Vec<(&'static str, Box<dyn AnyLocal>)>,
    pub held: Presence<Held>,
    pub layout: Rc<config::Bar>,
    pub workspaces: Option<Snapshot>,
    pub display: Option<Uuid>,
    /// The compositor snapshot changed, so the island's monitor and the features' per-island
    /// parts need another look.
    pub monitor_stale: bool,
}

/// The bar's width at rest, or with the pointer on it.
pub fn collapsed_width(bar: f32, hovered: bool) -> f32 {
    if hovered { bar + 4.0 * HOVER_GROW } else { bar }
}

impl Stage {
    pub fn new(
        layout: Rc<config::Bar>,
        features: Rc<Features>,
        workspaces: Option<Snapshot>,
    ) -> Self {
        let nowhere = IslandInfo {
            monitor: None,
            panel_open: false,
        };
        let locals = features
            .iter()
            .map(|feature| (feature.name(), feature.new_local(&nowhere)))
            .collect();
        let mut stage = Self {
            mode: Mode::Collapsed,
            closing: None,
            lent: RefCell::default(),
            hovered: false,
            fullscreen: false,
            morph: Morph::new(),
            features,
            locals,
            held: Presence::new(FOLD_SPRING),
            layout,
            monitor_stale: workspaces.is_some(),
            workspaces,
            display: None,
        };
        stage.sync_held();
        stage
    }

    pub fn hidden(&self) -> bool {
        self.mode == Mode::Collapsed && self.fullscreen
    }

    pub fn displays(&self) -> usize {
        self.workspaces
            .as_ref()
            .map_or(1, |snapshot| snapshot.monitors.len().max(1))
    }

    /// Whether what is open shows anything of `name`'s part, so changes when it moves.
    pub fn shows_local(&self, name: &str) -> bool {
        match self.mode {
            Mode::Collapsed | Mode::Open(Panel::System) => false,
            Mode::Open(Panel::Dashboard) => true,
            Mode::Open(Panel::Popups) => self.local(name).is_some_and(|local| local.popup()),
            Mode::Open(Panel::Feature(open)) => open == name || self.lent.borrow().contains(name),
        }
    }

    pub fn update_local(&mut self, name: &str, action: impl FnOnce(&mut dyn AnyLocal)) {
        if let Some((_, local)) = self.locals.iter_mut().find(|(owner, _)| *owner == name) {
            action(&mut **local);
        }
    }

    pub fn local(&self, name: &str) -> Option<&dyn AnyLocal> {
        self.locals
            .iter()
            .find(|(owner, _)| *owner == name)
            .map(|(_, local)| &**local)
    }

    pub fn popup_waiting(&self) -> bool {
        self.locals.iter().any(|(_, local)| local.popup())
    }

    pub fn next_due(&self) -> Option<Instant> {
        self.locals
            .iter()
            .filter_map(|(_, local)| local.due())
            .min()
    }

    /// Tells every feature whose time came, and gives their names.
    pub fn elapse(&mut self, now: Instant) -> Vec<&'static str> {
        let features = self.features.clone();
        let mut elapsed = Vec::new();
        for (name, local) in &mut self.locals {
            if local.due().is_some_and(|at| at <= now)
                && let Some(feature) = features.get(name)
            {
                feature.elapse(&mut **local, self.hovered);
                elapsed.push(*name);
            }
        }
        elapsed
    }

    pub fn set_pointer(&mut self, hovered: bool) {
        self.hovered = hovered;
        let features = self.features.clone();
        for (name, local) in &mut self.locals {
            local.pointer(hovered);
            if let Some(feature) = features.get(name) {
                feature.pointer(&mut **local, hovered);
            }
        }
    }

    /// Collapsing shows the pop-ups instead while any is waiting.
    pub fn resolved(&self, mode: Mode) -> Mode {
        match mode {
            Mode::Collapsed if self.popup_waiting() => Mode::Open(Panel::Popups),
            mode => mode,
        }
    }

    pub fn grab(&self, menu_open: bool) -> Grab {
        let wants_keys = |panel: Panel| {
            panel.takes_keyboard()
                && match panel {
                    Panel::Feature(name) => self.local(name).is_none_or(|local| local.keyboard()),
                    _ => true,
                }
        };
        if matches!(self.mode, Mode::Open(panel) if wants_keys(panel)) {
            Grab::Keyboard
        } else if menu_open {
            Grab::Clicks
        } else {
            Grab::None
        }
    }

    pub fn panel_shape(&self, kind: Panel) -> Option<Shape> {
        self.morph.frozen[kind.layer()].map(|(_, shape)| shape)
    }

    pub fn active_layer(&self) -> Option<usize> {
        match self.mode {
            Mode::Collapsed if self.fullscreen => None,
            Mode::Collapsed => Some(COLLAPSED_LAYER),
            Mode::Open(kind) => Some(kind.layer()),
        }
    }

    /// `bar` is the bar's width at rest, once it is known.
    pub fn desired_shape(&self, bar: Option<f32>) -> Option<Shape> {
        match self.mode {
            Mode::Collapsed if self.fullscreen => Some(Shape { w: 0.0, h: 0.0 }),
            Mode::Collapsed => bar.map(|bar| Shape {
                w: collapsed_width(bar, self.hovered),
                h: COLLAPSED_H,
            }),
            Mode::Open(kind) => self.panel_shape(kind),
        }
    }

    /// Whether the island, its layers and the features' parts have stopped moving, as
    /// `local_still` judges each part.
    pub fn still(&self, local_still: impl Fn(&dyn AnyLocal) -> bool) -> bool {
        self.morph.is_settled()
            && (self.hidden() || self.locals.iter().all(|(_, local)| local_still(&**local)))
            && self.held.is_settled()
    }

    pub fn advance(&mut self, dt: f32, now: Instant) {
        self.morph.advance(dt, now);
        for (_, local) in &mut self.locals {
            local.advance(dt, now);
        }
        self.held.advance(dt, now);
        if let Some(name) = self.closing {
            let layer = Panel::Feature(name).layer();
            if self.morph.layers[layer].value() <= 0.001 && self.morph.slides[layer].is_settled() {
                self.closing = None;
                self.update_local(name, |local| local.panel_closed());
            }
        }
    }

    /// Tells a panel still closing that it has, as everything has stopped.
    pub fn finish_closing(&mut self) {
        if let Some(name) = self.closing.take() {
            self.update_local(name, |local| local.panel_closed());
        }
    }

    /// Keeps the last state of every feature widget that has something to show, so a widget that
    /// loses its data can still be drawn while it folds away.
    pub fn sync_held(&mut self) {
        let items: Vec<(String, Held)> = self
            .layout
            .widgets()
            .filter_map(|kind| {
                let config::Widget(widget) = kind;
                let feature = self
                    .features
                    .owning(widget)
                    .filter(|feature| feature.present(widget))?;
                Some((widget.to_owned(), feature.snapshot()))
            })
            .collect();
        self.held.sync(&items, self.morph.sized);
    }

    /// The name of the output it is on, such as `DP-1`.
    pub fn output(&self) -> Option<String> {
        self.monitor().map(|monitor| monitor.name.clone())
    }

    pub fn monitor(&self) -> Option<&Monitor> {
        let monitors = &self.workspaces.as_ref()?.monitors;
        self.display
            .and_then(|display| {
                monitors.iter().find(|monitor| {
                    Uuid::new_v5(&Uuid::NAMESPACE_DNS, monitor.name.as_bytes()) == display
                })
            })
            .or_else(|| monitors.iter().find(|monitor| monitor.focused))
    }

    pub fn sync_local(&mut self, name: &'static str) {
        let animate = self.morph.sized;
        let monitor = self.monitor().cloned();
        let here = IslandInfo {
            monitor: monitor.as_ref(),
            panel_open: self.mode == Mode::Open(Panel::Feature(name)),
        };
        if let (Some(feature), Some((_, local))) = (
            self.features.get(name),
            self.locals.iter_mut().find(|(owner, _)| *owner == name),
        ) {
            feature.sync_local(&mut **local, &here, animate);
        }
    }

    pub fn sync_locals(&mut self, monitor: Option<&Monitor>) {
        let animate = self.morph.sized;
        for (name, local) in &mut self.locals {
            if let Some(feature) = self.features.get(name) {
                let here = IslandInfo {
                    monitor,
                    panel_open: self.mode == Mode::Open(Panel::Feature(name)),
                };
                feature.sync_local(&mut **local, &here, animate);
            }
        }
    }

    fn panel_cx<'a>(
        &'a self,
        name: &str,
        glyphs: &Glyphs,
        redraw: &'a Cell<Option<Duration>>,
    ) -> PanelCx<'a> {
        let lend = move |lender: &str| {
            self.lent.borrow_mut().insert(lender.to_owned());
            let (_, local) = self.locals.iter().find(|(owner, _)| *owner == lender)?;
            self.features.get(lender)?.lead(&**local)
        };
        PanelCx::new(glyphs.clone(), self.local(name), redraw, Box::new(lend))
    }

    pub fn feature_panel(
        &self,
        name: &str,
        glyphs: &Glyphs,
        redraw: &Cell<Option<Duration>>,
    ) -> Option<PanelView> {
        self.features
            .get(name)?
            .panel(&mut self.panel_cx(name, glyphs, redraw))
    }

    pub fn popups(&self, glyphs: &Glyphs, redraw: &Cell<Option<Duration>>) -> Option<PanelView> {
        let blocks: Vec<_> = self
            .locals
            .iter()
            .filter(|(_, local)| local.popup())
            .filter_map(|(name, _)| {
                self.features
                    .get(name)?
                    .popup(&mut self.panel_cx(name, glyphs, redraw))
            })
            .flat_map(|view| view.blocks)
            .collect();
        (!blocks.is_empty()).then_some(PanelView {
            header: crate::panel::Header::None,
            blocks,
            scroll: false,
        })
    }

    /// What every running feature puts on the dashboard's overview, in their places.
    pub fn dashboard(
        &self,
        glyphs: &Glyphs,
        redraw: &Cell<Option<Duration>>,
    ) -> (features::Dashboard, Vec<Battery>) {
        let mut features: Vec<_> = self.features.iter().collect();
        features.sort_by_key(|feature| feature.dashboard_order());
        let mut all = features::Dashboard::default();
        let mut batteries = Vec::new();
        for feature in features {
            let part = feature.dashboard(&mut self.panel_cx(feature.name(), glyphs, redraw));
            all.switches.extend(part.switches);
            all.cards.extend(part.cards);
            all.warnings.extend(part.warnings);
            batteries.extend(feature.batteries());
        }
        (all, batteries)
    }
}

#[cfg(test)]
#[path = "stage_test.rs"]
mod tests;
