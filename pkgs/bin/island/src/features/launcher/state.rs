use std::sync::Arc;

use super::{
    desktop::{self, App},
    history,
};

use crate::components::list::Selection;

#[derive(Clone, Default)]
pub struct Apps(pub(super) Arc<Vec<App>>);

impl PartialEq for Apps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Apps {
    fn matching(&self, query: &str) -> Vec<usize> {
        desktop::search(&self.0, query, &history::ranks())
    }
}

#[derive(Default)]
pub struct Local {
    pub(super) selection: Selection,
    apps: Apps,
    matching: Vec<usize>,
}

impl Local {
    pub(super) fn sync(&mut self, apps: &Apps) {
        let matching = apps.matching(&self.selection.query);
        let selected = self
            .selected()
            .and_then(|selected| {
                matching
                    .iter()
                    .position(|&index| apps.0[index].id == selected.id)
            })
            .unwrap_or(self.selection.selected);
        self.selection.select(selected, matching.len());
        if &self.apps != apps {
            self.apps = apps.clone();
        }
        self.matching = matching;
    }

    pub(super) fn matching(&self) -> impl ExactSizeIterator<Item = &App> {
        self.matching.iter().map(|&index| &self.apps.0[index])
    }

    pub(super) fn selected(&self) -> Option<&App> {
        self.matching
            .get(self.selection.selected)
            .map(|&index| &self.apps.0[index])
    }

    pub(super) fn select(&mut self, position: usize) {
        self.selection.select(position, self.matching.len());
    }

    pub(super) fn step(&mut self, step: i32) {
        self.selection.step(step, self.matching.len());
    }

    pub(super) fn search(&mut self, change: impl FnOnce(&mut String)) {
        self.selection.search(change);
        self.matching = self.apps.matching(&self.selection.query);
    }
}

impl crate::features::Local for Local {
    fn panel_closed(&mut self) {
        self.selection.reset();
        self.matching = self.apps.matching("");
    }
}

#[cfg(test)]
#[path = "state_test.rs"]
mod tests;
