//! Brightness: `island ctl brightness …` sets the screen's brightness, through logind for a
//! backlight or DDC/CI for an external monitor, and an OSD shows the new level. Neither tells of
//! changes made elsewhere, so only these show.

mod device;

use std::sync::mpsc::{self, Sender};

use tracing::warn;

use super::{
    Feature, Feed, GlyphSpec, IslandInfo, PanelCx,
    osd::{self, Osd, Shown},
};
use crate::{
    control::{self, Change},
    panel::PanelView,
    supervisor::Ended,
    theme::YELLOW,
};

/// The brightness after the latest change, and how many changes there have been, so a change that
/// leaves the level where it was, such as up at full, still shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Level {
    value: f32,
    changes: u64,
}

pub struct Brightness {
    changes: Sender<Change>,
}

/// The level after `change`, or `None` without a screen to set.
fn apply(change: Change) -> Option<f32> {
    match change {
        Change::Step(delta) => device::step(delta),
        Change::Set(value) => device::get().and_then(|current| device::step(value - current)),
        Change::ToggleMute => device::get(),
    }
}

impl Feature for Brightness {
    const NAME: &'static str = "brightness";
    const GLYPHS: &'static [GlyphSpec] = &[("brightness", &["display-brightness-symbolic"])];
    const COMMANDS: &'static [&'static str] = &["brightness"];
    const USAGE: &'static str = "  brightness up|down|set PERCENT        screen";

    type State = Level;
    type Local = Shown;
    type Event = ();

    fn start(feed: Feed<Level>) -> Self {
        let (changes, requests) = mpsc::channel();
        feed.supervise(move |feed| {
            let mut level = Level::default();
            for change in &requests {
                let Some(value) = apply(change) else {
                    warn!(?change, "cannot change the brightness: no backlight or DDC/CI monitor");
                    continue;
                };
                level = Level {
                    value,
                    changes: level.changes + 1,
                };
                if !feed.publish(level.clone()) {
                    return Ended::Unwanted;
                }
            }
            Ended::Unwanted
        });
        Self { changes }
    }

    fn sync_local(_: &Level, shown: &mut Shown, island: &IslandInfo<'_>, _animate: bool) {
        shown.sync(island);
    }

    fn popup<'a>(&self, _: &Level, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        osd::popup(p)
    }

    fn elapse(&self, shown: &mut Shown, _hovered: bool) {
        shown.expire();
    }

    fn osd(before: &Level, after: &Level) -> Option<Osd> {
        (after.changes != before.changes).then(|| Osd {
            glyph: "brightness",
            color: YELLOW,
            caption: "",
            label: "Brightness".to_owned(),
            level: Some((after.value, false)),
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        control::change(&args[1..], false).map(drop)
    }

    fn command(&self, _: &Level, args: &[&str]) {
        if let Ok(change) = control::change(&args[1..], false) {
            let _ = self.changes.send(change);
        }
    }
}
