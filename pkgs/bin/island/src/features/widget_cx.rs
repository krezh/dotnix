//! What a feature describes its bar widget with: the bar's measures, its glyphs, and handlers that
//! reach the feature or the island.

use std::{any::Any, cell::Cell, path::Path, rc::Rc, time::Duration};

use super::{AnyLocal, Battery, Local, Surface, instance::instance};
use crate::{
    bar::ICON_GAP,
    bar_widget::{BarWidget, Frame, Gauge, Part, Pointer, Press, Scroll},
    glyphs::Glyphs,
    panel::{SharedString, Ui},
    stage::Panel,
};

const LATE: Duration = Duration::from_millis(5);

pub struct WidgetCx<'a> {
    padding: f32,
    grouped: bool,
    glyphs: &'a Glyphs,
    local: Option<&'a dyn AnyLocal>,
    redraw: Cell<Option<Duration>>,
    pub(super) instance: Option<Rc<dyn Any>>,
    pub(super) live: Vec<f32>,
}

impl<'a> WidgetCx<'a> {
    pub(crate) fn new(
        padding: f32,
        grouped: bool,
        glyphs: &'a Glyphs,
        local: Option<&'a dyn AnyLocal>,
    ) -> Self {
        Self {
            padding,
            grouped,
            glyphs,
            local,
            redraw: Cell::new(None),
            instance: None,
            live: Vec::new(),
        }
    }

    /// Asks to redraw the widget after `after`, when time-based text it shows next changes.
    pub fn redraw_in(&self, after: Duration) {
        let after = after + LATE;
        self.redraw.set(Some(
            self.redraw.get().map_or(after, |prior| prior.min(after)),
        ));
    }

    pub(crate) fn next_redraw(&self) -> Option<Duration> {
        self.redraw.get()
    }

    pub fn local<T: Local>(&self) -> Option<&'a T> {
        self.local?.as_any().downcast_ref()
    }

    pub fn live(&self) -> &[f32] {
        &self.live
    }

    pub fn padding(&self) -> f32 {
        self.padding
    }

    pub fn base(&self, id: impl Into<SharedString>) -> BarWidget {
        BarWidget::new(id, Frame::Control)
    }

    /// A widget of marks beside short, strong text.
    pub fn compact(&self, id: impl Into<SharedString>) -> BarWidget {
        self.base(id).gap(ICON_GAP).small()
    }

    /// For a widget made of several clickable parts, each a [`Frame::Spot`].
    pub fn group(&self, id: impl Into<SharedString>) -> BarWidget {
        BarWidget::new(id, Frame::Bare)
    }

    pub fn gauge(&self, id: impl Into<SharedString>, gauge: Gauge<'_>) -> BarWidget {
        self.base(id)
            .gap(ICON_GAP)
            .glyph(gauge.glyph, gauge.glyph_color)
            .part(Part::Meter {
                level: gauge.level,
                color: gauge.level_color,
            })
            .part(Part::Busy(gauge.busy))
    }

    /// A battery's gauge, drawn as the dashboard draws it: its glyph, and how full it is in the
    /// shared colors.
    pub fn battery(&self, id: &'static str, battery: &Battery, busy: bool) -> BarWidget {
        self.gauge(
            id,
            Gauge {
                glyph: self.glyph(battery.glyph),
                glyph_color: battery.glyph_color(),
                busy,
                level: Some(battery.fraction()),
                level_color: battery.color(),
            },
        )
    }

    pub fn glyph(&self, key: &str) -> Option<&'a Path> {
        self.glyphs.get(key)
    }

    /// In a group of widgets, every part opens the dashboard instead, the group being one widget.
    pub fn toggle(&self, panel: Panel) -> Press {
        let panel = if self.grouped {
            Panel::Dashboard
        } else {
            panel
        };
        Rc::new(move |_, ui: &mut dyn Ui| ui.toggle(panel))
    }

    pub fn call<F: Surface>(&self, action: impl Fn(&F) + 'static) -> Press {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |_, _: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature);
            }
        })
    }

    pub fn on_island<F: Surface>(
        &self,
        action: impl Fn(&F, Pointer, &mut dyn Ui) + 'static,
    ) -> Press {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |pointer, ui: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature, pointer, ui);
            }
        })
    }

    pub fn on_scroll<F: Surface>(&self, action: impl Fn(&F, f32) + 'static) -> Scroll {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |delta, _: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature, delta);
            }
        })
    }
}
