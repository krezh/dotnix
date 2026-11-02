//! Applies what running features send to the GPUI islands.

use std::borrow::Cow;

use futures::StreamExt;
use gpui::{App, AssetSource, Entity, Global, Keystroke, SharedString, WeakEntity};

use crate::{
    config,
    features::{AnyLocal, Arrivals, Frontend},
    glyphs::Glyphs,
    icons,
    island::{Island, Placement},
    key::Key,
};

impl Global for Glyphs {}
impl Global for config::System {}

pub struct Files;

impl AssetSource for Files {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        Ok(icons::load(path))
    }

    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

impl From<&Keystroke> for Key {
    fn from(keystroke: &Keystroke) -> Self {
        Self {
            name: keystroke.key.clone(),
            typed: keystroke.key_char.clone(),
            control: keystroke.modifiers.control,
            shift: keystroke.modifiers.shift,
            alt: keystroke.modifiers.alt,
        }
    }
}

struct GpuiFrontend<'a> {
    islands: Vec<Entity<Island>>,
    cx: &'a mut App,
}

impl<'a> GpuiFrontend<'a> {
    fn new(cx: &'a mut App) -> Self {
        let islands = cx
            .try_global::<Placement>()
            .map(|placement| {
                placement
                    .islands
                    .iter()
                    .filter_map(WeakEntity::upgrade)
                    .collect()
            })
            .unwrap_or_default();
        Self { islands, cx }
    }
}

impl Frontend for GpuiFrontend<'_> {
    fn islands(&self) -> usize {
        self.islands.len()
    }

    fn local(&self, island: usize, name: &str) -> Option<&dyn AnyLocal> {
        self.islands
            .get(island)?
            .read(&*self.cx)
            .stage()
            .local(name)
    }

    fn update_local(
        &mut self,
        island: usize,
        name: &'static str,
        action: &mut dyn FnMut(&mut dyn AnyLocal),
    ) {
        let Some(entity) = self.islands.get(island).cloned() else {
            return;
        };
        entity.update(self.cx, |island, _| island.update_local(name, action));
        let island = entity.downgrade();
        self.cx.defer(move |cx| {
            let _ = island.update(cx, |island, cx| island.local_changed(name, cx));
        });
    }

    fn changed(&mut self, name: &'static str) {
        for island in &self.islands {
            island.update(self.cx, |island, cx| island.feature_changed(name, cx));
        }
    }

    fn live(&mut self, name: &'static str) {
        for island in &self.islands {
            island.update(self.cx, |island, cx| island.feature_live(name, cx));
        }
    }
}

pub fn follow(mut arrivals: Arrivals, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Some(arrival) = arrivals.next().await {
            cx.update(|cx| arrival(&mut GpuiFrontend::new(cx)));
        }
    })
    .detach();
}
