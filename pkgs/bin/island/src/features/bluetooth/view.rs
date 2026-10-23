//! Each island's own part of the Bluetooth tab: which connected device's menu is open.

use crate::features::Local;

#[derive(Default)]
pub struct View {
    /// The device whose menu is open, by its BlueZ path.
    pub menu: Option<String>,
}

impl Local for View {
    fn panel_closed(&mut self) {
        self.menu = None;
    }
}
