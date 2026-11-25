//! How island sees the clipboard: its history, the widget, the picker panel and its keys, and
//! `island ctl clipboard clear`.

use super::{
    commands, panel, picker::Picker, runtime, runtime::Clipboard, settings, state::State, widget,
};
use crate::bar_widget::BarWidget;
use crate::{
    control::{Command, Report},
    features::{Feature, Feed, GlyphSpec, Outcome, PanelCx, Surface, WidgetCx},
    key::Key,
    panel::PanelView,
};

impl Feature for Clipboard {
    const NAME: &'static str = "clipboard";

    const CONTROLS: &'static [Command] = commands::CONTROLS;

    type State = State;

    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        runtime::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn command(&self, _: &State, args: &[&str]) -> Report {
        self.handle_command(args)
    }
}

impl Surface for Clipboard {
    const GLYPHS: &'static [GlyphSpec] = &[
        ("clipboard", &["clipboard"]),
        ("clipboard-text", &["file-text"]),
        ("clipboard-link", &["link"]),
        ("clipboard-code", &["code"]),
        ("clipboard-color", &["palette"]),
        ("clipboard-image", &["image"]),
        ("clipboard-files", &["folder"]),
        ("clipboard-search", &["search"]),
        ("clipboard-site", &["globe"]),
        ("clipboard-paste", &["clipboard-paste"]),
        ("clipboard-copy", &["copy"]),
        ("clipboard-pin", &["pin"]),
        ("clipboard-forget", &["trash-2"]),
    ];

    const PANEL: Option<&'static str> = Some("clipboard");

    const KEYBOARD: bool = true;

    type Local = Picker;

    fn widget(&self, _: &str, _: &State, w: &mut WidgetCx<'_>) -> Option<BarWidget> {
        widget::widget(w)
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        panel::panel(state, p)
    }

    fn key(&self, state: &State, picker: &mut Picker, keystroke: &Key) -> Outcome {
        self.handle_key(state, picker, keystroke)
    }
}
