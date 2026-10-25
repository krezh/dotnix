use std::time::Duration;

use super::{
    Media,
    mpris::{Control, Track},
};
use crate::{
    components::card,
    features::PanelCx,
    panel::{Block, Header, PanelView, PlayerView},
};

impl Media {
    pub(super) fn panel_view(&self, state: &Option<Track>, p: &PanelCx<'_>) -> Option<PanelView> {
        let glyph = |key: &str| p.glyph(key);
        let block = match state {
            Some(track) => {
                let position = track.position_now();
                if track.playing && track.length.is_none_or(|length| position < length) {
                    p.redraw_in(Duration::from_secs_f64(1.0 - position.fract()));
                }
                Block::Player(PlayerView {
                    art: track.art.clone(),
                    title: track.title.clone().into(),
                    artist: track.artist.clone().into(),
                    album: track.album.clone().into(),
                    source: track.identity.clone().into(),
                    position,
                    length: track.length,
                    play: glyph(if track.playing { "pause" } else { "play" }),
                    previous: glyph("previous"),
                    next: glyph("next"),
                    on_play: p.call(|media: &Media| media.send(Control::PlayPause)),
                    on_previous: p.call(|media: &Media| media.send(Control::Previous)),
                    on_next: p.call(|media: &Media| media.send(Control::Next)),
                    on_seek: track.length.map(|length| {
                        p.slider(move |media: &Media, value| {
                            media.send(Control::Seek(length * f64::from(value)))
                        })
                    }),
                })
            }
            None => Block::note("Nothing is playing"),
        };
        Some(PanelView {
            header: Header::Tabs,
            blocks: vec![Block::Card(None, card::Size::Large, vec![block])],
        })
    }
}
