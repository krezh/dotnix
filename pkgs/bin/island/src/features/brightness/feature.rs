//! How island sees brightness: each screen's level, the OSD it shows, and `island ctl brightness`.

use super::{
    runtime::{self, Brightness},
    state::{self, Levels},
};
use crate::{
    control::{Command, Group, LEVEL, Reply, Row, Takes},
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
        about: "The screens' brightness",
        group: Group::SoundAndScreen,
        takes: Takes::Aimed {
            at: "monitor",
            forms: LEVEL,
            bare: Some("Show the brightness"),
        },
        examples: &[
            "island ctl brightness down 10",
            "island ctl brightness focused up",
            "island ctl brightness DP-1 set 40",
        ],
    }];

    type State = Levels;
    type Local = Shown;
    type Event = ();

    fn start(feed: Feed<Levels>) -> Self {
        Self::start(feed)
    }

    fn sync_local(_: &Levels, shown: &mut Shown, island: &IslandInfo<'_>, _animate: bool) {
        shown.sync(island);
    }

    fn popup<'a>(&self, _: &Levels, p: &PanelCx<'a>) -> Option<PanelView> {
        osd::popup(p)
    }

    fn elapse(&self, shown: &mut Shown, _hovered: bool) {
        shown.expire();
    }

    fn osd(before: &Levels, after: &Levels) -> Option<Osd> {
        state::osd(before, after)
    }

    fn reply(&self, _: &Levels, args: &[&str]) -> Reply {
        self.reply(args)
    }

    fn status(&self, levels: &Levels) -> Vec<Row> {
        runtime::status(levels)
    }
}
