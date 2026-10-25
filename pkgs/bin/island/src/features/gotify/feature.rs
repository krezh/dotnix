//! How island sees Gotify: its bar widget, its message panel, its `/stream` notification worker,
//! and `island ctl gotify`.

use gpui::AnyElement;

use super::{
    commands, panel,
    runtime::{self, Gotify},
    settings,
    state::State,
    widget,
};
use crate::{
    control::{Command, Report, Row},
    features::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx},
    panel::{IconButton, PanelView},
};

impl Feature for Gotify {
    const NAME: &'static str = "gotify";
    const PANEL: Option<&'static str> = Some("gotify");
    const CONTROLS: &'static [Command] = commands::CONTROLS;
    const GLYPHS: &'static [GlyphSpec] = &[(
        "gotify",
        &[
            "gotify",
            "mail-unread-symbolic",
            "notifications-symbolic",
            "preferences-system-notifications-symbolic",
        ],
    )];

    type State = State;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        runtime::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn present(_: &str, state: &State) -> bool {
        state.configured()
    }

    fn widget(&self, _: &str, state: &State, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        widget::widget(state, w)
    }

    fn panel<'a>(&self, state: &State, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(state, p))
    }

    fn system_button(&self, state: &State, p: &PanelCx<'_>) -> Option<IconButton> {
        widget::system_button(state, p)
    }

    fn command(&self, state: &State, args: &[&str]) -> Report {
        self.handle_command(state, args)
    }

    fn status(&self, state: &State) -> Vec<Row> {
        commands::status(state)
    }
}
