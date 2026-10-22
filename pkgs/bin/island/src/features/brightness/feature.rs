//! How island sees brightness: its level, the OSD it shows, and `island ctl brightness`.

use super::{
    runtime::{self, Brightness},
    state::{self, Level},
};
use crate::{
    control::{Command, Group, LEVEL, Report, Row, Takes},
    features::{
        Feature, Feed, GlyphSpec, IslandInfo, PanelCx,
        osd::{self, Osd, Shown},
    },
    panel::PanelView,
};

impl Feature for Brightness {
    const NAME: &'static str = "brightness";
    const GLYPHS: &'static [GlyphSpec] = &[("brightness", &["display-brightness-symbolic"])];
    const CONTROLS: &'static [Command] = &[Command {
        name: "brightness",
        about: "The screen's brightness",
        group: Group::SoundAndScreen,
        takes: Takes::Forms {
            forms: LEVEL,
            bare: Some("Show the brightness, once island has changed it"),
        },
        examples: &["island ctl brightness down 10"],
    }];

    type State = Level;
    type Local = Shown;
    type Event = ();

    fn start(feed: Feed<Level>) -> Self {
        Self::start(feed)
    }

    fn sync_local(_: &Level, shown: &mut Shown, island: &IslandInfo<'_>, _animate: bool) {
        shown.sync(island);
    }

    fn popup<'a>(&self, _: &Level, p: &PanelCx<'a>) -> Option<PanelView> {
        osd::popup(p)
    }

    fn elapse(&self, shown: &mut Shown, _hovered: bool) {
        shown.expire();
    }

    fn osd(before: &Level, after: &Level) -> Option<Osd> {
        state::osd(before, after)
    }

    fn command(&self, level: &Level, args: &[&str]) -> Report {
        self.command(level, args)
    }

    fn status(&self, level: &Level) -> Vec<Row> {
        vec![runtime::row(level, None)]
    }
}
