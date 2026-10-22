//! What a feature draws its bar widget with: the bar's look, its glyphs, and handlers that reach
//! the feature or the island.

use std::path::Path;

use gpui::{
    AnyEntity, App, Context, Div, FontWeight, SharedString, Stateful, WeakEntity, Window,
};

use super::{AnyLocal, Feature, Local, instance::instance};
use crate::{
    bar::{self, Gauge, Look, Slots},
    components::{
        marquee::{self, TextLayouts},
        tooltip::Tooltips,
    },
    glyphs::Glyphs,
    island::{Island, Panel},
};

pub struct WidgetCx<'a> {
    look: Look<'a>,
    slots: Slots,
    glyphs: &'a Glyphs,
    layouts: &'a TextLayouts,
    island: &'a WeakEntity<Island>,
    local: Option<&'a dyn AnyLocal>,
    pub(super) entity: Option<AnyEntity>,
    pub(super) live: Vec<f32>,
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
