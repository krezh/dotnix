//! Which elements the pointer is over or holding, by id, for parts that change with it, such as a
//! slider's handle or a name that scrolls while hovered.

use std::{cell::RefCell, collections::HashMap};

use gpui::{SharedString, Window};

#[derive(Clone, Copy, Default)]
pub struct Pointer {
    pub hovered: bool,
    pub held: bool,
}

thread_local! {
    static POINTERS: RefCell<HashMap<SharedString, Pointer>> = RefCell::default();
}

pub fn get(id: &SharedString) -> Pointer {
    POINTERS.with_borrow(|pointers| pointers.get(id).copied().unwrap_or_default())
}

/// Redraws, so what depends on it changes at once.
pub fn set(id: &SharedString, change: impl FnOnce(&mut Pointer), window: &mut Window) {
    POINTERS.with_borrow_mut(|pointers| change(pointers.entry(id.clone()).or_default()));
    window.refresh();
}
