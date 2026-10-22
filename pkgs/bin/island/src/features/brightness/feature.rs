//! How island sees brightness: its level, the OSD it shows, and `island ctl brightness`.

use super::{
    runtime::{self, Brightness},
    state::{self, Level},
};
use crate::{
    features::{
        Feature, Feed, GlyphSpec, IslandInfo, PanelCx,
        osd::{self, Osd, Shown},
    },
    panel::PanelView,
};

impl Feature for Brightness {
    const NAME: &'static str = "brightness";
    const GLYPHS: &'static [GlyphSpec] = &[("brightness", &["display-brightness-symbolic"])];
    const COMMANDS: &'static [&'static str] = &["brightness"];
    const USAGE: &'static str = "  brightness up|down|set PERCENT        screen";

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

    fn check(args: &[&str]) -> Result<(), String> {
        runtime::check(args)
    }

    fn command(&self, _: &Level, args: &[&str]) {
        self.command(args);
    }
}
