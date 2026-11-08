//! What is playing: cover art, title, artist and album, a seekable position, and play controls.

use std::path::PathBuf;

use crate::panel::SharedString;

use crate::{components::slider::OnChange, panel::Handler};

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
