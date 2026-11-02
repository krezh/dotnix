//! How island sees media: the track playing, the widget and visualizer, the panel, and
//! `island ctl media`.

use super::{
    commands,
    mpris::Track,
    runtime::Media,
    widget::{self, MEDIA, VISUALIZER},
};
use crate::bar_widget::BarWidget;
use crate::{
    control::{Command, Report, Row},
    features::{Feature, Feed, GlyphSpec, PanelCx, Surface, WidgetCx},
    panel::PanelView,
};

impl Feature for Media {
    const NAME: &'static str = "media";

    const CONTROLS: &'static [Command] = commands::CONTROLS;

    type State = Option<Track>;

    type Event = ();

    fn start(feed: Feed<Option<Track>>) -> Self {
        Self::start(feed)
    }

    fn command(&self, track: &Option<Track>, args: &[&str]) -> Report {
        commands::run(self, track, args)
    }

    fn status(&self, track: &Option<Track>) -> Vec<Row> {
        vec![commands::row(track)]
    }
}

impl Surface for Media {
    const GLYPHS: &'static [GlyphSpec] = &[
        ("tab-media", &["music"]),
        ("play", &["play"]),
        ("pause", &["pause"]),
        ("next", &["skip-forward"]),
        ("previous", &["skip-back"]),
    ];

    const WIDGETS: &'static [&'static str] = &[MEDIA, VISUALIZER];

    const LIVE: &'static [&'static str] = &[VISUALIZER];

    const PANEL: Option<&'static str> = Some("media");

    const TAB: Option<(&'static str, u8)> = Some(("Media", 2));

    type Local = ();

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
    ) -> Option<BarWidget> {
        widget::widget(widget, state, w)
    }

    fn glow<'a>(state: &'a Option<Track>, live: &'a [f32]) -> Option<&'a [f32]> {
        widget::glow(state, live)
    }

    fn panel<'a>(&self, state: &Option<Track>, p: &PanelCx<'a>) -> Option<PanelView> {
        self.panel_view(state, p)
    }
}
