use std::sync::Arc;

use super::state::{Provider, Source, State};
use crate::features::Local;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Account {
    source: Source,
    id: String,
}

impl Account {
    pub(super) fn of(provider: &Provider) -> Self {
        Self {
            source: provider.source,
            id: provider.id.clone(),
        }
    }

    fn matches(&self, provider: &Provider) -> bool {
        self.source == provider.source && self.id == provider.id
    }

    pub(super) fn element_id(&self) -> String {
        format!("ai-account-{}-{}", self.source.name(), self.id)
    }
}

#[derive(Default)]
pub struct View {
    selected: Option<Account>,
    order: Vec<Account>,
    seen: Option<Arc<Vec<Provider>>>,
    open: bool,
}

impl View {
    pub(super) fn sync(&mut self, state: &State, open: bool) {
        if self.open == open
            && self
                .seen
                .as_ref()
                .is_some_and(|seen| Arc::ptr_eq(seen, &state.providers))
        {
            return;
        }
        self.open = open;
        if !open {
            self.selected = None;
            self.order.clear();
        } else {
            self.order.retain(|account| {
                state
                    .providers
                    .iter()
                    .any(|provider| !provider.is_empty() && account.matches(provider))
            });
            if self.selected.as_ref().is_some_and(|account| {
                !state
                    .providers
                    .iter()
                    .any(|provider| !provider.is_empty() && account.matches(provider))
            }) {
                self.selected = None;
            }
        }
        for provider in state
            .providers
            .iter()
            .filter(|provider| !provider.is_empty())
        {
            if !self.order.iter().any(|account| account.matches(provider)) {
                self.order.push(Account::of(provider));
            }
        }
        self.seen = Some(state.providers.clone());
    }

    pub(super) fn select(&mut self, account: Account) {
        self.selected = Some(account);
    }

    pub(super) fn back(&mut self) {
        self.selected = None;
    }

    pub(super) fn selected<'a>(&self, state: &'a State) -> Option<&'a Provider> {
        let selected = self.selected.as_ref()?;
        state
            .providers
            .iter()
            .find(|provider| !provider.is_empty() && selected.matches(provider))
    }

    pub(super) fn ordered<'a>(&self, state: &'a State) -> Vec<&'a Provider> {
        if self.order.is_empty() {
            return state
                .providers
                .iter()
                .filter(|provider| !provider.is_empty())
                .collect();
        }
        self.order
            .iter()
            .filter_map(|account| {
                state
                    .providers
                    .iter()
                    .find(|provider| !provider.is_empty() && account.matches(provider))
            })
            .collect()
    }
}

impl Local for View {
    fn panel_closed(&mut self) {
        self.selected = None;
        self.order.clear();
        self.seen = None;
        self.open = false;
    }
}

#[cfg(test)]
#[path = "view_test.rs"]
mod tests;
