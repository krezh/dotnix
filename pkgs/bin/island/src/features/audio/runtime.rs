//! The audio feature itself: its line to the mixer, and what its panel remembers between frames.

use std::cell::Cell;

use super::{
    request::{Mixer, Request},
    state::Target,
};
use crate::{features::Feature, island::Panel};

pub struct Audio {
    pub(super) mixer: Mixer,
    /// Scroll left over below a whole notch.
    pub(super) scrolled: Cell<f32>,
    /// The app whose device list is open.
    pub(super) choosing: Cell<Option<u32>>,
    /// The default device list open on the dashboard.
    pub(super) picking: Cell<Option<Target>>,
}

impl Audio {
    pub(super) fn send(&self, request: Request) {
        self.mixer.send(request);
    }

    pub(super) fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }
}
