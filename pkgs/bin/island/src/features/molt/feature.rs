use crate::bar_widget::BarWidget;
use std::any::Any;

use super::{keys, panel, runtime::Molt, state::Updates, view::View, widget};
use crate::{
    features::{Feature, Feed, GlyphSpec, Outcome, PanelCx, Surface, WidgetCx},
    key::Key,
    panel::PanelView,
};

impl Feature for Molt {
    const NAME: &'static str = "molt";

    type State = Updates;

    type Event = ();

    fn start(feed: Feed<Updates>) -> Self {
        Molt::start(feed)
    }
}

impl Surface for Molt {
    const GLYPHS: &'static [GlyphSpec] = &[
        ("molt", &["package"]),
        ("molt-nixos", &["box"]),
        ("molt-home-manager", &["house"]),
        ("molt-repository", &["git-branch"]),
        ("molt-inputs", &["list"]),
        ("molt-search", &["search"]),
        ("molt-cleanup", &["trash-2"]),
        ("molt-watchlist", &["eye"]),
        ("molt-pin", &["pin"]),
        ("molt-track", &["plus"]),
        ("molt-package", &["package"]),
        ("molt-option", &["settings"]),
        ("molt-notes", &["file-text"]),
        ("molt-clean-nix", &["box"]),
        ("molt-clean-docker", &["container", "package"]),
        ("molt-clean-development", &["code"]),
        ("molt-clean-caches", &["folder"]),
        ("molt-clean-games", &["gamepad-2", "gamepad"]),
        ("molt-clean-trash", &["trash-2"]),
        ("molt-clean-journals", &["file-text", "scroll-text"]),
    ];

    const PANEL: Option<&'static str> = Some("molt");

    /// Worked in like molt's window rather than glanced at.
    const STAYS_OPEN: bool = true;

    const KEYBOARD: bool = true;

    type Local = View;

    /// So the panel opens on the latest without waiting for the network, and on a fresh Home.
    fn panel_opened(&self) -> Option<Box<dyn Any>> {
        self.look();
        self.open_home();
        None
    }

    fn widget(&self, _: &str, state: &Updates, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        widget::widget(state, w)
    }

    fn panel<'a>(&self, state: &Updates, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(state, p))
    }

    fn key(&self, state: &Updates, view: &mut View, key: &Key) -> Outcome {
        keys::key(self, state, view, key)
    }
}
