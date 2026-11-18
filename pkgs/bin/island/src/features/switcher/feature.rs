//! How island sees the switcher: the compositor's windows and pictures of them, its panel, and the
//! keys pressed in it.

use std::any::Any;

use super::{
    panel,
    runtime::{self, Switcher},
    state::{Local, Previews},
};
use crate::{
    features::{Feature, Feed, IslandInfo, Outcome, PanelCx, Surface},
    key::{Key, Modifier},
    panel::PanelView,
};

impl Feature for Switcher {
    const NAME: &'static str = "switcher";

    type State = Previews;

    type Event = ();

    fn start(feed: Feed<Previews>) -> Self {
        runtime::start(feed)
    }
}

impl Surface for Switcher {
    const PANEL: Option<&'static str> = Some("switcher");

    const KEYBOARD: bool = true;

    const VIEWS: &'static [&'static str] = &["next", "prev", "same"];

    type Local = Local;

    fn sync_local(previews: &Previews, local: &mut Local, island: &IslandInfo<'_>, _: bool) {
        local.sync(previews, island.snapshot);
    }

    /// Keeps the pictures live while the switcher shows.
    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        Some(Box::new(self.watch()))
    }

    /// Also tells the worker which windows are in view, the only ones kept live.
    fn panel<'a>(&self, previews: &Previews, p: &PanelCx<'a>) -> Option<PanelView> {
        if let Some(local) = p.local::<Local>() {
            self.in_view(local.in_view());
        }
        panel::panel(previews, p)
    }

    fn key(&self, _: &Previews, local: &mut Local, key: &Key) -> Outcome {
        self.handle_key(local, key)
    }

    fn released(&self, _: &Previews, local: &mut Local, modifier: Modifier) -> Outcome {
        self.handle_release(local, modifier)
    }
}
