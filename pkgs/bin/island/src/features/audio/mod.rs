//! While the sound server is gone the widget stays, drawn as an error.

mod commands;
mod glyphs;
mod headset_card;
mod meters;
mod panel;
mod popup;
mod pulse;
mod query;
mod request;
mod state;
mod widget;

use std::{cell::Cell, time::Duration};

use gpui::AnyElement;

use super::{
    Feature, Feed, GlyphSpec, IslandInfo, PanelCx, WidgetCx, headset,
    osd::{self, Osd, Shown},
};
use crate::{island::Panel, panel::PanelView};
use request::{Mixer, Request};
use state::AudioState;

/// The meters move this often while the panel is open.
const REFRESH: Duration = Duration::from_millis(33);

pub struct Audio {
    mixer: Mixer,
    /// Scroll left over below a whole notch.
    scrolled: Cell<f32>,
    /// The app whose device list is open.
    choosing: Cell<Option<u32>>,
}

impl Audio {
    fn send(&self, request: Request) {
        self.mixer.send(request);
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}

impl Feature for Audio {
    const NAME: &'static str = "audio";
    const GLYPHS: &'static [GlyphSpec] = glyphs::GLYPHS;
    const PANEL: Option<&'static str> = Some("audio");
    const TAB: Option<(&'static str, u8)> = Some(("Audio", 2));
    const PANEL_REFRESH: Option<Duration> = Some(REFRESH);
    const COMMANDS: &'static [&'static str] = commands::COMMANDS;
    const USAGE: &'static str = commands::USAGE;

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

    fn check(args: &[&str]) -> Result<(), String> {
        commands::check(args)
    }

    fn command(&self, _: &AudioState, args: &[&str]) {
        commands::run(self, args);
    }
}
