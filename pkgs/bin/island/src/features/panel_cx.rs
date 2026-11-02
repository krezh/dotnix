//! What a feature describes its panel with: its own island part, its glyphs, the marks other
//! features lend, and handlers that reach the feature or the island.

use std::{
    any::Any,
    cell::Cell,
    path::{Path, PathBuf},
    rc::Rc,
    time::Duration,
};

use super::{AnyLocal, Local, Surface, instance::instance};
use crate::{
    components::{dropdown::Select, slider::OnChange},
    glyphs::Glyphs,
    panel::{Handler, Mark, Ui},
};

const LATE: Duration = Duration::from_millis(5);

pub struct PanelCx<'a> {
    glyphs: Glyphs,
    local: Option<&'a dyn AnyLocal>,
    redraw: &'a Cell<Option<Duration>>,
    pub(super) instance: Option<Rc<dyn Any>>,
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
            instance: None,
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

    pub fn on_local<F: Surface>(&self, action: impl Fn(&F, &mut F::Local) + 'static) -> Handler {
        let with = self.on_local_with::<F, ()>(move |feature, local, ()| action(feature, local));
        Rc::new(move |ui: &mut dyn Ui| with((), ui))
    }

    pub fn on_local_with<F: Surface, T: 'static>(
        &self,
        action: impl Fn(&F, &mut F::Local, T) + 'static,
    ) -> Rc<dyn Fn(T, &mut dyn Ui)> {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |value, ui: &mut dyn Ui| {
            let Some(instance) = &instance else {
                return;
            };
            let mut value = Some(value);
            ui.update_local(F::NAME, &mut |local| {
                if let (Some(local), Some(value)) =
                    (local.as_any_mut().downcast_mut::<F::Local>(), value.take())
                {
                    action(&instance.feature, local, value);
                }
            });
        })
    }

    pub fn call<F: Surface>(&self, action: impl Fn(&F) + 'static) -> Handler {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |_: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature);
            }
        })
    }

    pub fn on_island<F: Surface>(&self, action: impl Fn(&F, &mut dyn Ui) + 'static) -> Handler {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |ui: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature, ui);
            }
        })
    }

    pub fn slider<F: Surface>(&self, action: impl Fn(&F, f32) + 'static) -> OnChange {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |value, _: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature, value);
            }
        })
    }

    /// For a dropdown: called with the index of the choice picked.
    pub fn pick<F: Surface>(&self, action: impl Fn(&F, usize) + 'static) -> Select {
        let instance = instance::<F>(&self.instance);
        Rc::new(move |index, _: &mut dyn Ui| {
            if let Some(instance) = &instance {
                action(&instance.feature, index);
            }
        })
    }
}
