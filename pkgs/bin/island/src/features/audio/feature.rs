//! How island sees audio: the mixer's state, the volume OSD, the widget, the panel, and
//! `island ctl` volume commands.

use std::{cell::Cell, time::Duration};

use gpui::AnyElement;

use super::{commands, glyphs, panel, popup, pulse, runtime::Audio, state::AudioState, widget};
use crate::{
    control::{Command, Report, Row},
    features::{
        Feature, Feed, GlyphSpec, IslandInfo, PanelCx, WidgetCx, headset,
        osd::{self, Osd, Shown},
    },
    panel::PanelView,
};

/// The meters move this often while the panel is open.
const REFRESH: Duration = Duration::from_millis(33);

impl Feature for Audio {
    const NAME: &'static str = "audio";
    const GLYPHS: &'static [GlyphSpec] = glyphs::GLYPHS;
    const PANEL: Option<&'static str> = Some("audio");
    const TAB: Option<(&'static str, u8)> = Some(("Audio", 2));
    const PANEL_REFRESH: Option<Duration> = Some(REFRESH);
    const CONTROLS: &'static [Command] = commands::CONTROLS;

    type State = AudioState;
    type Local = Shown;
    type Event = ();

    fn start(feed: Feed<AudioState>) -> Self {
        Self {
            mixer: pulse::start(feed),
            scrolled: Cell::new(0.0),
            choosing: Cell::new(None),
        }
    }

    fn sync_local(_: &AudioState, shown: &mut Shown, island: &IslandInfo<'_>, _animate: bool) {
        shown.sync(island);
    }

    fn popup<'a>(&self, _: &AudioState, p: &PanelCx<'a>) -> Option<PanelView> {
        osd::popup(p)
    }

    fn elapse(&self, shown: &mut Shown, _hovered: bool) {
        shown.expire();
    }

    /// Connected with no devices yet, so the widget waits for the first query instead of
    /// flashing the error state.
    fn initial() -> AudioState {
        AudioState {
            connected: true,
            ..AudioState::default()
        }
    }

    fn present(_: &str, state: &AudioState) -> bool {
        widget::present(state)
    }

    fn widget(&self, _: &str, state: &AudioState, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        widget::widget(state, w)
    }

    fn panel_opened(&self) {
        self.choosing.set(None);
        self.mixer.watch();
        headset::watch();
    }

    fn panel<'a>(&self, state: &AudioState, p: &PanelCx<'a>) -> Option<PanelView> {
        Some(panel::panel(self, state, p))
    }

    fn osd(before: &AudioState, after: &AudioState) -> Option<Osd> {
        popup::osd(before, after)
    }

    fn command(&self, state: &AudioState, args: &[&str]) -> Report {
        commands::run(self, state, args)
    }

    fn status(&self, state: &AudioState) -> Vec<Row> {
        commands::status(state)
    }
}
