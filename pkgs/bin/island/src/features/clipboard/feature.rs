//! How island sees the clipboard: its history, the widget, the picker panel and its keys, and
//! `island ctl clipboard clear`.

use std::time::Duration;

use gpui::{AnyElement, Keystroke};

use super::{commands, panel, runtime, runtime::Clipboard, settings, state::State, widget};
use crate::{
    components::list::Selection,
    control::{Command, Report},
    features::{Feature, Feed, GlyphSpec, Outcome, PanelCx, WidgetCx},
    panel::{IconButton, PanelView},
};

impl Feature for Clipboard {
    const NAME: &'static str = "clipboard";
    const GLYPHS: &'static [GlyphSpec] = &[
        ("clipboard", &["edit-paste-symbolic", "edit-paste"]),
        ("clipboard-text", &["text-x-generic"]),
        ("clipboard-image", &["image-x-generic"]),
        ("clipboard-files", &["folder", "inode-directory"]),
    ];
    const PANEL: Option<&'static str> = Some("clipboard");
    const PANEL_REFRESH: Option<Duration> = Some(Duration::from_secs(30));
    const CONTROLS: &'static [Command] = commands::CONTROLS;
    const KEYBOARD: bool = true;

    type State = State;
    type Local = Selection;
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        runtime::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn widget(&self, _: &str, _: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        widget::widget(w)
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        panel::panel(state, p)
    }

    fn key(&self, state: &State, picker: &mut Selection, keystroke: &Keystroke) -> Outcome {
        self.handle_key(state, picker, keystroke)
    }

    fn system_button(&self, _: &State, p: &PanelCx<'_>) -> Option<IconButton> {
        widget::system_button(p)
    }

    fn command(&self, _: &State, args: &[&str]) -> Report {
        self.handle_command(args)
    }
}
