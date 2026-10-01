use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gtk::prelude::{ButtonExt, WidgetExt};

use crate::changelog::ChangelogOutput;
use crate::config::Appearance;
use crate::ui::switch::SwitchConfirmation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Operation {
    #[default]
    Idle,
    Building,
    Updating,
    Switching,
}

pub(crate) struct UiState {
    pub(crate) home_buttons: RefCell<Vec<gtk::Button>>,
    pub(crate) home_shortcuts: RefCell<HashMap<char, gtk::Button>>,
    pub(crate) scroll: RefCell<Option<gtk::Adjustment>>,
    pub(crate) switch_confirmation: RefCell<Option<SwitchConfirmation>>,
    pub(crate) back_button: RefCell<Option<gtk::Button>>,
    pub(crate) operation: Cell<Operation>,
    pub(crate) generation: Cell<u64>,
    pub(crate) cancellation: RefCell<Option<Arc<AtomicBool>>>,
    pub(crate) view_cancellation: RefCell<Option<Arc<AtomicBool>>>,
    pub(crate) close_pending: Cell<bool>,
    pub(crate) changelog_cache: RefCell<HashMap<String, ChangelogOutput>>,
    pub(crate) appearance: Appearance,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            home_buttons: RefCell::new(Vec::new()),
            home_shortcuts: RefCell::new(HashMap::new()),
            scroll: RefCell::new(None),
            switch_confirmation: RefCell::new(None),
            back_button: RefCell::new(None),
            operation: Cell::new(Operation::Idle),
            generation: Cell::new(0),
            cancellation: RefCell::new(None),
            view_cancellation: RefCell::new(None),
            close_pending: Cell::new(false),
            changelog_cache: RefCell::new(HashMap::new()),
            appearance: Appearance::default(),
        }
    }
}

impl UiState {
    pub(crate) fn clear_actions(&self) {
        self.cancel_view();
        self.home_buttons.borrow_mut().clear();
        self.home_shortcuts.borrow_mut().clear();
        self.scroll.replace(None);
        let confirmation = self.switch_confirmation.borrow_mut().take();
        if let Some(confirmation) = confirmation {
            confirmation.reset();
        }
        self.back_button.replace(None);
    }
    pub(crate) fn register_home_action(&self, key: char, button: &gtk::Button) {
        self.home_shortcuts
            .borrow_mut()
            .insert(key.to_ascii_lowercase(), button.clone());
    }

    pub(crate) fn activate_home_action(&self, key: char) -> bool {
        let button = self
            .home_shortcuts
            .borrow()
            .get(&key.to_ascii_lowercase())
            .cloned();
        if let Some(button) = button
            && button.is_visible()
            && button.is_sensitive()
        {
            button.emit_clicked();
            true
        } else {
            false
        }
    }

    pub(crate) fn set_back_button(&self, button: &gtk::Button) {
        self.back_button.replace(Some(button.clone()));
    }

    pub(crate) fn set_scroll_adjustment(&self, adjustment: gtk::Adjustment) {
        self.scroll.replace(Some(adjustment));
    }

    pub(crate) fn set_view_cancellation(&self, cancellation: &Arc<AtomicBool>) {
        if let Some(previous) = self
            .view_cancellation
            .replace(Some(Arc::clone(cancellation)))
        {
            previous.store(true, Ordering::Relaxed);
        }
    }
    pub(crate) fn cancel_view(&self) {
        if let Some(cancellation) = self.view_cancellation.borrow_mut().take() {
            cancellation.store(true, Ordering::Relaxed);
        }
    }

    pub(crate) fn begin(&self, operation: Operation) -> Option<(u64, Arc<AtomicBool>)> {
        if self.operation.get() != Operation::Idle || self.close_pending.get() {
            return None;
        }
        let generation = self.generation.get().wrapping_add(1);
        let cancellation = Arc::new(AtomicBool::new(false));
        self.generation.set(generation);
        self.operation.set(operation);
        self.cancellation.replace(Some(Arc::clone(&cancellation)));
        Some((generation, cancellation))
    }

    pub(crate) fn finish(&self, generation: u64) -> bool {
        if self.generation.get() != generation {
            return false;
        }
        self.operation.set(Operation::Idle);
        self.cancellation.replace(None);
        true
    }

    pub(crate) fn request_close(&self) -> bool {
        match self.operation.get() {
            Operation::Switching => false,
            Operation::Building | Operation::Updating => {
                self.close_pending.set(true);
                if let Some(cancellation) = self.cancellation.borrow().as_ref() {
                    cancellation.store(true, Ordering::Relaxed);
                }
                false
            }
            Operation::Idle => self.cancel(),
        }
    }

    pub(crate) fn cancel(&self) -> bool {
        if self.operation.get() == Operation::Switching {
            return false;
        }
        let cancellation = self.cancellation.borrow().clone();
        if let Some(cancellation) = cancellation {
            cancellation.store(true, Ordering::Relaxed);
        }
        self.generation.set(self.generation.get().wrapping_add(1));
        self.operation.set(Operation::Idle);
        self.cancellation.replace(None);
        self.cancel_view();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_cannot_be_cancelled_from_navigation() {
        let state = UiState::default();
        state.operation.set(Operation::Switching);
        assert!(!state.cancel());
        assert_eq!(state.operation.get(), Operation::Switching);
    }

    #[test]
    fn close_waits_for_the_build_worker() {
        let state = UiState::default();
        let (generation, cancellation) = state.begin(Operation::Building).unwrap();

        assert!(!state.request_close());
        assert!(cancellation.load(Ordering::Relaxed));
        assert!(state.close_pending.get());
        assert_eq!(state.operation.get(), Operation::Building);

        assert!(state.finish(generation));
        assert!(state.close_pending.replace(false));
    }

    #[test]
    fn clear_actions_cancels_view_and_clears_state() {
        let state = UiState::default();
        let cancellation = Arc::new(AtomicBool::new(false));
        state.set_view_cancellation(&cancellation);
        state.clear_actions();
        assert!(cancellation.load(Ordering::Relaxed));
        assert!(state.view_cancellation.borrow().is_none());
        assert!(state.back_button.borrow().is_none());
    }
}
