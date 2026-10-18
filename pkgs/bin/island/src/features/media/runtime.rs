use super::{
    mpris::{self, Control, Player, Track},
    spectrum,
};
use crate::{
    features::{Feature, Feed},
    island::Panel,
};

pub struct Media {
    player: Option<Player>,
}

impl Media {
    pub(super) fn send(&self, control: Control) {
        if let Some(player) = &self.player {
            player.send(control);
        }
    }

    pub(super) fn panel_handle() -> Panel {
        Panel::Feature(<Self as Feature>::NAME)
    }

    pub(super) fn start(feed: Feed<Option<Track>>) -> Self {
        if feed.wants_live() {
            spectrum::start(feed.clone());
        }
        Self {
            player: mpris::start(feed),
        }
    }
}
