//! How island sees the launcher: its apps, its panel, and the keys typed into it.

use gpui::Keystroke;

use super::{
    panel, runtime,
    runtime::Launcher,
    settings,
    state::{Apps, Local},
};
use crate::{
    features::{Feature, Feed, IslandInfo, Outcome, PanelCx},
    panel::PanelView,
};

impl Feature for Launcher {
    const NAME: &'static str = "launcher";
    const PANEL: Option<&'static str> = Some("launcher");
    const KEYBOARD: bool = true;

    type State = Apps;
    type Local = Local;
    type Event = ();

    fn start(feed: Feed<Apps>) -> Self {
        runtime::start(feed)
    }

    fn sync_local(apps: &Apps, local: &mut Local, _: &IslandInfo<'_>, _: bool) {
        local.sync(apps);
    }

    fn panel_opened(&self) {
        self.refresh();
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn panel<'a>(&self, _: &Apps, p: &PanelCx<'a>) -> Option<PanelView> {
        panel::panel(p)
    }

    fn key(&self, _: &Apps, local: &mut Local, keystroke: &Keystroke) -> Outcome {
        self.handle_key(local, keystroke)
    }
}
