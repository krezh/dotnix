mod commands;
mod desktop;
mod history;
mod index;
mod panel;
mod runtime;
mod settings;
mod state;

use gpui::Keystroke;

use super::{Feature, Feed, IslandInfo, Outcome, PanelCx};
use crate::panel::PanelView;

pub use runtime::Launcher;
use state::{Apps, Local};

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
