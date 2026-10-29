//! How island sees Gotify: its `/stream` worker, posting each message to the notification
//! center, and `island ctl gotify`.

use super::{
    commands,
    runtime::{self, Gotify},
    settings,
    state::State,
};
use crate::{
    control::{Command, Report, Row},
    features::{Feature, Feed, GlyphSpec},
};

impl Feature for Gotify {
    const NAME: &'static str = "gotify";
    const CONTROLS: &'static [Command] = commands::CONTROLS;
    /// Marks its messages in the notification center.
    const GLYPHS: &'static [GlyphSpec] = &[("notice-gotify", &["radio-tower"])];

    type State = State;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<State>) -> Self {
        runtime::start(feed)
    }

    fn check_settings(settings: &toml::Value) -> Result<(), String> {
        settings::check(settings)
    }

    fn command(&self, state: &State, args: &[&str]) -> Report {
        self.handle_command(state, args)
    }

    fn status(&self, state: &State) -> Vec<Row> {
        commands::status(state)
    }
}
