use gpui::{AnyElement, FontWeight, MouseButton, div, prelude::*, px, rgb};

use super::{
    Media,
    mpris::{Control, Track},
    spectrum,
};
use crate::{
    bar::{self, GLYPH, ICON_GAP},
    components::{icon, tooltip::Tip},
    features::WidgetCx,
    theme::{ACCENT, OVERLAY2},
    ui::SMALL_SIZE,
};

const MEDIA_W: f32 = 180.0;
const SPECTRUM_W: f32 = 3.0;
const SPECTRUM_GAP: f32 = 2.0;
const SPECTRUM_H: f32 = 16.0;
pub(super) const MEDIA: &str = "media";
pub(super) const VISUALIZER: &str = "visualizer";

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

impl Media {
    /// Without a track it rests as a dimmed play icon, which the title unfolds out of when
    /// something starts playing.
    fn now_playing(track: Option<&Track>, w: &WidgetCx<'_>) -> AnyElement {
        let glyph = w.glyph(if track.is_some_and(|track| track.playing) {
            "pause"
        } else {
            "play"
        });
        let tint = if track.is_some() { ACCENT } else { OVERLAY2 };
        let slot = MEDIA_W - 2.0 * w.padding() - GLYPH - ICON_GAP;
        w.base("bar-media")
            .gap(px(ICON_GAP))
            .when(track.is_some(), |widget| {
                widget.w(px(MEDIA_W)).justify_start()
            })
            .text_size(px(SMALL_SIZE))
            .font_weight(FontWeight::SEMIBOLD)
            .tip(
                w.tips(),
                "bar-media",
                track.map_or_else(|| "Nothing playing".to_owned(), tip),
            )
            .when_some(glyph, |widget, path| {
                widget.child(div().flex_none().child(icon::glyph(path, GLYPH, tint)))
            })
            .when_some(track, |widget, track| {
                widget.child(div().flex_1().min_w(px(0.0)).child(w.marquee(
                    track.title.clone(),
                    SMALL_SIZE,
                    FontWeight::SEMIBOLD,
                    slot,
                )))
            })
            .on_click(w.toggle(Self::panel_handle()))
            .on_mouse_down(
                MouseButton::Middle,
                w.call(|media: &Media| media.send(Control::PlayPause)),
            )
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

pub(super) fn present(widget: &str, state: &Option<Track>) -> bool {
    match widget {
        VISUALIZER => playing(state),
        _ => state.is_some(),
    }
}

pub(super) fn rest_width(widget: &str, spacing: f32) -> f32 {
    match widget {
        MEDIA => GLYPH + 2.0 * bar::half(spacing),
        _ => 0.0,
    }
}

pub(super) fn widget(widget: &str, state: &Option<Track>, w: &WidgetCx<'_>) -> Option<AnyElement> {
    match widget {
        VISUALIZER => playing(state).then(|| Media::spectrum(w)),
        _ => Some(Media::now_playing(state.as_ref(), w)),
    }
}

pub(super) fn glow<'a>(state: &'a Option<Track>, live: &'a [f32]) -> Option<&'a [f32]> {
    playing(state).then_some(live)
}
