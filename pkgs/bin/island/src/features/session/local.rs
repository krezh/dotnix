use std::time::Instant;

use super::state::{App, Phase, State};
use crate::{
    features::{IslandInfo, Local},
    presence::Presence,
    stage::{SHAPE_SPRING, Stance},
};

/// Each island's part of the session ending: whether it shows the card, where it stands, and the
/// apps' rows easing out as they close.
pub struct Exit {
    /// Decided as the session starts to end, so the card stays on the screen it began on.
    holds: Option<bool>,
    stance: Stance,
    pub(super) rows: Presence<App>,
}

impl Default for Exit {
    fn default() -> Self {
        Self {
            holds: None,
            stance: Stance::Bar,
            rows: Presence::new(SHAPE_SPRING),
        }
    }
}

impl Local for Exit {
    fn advance(&mut self, dt: f32, now: Instant) {
        self.rows.advance(dt, now);
    }

    fn is_settled(&self) -> bool {
        self.rows.is_settled()
    }

    fn popup(&self) -> bool {
        self.holds == Some(true) && self.stance.centered()
    }

    fn stance(&self) -> Stance {
        self.stance
    }
}

pub(super) fn sync(state: &State, exit: &mut Exit, island: &IslandInfo<'_>, animate: bool) {
    let Some(leaving) = &state.leaving else {
        exit.holds = None;
        exit.stance = Stance::Bar;
        exit.rows.sync(&[], false);
        return;
    };
    let holds = *exit
        .holds
        .get_or_insert_with(|| island.monitor.is_none_or(|monitor| monitor.focused));
    exit.stance = match (&leaving.phase, holds) {
        (Phase::Off, _) => Stance::Off,
        (Phase::Closing, true) => Stance::Center {
            busy: Some(leaving.action.color()),
        },
        (_, true) => Stance::Center { busy: None },
        (_, false) => Stance::Backdrop,
    };
    let rows: Vec<(String, App)> = leaving
        .apps
        .iter()
        .map(|app| (app.key.clone(), app.clone()))
        .collect();
    exit.rows.sync(&rows, animate);
}
