use futures::channel::oneshot;
use tracing::debug;

use super::{Tray, sni::Command};
use crate::panel::Ui;

impl Tray {
    pub(super) fn open_menu(&self, key: String, anchor: f32, ui: &mut dyn Ui) {
        if ui.menu_open_for(&key) {
            ui.close_menu();
            return;
        }
        debug!(item = key, "opening the tray menu");
        let (reply, response) = oneshot::channel();
        self.send(Command::Menu(key.clone(), reply));
        let (tray, item) = (self.clone(), key.clone());
        ui.menu_when_ready(
            key,
            response,
            anchor,
            Box::new(move |id| tray.send(Command::Click(item.clone(), id))),
        );
    }
}
