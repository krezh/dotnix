use std::rc::Rc;

use futures::channel::oneshot;
use gpui::{Context, Pixels};
use tracing::{debug, warn};

use super::{Tray, sni::Command};
use crate::island::Island;

impl Tray {
    pub(super) fn open_menu(
        &self,
        key: String,
        anchor: Pixels,
        island: &mut Island,
        cx: &mut Context<Island>,
    ) {
        if island.menu_open_for(&key) {
            island.close_menu(cx);
            return;
        }
        debug!(item = key, "opening the tray menu");
        let (reply, response) = oneshot::channel();
        self.send(Command::Menu(key.clone(), reply));
        let tray = self.clone();
        cx.spawn(async move |island, cx| {
            let root = match response.await {
                Ok(Some(root)) => root,
                Ok(None) => return,
                Err(_) => {
                    warn!(item = key, "the tray worker dropped the menu request");
                    return;
                }
            };
            if root.children.is_empty() {
                debug!(item = key, "the tray menu is empty");
                return;
            }
            debug!(
                item = key,
                entries = root.children.len(),
                "showing the tray menu"
            );
            let item = key.clone();
            let pick = Rc::new(move |id, _: &mut Island, _: &mut Context<Island>| {
                tray.send(Command::Click(item.clone(), id))
            });
            let _ = island.update(cx, |island, cx| {
                island.show_menu(key, root, pick, anchor, cx)
            });
        })
        .detach();
    }
}
