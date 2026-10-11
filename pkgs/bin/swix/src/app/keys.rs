//! Keyboard shortcut mapping and key dispatch on the UI state.

use gtk::gdk;
use gtk::prelude::*;

use crate::app::state::UiState;
use crate::ui::glide;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyAction {
    Back,
    Close,
    FocusNext,
    Home(char),
    ScrollDown,
    ScrollEnd,
    ScrollHome,
    ScrollPageDown,
    ScrollPageUp,
    ScrollUp,
    Switch,
}

impl UiState {
    pub(super) fn is_home(&self) -> bool {
        !self.home_buttons.borrow().is_empty()
    }

    pub(super) fn focus_home(&self, delta: isize) -> bool {
        let button = {
            let buttons = self
                .home_buttons
                .borrow()
                .iter()
                .filter(|button| button.is_visible() && button.is_sensitive())
                .cloned()
                .collect::<Vec<_>>();
            if buttons.is_empty() {
                return false;
            }
            let current = buttons
                .iter()
                .position(|button| button.has_focus())
                .unwrap_or(0);
            let next = current
                .saturating_add_signed(delta)
                .min(buttons.len().saturating_sub(1));
            buttons[next].clone()
        };
        button.grab_focus();
        true
    }

    /// Glides the current scroll target by one step of `action`. Repeated keys step on from where
    /// the running glide is heading, not from where it has got to.
    pub(super) fn scroll(&self, clock: &gtk::ApplicationWindow, action: KeyAction) -> bool {
        let Some(adjustment) = self.scroll.borrow().clone() else {
            return false;
        };
        let from = match &*self.scroll_glide.borrow() {
            Some((running, target)) if running.get() => *target,
            _ => adjustment.value(),
        };
        let Some(target) = scroll_target(&adjustment, from, action) else {
            return false;
        };
        let running = glide(clock, &adjustment, target);
        self.scroll_glide.replace(Some((running, target)));
        true
    }
}

fn scroll_target(adjustment: &gtk::Adjustment, from: f64, action: KeyAction) -> Option<f64> {
    let lower = adjustment.lower();
    let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
    let step = adjustment.step_increment().max(72.0);
    let page = (adjustment.page_size() * 0.85).max(step);
    let target = match action {
        KeyAction::ScrollUp => from - step,
        KeyAction::ScrollDown => from + step,
        KeyAction::ScrollPageUp => from - page,
        KeyAction::ScrollPageDown => from + page,
        KeyAction::ScrollHome => lower,
        KeyAction::ScrollEnd => upper,
        _ => return None,
    };
    Some(target.clamp(lower, upper))
}

fn character_action(character: char) -> Option<KeyAction> {
    match character.to_ascii_lowercase() {
        key @ ('c' | 'i' | 'm' | 'n' | 'p' | 'r') => Some(KeyAction::Home(key)),
        's' => Some(KeyAction::Switch),
        _ => None,
    }
}

pub(super) fn key_action(key: gdk::Key, modifiers: gdk::ModifierType) -> Option<KeyAction> {
    let clean_mods = modifiers
        & (gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SUPER_MASK
            | gdk::ModifierType::SHIFT_MASK);
    if clean_mods.intersects(
        gdk::ModifierType::ALT_MASK
            | gdk::ModifierType::CONTROL_MASK
            | gdk::ModifierType::SUPER_MASK,
    ) {
        return None;
    }
    match key {
        gdk::Key::Escape => Some(KeyAction::Close),
        gdk::Key::Left if clean_mods.is_empty() => Some(KeyAction::Back),
        gdk::Key::Right if clean_mods.is_empty() => Some(KeyAction::FocusNext),
        gdk::Key::Up if clean_mods.is_empty() => Some(KeyAction::ScrollUp),
        gdk::Key::Down if clean_mods.is_empty() => Some(KeyAction::ScrollDown),
        gdk::Key::Page_Up if clean_mods.is_empty() => Some(KeyAction::ScrollPageUp),
        gdk::Key::Page_Down if clean_mods.is_empty() => Some(KeyAction::ScrollPageDown),
        gdk::Key::Home if clean_mods.is_empty() => Some(KeyAction::ScrollHome),
        gdk::Key::End if clean_mods.is_empty() => Some(KeyAction::ScrollEnd),
        _ if clean_mods.is_empty() || clean_mods == gdk::ModifierType::SHIFT_MASK => {
            key.to_unicode().and_then(character_action)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;

    #[test]
    fn maps_vim_and_action_shortcuts() {
        assert_eq!(character_action('j'), None);
        assert_eq!(character_action('k'), None);
        assert_eq!(character_action('h'), None);
        assert_eq!(character_action('g'), None);
        assert_eq!(character_action('G'), None);
        assert_eq!(character_action('n'), Some(KeyAction::Home('n')));
        assert_eq!(character_action('N'), Some(KeyAction::Home('n')));
        assert_eq!(character_action('m'), Some(KeyAction::Home('m')));
        assert_eq!(character_action('M'), Some(KeyAction::Home('m')));
        assert_eq!(character_action('i'), Some(KeyAction::Home('i')));
        assert_eq!(character_action('I'), Some(KeyAction::Home('i')));
        assert_eq!(character_action('p'), Some(KeyAction::Home('p')));
        assert_eq!(character_action('r'), Some(KeyAction::Home('r')));
        assert_eq!(character_action('R'), Some(KeyAction::Home('r')));
        assert_eq!(character_action('c'), Some(KeyAction::Home('c')));
        assert_eq!(character_action('C'), Some(KeyAction::Home('c')));
        assert_eq!(character_action('s'), Some(KeyAction::Switch));
        assert_eq!(character_action('x'), None);
    }

    #[test]
    fn back_button_action_can_clear_actions_without_double_borrow() {
        if gtk::init().is_err() {
            return;
        }
        let state = Rc::new(UiState::default());
        let button = gtk::Button::new();
        let action_state = Rc::clone(&state);
        button.connect_clicked(move |_| {
            action_state.clear_actions();
        });
        state.set_back_button(&button);

        let button = state.back_button.borrow().clone();
        if let Some(button) = button {
            button.emit_clicked();
        }
        assert!(state.back_button.borrow().is_none());
    }
}
