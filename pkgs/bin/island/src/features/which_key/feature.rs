//! A menu of keys, each running a command or opening a further menu, replacing wlr-which-key.

use std::sync::Arc;

use super::{panel, settings, settings::Settings, state::Local};
use crate::{
    features::{Feature, Feed, Outcome, PanelCx, Surface},
    key::Key,
    panel::PanelView,
};

#[derive(Clone)]
pub struct WhichKey {
    pub(super) settings: Arc<Settings>,
}

impl Feature for WhichKey {
    const NAME: &'static str = "which-key";

    type State = ();

    type Event = ();

    fn start(feed: Feed<()>) -> Self {
        Self {
            settings: Arc::new(feed.settings()),
        }
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }
}

impl Surface for WhichKey {
    const PANEL: Option<&'static str> = Some("which-key");

    const NAMED_VIEWS: bool = true;

    const KEYBOARD: bool = true;

    type Local = Local;

    fn panel<'a>(&self, _: &(), p: &PanelCx<'a>) -> Option<PanelView> {
        panel::panel(self, p)
    }

    fn key(&self, _: &(), local: &mut Local, key: &Key) -> Outcome {
        self.handle_key(local, key)
    }
}
