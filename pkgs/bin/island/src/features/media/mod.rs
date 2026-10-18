mod commands;
mod mpris;
mod panel;
mod runtime;
mod spectrum;
mod widget;

use std::time::Duration;

use gpui::AnyElement;

use super::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx};
use crate::panel::PanelView;
use mpris::Track;
use widget::{MEDIA, VISUALIZER};

pub use runtime::Media;

impl Feature for Media {
    const NAME: &'static str = "media";
    const GLYPHS: &'static [GlyphSpec] = &[
        ("play", &["media-playback-start-symbolic"]),
        ("pause", &["media-playback-pause-symbolic"]),
        ("next", &["media-skip-forward-symbolic"]),
        ("previous", &["media-skip-backward-symbolic"]),
    ];
    const WIDGETS: &'static [&'static str] = &[MEDIA, VISUALIZER];
    const LIVE: &'static [&'static str] = &[VISUALIZER];
    const PANEL: Option<&'static str> = Some("media");
    const TAB: Option<(&'static str, u8)> = Some(("Media", 1));
    const PANEL_REFRESH: Option<Duration> = Some(Duration::from_millis(500));
    const USAGE: &'static str = "  media play-pause|next|previous";

    type State = Option<Track>;
    type Local = ();
    type Event = ();

    fn start(feed: Feed<Option<Track>>) -> Self {
        Self::start(feed)
    }

    fn present(widget: &str, state: &Option<Track>) -> bool {
        widget::present(widget, state)
    }

    fn rest_width(widget: &str, spacing: f32) -> f32 {
        widget::rest_width(widget, spacing)
    }

    fn widget(
        &self,
        widget: &str,
        state: &Option<Track>,
        w: &mut WidgetCx<'_>,
    ) -> Option<AnyElement> {
        widget::widget(widget, state, w)
    }

    fn glow<'a>(state: &'a Option<Track>, live: &'a [f32]) -> Option<&'a [f32]> {
        widget::glow(state, live)
    }

    fn panel<'a>(&self, state: &Option<Track>, p: &PanelCx<'a>) -> Option<PanelView> {
        self.panel_view(state, p)
    }

    fn check(args: &[&str]) -> Result<(), String> {
        commands::check(args)
    }

    fn command(&self, _: &Option<Track>, args: &[&str]) {
        commands::run(self, args);
    }
}
