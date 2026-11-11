use super::menu::DEFAULT;

/// Which menu this island shows and the keys pressed inside it so far.
#[derive(Default)]
pub struct Local {
    pub(super) menu: Option<String>,
    pub(super) keys: Vec<String>,
}

impl Local {
    pub(super) fn menu(&self) -> &str {
        self.menu.as_deref().unwrap_or(DEFAULT)
    }

    pub(super) fn enter(&mut self, key: &str) {
        self.keys.push(key.to_owned());
    }

    /// Back up one level; false when already at the top.
    pub(super) fn back(&mut self) -> bool {
        self.keys.pop().is_some()
    }
}

impl crate::features::Local for Local {
    fn panel_closed(&mut self) {
        self.menu = None;
        self.keys.clear();
    }

    fn show(&mut self, view: &str) {
        self.menu = Some(view.to_owned());
        self.keys.clear();
    }
}
