//! Every island's own part of one feature, for the feature to find and change.

use std::marker::PhantomData;

use gpui::{App, Entity, WeakEntity};

use super::Local;
use crate::island::{Island, Placement};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandId(usize);

pub struct Islands<'a, L> {
    name: &'static str,
    list: Vec<Entity<Island>>,
    cx: &'a mut App,
    local: PhantomData<fn(&mut L)>,
}

impl<'a, L: Local> Islands<'a, L> {
    pub(super) fn new(name: &'static str, cx: &'a mut App) -> Self {
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
