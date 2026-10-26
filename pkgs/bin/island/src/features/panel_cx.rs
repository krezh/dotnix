//! What a feature describes its panel with: its own island part, its glyphs, the marks other
//! features lend, and handlers that reach the feature or the island.

use std::{
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use gpui::{AnyEntity, Context, WeakEntity};

use super::{AnyLocal, Feature, Local, instance::instance};
use crate::{
    components::{dropdown::Select, slider::OnChange},
    glyphs::Glyphs,
    island::Island,
    panel::{self, Handler, Mark},
};

const LATE: Duration = Duration::from_millis(5);

pub struct PanelCx<'a> {
    glyphs: Glyphs,
    local: Option<&'a dyn AnyLocal>,
    redraw: &'a Cell<Option<Duration>>,
    pub(super) entity: Option<AnyEntity>,
    /// The mark a feature lends, by its name, as [`Feature::lead`] gives it; asked only when needed.
    lend: Box<dyn Fn(&str) -> Option<Mark> + 'a>,
}

impl<'a> PanelCx<'a> {
    pub(crate) fn new(
        glyphs: Glyphs,
        local: Option<&'a dyn AnyLocal>,
        redraw: &'a Cell<Option<Duration>>,
        lend: Box<dyn Fn(&str) -> Option<Mark> + 'a>,
    ) -> Self {
        Self {
            glyphs,
            local,
            redraw,
            entity: None,
            lend,
        }
    }

    /// Asks to redraw the panel after `after`, when time-based text it shows next changes.
    pub fn redraw_in(&self, after: Duration) {
        let after = after + LATE;
        self.redraw.set(Some(
            self.redraw.get().map_or(after, |prior| prior.min(after)),
        ));
    }

    /// The mark the feature named `name` lends, if it lends one now.
    pub fn lead(&self, name: &str) -> Option<Mark> {
        (self.lend)(name)
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
