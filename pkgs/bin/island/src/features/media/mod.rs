mod mpris;
mod spectrum;

use std::time::Duration;

use gpui::{AnyElement, FontWeight, MouseButton, div, prelude::*, px, rgb};

use super::{Feature, Feed, GlyphSpec, PanelCx, WidgetCx};
use crate::{
    bar::{self, GLYPH, ICON_GAP},
    components::{card, icon, tooltip::Tip},
    island::Panel,
    panel::{Block, Header, MediaView, PanelView},
    theme::{ACCENT, OVERLAY2},
    ui::SMALL_SIZE,
};
use mpris::{Control, Player, Track};

const MEDIA_W: f32 = 180.0;
const SPECTRUM_W: f32 = 3.0;
const SPECTRUM_GAP: f32 = 2.0;
const SPECTRUM_H: f32 = 16.0;
const MEDIA: &str = "media";
const VISUALIZER: &str = "visualizer";

fn playing(state: &Option<Track>) -> bool {
    state.as_ref().is_some_and(|track| track.playing)
}

fn tip(track: &Track) -> String {
    if track.artist.is_empty() {
        format!("{} · {}", track.title, track.identity)
    } else {
        format!("{} · {} · {}", track.title, track.artist, track.identity)
    }
}

pub struct Media {
    player: Option<Player>,
}

impl Media {
    fn send(&self, control: Control) {
        if let Some(player) = &self.player {
            player.send(control);
        }
    }

    fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }

    /// Without a track it rests as a dimmed play icon, which the title unfolds out of when
    /// something starts playing.
    fn now_playing(track: Option<&Track>, w: &WidgetCx<'_>) -> AnyElement {
        let glyph = w.glyph(if track.is_some_and(|track| track.playing) { "pause" } else { "play" });
        let tint = if track.is_some() { ACCENT } else { OVERLAY2 };
        let slot = MEDIA_W - 2.0 * w.padding() - GLYPH - ICON_GAP;
        w.base("bar-media")
            .gap(px(ICON_GAP))
            .when(track.is_some(), |widget| widget.w(px(MEDIA_W)).justify_start())
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .tip(w.tips(), "bar-media", track.map_or_else(|| "Nothing playing".to_owned(), tip))
            .when_some(glyph, |widget, path| widget.child(div().flex_none().child(icon::glyph(path, GLYPH, tint))))
            .when_some(track, |widget, track| {
                widget.child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .child(w.marquee(track.title.clone(), SMALL_SIZE, FontWeight::SEMIBOLD, slot)),
                )
            })
            .on_click(w.toggle(Self::panel_handle()))
            .on_mouse_down(MouseButton::Middle, w.call(|media: &Media| media.send(Control::PlayPause)))
            .into_any_element()
    }

    fn spectrum(w: &WidgetCx<'_>) -> AnyElement {
        let level = |index: usize| w.live().get(index).copied().unwrap_or(0.0);
        w.base("bar-visualizer")
            .gap(px(SPECTRUM_GAP))
            .children((0..spectrum::BARS).map(|index| {
                div()
                    .flex_none()
                    .w(px(SPECTRUM_W))
                    .h(px(SPECTRUM_W.max(SPECTRUM_H * level(index))))
                    .rounded_full()
                    .bg(rgb(ACCENT))
            }))
            .on_click(w.toggle(Self::panel_handle()))
            .into_any_element()
    }
}

fn control_of(word: &str) -> Option<Control> {
    match word {
        "play-pause" => Some(Control::PlayPause),
        "next" => Some(Control::Next),
        "previous" => Some(Control::Previous),
        _ => None,
    }
}

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
        if feed.wants_live() {
            spectrum::start(feed.clone());
        }
        Self {
            player: mpris::start(feed),
        }
    }

    fn present(widget: &str, state: &Option<Track>) -> bool {
        match widget {
            VISUALIZER => playing(state),
            _ => state.is_some(),
        }
    }

    fn rest_width(widget: &str, spacing: f32) -> f32 {
        match widget {
            MEDIA => GLYPH + 2.0 * bar::half(spacing),
            _ => 0.0,
        }
    }

    fn widget(&self, widget: &str, state: &Option<Track>, w: &mut WidgetCx<'_>) -> Option<AnyElement> {
        match widget {
            VISUALIZER => playing(state).then(|| Self::spectrum(w)),
            _ => Some(Self::now_playing(state.as_ref(), w)),
        }
    }

    fn glow<'a>(state: &'a Option<Track>, live: &'a [f32]) -> Option<&'a [f32]> {
        playing(state).then_some(live)
    }

    fn panel<'a>(&self, state: &Option<Track>, p: &PanelCx<'a>) -> Option<PanelView<'a>> {
        let glyph = |key: &str| p.glyph(key);
        let block = match state {
            Some(track) => Block::Media(MediaView {
                art: track.art.clone(),
                title: track.title.clone().into(),
                artist: track.artist.clone().into(),
                album: track.album.clone().into(),
                source: track.identity.clone().into(),
                position: track.position_now(),
                length: track.length,
                play: glyph(if track.playing { "pause" } else { "play" }),
                previous: glyph("previous"),
                next: glyph("next"),
                on_play: p.call(|media: &Media| media.send(Control::PlayPause)),
                on_previous: p.call(|media: &Media| media.send(Control::Previous)),
                on_next: p.call(|media: &Media| media.send(Control::Next)),
                on_seek: track.length.map(|length| {
                    p.slider(move |media: &Media, value| media.send(Control::Seek(length * f64::from(value))))
                }),
            }),
            None => Block::Note("Nothing is playing".into()),
        };
        Some(PanelView {
            header: Header::Tabs,
            blocks: vec![Block::Card(None, card::Size::Large, vec![block])],
        })
    }

    fn check(args: &[&str]) -> Result<(), String> {
        match args {
            [_, word] if control_of(word).is_some() => Ok(()),
            _ => Err("expected play-pause, next or previous".to_owned()),
        }
    }

    fn command(&self, _: &Option<Track>, args: &[&str]) {
        if let Some(control) = args.get(1).and_then(|word| control_of(word)) {
            self.send(control);
        }
    }
}
