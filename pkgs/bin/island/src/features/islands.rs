//! Every island's own part of one feature, for the feature to find and change, and what the
//! frontend does for a running feature.

use std::marker::PhantomData;

use super::{AnyLocal, Local};

/// What whatever draws the islands does for the features.
pub trait Frontend {
    fn islands(&self) -> usize;
    fn local(&self, island: usize, name: &str) -> Option<&dyn AnyLocal>;
    fn update_local(
        &mut self,
        island: usize,
        name: &'static str,
        action: &mut dyn FnMut(&mut dyn AnyLocal),
    );
    /// The feature's state changed.
    fn changed(&mut self, name: &'static str);
    /// The feature's live levels changed.
    fn live(&mut self, name: &'static str);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IslandId(usize);

pub struct Islands<'a, L> {
    name: &'static str,
    frontend: &'a mut dyn Frontend,
    local: PhantomData<fn(&mut L)>,
}

impl<'a, L: Local> Islands<'a, L> {
    pub(super) fn new(name: &'static str, frontend: &'a mut dyn Frontend) -> Self {
        Self {
            name,
            frontend,
            local: PhantomData,
        }
    }

    pub fn find(&self, test: impl Fn(&L) -> bool) -> Option<IslandId> {
        (0..self.frontend.islands())
            .find(|island| {
                self.frontend
                    .local(*island, self.name)
                    .and_then(|local| local.as_any().downcast_ref())
                    .is_some_and(&test)
            })
            .map(IslandId)
    }

    pub fn update(&mut self, island: IslandId, action: impl FnOnce(&mut L)) {
        let mut action = Some(action);
        self.frontend
            .update_local(island.0, self.name, &mut |local| {
                if let (Some(local), Some(action)) =
                    (local.as_any_mut().downcast_mut::<L>(), action.take())
                {
                    action(local);
                }
            });
    }

    pub fn update_all(&mut self, mut action: impl FnMut(&mut L)) {
        for index in 0..self.frontend.islands() {
            self.update(IslandId(index), &mut action);
        }
    }
}
