//! What is playing: cover art, title, artist and album, a seekable position, and play controls.

use std::path::PathBuf;

use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};

use crate::{
    components::{
        button, card, icon,
        marquee::{TextLayouts, marquee},
        motion,
        slider::{OnChange, slider},
    },
    island::Island,
    panel::Handler,
    theme::{ACCENT, CONTROL_ALPHA, ON_ACCENT, SUBTEXT1, SURFACE1, TEXT, with_alpha},
    ui::{self, CARD_PAD, Radius, SMALL_SIZE, TEXT_SIZE, TITLE_SIZE},
};

const ART: f32 = 112.0;
const ART_GAP: f32 = 16.0;

pub struct PlayerView {
    pub art: Option<PathBuf>,
    pub title: SharedString,
    pub artist: SharedString,
    pub album: SharedString,
    pub source: SharedString,
    pub position: f64,
    pub length: Option<f64>,
    pub play: Option<PathBuf>,
    pub previous: Option<PathBuf>,
    pub next: Option<PathBuf>,
    pub on_play: Handler,
    pub on_previous: Handler,
    pub on_next: Handler,
    /// `None` when the track has no length.
    pub on_seek: Option<OnChange>,
}

fn clock_text(seconds: f64) -> String {
    let seconds = seconds.max(0.0) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

pub fn player(view: PlayerView, layouts: &TextLayouts, hover: bool, cx: &mut Context<Island>) -> AnyElement {
    let text_w = card::Size::Large.width() - 2.0 * CARD_PAD - ART - ART_GAP;
    let line = |text: SharedString, size: f32, weight: FontWeight| marquee(layouts, text, size, weight, text_w);
    let art = div()
        .size(px(ART))
        .flex_none()
        .rounded(px(ui::PANEL_RADIUS))
        .overflow_hidden()
        .bg(rgba(with_alpha(SURFACE1, CONTROL_ALPHA)))
        .when_some(view.art.clone(), |art, path| {
            art.child(gpui::img(path).size_full().object_fit(gpui::ObjectFit::Cover))
        });
    let control = |id: &'static str, glyph: Option<PathBuf>, size: f32, primary: bool, action: Handler, cx: &mut Context<Island>| {
        let (rest, tint) = if primary { (Some(rgb(ACCENT).into()), ON_ACCENT) } else { (None, TEXT) };
        button::fill(id, Radius::Pill, rest, hover)
            .size(px(size))
            .flex_none()
            .when_some(glyph, |control, path| control.child(icon::glyph(&path, size * 0.45, tint)))
            .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
    };
    let progress = view.length.map(|length| {
        let fraction = (view.position / length.max(1.0)) as f32;
        div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(slider("media-progress", fraction, ACCENT, view.on_seek.clone(), cx))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_size(px(SMALL_SIZE))
                    .text_color(rgb(SUBTEXT1))
                    .child(motion::text("media-position", clock_text(view.position)))
                    .child(motion::text("media-length", clock_text(length))),
            )
    });
    div()
        .flex()
        .gap(px(ART_GAP))
        .child(art)
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(div().text_size(px(SMALL_SIZE)).text_color(rgb(SUBTEXT1)).child(view.source))
                        .child(line(view.title, TITLE_SIZE, FontWeight::SEMIBOLD))
                        .child(line(view.artist, TEXT_SIZE, FontWeight::MEDIUM))
                        .when(!view.album.is_empty(), |text| text.child(line(view.album, SMALL_SIZE, FontWeight::MEDIUM))),
                )
                .children(progress)
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .items_center()
                        .gap(px(12.0))
                        .child(control("media-previous", view.previous, 36.0, false, view.on_previous, cx))
                        .child(control("media-play", view.play, 44.0, true, view.on_play, cx))
                        .child(control("media-next", view.next, 36.0, false, view.on_next, cx)),
                ),
        )
        .into_any_element()
}
