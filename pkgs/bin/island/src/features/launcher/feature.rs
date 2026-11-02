//! How island sees the launcher: its apps, its panel, and the keys typed into it.

use std::any::Any;

use super::{
    panel, runtime,
    runtime::Launcher,
    settings,
    state::{Apps, Local},
};
use crate::{
    features::{Feature, Feed, IslandInfo, Outcome, PanelCx, Surface},
    key::Key,
    panel::PanelView,
};

impl Feature for Launcher {
    const NAME: &'static str = "launcher";

    type State = Apps;

    type Event = ();

    fn start(feed: Feed<Apps>) -> Self {
        runtime::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }
}

impl Surface for Launcher {
    const PANEL: Option<&'static str> = Some("launcher");

    const KEYBOARD: bool = true;

    type Local = Local;

    fn sync_local(apps: &Apps, local: &mut Local, _: &IslandInfo<'_>, _: bool) {
        local.sync(apps);
    }

    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        self.refresh();
        None
    }

    fn panel<'a>(&self, _: &Apps, p: &PanelCx<'a>) -> Option<PanelView> {
        panel::panel(p)
    }

    fn key(&self, _: &Apps, local: &mut Local, keystroke: &Key) -> Outcome {
        self.handle_key(local, keystroke)
    }
}
